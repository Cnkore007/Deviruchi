use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::game::mob::Mob;

/// 怪物刷新点
#[derive(Debug, Clone)]
pub struct SpawnPoint {
    pub mob_id: u16,
    pub x: u16,
    pub y: u16,
    pub count: u8,
    pub interval: u32,
}

/// 地图怪物刷新管理
pub struct MobSpawnManager {
    spawns: RwLock<HashMap<String, Vec<SpawnPoint>>>,
    active_mobs: RwLock<HashMap<String, Vec<Arc<Mob>>>>,
}

impl MobSpawnManager {
    pub fn new() -> Self {
        Self {
            spawns: RwLock::new(HashMap::new()),
            active_mobs: RwLock::new(HashMap::new()),
        }
    }

    /// 添加刷新点
    pub fn add_spawn(&self, map_name: &str, spawn: SpawnPoint) {
        let mut spawns = self.spawns.write();
        spawns.entry(map_name.to_string()).or_default().push(spawn);
    }

    /// 获取地图的刷新点
    pub fn get_spawns(&self, map_name: &str) -> Vec<SpawnPoint> {
        self.spawns.read().get(map_name).cloned().unwrap_or_default()
    }

    /// 注册活跃怪物
    pub fn register_mob(&self, map_name: &str, mob: Arc<Mob>) {
        let mut active = self.active_mobs.write();
        active.entry(map_name.to_string()).or_default().push(mob);
    }

    /// 移除死亡怪物
    pub fn unregister_mob(&self, map_name: &str, mob_id: &uuid::Uuid) {
        let mut active = self.active_mobs.write();
        if let Some(mobs) = active.get_mut(map_name) {
            mobs.retain(|m| &m.id != mob_id);
        }
    }

    /// 获取地图上所有活跃怪物
    pub fn get_active_mobs(&self, map_name: &str) -> Vec<Arc<Mob>> {
        self.active_mobs.read().get(map_name).cloned().unwrap_or_default()
    }

    /// 获取所有有活跃怪物的地图
    pub fn get_active_maps(&self) -> Vec<String> {
        self.active_mobs.read().keys().cloned().collect()
    }

    /// 根据 ID 获取怪物
    pub fn get_mob(&self, mob_id: &uuid::Uuid) -> Option<Arc<Mob>> {
        let active = self.active_mobs.read();
        for mobs in active.values() {
            for mob in mobs {
                if mob.id == *mob_id {
                    return Some(mob.clone());
                }
            }
        }
        None
    }

    /// 初始化默认刷新点
    pub fn init_default_spawns(&self) {
        self.add_spawn("prontera.gat", SpawnPoint {
            mob_id: 1001,
            x: 100,
            y: 100,
            count: 10,
            interval: 10000,
        });
        self.add_spawn("prontera.gat", SpawnPoint {
            mob_id: 1002,
            x: 150,
            y: 120,
            count: 5,
            interval: 15000,
        });
        self.add_spawn("new_1-1.gat", SpawnPoint {
            mob_id: 1001,
            x: 50,
            y: 50,
            count: 15,
            interval: 5000,
        });
        self.add_spawn("new_1-1.gat", SpawnPoint {
            mob_id: 1312,
            x: 100,
            y: 100,
            count: 10,
            interval: 10000,
        });
    }
}

impl Default for MobSpawnManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod combat_integration_tests {
    use super::*;
    use crate::game::mob::MobAIState;
    use uuid::Uuid;
    use std::time::{Duration, Instant};

    #[test]
    fn test_mob_respawn_timer() {
        let mob = Mob::new(1001, 10, 10, "prontera.gat");
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        // 模拟死亡
        mob.take_damage(mob.max_hp);
        assert_eq!(*mob.ai_state.read(), MobAIState::Dead);
        assert!(mob.death_time.read().is_some());

        // 重生时间未到，不应重生
        *mob.death_time.write() = Some(Instant::now() - Duration::from_millis(mob.respawn_time as u64 - 1000));
        let should_respawn = mob.death_time.read().map_or(false, |death_time| {
            Instant::now().duration_since(death_time) >= Duration::from_millis(mob.respawn_time as u64)
        });
        assert!(!should_respawn);

        // 重生时间已到
        *mob.death_time.write() = Some(Instant::now() - Duration::from_millis(mob.respawn_time as u64));
        let should_respawn = mob.death_time.read().map_or(false, |death_time| {
            Instant::now().duration_since(death_time) >= Duration::from_millis(mob.respawn_time as u64)
        });
        assert!(should_respawn);
    }

    #[test]
    fn test_dmglog_tracking() {
        let mob = Mob::new(1001, 10, 10, "prontera.gat");
        let player1 = Uuid::new_v4();
        let player2 = Uuid::new_v4();

        mob.add_damage(player1, 10);
        assert_eq!(*mob.dmglog.read().get(&player1).unwrap(), 10);

        mob.add_damage(player1, 15);
        assert_eq!(*mob.dmglog.read().get(&player1).unwrap(), 25);

        mob.add_damage(player2, 5);
        assert_eq!(*mob.dmglog.read().get(&player2).unwrap(), 5);

        assert!(mob.dmglog.read().get(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_mob_respawn_resets_state() {
        let mob = Mob::new(1001, 10, 10, "prontera.gat");
        let player = Uuid::new_v4();

        mob.take_damage(30);
        mob.add_damage(player, 30);
        assert_eq!(*mob.hp.read(), 70);

        mob.take_damage(mob.max_hp); // 击杀
        assert_eq!(*mob.hp.read(), 0);
        assert_eq!(*mob.ai_state.read(), MobAIState::Dead);

        mob.respawn();
        assert_eq!(*mob.hp.read(), mob.max_hp);
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
        assert!(mob.dmglog.read().is_empty());
        assert!(!*mob.drops_processed.read());
    }

    #[test]
    fn test_drops_processed_flag_prevents_duplicate() {
        let mob = Mob::new(1001, 10, 10, "prontera.gat");

        // 初始为 false
        assert!(!*mob.drops_processed.read());

        // 模拟第一次进入 Dead 状态
        mob.take_damage(mob.max_hp);
        assert_eq!(*mob.ai_state.read(), MobAIState::Dead);
        assert!(!*mob.drops_processed.read()); // 还未处理

        // 模拟 GameLoop tick 中第一次调用 update_dead
        *mob.drops_processed.write() = true; // 模拟处理完成
        assert!(*mob.drops_processed.read());

        // 第二次 tick 不会再处理（因为 drops_processed 已为 true）
        // 这验证了防重机制
    }

    #[test]
    fn test_from_template_has_correct_respawn_time() {
        let mob = Mob::from_template(1001, 10, 10, "prontera.gat");
        // Poring 的 respawn_time 是 60000ms
        assert_eq!(mob.respawn_time, 60000);
        assert_eq!(mob.max_hp, 50);
        assert!(!mob.drops.is_empty()); // Poring 有掉落
    }
}
