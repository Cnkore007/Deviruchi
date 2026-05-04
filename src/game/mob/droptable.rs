//! 怪物掉落表系统
//!
//! 支持 YAML 格式的掉落表配置，包含普通掉落和 MVP 奖励掉落。

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use uuid::Uuid;

use crate::game::map::drop_item::DropItem;
use crate::game::rand::GameRng;

/// 掉落表条目
#[derive(Debug, Clone)]
pub struct DropTableEntry {
    pub item_id: u32,
    pub min_amount: u16,
    pub max_amount: u16,
    pub chance: u32, // basis points 0-10000
    pub is_zeny: bool,
    pub is_mvp_bonus: bool,
}

/// 怪物掉落表
#[derive(Debug, Clone)]
pub struct MobDropTable {
    entries: Vec<DropTableEntry>,
}

impl MobDropTable {
    /// 获取所有掉落条目
    pub fn entries(&self) -> &[DropTableEntry] {
        &self.entries
    }

    /// 创建新的掉落表
    pub fn new(entries: Vec<DropTableEntry>) -> Self {
        Self { entries }
    }
}

/// YAML 格式的掉落条目
#[derive(Deserialize, Debug)]
struct YamlDropEntry {
    item_id: u32,
    min_amount: u16,
    max_amount: u16,
    chance: u32,
    #[serde(default)]
    is_zeny: bool,
    #[serde(default)]
    is_mvp_bonus: bool,
}

/// 掉落表加载器
pub struct DropTableLoader;

impl DropTableLoader {
    /// 从 YAML 文件加载掉落表
    ///
    /// # YAML 格式
    /// ```yaml
    /// 1001:  # Mob ID
    ///   - item_id: 501
    ///     min_amount: 1
    ///     max_amount: 1
    ///     chance: 5000
    ///     is_zeny: false
    ///     is_mvp_bonus: false
    /// ```
    pub fn load_from_yaml(path: &str) -> HashMap<u32, MobDropTable> {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read drop table file '{}': {}", path, e));

        let yaml_data: HashMap<String, Vec<YamlDropEntry>> = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse drop table YAML '{}': {}", path, e));

        let mut tables = HashMap::new();

        for (mob_id_str, entries) in yaml_data {
            let mob_id: u32 = mob_id_str
                .parse()
                .unwrap_or_else(|e| panic!("Invalid mob ID '{}': {}", mob_id_str, e));

            let drop_entries: Vec<DropTableEntry> = entries
                .into_iter()
                .map(|e| DropTableEntry {
                    item_id: e.item_id,
                    min_amount: e.min_amount,
                    max_amount: e.max_amount,
                    chance: e.chance,
                    is_zeny: e.is_zeny,
                    is_mvp_bonus: e.is_mvp_bonus,
                })
                .collect();

            tables.insert(mob_id, MobDropTable::new(drop_entries));
        }

        tables
    }
}

/// 掉落解析器
pub struct DropResolver;

impl DropResolver {
    /// 解析掉落表，生成掉落物品列表
    ///
    /// - 遍历所有条目，根据 chance 进行独立判定
    /// - MVP bonus 条目仅在 mvp_id 为 Some 时才会掉落
    pub fn resolve(
        &self,
        table: &MobDropTable,
        rng: &dyn GameRng,
        _mob_level: u16,
        mvp_id: Option<Uuid>,
    ) -> Vec<DropItem> {
        let mut drops = Vec::new();

        for entry in table.entries() {
            // MVP bonus 条目只在有 MVP 时掉落
            if entry.is_mvp_bonus && mvp_id.is_none() {
                continue;
            }

            // 基础点判定 (0-10000)
            let roll = rng.rand_bp(entry.chance);
            if roll < entry.chance {
                let amount = if entry.min_amount == entry.max_amount {
                    entry.min_amount
                } else {
                    rng.rand_range(entry.min_amount as u32, entry.max_amount as u32) as u16
                };

                drops.push(DropItem {
                    id: Uuid::new_v4(),
                    item_id: entry.item_id,
                    amount,
                    x: 0,                    // 由调用者设置
                    y: 0,                    // 由调用者设置
                    map_name: String::new(), // 由调用者设置
                    dropped_at: std::time::Instant::now(),
                });
            }
        }

        drops
    }
}

/// MVP 伤害记录解析器
pub struct MVPResolver;

impl MVPResolver {
    /// 从伤害记录中选择 MVP (造成最高伤害的玩家)
    ///
    /// 返回造成最高累计伤害的玩家 UUID
    pub fn pick_mvp(damage_log: &HashMap<Uuid, u64>) -> Option<Uuid> {
        damage_log
            .iter()
            .max_by(|a, b| a.1.cmp(b.1))
            .map(|(id, _)| *id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::UnsafeCell;

    /// 确定性测试 RNG - 支持 rand_bp
    pub struct TestRng {
        values: Vec<u32>,
        index: UnsafeCell<usize>,
    }

    impl TestRng {
        pub fn new(values: Vec<u32>) -> Self {
            Self {
                values,
                index: UnsafeCell::new(0),
            }
        }
    }

    unsafe impl Send for TestRng {}
    unsafe impl Sync for TestRng {}

    impl GameRng for TestRng {
        fn rand_range(&self, min: u32, max: u32) -> u32 {
            let idx = {
                let p = unsafe { &mut *self.index.get() };
                let current = *p;
                *p = current.wrapping_add(1);
                current
            };
            let val = self
                .values
                .get(idx % self.values.len())
                .copied()
                .unwrap_or(min);
            val.min(max).max(min)
        }

        fn rand_bool(&self, _probability: f32) -> bool {
            let idx = {
                let p = unsafe { &mut *self.index.get() };
                let current = *p;
                *p = current.wrapping_add(1);
                current
            };
            let val = self
                .values
                .get(idx % self.values.len())
                .copied()
                .unwrap_or(0);
            val % 2 == 0
        }

        fn rand_bp(&self, _chance: u32) -> u32 {
            // basis points: return 0 for always drop, 10001 for never drop
            let idx = unsafe { *self.index.get() };
            let val = self
                .values
                .get(idx % self.values.len())
                .copied()
                .unwrap_or(0);
            val
        }
    }

    fn create_entry(
        item_id: u32,
        min_amount: u16,
        max_amount: u16,
        chance: u32,
        is_zeny: bool,
        is_mvp_bonus: bool,
    ) -> DropTableEntry {
        DropTableEntry {
            item_id,
            min_amount,
            max_amount,
            chance,
            is_zeny,
            is_mvp_bonus,
        }
    }

    // Test 1: 100% chance -> always drops
    #[test]
    fn test_always_drops() {
        // Test with 100% chance - should always drop
        let resolver = DropResolver;
        let table = MobDropTable::new(vec![create_entry(501, 1, 1, 10000, false, false)]);

        // Use TestRng with value 5000 (which is < 10000, so always drops)
        let test_rng = TestRng::new(vec![5000]);

        let drops = resolver.resolve(&table, &test_rng, 1, None);
        assert!(!drops.is_empty(), "100% chance should always drop");
        assert_eq!(drops.len(), 1);
    }

    // Test 2: 0% chance -> never drops
    #[test]
    fn test_never_drops() {
        // Test with 0% chance - should never drop
        let resolver = DropResolver;
        let table = MobDropTable::new(vec![create_entry(501, 1, 1, 0, false, false)]);

        // Use TestRng with value 0 (roll >= 0 always, but we check < chance which is 0)
        let test_rng = TestRng::new(vec![0]);

        let drops = resolver.resolve(&table, &test_rng, 1, None);
        assert!(drops.is_empty(), "0% chance should never drop");
    }

    // Test 3: Range amount (1-5) -> within range
    #[test]
    fn test_range_amount() {
        let resolver = DropResolver;
        let table = MobDropTable::new(vec![create_entry(501, 1, 5, 10000, false, false)]);

        // Test with deterministic values for amount calculation
        let test_values = vec![0, 1, 2, 3, 4, 5, 6, 7, 100];
        let test_rng = TestRng::new(test_values);

        let drops = resolver.resolve(&table, &test_rng, 1, None);
        assert!(!drops.is_empty());

        for drop in drops {
            assert!(
                drop.amount >= 1 && drop.amount <= 5,
                "Amount should be in range 1-5"
            );
        }
    }

    // Test 4: Multiple entries -> each rolls independently
    #[test]
    fn test_multiple_entries_independent_rolls() {
        let resolver = DropResolver;
        let table = MobDropTable::new(vec![
            create_entry(501, 1, 1, 10000, false, false), // Always drops
            create_entry(502, 1, 1, 0, false, false),     // Never drops
        ]);

        // Both entries should be rolled independently
        // Entry 1: 0 < 10000 = true (pass)
        // Entry 2: 0 < 0 = false (fail)
        let test_rng = TestRng::new(vec![0]);
        let drops = resolver.resolve(&table, &test_rng, 1, None);

        // Only entry 1 should drop
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].item_id, 501);
    }

    // Test 5: MVP bonus entry only drops when mvp_id provided
    #[test]
    fn test_mvp_bonus_requires_mvp_id() {
        let resolver = DropResolver;
        let table = MobDropTable::new(vec![create_entry(501, 1, 1, 10000, false, true)]);

        let test_rng = TestRng::new(vec![0]);

        // Without MVP ID - should not drop MVP bonus
        let drops_no_mvp = resolver.resolve(&table, &test_rng, 1, None);
        assert!(
            drops_no_mvp.is_empty(),
            "MVP bonus should not drop without mvp_id"
        );

        // With MVP ID - should drop
        let mvp_id = Uuid::new_v4();
        let drops_with_mvp = resolver.resolve(&table, &test_rng, 1, Some(mvp_id));
        assert!(
            !drops_with_mvp.is_empty(),
            "MVP bonus should drop with mvp_id"
        );
    }

    // Test 6: pick_mvp -> returns highest damage player
    #[test]
    fn test_pick_mvp_highest_damage() {
        let player1 = Uuid::new_v4();
        let player2 = Uuid::new_v4();
        let player3 = Uuid::new_v4();

        let damage_log: HashMap<Uuid, u64> = HashMap::from([
            (player1, 5000),
            (player2, 15000), // Highest
            (player3, 8000),
        ]);

        let mvp = MVPResolver::pick_mvp(&damage_log);
        assert!(mvp.is_some());
        assert_eq!(mvp.unwrap(), player2);
    }

    #[test]
    fn test_pick_mvp_empty_log() {
        let damage_log: HashMap<Uuid, u64> = HashMap::new();
        let mvp = MVPResolver::pick_mvp(&damage_log);
        assert!(mvp.is_none());
    }

    #[test]
    fn test_pick_mvp_single_player() {
        let player = Uuid::new_v4();
        let damage_log: HashMap<Uuid, u64> = HashMap::from([(player, 1000)]);

        let mvp = MVPResolver::pick_mvp(&damage_log);
        assert!(mvp.is_some());
        assert_eq!(mvp.unwrap(), player);
    }

    #[test]
    fn test_drop_table_entry_fields() {
        let entry = create_entry(1234, 5, 10, 5000, true, false);

        assert_eq!(entry.item_id, 1234);
        assert_eq!(entry.min_amount, 5);
        assert_eq!(entry.max_amount, 10);
        assert_eq!(entry.chance, 5000);
        assert!(entry.is_zeny);
        assert!(!entry.is_mvp_bonus);
    }

    #[test]
    fn test_mob_drop_table_entries() {
        let entries = vec![
            create_entry(1, 1, 1, 10000, false, false),
            create_entry(2, 1, 3, 5000, false, false),
        ];
        let table = MobDropTable::new(entries.clone());

        assert_eq!(table.entries().len(), 2);
        assert_eq!(table.entries()[0].item_id, 1);
        assert_eq!(table.entries()[1].item_id, 2);
    }
}
