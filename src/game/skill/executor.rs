//! 技能效果执行器
//!
//! 提供技能释放后的实际效果执行逻辑，包括伤害、治疗、增益和范围技能。
//! 与 `SkillEffect`（计算原始数值）不同，本模块负责将效果应用到目标实体。

use super::data::{Skill, SkillDatabase, SkillType};
use super::effect::{SkillEffect, SkillResult};
use crate::game::battle::element::Element;
use crate::game::battle::formula::{self, DamageResult};
use crate::game::map::{MapState, Player};

/// 伤害技能执行结果
#[derive(Debug, Clone)]
pub struct DamageSkillResult {
    /// 实际造成的伤害
    pub damage: i32,
    /// 技能元素属性
    pub element: u8,
    /// 元素修正倍率（1.0 = 100%）
    pub element_modifier: f32,
    /// 目标是否死亡
    pub target_died: bool,
}

/// 治疗技能执行结果
#[derive(Debug, Clone)]
pub struct HealSkillResult {
    /// 计算的治疗量
    pub heal_amount: u32,
    /// 实际恢复的 HP（不超过 max_hp）
    pub actual_heal: u32,
    /// 目标当前 HP
    pub current_hp: u32,
    /// 目标最大 HP
    pub max_hp: u32,
}

/// Buff 技能执行结果
#[derive(Debug, Clone)]
pub struct BuffSkillResult {
    /// 技能 ID
    pub skill_id: u16,
    /// Buff 持续时间（毫秒）
    pub duration_ms: u32,
    /// 施加的目标名称
    pub target_name: String,
}

/// 范围技能执行结果
#[derive(Debug, Clone)]
pub struct AreaSkillResult {
    /// 受影响的目标数量
    pub hit_count: u32,
    /// 每个目标的伤害结果（伤害技能时有值）
    pub damages: Vec<DamageSkillResult>,
    /// 每个目标的治疗结果（治疗技能时有值）
    pub heals: Vec<HealSkillResult>,
}

/// 技能效果执行器
///
/// 负责将技能效果实际应用到游戏实体上。
/// 区别于 `SkillEffect::apply`（仅计算原始数值），本执行器完成：
/// - 伤害计算与 HP 扣除
/// - 治疗量计算与 HP 恢复
/// - 状态效果施加
/// - 范围技能的多目标处理
pub struct SkillExecutor;

impl SkillExecutor {
    /// 执行伤害技能
    ///
    /// 完整流程：
    /// 1. 从技能数据库获取技能数据
    /// 2. 根据技能元素属性选择物理/魔法伤害公式
    /// 3. 应用元素修正
    /// 4. 将伤害应用到目标 HP
    /// 5. 返回伤害结果
    ///
    /// # 参数
    /// - `skill_id`: 技能 ID
    /// - `skill_level`: 技能等级
    /// - `caster`: 施法者
    /// - `target`: 目标
    ///
    /// # 返回
    /// `DamageSkillResult` 包含伤害值、元素修正和目标死亡状态。
    /// 如果技能不存在或非攻击类型，返回 `None`。
    pub fn execute_damage_skill(
        skill_id: u16,
        skill_level: u8,
        caster: &Player,
        target: &Player,
    ) -> Option<DamageSkillResult> {
        // 通过默认数据库获取技能数据
        let db = SkillDatabase::new();
        let skill = db.get(skill_id)?;

        // 验证技能类型
        if skill.type_ != SkillType::Attack {
            tracing::warn!("技能 {} 非攻击类型，无法执行伤害", skill.name);
            return None;
        }

        Self::execute_damage_with_skill(skill, skill_level, caster, target)
    }

    /// 使用已加载的技能数据执行伤害技能（避免重复创建数据库）
    pub fn execute_damage_with_skill(
        skill: &Skill,
        skill_level: u8,
        caster: &Player,
        target: &Player,
    ) -> Option<DamageSkillResult> {
        // 获取施法者属性
        let caster_int = caster.int() as i32;
        let caster_dex = caster.dex() as i32;
        let caster_base_level = caster.base_level() as i32;

        // 获取目标属性
        let target_element = Element::Neutral; // TODO: 从目标实体获取元素属性
        let target_mdef = 0u32; // TODO: 从目标装备/状态获取魔法防御

        // 技能元素属性（Skill.element 使用 u8 编码，与 Element 枚举一致）
        let skill_element = Element::from_u8(skill.element).unwrap_or(Element::Neutral);

        // 技能倍率：使用 skill.damage 字段作为基础倍率，每级增加
        let skill_multiplier = if skill.damage > 0 {
            skill.damage + (skill_level as i32 * 10)
        } else {
            100 + (skill_level as i32 * 10)
        };

        // 根据技能元素属性判断物理/魔法
        // element == 0 (Neutral/Weapon) 使用物理公式，其他使用魔法公式
        let raw_damage = if skill.element == 0 {
            // 物理伤害：base_atk * multiplier / 100
            let base_atk = caster_base_level * 2
                + caster.str() as i32
                + caster.dex() as i32 / 2;
            base_atk * skill_multiplier / 100
        } else {
            // 魔法伤害：使用 calc_magic_damage 公式
            let matk = caster_int * 2 + caster_dex;
            let DamageResult { damage, element_modifier: _ } = formula::calc_magic_damage(
                matk as u32,
                skill_level,
                skill_element,
                target_element,
                target_mdef,
                target.base_level() as u32,
            );
            // calc_magic_damage 内部已包含元素修正，直接返回
            let died = target.take_damage(damage.max(1) as u32);

            tracing::info!(
                "魔法技能 {} 对 {} 造成 {} 伤害（元素修正后）",
                skill.name,
                target.name,
                damage
            );

            return Some(DamageSkillResult {
                damage: damage.max(1),
                element: skill.element,
                element_modifier: formula::element_modifier(skill.element, 0),
                target_died: died,
            });
        };

        // 物理伤害路径：应用元素修正
        let elem_mod = formula::element_modifier(skill.element, 0); // TODO: 使用目标实际元素
        let final_damage = ((raw_damage as f64 * elem_mod as f64) as i32).max(1);

        // 应用伤害到目标
        let died = target.take_damage(final_damage as u32);

        tracing::info!(
            "物理技能 {} 对 {} 造成 {} 伤害（倍率 {}%）",
            skill.name,
            target.name,
            final_damage,
            skill_multiplier
        );

        Some(DamageSkillResult {
            damage: final_damage,
            element: skill.element,
            element_modifier: elem_mod,
            target_died: died,
        })
    }

    /// 执行治疗技能
    ///
    /// 完整流程：
    /// 1. 从技能数据库获取技能数据
    /// 2. 计算治疗量（基于 INT、VIT、技能等级）
    /// 3. 应用到目标 HP（不超过 max_hp）
    ///
    /// # 参数
    /// - `skill_id`: 技能 ID
    /// - `skill_level`: 技能等级
    /// - `caster`: 施法者
    /// - `target`: 治疗目标
    ///
    /// # 返回
    /// `HealSkillResult` 包含治疗量和目标 HP 状态。
    pub fn execute_heal_skill(
        skill_id: u16,
        skill_level: u8,
        caster: &Player,
        target: &Player,
    ) -> Option<HealSkillResult> {
        let db = SkillDatabase::new();
        let skill = db.get(skill_id)?;

        if skill.type_ != SkillType::Healing {
            tracing::warn!("技能 {} 非治疗类型", skill.name);
            return None;
        }

        Self::execute_heal_with_skill(skill, skill_level, caster, target)
    }

    /// 使用已加载的技能数据执行治疗技能
    pub fn execute_heal_with_skill(
        skill: &Skill,
        skill_level: u8,
        caster: &Player,
        target: &Player,
    ) -> Option<HealSkillResult> {
        // 记录目标治疗前 HP
        let hp_before = target.hp();
        let max_hp = target.max_hp();

        // 使用 SkillEffect 计算治疗量
        let result = SkillEffect::apply(skill, caster, target, skill_level);
        if let SkillResult::Heal { amount } = result {
            // 应用治疗到目标（Player::apply_heal 内部处理 max_hp 限制）
            target.apply_heal(amount);

            let hp_after = target.hp();
            let actual_heal = hp_after.saturating_sub(hp_before);

            tracing::info!(
                "治疗技能 {} 恢复 {} HP（实际 {}）, {}/{}",
                skill.name,
                amount,
                actual_heal,
                hp_after,
                max_hp
            );

            Some(HealSkillResult {
                heal_amount: amount,
                actual_heal,
                current_hp: hp_after,
                max_hp,
            })
        } else {
            tracing::warn!("治疗技能 {} 计算结果异常: {:?}", skill.name, result);
            None
        }
    }

    /// 执行 Buff 技能
    ///
    /// 完整流程：
    /// 1. 从技能数据库获取技能数据
    /// 2. 根据技能 ID 确定状态效果类型
    /// 3. 调用状态系统施加效果
    ///
    /// # 参数
    /// - `skill_id`: 技能 ID
    /// - `skill_level`: 技能等级
    /// - `caster`: 施法者（部分 buff 效果受施法者属性影响）
    /// - `target`: 目标
    pub fn execute_buff_skill(
        skill_id: u16,
        skill_level: u8,
        caster: &Player,
        target: &Player,
    ) -> Option<BuffSkillResult> {
        let db = SkillDatabase::new();
        let skill = db.get(skill_id)?;

        if skill.type_ != SkillType::Support {
            tracing::warn!("技能 {} 非辅助类型", skill.name);
            return None;
        }

        Self::execute_buff_with_skill(skill, skill_level, caster, target)
    }

    /// 使用已加载的技能数据执行 Buff 技能
    pub fn execute_buff_with_skill(
        skill: &Skill,
        skill_level: u8,
        caster: &Player,
        target: &Player,
    ) -> Option<BuffSkillResult> {
        // 调用 SkillEffect::apply 来施加 buff（内部会调用 target.add_status）
        let result = SkillEffect::apply(skill, caster, target, skill_level);

        match result {
            SkillResult::Buff { buff_type, duration } => {
                tracing::info!(
                    "Buff 技能 {} 施加给 {}，持续 {}ms",
                    skill.name,
                    target.name,
                    duration
                );

                Some(BuffSkillResult {
                    skill_id: buff_type,
                    duration_ms: duration,
                    target_name: target.name.clone(),
                })
            }
            _ => {
                tracing::warn!("Buff 技能 {} 执行结果异常: {:?}", skill.name, result);
                None
            }
        }
    }

    /// 执行范围技能
    ///
    /// 完整流程：
    /// 1. 获取技能范围（splash_area 或 range）
    /// 2. 从地图状态获取范围内的所有实体
    /// 3. 对每个实体执行效果（伤害/Buff）
    ///
    /// # 参数
    /// - `skill_id`: 技能 ID
    /// - `skill_level`: 技能等级
    /// - `caster`: 施法者
    /// - `center_pos`: 技能中心点 (x, y)
    /// - `map_state`: 地图状态（用于查询范围内的实体）
    pub fn execute_area_skill(
        skill_id: u16,
        skill_level: u8,
        caster: &Player,
        center_pos: (u16, u16),
        map_state: &MapState,
    ) -> Option<AreaSkillResult> {
        let db = SkillDatabase::new();
        let skill = db.get(skill_id)?;

        Self::execute_area_with_skill(skill, skill_level, caster, center_pos, map_state)
    }

    /// 使用已加载的技能数据执行范围技能
    pub fn execute_area_with_skill(
        skill: &Skill,
        skill_level: u8,
        caster: &Player,
        center_pos: (u16, u16),
        map_state: &MapState,
    ) -> Option<AreaSkillResult> {
        // 获取范围（使用 range 字段作为 AoE 半径）
        let aoe_range = skill.range;

        // 获取同地图上的所有玩家
        let all_players = map_state.get_players_on_map(&caster.map_name);

        // 筛选范围内的目标（排除施法者自身）
        let targets: Vec<Player> = all_players
            .into_iter()
            .filter(|p| {
                // 排除施法者自身
                if p.id == caster.id {
                    return false;
                }
                // 计算距离
                let (tx, ty) = p.get_position();
                let dx = (center_pos.0 as i32 - tx as i32).unsigned_abs() as u16;
                let dy = (center_pos.1 as i32 - ty as i32).unsigned_abs() as u16;
                dx <= aoe_range && dy <= aoe_range
            })
            .collect();

        if targets.is_empty() {
            tracing::info!("范围技能 {} 无命中目标", skill.name);
            return Some(AreaSkillResult {
                hit_count: 0,
                damages: Vec::new(),
                heals: Vec::new(),
            });
        }

        let mut damages = Vec::new();
        let mut heals = Vec::new();

        // 根据技能类型对每个目标执行效果
        for target in &targets {
            match skill.type_ {
                SkillType::Attack => {
                    if let Some(result) =
                        Self::execute_damage_with_skill(skill, skill_level, caster, target)
                    {
                        damages.push(result);
                    }
                }
                SkillType::Healing => {
                    if let Some(result) =
                        Self::execute_heal_with_skill(skill, skill_level, caster, target)
                    {
                        heals.push(result);
                    }
                }
                SkillType::Support => {
                    Self::execute_buff_with_skill(skill, skill_level, caster, target);
                }
                _ => {
                    tracing::warn!("范围技能 {} 类型 {:?} 暂不支持", skill.name, skill.type_);
                }
            }
        }

        let hit_count = (damages.len() + heals.len()) as u32;

        tracing::info!(
            "范围技能 {} 命中 {} 个目标",
            skill.name,
            hit_count
        );

        Some(AreaSkillResult {
            hit_count,
            damages,
            heals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::constants;
    use crate::game::map::player::{CombatStats, PlayerState, Position, LevelStats, Attributes, Economy, SavePoint};
    use crate::game::item::Equipment;
    use crate::game::status::PlayerStatus;
    use crate::game::map::MapState;
    use parking_lot::RwLock;
    use std::sync::Arc;
    use uuid::Uuid;

    /// 创建测试用技能数据库（与 SkillDatabase 默认数据一致）
    fn make_test_db() -> SkillDatabase {
        SkillDatabase::new()
    }

    /// 创建测试用玩家，可指定元素属性（预留）
    fn make_player(name: &str, x: u16, y: u16) -> Arc<Player> {
        Arc::new(Player {
            id: Uuid::new_v4(),
            char_id: rand::random::<u32>(),
            account_id: 1,
            name: name.to_string(),
            map_name: "test_map".to_string(),
            combat: RwLock::new(CombatStats {
                hp: 1000,
                max_hp: 1000,
                sp: 500,
                max_sp: 500,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: RwLock::new(Position { x, y }),
            level: RwLock::new(LevelStats {
                base_level: 50,
                job_level: 25,
                base_exp: 0,
                job_exp: 0,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: RwLock::new(Attributes {
                str: 10,
                agi: 10,
                vit: 10,
                int: 30,  // 高智力用于魔法伤害测试
                dex: 20,
                luk: 5,
            }),
            economy: RwLock::new(Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        })
    }

    // ==================== 伤害技能测试 ====================

    #[test]
    fn test_execute_damage_skill_fire_ball() {
        // 火球术 (ID=25) 为魔法技能，element=1（硬编码数据中的值）
        // 施法者: INT=30, DEX=20, level=50
        // MATK = 30*2 + 20 = 80
        // calc_magic_damage: matk=80, skill_level=1, element=1, target=Neutral, mdef=0
        //   skill_multiplier = 100% (level 1)
        //   base = 80 * 100/100 - 0 = 80
        //   element_modifier(1, 0) = 100%
        //   final = 80
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_damage_skill(25, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        assert!(r.damage > 0, "火球术应造成正数伤害，实际: {}", r.damage);
        assert!(!r.target_died, "1000 HP 目标不应被 80 伤害击杀");

        // 目标 HP 应减少
        assert!(target.hp() < 1000, "目标 HP 应减少");
    }

    #[test]
    fn test_execute_damage_skill_element_advantage() {
        // 风属性技能 vs 水属性目标
        // 创建一个风属性技能进行测试
        let skill = Skill {
            id: 9001,
            name: "Wind Cutter".to_string(),
            type_: SkillType::Attack,
            target: super::super::data::SkillTarget::Enemy,
            level: 10,
            sp_cost: 10,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 100,
            hit: 0,
            element: 4, // Wind
            flags: 0,
        };

        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_damage_with_skill(&skill, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        // MATK = 30*2 + 20 = 80
        // calc_magic_damage: matk=80, level=1, Wind vs Neutral
        //   multiplier = 100%, base = 80, element = 100%
        //   damage = 80
        assert_eq!(r.damage, 80);
        assert_eq!(r.element, 4); // Wind
    }

    #[test]
    fn test_execute_damage_skill_kills_target() {
        // 低 HP 目标被高伤害击杀
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);
        // 设置目标 HP 为 10
        target.combat_mut().hp = 10;

        let result = SkillExecutor::execute_damage_skill(25, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        assert!(r.target_died, "10 HP 目标应被击杀");
        assert_eq!(target.hp(), 0);
    }

    #[test]
    fn test_execute_damage_skill_non_attack_type_returns_none() {
        // 治疗技能不应执行伤害
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_damage_skill(28, 1, &caster, &target);
        assert!(result.is_none(), "非攻击技能应返回 None");
    }

    #[test]
    fn test_execute_damage_skill_invalid_id_returns_none() {
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_damage_skill(9999, 1, &caster, &target);
        assert!(result.is_none(), "不存在的技能应返回 None");
    }

    // ==================== 治疗技能测试 ====================

    #[test]
    fn test_execute_heal_skill_basic() {
        // 治愈术 (ID=28)
        // 施法者: INT=30, VIT=10, level=50
        // SkillEffect::apply_healing: heal_base = 30 + 10/2 + 50 = 85
        // multiplier = 35 + 1*20 = 55%
        // total = 85 * 55 / 100 = 46
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);
        // 设置目标 HP 低于 max_hp
        target.combat_mut().hp = 500;

        let result = SkillExecutor::execute_heal_skill(28, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        assert!(r.heal_amount > 0, "治疗量应为正数");
        assert!(r.actual_heal > 0, "实际恢复应为正数");
        assert_eq!(r.current_hp, 500 + r.actual_heal);
        assert_eq!(r.max_hp, 1000);
    }

    #[test]
    fn test_execute_heal_skill_cannot_exceed_max_hp() {
        // 治疗不应超过 max_hp
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);
        // 设置目标 HP 接近满
        target.combat_mut().hp = 990;

        let result = SkillExecutor::execute_heal_skill(28, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        assert!(r.current_hp <= r.max_hp, "HP 不应超过 max_hp");
        // actual_heal 应该被 max_hp 限制
        assert!(r.actual_heal <= 10, "实际恢复应被 max_hp 限制");
    }

    #[test]
    fn test_execute_heal_skill_already_full_hp() {
        // 满 HP 时治疗无效
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);
        // 目标满血
        assert_eq!(target.hp(), 1000);

        let result = SkillExecutor::execute_heal_skill(28, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        assert_eq!(r.actual_heal, 0, "满血时实际恢复应为 0");
        assert_eq!(r.current_hp, 1000);
    }

    #[test]
    fn test_execute_heal_skill_non_healing_type_returns_none() {
        // 攻击技能不应执行治疗
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_heal_skill(1, 1, &caster, &target);
        assert!(result.is_none(), "非治疗技能应返回 None");
    }

    #[test]
    fn test_execute_heal_skill_level_scaling() {
        // 高等级治疗应恢复更多
        let caster = make_player("Caster", 100, 100);
        let target_low = make_player("TargetLow", 105, 100);
        let target_high = make_player("TargetHigh", 110, 100);
        target_low.combat_mut().hp = 500;
        target_high.combat_mut().hp = 500;

        let result_low = SkillExecutor::execute_heal_skill(28, 1, &caster, &target_low);
        let result_high = SkillExecutor::execute_heal_skill(28, 5, &caster, &target_high);

        assert!(result_low.is_some());
        assert!(result_high.is_some());

        // 高等级治疗量应更大
        assert!(
            result_high.unwrap().heal_amount >= result_low.unwrap().heal_amount,
            "高等级治疗量应 >= 低等级"
        );
    }

    // ==================== Buff 技能测试 ====================

    #[test]
    fn test_execute_buff_skill_increase_agi() {
        // 加速术 (ID=29) 施加 IncreaseAgi 状态
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_buff_skill(29, 1, &caster, &target);
        assert!(result.is_some());

        let r = result.unwrap();
        assert_eq!(r.skill_id, 29);
        assert!(r.duration_ms > 0, "Buff 持续时间应 > 0");
        assert_eq!(r.target_name, "Target");

        // 验证状态效果已施加
        assert!(target.has_status(crate::game::status::StatusChange::IncreaseAgi));
    }

    #[test]
    fn test_execute_buff_skill_non_support_type_returns_none() {
        // 攻击技能不应执行 buff
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        let result = SkillExecutor::execute_buff_skill(1, 1, &caster, &target);
        assert!(result.is_none(), "非辅助技能应返回 None");
    }

    // ==================== 范围技能测试 ====================

    #[test]
    fn test_execute_area_skill_hits_multiple_targets() {
        let map_state = MapState::new();
        let caster = make_player("Caster", 100, 100);
        let target1 = make_player("Target1", 103, 100); // 距离 3
        let target2 = make_player("Target2", 105, 102); // 距离 5
        let target3 = make_player("Target3", 108, 100); // 距离 8（超出 range=9？不，在范围内）

        map_state.add_player((*caster).clone());
        map_state.add_player((*target1).clone());
        map_state.add_player((*target2).clone());
        map_state.add_player((*target3).clone());

        // 火球术 (ID=25) range=9
        let result = SkillExecutor::execute_area_skill(
            25,
            1,
            &caster,
            (100, 100), // 中心点 = 施法者位置
            &map_state,
        );

        assert!(result.is_some());
        let r = result.unwrap();
        // target1: dx=3, dy=0 -> 在范围内
        // target2: dx=5, dy=2 -> 在范围内
        // target3: dx=8, dy=0 -> 在范围内
        assert_eq!(r.hit_count, 3, "应命中 3 个目标");
        assert_eq!(r.damages.len(), 3, "应有 3 个伤害结果");

        // 每个目标应受到伤害
        for d in &r.damages {
            assert!(d.damage > 0);
        }
    }

    #[test]
    fn test_execute_area_skill_excludes_out_of_range() {
        let map_state = MapState::new();
        let caster = make_player("Caster", 100, 100);
        let near_target = make_player("Near", 105, 100);   // 距离 5，在范围内
        let far_target = make_player("Far", 200, 200);     // 距离 100，超出范围

        map_state.add_player((*caster).clone());
        map_state.add_player((*near_target).clone());
        map_state.add_player((*far_target).clone());

        let result = SkillExecutor::execute_area_skill(
            25,
            1,
            &caster,
            (100, 100),
            &map_state,
        );

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.hit_count, 1, "只应命中范围内的目标");
    }

    #[test]
    fn test_execute_area_skill_excludes_caster() {
        let map_state = MapState::new();
        let caster = make_player("Caster", 100, 100);
        let target = make_player("Target", 105, 100);

        map_state.add_player((*caster).clone());
        map_state.add_player((*target).clone());

        let result = SkillExecutor::execute_area_skill(
            25,
            1,
            &caster,
            (100, 100),
            &map_state,
        );

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.hit_count, 1, "施法者自身不应被命中");
        assert_eq!(r.damages[0].target_died, false);
    }

    #[test]
    fn test_execute_area_skill_heal_type() {
        // 范围治疗技能
        let map_state = MapState::new();
        let caster = make_player("Caster", 100, 100);
        let ally1 = make_player("Ally1", 103, 100);
        let ally2 = make_player("Ally2", 105, 100);
        ally1.combat_mut().hp = 500;
        ally2.combat_mut().hp = 300;

        map_state.add_player((*caster).clone());
        map_state.add_player((*ally1).clone());
        map_state.add_player((*ally2).clone());

        // 治愈术 (ID=28) range=9
        let result = SkillExecutor::execute_area_skill(
            28,
            1,
            &caster,
            (100, 100),
            &map_state,
        );

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.hit_count, 2, "应治疗 2 个队友");
        assert_eq!(r.heals.len(), 2);
        assert!(r.damages.is_empty(), "治疗技能不应有伤害结果");

        // 每个目标应恢复 HP
        for h in &r.heals {
            assert!(h.actual_heal > 0, "应实际恢复 HP");
        }
    }

    #[test]
    fn test_execute_area_skill_no_targets() {
        let map_state = MapState::new();
        let caster = make_player("Caster", 100, 100);
        map_state.add_player((*caster).clone());

        let result = SkillExecutor::execute_area_skill(
            25,
            1,
            &caster,
            (100, 100),
            &map_state,
        );

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.hit_count, 0, "无目标时命中数应为 0");
    }
}
