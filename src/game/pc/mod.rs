//! 玩家角色系统
//!
//! 处理玩家角色的详细机制，包括属性计算、状态管理等。
//! 对应 rAthena 的 pc.cpp。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 玩家属性类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatType {
    /// 力量
    Str,
    /// 敏捷
    Agi,
    /// 体质
    Vit,
    /// 智力
    Int,
    /// 灵巧
    Dex,
    /// 幸运
    Luk,
}

/// 玩家属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    /// 基础等级
    pub base_level: u16,
    /// 职业等级
    pub job_level: u16,
    /// 基础经验值
    pub base_exp: u64,
    /// 职业经验值
    pub job_exp: u64,
    /// 力量
    pub str: u16,
    /// 敏捷
    pub agi: u16,
    /// 体质
    pub vit: u16,
    /// 智力
    pub int: u16,
    /// 灵巧
    pub dex: u16,
    /// 幸运
    pub luk: u16,
    /// 最大 HP
    pub max_hp: u32,
    /// 最大 SP
    pub max_sp: u32,
    /// 当前 HP
    pub current_hp: u32,
    /// 当前 SP
    pub current_sp: u32,
    /// 攻击力
    pub attack: u16,
    /// 魔法攻击力
    pub magic_attack: u16,
    /// 防御力
    pub defense: u16,
    /// 魔法防御力
    pub magic_defense: u16,
    /// 命中率
    pub hit: u16,
    /// 闪避率
    pub flee: u16,
    /// 暴击率
    pub critical: u16,
    /// 攻击速度
    pub attack_speed: u16,
    /// 移动速度
    pub move_speed: u16,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            str: 1,
            agi: 1,
            vit: 1,
            int: 1,
            dex: 1,
            luk: 1,
            max_hp: 100,
            max_sp: 50,
            current_hp: 100,
            current_sp: 50,
            attack: 10,
            magic_attack: 10,
            defense: 5,
            magic_defense: 5,
            hit: 100,
            flee: 100,
            critical: 1,
            attack_speed: 150,
            move_speed: 100,
        }
    }
}

const BASE_HP: u32 = 100;
const HP_PER_LEVEL: u32 = 10;
const VIT_HP_BONUS: u32 = 5;
const HIT_BASE: u16 = 175;
const FLEE_BASE: u16 = 100;
const ASPD_BASE: u16 = 150;

/// 属性计算器
pub struct StatCalculator;

impl StatCalculator {
    /// 计算最大 HP
    pub fn calculate_max_hp(base_level: u16, vit: u16, job_class: u16) -> u32 {
        let base_hp = BASE_HP.saturating_add((base_level as u32).saturating_mul(HP_PER_LEVEL));
        let vit_bonus = (vit as u32).saturating_mul(VIT_HP_BONUS);
        let job_bonus = (job_class as u32).saturating_mul(50);
        base_hp.saturating_add(vit_bonus).saturating_add(job_bonus)
    }

    /// 计算最大 SP
    pub fn calculate_max_sp(base_level: u16, int: u16, job_class: u16) -> u32 {
        let base_sp = 50u32.saturating_add((base_level as u32).saturating_mul(5));
        let int_bonus = (int as u32).saturating_mul(3);
        let job_bonus = (job_class as u32).saturating_mul(20);
        base_sp.saturating_add(int_bonus).saturating_add(job_bonus)
    }

    /// 计算攻击力
    pub fn calculate_attack(str: u16, dex: u16, level: u16) -> u16 {
        let base_attack = str.saturating_add(level / 4);
        let dex_bonus = dex / 10;
        base_attack.saturating_add(dex_bonus)
    }

    /// 计算魔法攻击力
    pub fn calculate_magic_attack(int: u16, dex: u16, level: u16) -> u16 {
        let base_attack = int.saturating_add(level / 4);
        let dex_bonus = dex / 10;
        base_attack.saturating_add(dex_bonus)
    }

    /// 计算防御力
    pub fn calculate_defense(vit: u16, agi: u16) -> u16 {
        vit.saturating_add(agi / 5)
    }

    /// 计算魔法防御力
    pub fn calculate_magic_defense(int: u16, vit: u16) -> u16 {
        int.saturating_add(vit / 5)
    }

    /// 计算命中率
    pub fn calculate_hit(dex: u16, luk: u16, level: u16) -> u16 {
        HIT_BASE
            .saturating_add(dex)
            .saturating_add(luk / 3)
            .saturating_add(level / 2)
    }

    /// 计算闪避率
    pub fn calculate_flee(agi: u16, luk: u16, level: u16) -> u16 {
        FLEE_BASE
            .saturating_add(agi)
            .saturating_add(luk / 5)
            .saturating_add(level / 2)
    }

    /// 计算暴击率
    pub fn calculate_critical(luk: u16) -> u16 {
        1u16.saturating_add(luk / 3)
    }

    /// 计算攻击速度
    pub fn calculate_attack_speed(agi: u16, dex: u16) -> u16 {
        ASPD_BASE
            .saturating_sub(agi / 2)
            .saturating_sub(dex / 5)
            .max(100)
    }
}

/// 玩家状态效果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    /// 效果 ID
    pub id: u32,
    /// 效果名称
    pub name: String,
    /// 持续时间（秒）
    pub duration: u32,
    /// 开始时间
    pub start_time: u64,
    /// 效果参数
    pub params: HashMap<String, i32>,
}

/// 玩家角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerCharacter {
    /// 角色 ID
    pub id: u32,
    /// 账号 ID
    pub account_id: u32,
    /// 角色名称
    pub name: String,
    /// 职业 ID
    pub job_class: u16,
    /// 性别
    pub gender: u8,
    /// 属性
    pub stats: PlayerStats,
    /// 状态效果
    pub status_effects: Vec<StatusEffect>,
    /// 技能点数
    pub skill_points: u16,
    /// 属性点数
    pub stat_points: u16,
    /// 金币
    pub zeny: u32,
    /// 地图 ID
    pub map_id: u32,
    /// X 坐标
    pub x: u16,
    /// Y 坐标
    pub y: u16,
}

impl PlayerCharacter {
    /// 创建新角色
    pub fn new(id: u32, account_id: u32, name: String, job_class: u16) -> Self {
        let mut character = Self {
            id,
            account_id,
            name,
            job_class,
            gender: 0,
            stats: PlayerStats::default(),
            status_effects: Vec::new(),
            skill_points: 0,
            stat_points: 0,
            zeny: 0,
            map_id: 0,
            x: 0,
            y: 0,
        };
        character.recalculate_stats();
        character
    }

    /// 升级基础等级
    pub fn level_up_base(&mut self) {
        self.stats.base_level += 1;
        self.stat_points += 5;
        self.recalculate_stats();
    }

    /// 升级职业等级
    pub fn level_up_job(&mut self) {
        self.stats.job_level += 1;
        self.skill_points += 1;
        self.recalculate_stats();
    }

    /// 重新计算属性
    pub fn recalculate_stats(&mut self) {
        self.stats.max_hp =
            StatCalculator::calculate_max_hp(self.stats.base_level, self.stats.vit, self.job_class);
        self.stats.max_sp =
            StatCalculator::calculate_max_sp(self.stats.base_level, self.stats.int, self.job_class);
        self.stats.attack =
            StatCalculator::calculate_attack(self.stats.str, self.stats.dex, self.stats.base_level);
        self.stats.magic_attack = StatCalculator::calculate_magic_attack(
            self.stats.int,
            self.stats.dex,
            self.stats.base_level,
        );
        self.stats.defense = StatCalculator::calculate_defense(self.stats.vit, self.stats.agi);
        self.stats.magic_defense =
            StatCalculator::calculate_magic_defense(self.stats.int, self.stats.vit);
        self.stats.hit =
            StatCalculator::calculate_hit(self.stats.dex, self.stats.luk, self.stats.base_level);
        self.stats.flee =
            StatCalculator::calculate_flee(self.stats.agi, self.stats.luk, self.stats.base_level);
        self.stats.critical = StatCalculator::calculate_critical(self.stats.luk);
        self.stats.attack_speed =
            StatCalculator::calculate_attack_speed(self.stats.agi, self.stats.dex);
    }

    /// 增加属性点
    pub fn add_stat_point(&mut self, stat: StatType, amount: u16) -> bool {
        if self.stat_points < amount {
            return false;
        }

        match stat {
            StatType::Str => self.stats.str += amount,
            StatType::Agi => self.stats.agi += amount,
            StatType::Vit => self.stats.vit += amount,
            StatType::Int => self.stats.int += amount,
            StatType::Dex => self.stats.dex += amount,
            StatType::Luk => self.stats.luk += amount,
        }

        self.stat_points -= amount;
        self.recalculate_stats();
        true
    }

    /// 添加状态效果
    pub fn add_status_effect(&mut self, effect: StatusEffect) {
        self.status_effects.push(effect);
    }

    /// 移除状态效果
    pub fn remove_status_effect(&mut self, effect_id: u32) -> bool {
        if let Some(pos) = self.status_effects.iter().position(|e| e.id == effect_id) {
            self.status_effects.remove(pos);
            true
        } else {
            false
        }
    }

    /// 检查是否有状态效果
    pub fn has_status_effect(&self, effect_id: u32) -> bool {
        self.status_effects.iter().any(|e| e.id == effect_id)
    }

    /// 恢复 HP
    pub fn heal_hp(&mut self, amount: u32) {
        self.stats.current_hp = (self.stats.current_hp + amount).min(self.stats.max_hp);
    }

    /// 恢复 SP
    pub fn heal_sp(&mut self, amount: u32) {
        self.stats.current_sp = (self.stats.current_sp + amount).min(self.stats.max_sp);
    }

    /// 消耗 HP
    pub fn consume_hp(&mut self, amount: u32) -> bool {
        if self.stats.current_hp >= amount {
            self.stats.current_hp -= amount;
            true
        } else {
            false
        }
    }

    /// 消耗 SP
    pub fn consume_sp(&mut self, amount: u32) -> bool {
        if self.stats.current_sp >= amount {
            self.stats.current_sp -= amount;
            true
        } else {
            false
        }
    }

    /// 增加经验值
    pub fn add_exp(&mut self, base_exp: u64, job_exp: u64) {
        self.stats.base_exp += base_exp;
        self.stats.job_exp += job_exp;
    }

    /// 增加金币
    pub fn add_zeny(&mut self, amount: u32) {
        self.zeny = self.zeny.saturating_add(amount);
    }

    /// 消耗金币
    pub fn consume_zeny(&mut self, amount: u32) -> bool {
        if self.zeny >= amount {
            self.zeny -= amount;
            true
        } else {
            false
        }
    }

    /// 传送
    pub fn teleport(&mut self, map_id: u32, x: u16, y: u16) {
        self.map_id = map_id;
        self.x = x;
        self.y = y;
    }
}

/// 玩家角色管理器
pub struct PlayerCharacterManager {
    /// 角色映射
    characters: RwLock<HashMap<u32, PlayerCharacter>>,
}

impl PlayerCharacterManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            characters: RwLock::new(HashMap::new()),
        }
    }

    /// 添加角色
    pub fn add_character(&self, character: PlayerCharacter) {
        self.characters.write().insert(character.id, character);
    }

    /// 移除角色
    pub fn remove_character(&self, id: u32) -> bool {
        self.characters.write().remove(&id).is_some()
    }

    /// 获取角色
    pub fn get_character(&self, id: u32) -> Option<PlayerCharacter> {
        self.characters.read().get(&id).cloned()
    }

    /// 获取可变角色引用
    pub fn with_character_mut<F, R>(&self, id: u32, f: F) -> Option<R>
    where
        F: FnOnce(&mut PlayerCharacter) -> R,
    {
        let mut chars = self.characters.write();
        chars.get_mut(&id).map(f)
    }

    /// 获取所有角色
    pub fn get_all_characters(&self) -> Vec<PlayerCharacter> {
        self.characters.read().values().cloned().collect()
    }

    /// 获取在线角色数量
    pub fn online_count(&self) -> usize {
        self.characters.read().len()
    }
}

impl Default for PlayerCharacterManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_character() {
        let char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);
        assert_eq!(char.stats.base_level, 1);
        assert_eq!(char.stats.max_hp, 115);
    }

    #[test]
    fn test_level_up() {
        let mut char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);
        char.level_up_base();

        assert_eq!(char.stats.base_level, 2);
        assert_eq!(char.stat_points, 5);
    }

    #[test]
    fn test_add_stat_point() {
        let mut char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);
        char.stat_points = 10;

        assert!(char.add_stat_point(StatType::Str, 5));
        assert_eq!(char.stats.str, 6);
        assert_eq!(char.stat_points, 5);
    }

    #[test]
    fn test_heal_consume() {
        let mut char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);
        char.stats.current_hp = 50;

        char.heal_hp(30);
        assert_eq!(char.stats.current_hp, 80);

        assert!(char.consume_hp(20));
        assert_eq!(char.stats.current_hp, 60);

        assert!(!char.consume_hp(100));
    }

    #[test]
    fn test_zeny() {
        let mut char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);
        char.add_zeny(1000);

        assert_eq!(char.zeny, 1000);
        assert!(char.consume_zeny(500));
        assert_eq!(char.zeny, 500);
        assert!(!char.consume_zeny(600));
    }

    #[test]
    fn test_status_effects() {
        let mut char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);

        let effect = StatusEffect {
            id: 1,
            name: "中毒".to_string(),
            duration: 60,
            start_time: 0,
            params: HashMap::new(),
        };

        char.add_status_effect(effect);
        assert!(char.has_status_effect(1));

        char.remove_status_effect(1);
        assert!(!char.has_status_effect(1));
    }

    #[test]
    fn test_manager() {
        let manager = PlayerCharacterManager::new();
        let char = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);

        manager.add_character(char);
        assert_eq!(manager.online_count(), 1);

        let char = manager.get_character(1).unwrap();
        assert_eq!(char.name, "测试角色");
    }
}
