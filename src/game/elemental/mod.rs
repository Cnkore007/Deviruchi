//! 精灵召唤系统
//!
//! 对应 rAthena 的 `src/map/elemental.cpp`，提供元素精灵召唤功能。
//!
//! 精灵由特定职业（如 Elemental Master）召唤，有独立的 AI 和技能系统。

use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

/// 精灵元素类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ElementalElement {
    /// 火
    Fire = 1,
    /// 水
    Water = 2,
    /// 风
    Wind = 3,
    /// 土
    Earth = 4,
}

/// 精灵 AI 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementalAIState {
    /// 空闲
    Idle,
    /// 跟随主人
    Follow,
    /// 攻击目标
    Attack,
    /// 返回主人身边
    Return,
}

/// 精灵模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementalMode {
    /// 被动模式（只跟随）
    Passive,
    /// 防御模式（攻击主人的目标）
    Defensive,
    /// 攻击模式（主动攻击）
    Aggressive,
}

/// 精灵技能
#[derive(Debug, Clone)]
pub struct ElementalSkill {
    /// 技能 ID
    pub skill_id: u16,
    /// 技能等级
    pub level: u8,
    /// 冷却时间（毫秒）
    pub cooldown_ms: u64,
    /// 上次使用时间
    pub last_used: u64,
}

impl ElementalSkill {
    /// 检查技能是否可用
    pub fn is_ready(&self, current_time: u64) -> bool {
        current_time.saturating_sub(self.last_used) >= self.cooldown_ms
    }

    /// 使用技能
    pub fn use_skill(&mut self, current_time: u64) {
        self.last_used = current_time;
    }
}

/// 精灵数据
#[derive(Debug, Clone)]
pub struct Elemental {
    /// 精灵实例 ID
    pub id: Uuid,
    /// 精灵类型 ID（对应数据库）
    pub elemental_id: u16,
    /// 元素类型
    pub element: ElementalElement,
    /// 主人 ID
    pub owner_id: Uuid,
    /// 当前 HP
    pub hp: u32,
    /// 最大 HP
    pub max_hp: u32,
    /// 当前 SP
    pub sp: u32,
    /// 最大 SP
    pub max_sp: u32,
    /// 攻击力
    pub atk: u32,
    /// 魔法攻击力
    pub matk: u32,
    /// 防御力
    pub def: u16,
    /// 魔法防御
    pub mdef: u16,
    /// 命中
    pub hit: u16,
    /// 回避
    pub flee: u16,
    /// 攻击速度
    pub aspd: u16,
    /// 生命时间（毫秒）
    pub lifetime_ms: u64,
    /// 剩余生命时间（毫秒）
    pub remaining_ms: u64,
    /// 当前 AI 状态
    pub ai_state: ElementalAIState,
    /// 当前模式
    pub mode: ElementalMode,
    /// 技能列表
    pub skills: Vec<ElementalSkill>,
    /// 攻击目标 ID
    pub target_id: Option<Uuid>,
    /// 创建时间
    pub created_at: u64,
}

impl Elemental {
    /// 创建新精灵
    pub fn new(
        elemental_id: u16,
        element: ElementalElement,
        owner_id: Uuid,
        max_hp: u32,
        max_sp: u32,
        atk: u32,
        matk: u32,
        def: u16,
        mdef: u16,
        lifetime_ms: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            elemental_id,
            element,
            owner_id,
            hp: max_hp,
            max_hp,
            sp: max_sp,
            max_sp,
            atk,
            matk,
            def,
            mdef,
            hit: 100,
            flee: 50,
            aspd: 1000,
            lifetime_ms,
            remaining_ms: lifetime_ms,
            ai_state: ElementalAIState::Idle,
            mode: ElementalMode::Passive,
            skills: Vec::new(),
            target_id: None,
            created_at: Self::current_time(),
        }
    }

    /// 精灵是否存活
    pub fn is_alive(&self) -> bool {
        self.hp > 0 && self.remaining_ms > 0
    }

    /// 受到伤害
    pub fn take_damage(&mut self, damage: u32) -> bool {
        self.hp = self.hp.saturating_sub(damage);
        self.hp == 0
    }

    /// 治疗
    pub fn heal(&mut self, amount: u32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }

    /// 更新剩余时间
    pub fn tick(&mut self, delta_ms: u64) {
        self.remaining_ms = self.remaining_ms.saturating_sub(delta_ms);
    }

    /// 切换模式
    pub fn change_mode(&mut self, mode: ElementalMode) {
        self.mode = mode;
        self.ai_state = ElementalAIState::Idle;
        self.target_id = None;
    }

    /// 设置攻击目标
    pub fn set_target(&mut self, target_id: Uuid) {
        self.target_id = Some(target_id);
        self.ai_state = ElementalAIState::Attack;
    }

    /// 清除目标
    pub fn clear_target(&mut self) {
        self.target_id = None;
        self.ai_state = ElementalAIState::Follow;
    }

    /// 解散精灵
    pub fn dismiss(&mut self) {
        self.hp = 0;
        self.remaining_ms = 0;
    }

    fn current_time() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// 精灵召唤管理器
pub struct ElementalManager {
    /// 活跃精灵 (elemental_id -> Elemental)
    elementals: RwLock<HashMap<Uuid, Elemental>>,
    /// 玩家精灵映射 (player_id -> elemental_id)
    player_elementals: RwLock<HashMap<Uuid, Uuid>>,
}

impl ElementalManager {
    /// 创建空的管理器
    pub fn new() -> Self {
        Self {
            elementals: RwLock::new(HashMap::new()),
            player_elementals: RwLock::new(HashMap::new()),
        }
    }

    /// 召唤精灵
    pub fn summon(
        &self,
        elemental_id: u16,
        element: ElementalElement,
        owner_id: Uuid,
        max_hp: u32,
        max_sp: u32,
        atk: u32,
        matk: u32,
        def: u16,
        mdef: u16,
        lifetime_ms: u64,
    ) -> Uuid {
        let elemental = Elemental::new(
            elemental_id,
            element,
            owner_id,
            max_hp,
            max_sp,
            atk,
            matk,
            def,
            mdef,
            lifetime_ms,
        );

        let id = elemental.id;

        // 如果玩家已有精灵，先解散（避免死锁，分步操作）
        let old_id = self.player_elementals.read().get(&owner_id).copied();
        if let Some(old_id) = old_id {
            self.dismiss_elemental(old_id);
        }

        self.elementals.write().insert(id, elemental);
        self.player_elementals.write().insert(owner_id, id);

        tracing::info!("Elemental {} summoned for player {:?}", id, owner_id);
        id
    }

    /// 解散精灵
    pub fn dismiss_elemental(&self, elemental_id: Uuid) -> bool {
        // 先获取 owner_id，然后释放锁
        let owner_id = {
            let elementals = self.elementals.read();
            match elementals.get(&elemental_id) {
                Some(e) => e.owner_id,
                None => return false,
            }
        };

        // 分别操作两个 map，避免死锁
        self.elementals.write().remove(&elemental_id);
        self.player_elementals.write().remove(&owner_id);

        tracing::info!("Elemental {} dismissed", elemental_id);
        true
    }

    /// 获取玩家的精灵
    pub fn get_player_elemental(&self, player_id: &Uuid) -> Option<Elemental> {
        let elemental_id = self.player_elementals.read().get(player_id).copied()?;
        self.elementals.read().get(&elemental_id).cloned()
    }

    /// 更新所有精灵
    pub fn tick(&self, delta_ms: u64) {
        let mut elementals = self.elementals.write();
        let mut to_remove = Vec::new();

        for (id, elemental) in elementals.iter_mut() {
            elemental.tick(delta_ms);
            if !elemental.is_alive() {
                to_remove.push((*id, elemental.owner_id));
            }
        }

        drop(elementals);

        for (id, owner_id) in to_remove {
            self.elementals.write().remove(&id);
            self.player_elementals.write().remove(&owner_id);
            tracing::info!("Elemental {} expired", id);
        }
    }

    /// 获取活跃精灵数量
    pub fn active_count(&self) -> usize {
        self.elementals.read().len()
    }

    /// 清理所有精灵
    pub fn clear(&self) {
        self.elementals.write().clear();
        self.player_elementals.write().clear();
    }
}

impl Default for ElementalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elemental_create() {
        let manager = ElementalManager::new();
        let owner = Uuid::new_v4();

        let id = manager.summon(
            2114,
            ElementalElement::Fire,
            owner,
            10000, 5000, 500, 300, 100, 50, 600000,
        );

        let elemental = manager.get_player_elemental(&owner).unwrap();
        assert_eq!(elemental.elemental_id, 2114);
        assert_eq!(elemental.element, ElementalElement::Fire);
        assert_eq!(elemental.hp, 10000);
        assert!(elemental.is_alive());
    }

    #[test]
    fn test_elemental_damage() {
        let manager = ElementalManager::new();
        let owner = Uuid::new_v4();

        manager.summon(
            2114,
            ElementalElement::Fire,
            owner,
            10000, 5000, 500, 300, 100, 50, 600000,
        );

        let mut elemental = manager.get_player_elemental(&owner).unwrap();
        assert!(!elemental.take_damage(9999));
        assert_eq!(elemental.hp, 1);

        assert!(elemental.take_damage(1));
        assert_eq!(elemental.hp, 0);
    }

    #[test]
    fn test_elemental_mode_change() {
        let manager = ElementalManager::new();
        let owner = Uuid::new_v4();

        manager.summon(
            2114,
            ElementalElement::Fire,
            owner,
            10000, 5000, 500, 300, 100, 50, 600000,
        );

        let mut elemental = manager.get_player_elemental(&owner).unwrap();
        assert_eq!(elemental.mode, ElementalMode::Passive);

        elemental.change_mode(ElementalMode::Aggressive);
        assert_eq!(elemental.mode, ElementalMode::Aggressive);
    }

    #[test]
    fn test_elemental_tick_expire() {
        let manager = ElementalManager::new();
        let owner = Uuid::new_v4();

        manager.summon(
            2114,
            ElementalElement::Fire,
            owner,
            10000, 5000, 500, 300, 100, 50, 1000,
        );

        assert_eq!(manager.active_count(), 1);

        manager.tick(500);
        assert_eq!(manager.active_count(), 1);

        manager.tick(600);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_elemental_replace() {
        let manager = ElementalManager::new();
        let owner = Uuid::new_v4();

        manager.summon(
            2114,
            ElementalElement::Fire,
            owner,
            10000, 5000, 500, 300, 100, 50, 600000,
        );
        assert_eq!(manager.active_count(), 1);

        // 再次召唤会替换旧的
        manager.summon(
            2115,
            ElementalElement::Water,
            owner,
            12000, 6000, 600, 400, 120, 60, 600000,
        );
        assert_eq!(manager.active_count(), 1);

        let elemental = manager.get_player_elemental(&owner).unwrap();
        assert_eq!(elemental.elemental_id, 2115);
    }

    #[test]
    fn test_elemental_dismiss() {
        let manager = ElementalManager::new();
        let owner = Uuid::new_v4();

        let id = manager.summon(
            2114,
            ElementalElement::Fire,
            owner,
            10000, 5000, 500, 300, 100, 50, 600000,
        );

        assert!(manager.dismiss_elemental(id));
        assert_eq!(manager.active_count(), 0);
        assert!(manager.get_player_elemental(&owner).is_none());
    }
}
