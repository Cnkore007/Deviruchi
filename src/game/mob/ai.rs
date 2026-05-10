use crate::game::battle::{BattleHandler, ExpDistributor};
use crate::game::map::data::MapDatabase;
use crate::game::map::{ChannelBus, DropManager, GameEvent, MapState};
use crate::game::mob::data::{MobSkill, MobSkillCondition, MobSkillTarget};
use crate::game::mob::droptable::{DropResolver, MVPResolver, MobDropTable};
use crate::game::mob::{Mob, MobAIState, MobBehavior, MobSpawnManager};
use crate::game::party::PartyManager;
use crate::game::rand::GameRng;
use crate::game::skill::data::{SkillDatabase, SkillType};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// 远程攻击距离阈值（格数）
/// 超过此距离的目标视为远程目标
const LONG_RANGE_THRESHOLD: u16 = 3;

/// 围攻检测所需的最小攻击者数量
const RUDE_ATTACK_MIN_ATTACKERS: usize = 2;

/// 逃跑触发阈值：HP 百分比低于此值时进入逃跑状态
const FLEE_HP_THRESHOLD: u32 = 25;

/// 逃跑恢复阈值：HP 百分比回升到此值以上时停止逃跑
const FLEE_RECOVERY_THRESHOLD: u32 = 50;

/// 检查技能触发条件是否满足
///
/// 根据技能的 condition 类型和当前战斗状态判断：
/// - Any: 无条件满足
/// - HpCertain: 当 HP 百分比低于 condition_value 时满足
/// - RudeAttacked: 被围攻时满足（攻击者数量 >= 2）
/// - LongRange: 目标距离 > 3 格时满足
fn check_skill_condition(
    skill: &MobSkill,
    hp_percent: u32,
    attacker_count: usize,
    target_distance: u16,
) -> bool {
    match skill.condition {
        MobSkillCondition::Any => true,
        MobSkillCondition::HpCertain => {
            // HP 百分比低于阈值时触发
            hp_percent <= skill.condition_value
        }
        MobSkillCondition::RudeAttacked => {
            // 被围攻条件：有 2 个及以上不同攻击者时触发
            attacker_count >= RUDE_ATTACK_MIN_ATTACKERS
        }
        MobSkillCondition::LongRange => {
            // 远程目标条件：目标距离超过阈值时触发
            target_distance > LONG_RANGE_THRESHOLD
        }
    }
}

/// 怪物AI处理器
#[allow(dead_code)]
pub struct MobAI {
    spawn_manager: Arc<MobSpawnManager>,
    channel_bus: Arc<ChannelBus>,
    drop_manager: Arc<DropManager>,
    party_manager: Arc<PartyManager>,
    map_database: Arc<MapDatabase>,
    rng: Arc<dyn GameRng>,
    battle_handler: Arc<BattleHandler>,
    skill_db: Arc<SkillDatabase>,
    drop_resolver: DropResolver,
    drop_tables: std::collections::HashMap<u16, MobDropTable>,
}

impl MobAI {
    pub fn new(
        spawn_manager: Arc<MobSpawnManager>,
        channel_bus: Arc<ChannelBus>,
        drop_manager: Arc<DropManager>,
        party_manager: Arc<PartyManager>,
        map_database: Arc<MapDatabase>,
        rng: Arc<dyn GameRng>,
        battle_handler: Arc<BattleHandler>,
        skill_db: Arc<SkillDatabase>,
    ) -> Self {
        Self {
            spawn_manager,
            channel_bus,
            drop_manager,
            party_manager,
            map_database,
            rng,
            battle_handler,
            skill_db,
            drop_resolver: DropResolver,
            drop_tables: std::collections::HashMap::new(),
        }
    }

    /// 从 YAML 文件加载掉落表
    pub fn load_drop_tables(&mut self, path: &str) {
        // DropTableLoader 加载后返回 HashMap<u32, MobDropTable>，需要转换为 u16
        let tables = crate::game::mob::droptable::DropTableLoader::load_from_yaml(path);
        self.drop_tables = tables.into_iter().map(|(k, v)| (k as u16, v)).collect();
    }

    /// 设置掉落表（用于测试）
    pub fn set_drop_tables(&mut self, tables: std::collections::HashMap<u16, MobDropTable>) {
        self.drop_tables = tables;
    }

    /// 更新怪物AI
    pub fn update(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let state = *mob.ai_state.read();

        match state {
            MobAIState::Idle => self.update_idle(mob, map_state),
            MobAIState::Patrol => self.update_patrol(mob),
            MobAIState::Chase => self.update_chase(mob, map_state),
            MobAIState::Attack => self.update_attack(mob, map_state),
            MobAIState::Return => self.update_return(mob),
            MobAIState::Flee => self.update_flee(mob, map_state),
            MobAIState::Dead => self.update_dead(mob, map_state),
        }
    }

    fn update_idle(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let (x, y) = mob.get_position();

        match mob.behavior {
            MobBehavior::Aggressive => {
                // 主动攻击：进入视野范围后追击
                let players = map_state.get_players_on_map(&mob.spawn_map);

                for player in players {
                    let (px, py) = player.get_position();
                    let distance = Self::calculate_distance(x, y, px, py);

                    if distance <= mob.sight_range {
                        *mob.ai_state.write() = MobAIState::Chase;
                        *mob.target_id.write() = Some(player.id);
                        return;
                    }
                }
            }
            MobBehavior::Passive | MobBehavior::Immobile => {
                // 被动/固定：不主动追击，被攻击时由 take_damage 或 attack handler 设置目标
            }
            MobBehavior::FleeWhenLowHp => {
                // 低血量逃跑：HP 低于 25% 时切换到逃跑状态
                let hp_pct = mob.hp_percent();
                if hp_pct <= FLEE_HP_THRESHOLD && hp_pct > 0 {
                    // 记录最后攻击者作为逃跑目标
                    let last_attacker = mob.damage_log.read().keys().next().copied();
                    if let Some(attacker_id) = last_attacker {
                        *mob.flee_from.write() = Some(attacker_id);
                        *mob.ai_state.write() = MobAIState::Flee;
                        return;
                    }
                }
            }
            MobBehavior::Assist => {
                // 主动协助：当视野内的同类怪物被攻击时，协助攻击该玩家
                // 获取同地图上所有活跃怪物
                let nearby_mobs = self.spawn_manager.get_active_mobs(&mob.spawn_map);
                for other_mob in &nearby_mobs {
                    // 跳过自己
                    if other_mob.id == mob.id {
                        continue;
                    }
                    // 检查距离是否在视野内
                    let (ox, oy) = other_mob.get_position();
                    let dist = Self::calculate_distance(x, y, ox, oy);
                    if dist > mob.sight_range {
                        continue;
                    }
                    // 检查该怪物是否正在被玩家攻击（有伤害记录且处于战斗状态）
                    let other_state = *other_mob.ai_state.read();
                    if matches!(other_state, MobAIState::Chase | MobAIState::Attack) {
                        if let Some(attacker_id) = *other_mob.target_id.read() {
                            *mob.ai_state.write() = MobAIState::Chase;
                            *mob.target_id.write() = Some(attacker_id);
                            return;
                        }
                    }
                }
            }
            MobBehavior::PassiveAssist => {
                // 被动协助：只有自身被攻击过（有伤害记录）后，才会协助其他怪物
                let has_been_attacked = !mob.damage_log.read().is_empty();
                if has_been_attacked {
                    let nearby_mobs = self.spawn_manager.get_active_mobs(&mob.spawn_map);
                    for other_mob in &nearby_mobs {
                        if other_mob.id == mob.id {
                            continue;
                        }
                        let (ox, oy) = other_mob.get_position();
                        let dist = Self::calculate_distance(x, y, ox, oy);
                        if dist > mob.sight_range {
                            continue;
                        }
                        let other_state = *other_mob.ai_state.read();
                        if matches!(other_state, MobAIState::Chase | MobAIState::Attack) {
                            if let Some(attacker_id) = *other_mob.target_id.read() {
                                *mob.ai_state.write() = MobAIState::Chase;
                                *mob.target_id.write() = Some(attacker_id);
                                return;
                            }
                        }
                    }
                }
            }
        }

        // 随机移动（Immobile 怪物不移动）
        if !matches!(mob.behavior, MobBehavior::Immobile) && self.rand_simple() < 5 {
            let new_x = (x as i32 + self.rand_offset(-3, 3)).max(0) as u16;
            let new_y = (y as i32 + self.rand_offset(-3, 3)).max(0) as u16;
            mob.move_to(new_x, new_y);
        }
    }

    fn update_patrol(&self, mob: &Arc<Mob>) {
        let (x, y) = mob.get_position();
        let new_x = (x as i32 + self.rand_offset(-5, 5)).max(0) as u16;
        let new_y = (y as i32 + self.rand_offset(-5, 5)).max(0) as u16;
        mob.move_to(new_x, new_y);
    }

    fn update_chase(&self, mob: &Arc<Mob>, map_state: &MapState) {
        // FleeWhenLowHp 行为：追击过程中 HP 过低时逃跑
        if mob.behavior == MobBehavior::FleeWhenLowHp {
            let hp_pct = mob.hp_percent();
            if hp_pct <= FLEE_HP_THRESHOLD && hp_pct > 0 {
                let target = *mob.target_id.read();
                *mob.flee_from.write() = target;
                *mob.ai_state.write() = MobAIState::Flee;
                return;
            }
        }

        let target_id = *mob.target_id.read();

        if let Some(target_id) = target_id {
            if let Some(target) = map_state.get_player(&target_id) {
                let (x, y) = mob.get_position();
                let (tx, ty) = target.get_position();
                let distance = Self::calculate_distance(x, y, tx, ty);

                if distance <= mob.atk_range {
                    *mob.ai_state.write() = MobAIState::Attack;
                } else if distance > mob.chase_range {
                    *mob.ai_state.write() = MobAIState::Return;
                    *mob.target_id.write() = None;
                    mob.path_manager.write().stop_chase();
                } else {
                    // 使用 MobPathManager 寻路
                    let mut pm = mob.path_manager.write();

                    // 检查是否需要重新计算路径
                    let needs_recalc =
                        !pm.is_chasing || pm.target_pos != Some((tx, ty)) || pm.path_invalid;

                    if needs_recalc {
                        // 获取地图数据
                        if let Some(map_data) = self.map_database.get(&mob.map_name) {
                            // 创建可通行性检查闭包
                            let is_walkable =
                                |wx: u16, wy: u16| -> bool { map_data.is_walkable(wx, wy) };

                            // 计算新路径
                            if let Some(path) = crate::game::mob::pathfinder::Pathfinder::find_path(
                                (x, y),
                                (tx, ty),
                                is_walkable,
                                mob.chase_range,
                            ) {
                                pm.start_chase((tx, ty));
                                pm.set_path(path);
                            } else {
                                // 无法找到路径，停止追击
                                pm.stop_chase();
                                drop(pm);
                                *mob.ai_state.write() = MobAIState::Return;
                                *mob.target_id.write() = None;
                                return;
                            }
                        } else {
                            // 没有地图数据，使用简单追击
                            pm.stop_chase();
                        }
                    }

                    // 沿路径移动一步
                    let mut pm = mob.path_manager.write();
                    if let Some(next_pos) = pm.advance_step() {
                        let (nx, ny) = next_pos;
                        // 验证下一步可通行（双重检查）
                        if let Some(map_data) = self.map_database.get(&mob.map_name) {
                            if map_data.is_walkable(nx, ny) {
                                mob.move_to(nx, ny);
                            } else {
                                // 路径失效，需要下次重算
                                pm.invalidate();
                            }
                        }
                    }
                }
            } else {
                *mob.ai_state.write() = MobAIState::Return;
                *mob.target_id.write() = None;
                mob.path_manager.write().stop_chase();
            }
        }
    }

    fn update_attack(&self, mob: &Arc<Mob>, map_state: &MapState) {
        // FleeWhenLowHp 行为：战斗中 HP 过低时逃跑
        if mob.behavior == MobBehavior::FleeWhenLowHp {
            let hp_pct = mob.hp_percent();
            if hp_pct <= FLEE_HP_THRESHOLD && hp_pct > 0 {
                let target = *mob.target_id.read();
                *mob.flee_from.write() = target;
                *mob.ai_state.write() = MobAIState::Flee;
                return;
            }
        }

        let target_id = *mob.target_id.read();

        if let Some(target_id) = target_id {
            // 优先尝试使用怪物技能
            // 如果技能触发成功，本 tick 不执行普通攻击
            if self.try_use_skill(mob, target_id, map_state) {
                return;
            }

            if let Some(target) = map_state.get_player(&target_id) {
                let result = self.battle_handler.mob_attack(mob, &target);

                match result {
                    crate::game::battle::handler::MobAttackResult::Miss => {
                        // 攻击未命中，不做任何处理
                    }
                    crate::game::battle::handler::MobAttackResult::Hit { damage, killed } => {
                        // 记录对玩家的伤害（用于确定 MVP）
                        if damage > 0 {
                            let mut damage_log = mob.damage_log.write();
                            let entry = damage_log.entry(target_id).or_insert(0);
                            *entry += damage as u64;
                        }

                        if killed {
                            // 发布玩家死亡事件
                            let channel_name = format!("map:{}", target.map_name);
                            let event = GameEvent::PlayerDeath {
                                player_id: target_id,
                            };
                            let packet = event.to_packet_bytes();
                            self.channel_bus.publish(&channel_name, &event, packet);

                            *mob.ai_state.write() = MobAIState::Idle;
                            *mob.target_id.write() = None;
                        }
                    }
                }
            } else {
                *mob.ai_state.write() = MobAIState::Return;
                *mob.target_id.write() = None;
            }
        }
    }

    /// 尝试使用怪物技能
    ///
    /// 遍历 mob 的技能列表，检查条件（概率、HP、冷却），
    /// 如果触发了技能则执行效果并返回 true，否则返回 false。
    fn try_use_skill(&self, mob: &Arc<Mob>, target_id: Uuid, map_state: &MapState) -> bool {
        if mob.skills.is_empty() {
            return false;
        }

        let hp_percent = mob.hp_percent();
        // 计算攻击者数量（用于 RudeAttacked 条件）
        let attacker_count = mob.dmglog.read().len();
        // 计算目标距离（用于 LongRange 条件）
        let target_distance = if let Some(target) = map_state.get_player(&target_id) {
            let (mx, my) = mob.get_position();
            let (tx, ty) = target.get_position();
            Self::calculate_distance(mx, my, tx, ty)
        } else {
            0
        };

        for skill in &mob.skills {
            // 检查冷却时间
            if !self.is_skill_ready(mob, skill) {
                continue;
            }

            // 检查触发条件
            if !check_skill_condition(skill, hp_percent, attacker_count, target_distance) {
                continue;
            }

            // 概率判定（万分比）
            let roll = self.rng.rand_range(0, 9999);
            if roll >= skill.chance {
                continue;
            }

            // 技能触发！执行效果
            self.execute_mob_skill(mob, skill, target_id, map_state);

            // 设置冷却
            if skill.cooldown_ms > 0 {
                mob.skill_cooldowns
                    .write()
                    .insert(skill.skill_id, Instant::now());
            }

            return true;
        }

        false
    }

    /// 检查技能是否冷却完成
    fn is_skill_ready(&self, mob: &Arc<Mob>, skill: &MobSkill) -> bool {
        let cooldowns = mob.skill_cooldowns.read();
        match cooldowns.get(&skill.skill_id) {
            Some(last_used) => {
                let elapsed = Instant::now().duration_since(*last_used);
                elapsed.as_millis() as u64 >= skill.cooldown_ms
            }
            None => true, // 从未使用过，可以使用
        }
    }

    /// 执行怪物技能效果
    ///
    /// 根据技能的目标类型和技能数据库中的类型来决定效果：
    /// - 攻击类技能：对目标造成基于 mob ATK 的额外伤害
    /// - 治疗类技能：恢复自身 HP
    /// - 辅助类技能：暂时简化为自我治疗
    fn execute_mob_skill(
        &self,
        mob: &Arc<Mob>,
        skill: &MobSkill,
        target_id: Uuid,
        map_state: &MapState,
    ) {
        let skill_type = self.skill_db.get(skill.skill_id).map(|s| s.type_);

        match skill.target {
            MobSkillTarget::Self_ => {
                // 对自身使用：治疗或增益
                self.execute_self_skill(mob, skill, skill_type);
            }
            MobSkillTarget::Target => {
                // 对目标使用：攻击或减益
                self.execute_target_skill(mob, skill, skill_type, target_id, map_state);
            }
        }
    }

    /// 执行对自身使用的技能（治疗/增益）
    fn execute_self_skill(
        &self,
        mob: &Arc<Mob>,
        skill: &MobSkill,
        skill_type: Option<SkillType>,
    ) {
        // 未知技能跳过执行，避免误判类型
        if skill_type.is_none() {
            tracing::warn!(
                "怪物 {}({}) 使用未知技能 ID={}，跳过执行",
                mob.name, mob.mob_id, skill.skill_id
            );
            return;
        }

        let is_heal = matches!(skill_type, Some(SkillType::Healing));

        if is_heal {
            // 治疗效果：恢复 max_hp 的 (level * 5)% ，至少恢复 1 点
            let heal_percent = (skill.level as u32).saturating_mul(5).max(1);
            let heal_amount = (mob.max_hp as u64 * heal_percent as u64 / 100) as u32;
            let heal_amount = heal_amount.max(1);
            mob.heal(heal_amount);

            tracing::debug!(
                "怪物 {}({}) 使用技能 ID={} Lv={} 恢复了 {} HP (当前 HP: {}/{})",
                mob.name,
                mob.mob_id,
                skill.skill_id,
                skill.level,
                heal_amount,
                mob.hp.read(),
                mob.max_hp
            );
        } else {
            // 辅助/增益类技能：当前简化处理，记录日志
            tracing::debug!(
                "怪物 {}({}) 使用辅助技能 ID={} Lv={}（效果暂未实现）",
                mob.name,
                mob.mob_id,
                skill.skill_id,
                skill.level
            );
        }
    }

    /// 执行对目标使用的技能（攻击/减益）
    fn execute_target_skill(
        &self,
        mob: &Arc<Mob>,
        skill: &MobSkill,
        skill_type: Option<SkillType>,
        target_id: Uuid,
        map_state: &MapState,
    ) {
        // 未知技能跳过执行，避免误判类型
        if skill_type.is_none() {
            tracing::warn!(
                "怪物 {}({}) 对目标使用未知技能 ID={}，跳过执行",
                mob.name, mob.mob_id, skill.skill_id
            );
            return;
        }

        if let Some(target) = map_state.get_player(&target_id) {
            let is_attack = matches!(skill_type, Some(SkillType::Attack))
                || matches!(skill_type, Some(SkillType::Debuff));

            if is_attack {
                // 攻击技能：基于 mob ATK 计算额外伤害
                // 伤害公式: ATK * (100 + level * 20) / 100
                let multiplier = 100i32 + (skill.level as i32) * 20;
                let damage = ((mob.atk as i64) * multiplier as i64 / 100).max(1) as i32;
                let killed = target.take_damage(damage.max(0) as u32);

                // 记录伤害
                if damage > 0 {
                    let mut damage_log = mob.damage_log.write();
                    let entry = damage_log.entry(target_id).or_insert(0);
                    *entry += damage.max(0) as u64;
                }

                tracing::debug!(
                    "怪物 {}({}) 使用攻击技能 ID={} Lv={} 对 {} 造成 {} 伤害",
                    mob.name,
                    mob.mob_id,
                    skill.skill_id,
                    skill.level,
                    target.name,
                    damage
                );

                if killed {
                    let channel_name = format!("map:{}", target.map_name);
                    let event = GameEvent::PlayerDeath {
                        player_id: target_id,
                    };
                    let packet = event.to_packet_bytes();
                    self.channel_bus.publish(&channel_name, &event, packet);

                    *mob.ai_state.write() = MobAIState::Idle;
                    *mob.target_id.write() = None;
                }
            } else {
                // 其他类型暂不实现
                tracing::debug!(
                    "怪物 {}({}) 使用技能 ID={} Lv={}（效果暂未实现）",
                    mob.name,
                    mob.mob_id,
                    skill.skill_id,
                    skill.level
                );
            }
        }
    }

    fn update_return(&self, mob: &Arc<Mob>) {
        *mob.ai_state.write() = MobAIState::Idle;
    }

    /// 逃跑状态更新
    ///
    /// FleeWhenLowHp 怪物的逃跑逻辑：
    /// 1. 如果 HP 恢复到 50% 以上，停止逃跑回到 Idle
    /// 2. 否则远离攻击者方向移动
    fn update_flee(&self, mob: &Arc<Mob>, map_state: &MapState) {
        // 检查 HP 是否已恢复到可以停止逃跑的阈值
        let hp_pct = mob.hp_percent();
        if hp_pct >= FLEE_RECOVERY_THRESHOLD {
            *mob.ai_state.write() = MobAIState::Idle;
            *mob.flee_from.write() = None;
            mob.path_manager.write().stop_chase();
            return;
        }

        let (x, y) = mob.get_position();
        let flee_from_id = *mob.flee_from.read();

        // 获取逃跑方向（远离攻击者）
        if let Some(attacker_id) = flee_from_id {
            if let Some(attacker) = map_state.get_player(&attacker_id) {
                let (ax, ay) = attacker.get_position();
                // 计算远离方向：从攻击者位置指向怪物位置
                let dx = x as i32 - ax as i32;
                let dy = y as i32 - ay as i32;
                let dist = Self::calculate_distance(x, y, ax, ay);

                if dist > 0 {
                    // 归一化方向后移动 2 格
                    let move_x = (dx as f64 / dist as f64 * 2.0) as i32;
                    let move_y = (dy as f64 / dist as f64 * 2.0) as i32;
                    let new_x = (x as i32 + move_x).max(0) as u16;
                    let new_y = (y as i32 + move_y).max(0) as u16;
                    mob.move_to(new_x, new_y);
                }
            } else {
                // 攻击者已不在地图上，停止逃跑
                *mob.ai_state.write() = MobAIState::Idle;
                *mob.flee_from.write() = None;
            }
        } else {
            // 没有逃跑目标，回到 Idle
            *mob.ai_state.write() = MobAIState::Idle;
        }
    }

    fn update_dead(&self, mob: &Arc<Mob>, map_state: &MapState) {
        // 首次死亡处理：发布事件 + 掉落 + 经验
        if !*mob.drops_processed.read() {
            *mob.drops_processed.write() = true;

            let killer_id = mob.target_id.read().unwrap_or(Uuid::nil());

            // 发布 MobDeath 事件
            let channel_name = format!("map:{}", mob.spawn_map);
            let event = GameEvent::MobDeath {
                mob_id: mob.id,
                killer_id,
            };
            let packet = event.to_packet_bytes();
            self.channel_bus.publish(&channel_name, &event, packet);

            // 使用 DropTableResolver 计算掉落
            self.process_drops_with_resolver(mob);

            // 从伤害记录中确定 MVP
            let mvp_id = {
                let damage_log = mob.damage_log.read();
                MVPResolver::pick_mvp(&damage_log)
            };

            // 分发经验给击杀者及其队伍（含 MVP 加成）
            if !killer_id.is_nil() {
                ExpDistributor::distribute_mob_exp(
                    map_state,
                    &self.party_manager,
                    killer_id,
                    mvp_id,
                    mob.level,
                    mob.base_exp,
                    mob.job_exp,
                );

                // 分发 Zeny 掉落
                // TODO: 从掉落表或 mob 数据中获取 zeny_amount
                let zeny_amount = mob.zeny.unwrap_or(0);
                if zeny_amount > 0 {
                    ExpDistributor::distribute_zeny(
                        map_state,
                        &self.party_manager,
                        killer_id,
                        mob.level,
                        zeny_amount,
                    );
                }
            }

            // 清空伤害记录
            mob.damage_log.write().clear();
        }

        // 检查是否可以重生
        let should_respawn = mob.death_time.read().is_some_and(|death_time| {
            Instant::now().duration_since(death_time)
                >= Duration::from_millis(mob.respawn_time as u64)
        });

        if should_respawn {
            mob.respawn();
        }
    }

    /// 处理怪物掉落
    #[allow(dead_code)]
    fn process_drops(&self, mob: &Arc<Mob>) {
        if mob.drops.is_empty() {
            return;
        }

        let (mob_x, mob_y) = mob.get_position();
        let channel_name = format!("map:{}", mob.spawn_map);

        for drop in &mob.drops {
            let roll = self.rand_u32() % 10000;
            if roll < drop.chance {
                let amount = if drop.min_amount >= drop.max_amount {
                    drop.min_amount
                } else {
                    drop.min_amount
                        + (self.rand_u32() % ((drop.max_amount - drop.min_amount + 1) as u32))
                            as u16
                };

                // 添加掉落物到地图
                self.drop_manager
                    .add(drop.item_id, amount, mob_x, mob_y, &mob.spawn_map);

                // 发布 ItemDrop 事件
                let drop_event = GameEvent::ItemDrop {
                    item_id: drop.item_id,
                    x: mob_x,
                    y: mob_y,
                    amount,
                };
                let packet = drop_event.to_packet_bytes();
                self.channel_bus.publish(&channel_name, &drop_event, packet);
            }
        }
    }

    /// 使用 DropTableResolver 处理怪物掉落
    ///
    /// 根据 mob_id 获取掉落表，使用 DropResolver 解析掉落，
    /// 并通过 DropManager::add_with_broadcast 发布掉落事件。
    fn process_drops_with_resolver(&self, mob: &Arc<Mob>) {
        // 获取该怪物类型的掉落表
        let table = match self.drop_tables.get(&mob.mob_id) {
            Some(t) => t,
            None => return, // 没有掉落表，不掉落
        };

        // 从伤害记录中确定 MVP
        let damage_log = mob.damage_log.read();
        let mvp_id = MVPResolver::pick_mvp(&damage_log);
        drop(damage_log);

        // 使用 DropResolver 解析掉落
        let drops = self
            .drop_resolver
            .resolve(table, self.rng.as_ref(), mob.level, mvp_id);

        let (mob_x, mob_y) = mob.get_position();

        // 使用 add_with_broadcast 添加掉落物并发布事件
        for drop in drops {
            self.drop_manager.add_with_broadcast(
                drop.item_id,
                drop.amount,
                mob_x,
                mob_y,
                &mob.spawn_map,
                &self.channel_bus,
            );
        }
    }

    fn calculate_distance(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
        let dx = (x1 as i32 - x2 as i32).abs();
        let dy = (y1 as i32 - y2 as i32).abs();
        ((dx * dx + dy * dy) as f32).sqrt() as u16
    }

    /// 返回 [0, 100) 范围内的随机数
    fn rand_simple(&self) -> i32 {
        (self.rng.rand_range(0, 99)) as i32
    }

    /// 返回随机 u32
    #[allow(dead_code)]
    fn rand_u32(&self) -> u32 {
        self.rng.rand_range(u32::MIN, u32::MAX)
    }

    /// 返回 [min, max] 范围内的随机偏移
    fn rand_offset(&self, min: i32, max: i32) -> i32 {
        let range = (max - min + 1) as u32;
        let val = self.rng.rand_range(0, range - 1);
        min + (val as i32)
    }
}

impl Default for MobAI {
    fn default() -> Self {
        Self::new(
            Arc::new(MobSpawnManager::new()),
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            crate::game::rand::thread_rng(),
            Arc::new(BattleHandler::default()),
            Arc::new(SkillDatabase::new()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::constants;
    use crate::game::map::{MapState, Player};
    use crate::game::mob::data::{MobPathManager, MobPosition, MobSkill, MobSkillCondition, MobSkillTarget};
    use crate::game::rand::{GameRng, MockRng};

    fn create_test_mob_ai(values: Vec<u32>) -> MobAI {
        MobAI::new(
            Arc::new(MobSpawnManager::new()),
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(values)),
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        )
    }

    #[test]
    fn test_rand_offset_range() {
        let ai = create_test_mob_ai(vec![0, 1, 2, 3, 4, 5]);
        // Mock returns 0, so offset should be -3 + 0 = -3
        assert_eq!(ai.rand_offset(-3, 3), -3);
        // Mock returns 1, so offset should be -3 + 1 = -2
        assert_eq!(ai.rand_offset(-3, 3), -2);
    }

    #[test]
    fn test_rand_simple_range() {
        let ai = create_test_mob_ai(vec![0, 99, 50]);
        assert_eq!(ai.rand_simple(), 0);
        assert_eq!(ai.rand_simple(), 99);
        assert_eq!(ai.rand_simple(), 50);
    }

    // ============================================
    // MobAI State Machine Tests
    // ============================================

    fn create_test_mob(position: (u16, u16), level: u16) -> Arc<Mob> {
        Arc::new(Mob {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
            mob_id: 1001,
            name: "TestMob".to_string(),
            pos: parking_lot::RwLock::new(MobPosition { x: position.0, y: position.1 }),
            map_name: "test_map".to_string(),
            level,
            hp: parking_lot::RwLock::new(500),
            max_hp: 500,
            sp: parking_lot::RwLock::new(0),
            max_sp: 0,
            atk: 30,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 5,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            race: crate::game::mob::MobRace::Formless,
            mob_type: crate::game::mob::MobType::Normal,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Aggressive,
            skills: Vec::new(),
            skill_cooldowns: parking_lot::RwLock::new(std::collections::HashMap::new()),
            sight_range: 10,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: position.0,
            spawn_y: position.1,
            spawn_map: "test_map".to_string(),
            death_time: parking_lot::RwLock::new(None),
            drops: vec![],
            base_exp: 10,
            job_exp: 5,
            zeny: Some(100),
            drops_processed: parking_lot::RwLock::new(false),
            path_manager: parking_lot::RwLock::new(MobPathManager::new()),
            damage_log: parking_lot::RwLock::new(std::collections::HashMap::new()),
            dmglog: parking_lot::RwLock::new(std::collections::HashMap::new()),
            flee_from: parking_lot::RwLock::new(None),
        })
    }

    fn create_test_player(position: (u16, u16), level: u16) -> Arc<Player> {
        Arc::new(Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            map_name: "test_map".to_string(),
            combat: parking_lot::RwLock::new(crate::game::map::player::CombatStats {
                hp: 1000,
                max_hp: 1000,
                sp: 100,
                max_sp: 100,
                state: crate::game::map::player::PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: parking_lot::RwLock::new(crate::game::map::player::Position { x: position.0, y: position.1 }),
            level: parking_lot::RwLock::new(crate::game::map::player::LevelStats {
                base_level: level,
                job_level: 5,
                base_exp: 0,
                job_exp: 0,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: parking_lot::RwLock::new(crate::game::map::player::Attributes {
                str: 10,
                agi: 10,
                vit: 10,
                int: 10,
                dex: 10,
                luk: 10,
            }),
            economy: parking_lot::RwLock::new(crate::game::map::player::Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: parking_lot::RwLock::new(crate::game::map::player::SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: parking_lot::RwLock::new(crate::game::item::Equipment::new()),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            inventory: parking_lot::RwLock::new(Vec::new()),
            hotkeys: parking_lot::RwLock::new(Vec::new()),
        })
    }

    fn create_test_map_state() -> Arc<MapState> {
        Arc::new(MapState::new())
    }

    fn create_test_mob_ai_with_map(
        values: Vec<u32>,
        map_state: Arc<MapState>,
    ) -> (MobAI, Arc<MapState>) {
        (
            MobAI::new(
                Arc::new(MobSpawnManager::new()),
                Arc::new(ChannelBus::new()),
                Arc::new(DropManager::new()),
                Arc::new(PartyManager::new()),
                Arc::new(MapDatabase::new()),
                Arc::new(MockRng::new(values)),
                Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
                Arc::new(SkillDatabase::new()),
            ),
            map_state,
        )
    }

    #[test]
    fn idle_to_chase_when_player_in_sight() {
        // Mob at (100, 100), Player at (105, 105) - distance = sqrt(25+25) = 7.07 < 10 (sight range)
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((105, 105), 10);

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Verify initial state is Idle
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        // Create AI and update
        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should transition to Chase
        assert_eq!(*mob.ai_state.read(), MobAIState::Chase);
        assert_eq!(*mob.target_id.read(), Some(player.id));
    }

    #[test]
    fn passive_mob_stays_idle_when_player_in_sight() {
        // Passive mob should NOT chase players even when in sight range
        let position = (100u16, 100u16);
        let mob = Arc::new(Mob {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
            mob_id: 1001,
            name: "PassiveMob".to_string(),
            pos: parking_lot::RwLock::new(MobPosition { x: position.0, y: position.1 }),
            map_name: "test_map".to_string(),
            level: 5,
            hp: parking_lot::RwLock::new(500),
            max_hp: 500,
            sp: parking_lot::RwLock::new(0),
            max_sp: 0,
            atk: 30,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 5,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            race: crate::game::mob::MobRace::Formless,
            mob_type: crate::game::mob::MobType::Normal,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Passive,
            skills: Vec::new(),
            skill_cooldowns: parking_lot::RwLock::new(std::collections::HashMap::new()),
            sight_range: 10,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: position.0,
            spawn_y: position.1,
            spawn_map: "test_map".to_string(),
            death_time: parking_lot::RwLock::new(None),
            drops: vec![],
            base_exp: 10,
            job_exp: 5,
            zeny: Some(100),
            drops_processed: parking_lot::RwLock::new(false),
            path_manager: parking_lot::RwLock::new(MobPathManager::new()),
            damage_log: parking_lot::RwLock::new(std::collections::HashMap::new()),
            dmglog: parking_lot::RwLock::new(std::collections::HashMap::new()),
            flee_from: parking_lot::RwLock::new(None),
        });


        let player = create_test_player((105, 105), 10); // Distance = 7.07 < sight_range

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![10], map_state);
        ai.update(&mob, &map_state);

        // Passive mob should stay Idle, not chase
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
        assert_eq!(*mob.target_id.read(), None);
    }

    #[test]
    fn idle_stays_idle_when_no_player_in_sight() {
        // Mob at (100, 100), Player at (200, 200) - distance > sight range
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((200, 200), 10);

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Verify initial state is Idle
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        // Create AI with random > 5 to prevent random movement
        let (ai, map_state) = create_test_mob_ai_with_map(vec![10], map_state);
        ai.update(&mob, &map_state);

        // Should stay Idle (no player in sight)
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
    }

    #[test]
    fn chase_to_attack_in_range() {
        // Mob in Chase state, Player is now in attack range
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((100, 101), 10); // Distance = 1, within atk_range

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Set mob to Chase state with target
        *mob.ai_state.write() = MobAIState::Chase;
        *mob.target_id.write() = Some(player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should transition to Attack
        assert_eq!(*mob.ai_state.read(), MobAIState::Attack);
    }

    #[test]
    fn chase_to_return_when_player_out_of_sight() {
        // Mob in Chase state, Player is out of chase range
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((200, 200), 10); // Distance = 141.4 > 20 (chase range)

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Set mob to Chase state with target
        *mob.ai_state.write() = MobAIState::Chase;
        *mob.target_id.write() = Some(player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should transition to Return
        assert_eq!(*mob.ai_state.read(), MobAIState::Return);
        assert_eq!(*mob.target_id.read(), None);
    }

    #[test]
    fn chase_returns_when_target_disappears() {
        // Mob in Chase state but target player is removed from map
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((105, 105), 10);

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Set mob to Chase state with target
        *mob.ai_state.write() = MobAIState::Chase;
        *mob.target_id.write() = Some(player.id);

        // Remove player from map
        map_state.remove_player(&player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should transition to Return
        assert_eq!(*mob.ai_state.read(), MobAIState::Return);
        assert_eq!(*mob.target_id.read(), None);
    }

    #[test]
    fn attack_to_dead_when_hp_zero() {
        // Mob in Attack state, HP reduced to 0
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((100, 101), 10);

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Set mob to Attack state with target
        *mob.ai_state.write() = MobAIState::Attack;
        *mob.target_id.write() = Some(player.id);

        // Reduce HP to 0
        *mob.hp.write() = 0;
        *mob.ai_state.write() = MobAIState::Dead;
        *mob.death_time.write() = Some(std::time::Instant::now());

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should remain Dead
        assert_eq!(*mob.ai_state.read(), MobAIState::Dead);
    }

    #[test]
    fn attack_to_return_when_target_removed() {
        // Mob in Attack state but target player is removed from map
        // Note: MobAI doesn't check if target is alive, only if they're in the map
        // So we remove the player from map entirely to test target disappearance
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((100, 101), 10);

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        // Set mob to Attack state with target
        *mob.ai_state.write() = MobAIState::Attack;
        *mob.target_id.write() = Some(player.id);

        // Remove player from map (simulates player disconnecting)
        map_state.remove_player(&player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should transition to Return (no valid target in map)
        assert_eq!(*mob.ai_state.read(), MobAIState::Return);
    }

    #[test]
    fn return_transitions_to_idle() {
        // Mob in Return state should go back to Idle
        let mob = create_test_mob((100, 100), 5);

        *mob.ai_state.write() = MobAIState::Return;

        let map_state = create_test_map_state();
        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Return should transition to Idle
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
    }

    #[test]
    fn mob_prioritizes_closest_player() {
        // Two players, mob should chase the closer one
        let mob = create_test_mob((100, 100), 5);
        let player1 = create_test_player((105, 105), 10); // Distance = 7.07
        let player2 = create_test_player((150, 150), 10); // Distance = 70.71

        let map_state = create_test_map_state();
        let player1_for_map = (*player1).clone();
        let player2_for_map = (*player2).clone();
        map_state.add_player(player1_for_map);
        map_state.add_player(player2_for_map);

        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // Should chase the closer player
        assert_eq!(*mob.ai_state.read(), MobAIState::Chase);
        assert_eq!(*mob.target_id.read(), Some(player1.id));
    }

    #[test]
    fn mob_picks_any_player_in_sight() {
        // Only one player in sight, within sight_range
        // Mob at (100, 100), sight_range = 10
        // Player at (108, 105) -> distance = sqrt(64+25) = sqrt(89) = 9.43 < 10 (within sight_range)
        let mob = create_test_mob((100, 100), 5);
        let player = create_test_player((108, 105), 10);

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        assert_eq!(*mob.ai_state.read(), MobAIState::Chase);
        assert_eq!(*mob.target_id.read(), Some(player.id));
    }

    #[test]
    fn mob_ignores_players_on_different_map() {
        // Player on different map should not be detected
        let mob = create_test_mob((100, 100), 5);

        // Create player directly on "other_map"
        let player = Arc::new(Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "OtherMapPlayer".to_string(),
            map_name: "other_map".to_string(), // Different map
            combat: parking_lot::RwLock::new(crate::game::map::player::CombatStats {
                hp: 1000,
                max_hp: 1000,
                sp: 100,
                max_sp: 100,
                state: crate::game::map::player::PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: parking_lot::RwLock::new(crate::game::map::player::Position { x: 105, y: 105 }),
            level: parking_lot::RwLock::new(crate::game::map::player::LevelStats {
                base_level: 10,
                job_level: 5,
                base_exp: 0,
                job_exp: 0,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: parking_lot::RwLock::new(crate::game::map::player::Attributes {
                str: 10,
                agi: 10,
                vit: 10,
                int: 10,
                dex: 10,
                luk: 10,
            }),
            economy: parking_lot::RwLock::new(crate::game::map::player::Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: parking_lot::RwLock::new(crate::game::map::player::SavePoint {
                map: "other_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: parking_lot::RwLock::new(crate::game::item::Equipment::new()),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            inventory: parking_lot::RwLock::new(Vec::new()),
            hotkeys: parking_lot::RwLock::new(Vec::new()),
        });

        let map_state = create_test_map_state();
        let player_for_map = (*player).clone();
        map_state.add_player(player_for_map);

        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);

        // Use random > 5 to prevent random movement
        let (ai, map_state) = create_test_mob_ai_with_map(vec![50], map_state);
        ai.update(&mob, &map_state);

        // Should stay Idle (player on different map)
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
    }

    // ============================================
    // 怪物技能系统测试
    // ============================================

    /// 创建带有技能的测试怪物
    fn create_test_mob_with_skills(
        position: (u16, u16),
        level: u16,
        skills: Vec<MobSkill>,
    ) -> Arc<Mob> {
        Arc::new(Mob {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
            mob_id: 1002,
            name: "SkilledMob".to_string(),
            pos: parking_lot::RwLock::new(MobPosition { x: position.0, y: position.1 }),
            map_name: "test_map".to_string(),
            level,
            hp: parking_lot::RwLock::new(500),
            max_hp: 500,
            sp: parking_lot::RwLock::new(0),
            max_sp: 0,
            atk: 30,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 5,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            race: crate::game::mob::MobRace::Formless,
            mob_type: crate::game::mob::MobType::Normal,
            ai_state: parking_lot::RwLock::new(MobAIState::Attack),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Aggressive,
            skills,
            skill_cooldowns: parking_lot::RwLock::new(std::collections::HashMap::new()),
            sight_range: 10,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: position.0,
            spawn_y: position.1,
            spawn_map: "test_map".to_string(),
            death_time: parking_lot::RwLock::new(None),
            drops: vec![],
            base_exp: 10,
            job_exp: 5,
            zeny: Some(100),
            drops_processed: parking_lot::RwLock::new(false),
            path_manager: parking_lot::RwLock::new(MobPathManager::new()),
            damage_log: parking_lot::RwLock::new(std::collections::HashMap::new()),
            dmglog: parking_lot::RwLock::new(std::collections::HashMap::new()),
            flee_from: parking_lot::RwLock::new(None),
        })
    }

    #[test]
    fn test_check_skill_condition_any() {
        let skill = MobSkill {
            skill_id: 5,
            level: 1,
            chance: 1000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::Any,
            condition_value: 0,
            cooldown_ms: 0,
        };
        // Any 条件始终满足（attacker_count 和 target_distance 不影响结果）
        assert!(check_skill_condition(&skill, 100, 0, 0));
        assert!(check_skill_condition(&skill, 50, 3, 10));
        assert!(check_skill_condition(&skill, 0, 0, 0));
    }

    #[test]
    fn test_check_skill_condition_hp_certain() {
        let skill = MobSkill {
            skill_id: 28,
            level: 3,
            chance: 1000,
            target: MobSkillTarget::Self_,
            condition: MobSkillCondition::HpCertain,
            condition_value: 50,
            cooldown_ms: 10000,
        };
        // HP 低于 50% 时满足
        assert!(!check_skill_condition(&skill, 100, 0, 0));
        assert!(!check_skill_condition(&skill, 51, 0, 0));
        assert!(check_skill_condition(&skill, 50, 0, 0));
        assert!(check_skill_condition(&skill, 25, 0, 0));
        assert!(check_skill_condition(&skill, 1, 0, 0));
    }

    #[test]
    fn test_is_skill_ready_no_cooldown() {
        let ai = create_test_mob_ai(vec![]);
        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 5,
                level: 1,
                chance: 1000,
                target: MobSkillTarget::Target,
                condition: MobSkillCondition::Any,
                condition_value: 0,
                cooldown_ms: 5000,
            },
        ]);

        // 从未使用过，应该可用
        assert!(ai.is_skill_ready(&mob, &mob.skills[0]));
    }

    #[test]
    fn test_is_skill_ready_on_cooldown() {
        let ai = create_test_mob_ai(vec![]);
        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 5,
                level: 1,
                chance: 1000,
                target: MobSkillTarget::Target,
                condition: MobSkillCondition::Any,
                condition_value: 0,
                cooldown_ms: 60000, // 60秒冷却
            },
        ]);

        // 刚刚使用过
        mob.skill_cooldowns.write().insert(5, Instant::now());
        assert!(!ai.is_skill_ready(&mob, &mob.skills[0]));
    }

    #[test]
    fn test_try_use_skill_no_skills() {
        let ai = create_test_mob_ai(vec![]);
        let mob = create_test_mob_with_skills((100, 100), 5, vec![]);
        let player = create_test_player((100, 101), 10);
        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 没有技能时应该返回 false
        assert!(!ai.try_use_skill(&mob, player.id, &map_state));
    }

    #[test]
    fn test_try_use_skill_triggers_on_high_chance() {
        // 使用 MockRng 返回 0（在 0-9999 范围内，0 < 10000 概率）
        let ai = MobAI::new(
            Arc::new(MobSpawnManager::new()),
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(vec![0])), // roll = 0 < 10000
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        );

        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 5,
                level: 3,
                chance: 10000, // 100% 概率
                target: MobSkillTarget::Target,
                condition: MobSkillCondition::Any,
                condition_value: 0,
                cooldown_ms: 0, // 无冷却
            },
        ]);

        let player = create_test_player((100, 101), 10);
        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 100% 概率 + 无冷却 + Any 条件 = 一定触发
        assert!(ai.try_use_skill(&mob, player.id, &map_state));
    }

    #[test]
    fn test_try_use_skill_does_not_trigger_on_zero_chance() {
        let ai = create_test_mob_ai(vec![5000]); // roll = 5000 > 0 概率

        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 5,
                level: 3,
                chance: 0, // 0% 概率
                target: MobSkillTarget::Target,
                condition: MobSkillCondition::Any,
                condition_value: 0,
                cooldown_ms: 0,
            },
        ]);

        let player = create_test_player((100, 101), 10);
        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 0% 概率 = 永远不触发
        assert!(!ai.try_use_skill(&mob, player.id, &map_state));
    }

    #[test]
    fn test_try_use_skill_hp_condition_blocks() {
        let ai = create_test_mob_ai(vec![0]);

        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 28,
                level: 3,
                chance: 10000, // 100%
                target: MobSkillTarget::Self_,
                condition: MobSkillCondition::HpCertain,
                condition_value: 50, // HP 低于 50% 才触发
                cooldown_ms: 0,
            },
        ]);

        let player = create_test_player((100, 101), 10);
        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // HP 100% (500/500) > 50%，不应触发
        assert!(!ai.try_use_skill(&mob, player.id, &map_state));

        // 降低 HP 到 50% 以下
        *mob.hp.write() = 200; // 40%
        assert!(ai.try_use_skill(&mob, player.id, &map_state));
    }

    #[test]
    fn test_try_use_skill_cooldown_blocks() {
        let ai = create_test_mob_ai(vec![0]);

        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 5,
                level: 3,
                chance: 10000, // 100%
                target: MobSkillTarget::Target,
                condition: MobSkillCondition::Any,
                condition_value: 0,
                cooldown_ms: 60000, // 60 秒冷却
            },
        ]);

        let player = create_test_player((100, 101), 10);
        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 首次使用应该成功
        assert!(ai.try_use_skill(&mob, player.id, &map_state));

        // 冷却中，不应再次触发
        assert!(!ai.try_use_skill(&mob, player.id, &map_state));
    }

    #[test]
    fn test_update_attack_uses_skill_before_normal_attack() {
        // 创建一个 100% 触发攻击技能的 mob（技能有冷却，普通攻击无冷却）
        let ai = MobAI::new(
            Arc::new(MobSpawnManager::new()),
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(vec![0])), // roll = 0 < 10000
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        );

        let mob = create_test_mob_with_skills((100, 100), 5, vec![
            MobSkill {
                skill_id: 1,    // Bash (硬编码 id=1)
                level: 3,
                chance: 10000,  // 100%
                target: MobSkillTarget::Target,
                condition: MobSkillCondition::Any,
                condition_value: 0,
                cooldown_ms: 5000, // 有冷却
            },
        ]);

        let player = create_test_player((100, 101), 10);
        *mob.target_id.write() = Some(player.id);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 执行攻击 - 应该先用技能（冷却=5000ms）
        ai.update_attack(&mob, &map_state);

        // 验证技能被使用：冷却应该被设置
        let cooldowns = mob.skill_cooldowns.read();
        assert!(cooldowns.contains_key(&1), "技能 ID=1 的冷却应该被设置，说明技能被触发了");
        drop(cooldowns);

        // 验证伤害记录被更新（技能执行时会记录伤害）
        let damage_log = mob.damage_log.read();
        assert!(damage_log.contains_key(&player.id),
            "伤害记录应该包含对玩家的伤害");
        assert!(*damage_log.get(&player.id).unwrap() > 0,
            "技能应该造成正数伤害");
    }

    // ============================================
    // RudeAttacked 条件测试
    // ============================================

    #[test]
    fn test_check_skill_condition_rude_attacked_with_multiple_attackers() {
        let skill = MobSkill {
            skill_id: 100,
            level: 1,
            chance: 10000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::RudeAttacked,
            condition_value: 0,
            cooldown_ms: 0,
        };
        // 2 个攻击者 -> 围攻，条件满足
        assert!(check_skill_condition(&skill, 100, 2, 0));
        // 3 个攻击者 -> 围攻，条件满足
        assert!(check_skill_condition(&skill, 100, 3, 0));
    }

    #[test]
    fn test_check_skill_condition_rude_attacked_with_single_attacker() {
        let skill = MobSkill {
            skill_id: 100,
            level: 1,
            chance: 10000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::RudeAttacked,
            condition_value: 0,
            cooldown_ms: 0,
        };
        // 1 个攻击者 -> 不算围攻
        assert!(!check_skill_condition(&skill, 100, 1, 0));
        // 0 个攻击者 -> 不算围攻
        assert!(!check_skill_condition(&skill, 100, 0, 0));
    }

    // ============================================
    // LongRange 条件测试
    // ============================================

    #[test]
    fn test_check_skill_condition_long_range_far_target() {
        let skill = MobSkill {
            skill_id: 100,
            level: 1,
            chance: 10000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::LongRange,
            condition_value: 0,
            cooldown_ms: 0,
        };
        // 距离 > 3 -> 远程目标，条件满足
        assert!(check_skill_condition(&skill, 100, 0, 4));
        assert!(check_skill_condition(&skill, 100, 0, 10));
        assert!(check_skill_condition(&skill, 100, 0, 100));
    }

    #[test]
    fn test_check_skill_condition_long_range_close_target() {
        let skill = MobSkill {
            skill_id: 100,
            level: 1,
            chance: 10000,
            target: MobSkillTarget::Target,
            condition: MobSkillCondition::LongRange,
            condition_value: 0,
            cooldown_ms: 0,
        };
        // 距离 <= 3 -> 近程目标，条件不满足
        assert!(!check_skill_condition(&skill, 100, 0, 0));
        assert!(!check_skill_condition(&skill, 100, 0, 1));
        assert!(!check_skill_condition(&skill, 100, 0, 3));
    }

    // ============================================
    // FleeWhenLowHp 行为测试
    // ============================================

    /// 创建 FleeWhenLowHp 行为的测试怪物
    fn create_flee_mob(position: (u16, u16), hp: u32, max_hp: u32) -> Arc<Mob> {
        Arc::new(Mob {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
            mob_id: 1001,
            name: "FleeMob".to_string(),
            pos: parking_lot::RwLock::new(MobPosition { x: position.0, y: position.1 }),
            map_name: "test_map".to_string(),
            level: 5,
            hp: parking_lot::RwLock::new(hp),
            max_hp,
            sp: parking_lot::RwLock::new(0),
            max_sp: 0,
            atk: 30,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 5,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            race: crate::game::mob::MobRace::Formless,
            mob_type: crate::game::mob::MobType::Normal,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::FleeWhenLowHp,
            skills: Vec::new(),
            skill_cooldowns: parking_lot::RwLock::new(std::collections::HashMap::new()),
            sight_range: 10,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: position.0,
            spawn_y: position.1,
            spawn_map: "test_map".to_string(),
            death_time: parking_lot::RwLock::new(None),
            drops: vec![],
            base_exp: 10,
            job_exp: 5,
            zeny: Some(100),
            drops_processed: parking_lot::RwLock::new(false),
            path_manager: parking_lot::RwLock::new(MobPathManager::new()),
            damage_log: parking_lot::RwLock::new(std::collections::HashMap::new()),
            dmglog: parking_lot::RwLock::new(std::collections::HashMap::new()),
            flee_from: parking_lot::RwLock::new(None),
        })
    }

    #[test]
    fn flee_mob_stays_idle_when_hp_above_threshold() {
        // HP 75% > 25% 阈值，不应逃跑
        let mob = create_flee_mob((100, 100), 375, 500);
        let player = create_test_player((105, 105), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 模拟被该玩家攻击过
        mob.damage_log.write().insert(player.id, 100);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![10], map_state);
        ai.update(&mob, &map_state);

        // HP 75% > 25%，不应进入 Flee
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
    }

    #[test]
    fn flee_mob_enters_flee_when_hp_low() {
        // HP 20% < 25% 阈值，应该逃跑
        let mob = create_flee_mob((100, 100), 100, 500);
        let player = create_test_player((105, 105), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 模拟被该玩家攻击过
        mob.damage_log.write().insert(player.id, 100);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![10], map_state);
        ai.update(&mob, &map_state);

        // HP 20% <= 25%，应进入 Flee
        assert_eq!(*mob.ai_state.read(), MobAIState::Flee);
        assert_eq!(*mob.flee_from.read(), Some(player.id));
    }

    #[test]
    fn flee_mob_chase_transitions_to_flee_when_hp_low() {
        // 追击中 HP 降低到 25% 以下时应切换到逃跑
        let mob = create_flee_mob((100, 100), 100, 500); // HP 20%
        let player = create_test_player((105, 105), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 设置为 Chase 状态
        *mob.ai_state.write() = MobAIState::Chase;
        *mob.target_id.write() = Some(player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // 应切换到 Flee
        assert_eq!(*mob.ai_state.read(), MobAIState::Flee);
        assert_eq!(*mob.flee_from.read(), Some(player.id));
    }

    #[test]
    fn flee_mob_attack_transitions_to_flee_when_hp_low() {
        // 攻击中 HP 降低到 25% 以下时应切换到逃跑
        let mob = create_flee_mob((100, 100), 100, 500); // HP 20%
        let player = create_test_player((100, 101), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 设置为 Attack 状态
        *mob.ai_state.write() = MobAIState::Attack;
        *mob.target_id.write() = Some(player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // 应切换到 Flee
        assert_eq!(*mob.ai_state.read(), MobAIState::Flee);
    }

    #[test]
    fn flee_mob_moves_away_from_attacker() {
        // 怪物在 (100, 100)，攻击者在 (103, 100)
        // 逃跑方向应该是向左（远离攻击者）
        let mob = create_flee_mob((100, 100), 100, 500);
        let player = create_test_player((103, 100), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 设置逃跑状态
        *mob.ai_state.write() = MobAIState::Flee;
        *mob.flee_from.write() = Some(player.id);

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // 怪物应该向远离攻击者的方向移动
        let (new_x, new_y) = mob.get_position();
        // 攻击者在右边 (103, 100)，怪物应该向左移动
        assert!(new_x < 100, "怪物应该向左移动远离攻击者，实际位置: ({}, {})", new_x, new_y);
    }

    #[test]
    fn flee_mob_recovers_when_hp_restored() {
        // HP 恢复到 50% 以上时停止逃跑
        let mob = create_flee_mob((100, 100), 100, 500); // HP 20%
        let player = create_test_player((103, 100), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 设置逃跑状态
        *mob.ai_state.write() = MobAIState::Flee;
        *mob.flee_from.write() = Some(player.id);

        // 恢复 HP 到 60%
        *mob.hp.write() = 300;

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // HP 60% >= 50%，应停止逃跑
        assert_eq!(*mob.ai_state.read(), MobAIState::Idle);
        assert_eq!(*mob.flee_from.read(), None);
    }

    #[test]
    fn flee_mob_stays_flee_when_hp_below_recovery() {
        // HP 仍在恢复阈值以下时继续逃跑
        let mob = create_flee_mob((100, 100), 100, 500); // HP 20%
        let player = create_test_player((103, 100), 10);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 设置逃跑状态
        *mob.ai_state.write() = MobAIState::Flee;
        *mob.flee_from.write() = Some(player.id);

        // HP 恢复到 40%（仍低于 50% 阈值）
        *mob.hp.write() = 200;

        let (ai, map_state) = create_test_mob_ai_with_map(vec![0], map_state);
        ai.update(&mob, &map_state);

        // HP 40% < 50%，应继续逃跑
        assert_eq!(*mob.ai_state.read(), MobAIState::Flee);
    }

    // ============================================
    // Assist 行为测试
    // ============================================

    /// 创建 Assist 行为的测试怪物
    fn create_assist_mob(position: (u16, u16), behavior: crate::game::mob::MobBehavior) -> Arc<Mob> {
        Arc::new(Mob {
            id: Uuid::new_v4(),
            entity_id: std::sync::atomic::AtomicU32::new(0),
            mob_id: 1001,
            name: "AssistMob".to_string(),
            pos: parking_lot::RwLock::new(MobPosition { x: position.0, y: position.1 }),
            map_name: "test_map".to_string(),
            level: 5,
            hp: parking_lot::RwLock::new(500),
            max_hp: 500,
            sp: parking_lot::RwLock::new(0),
            max_sp: 0,
            atk: 30,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 5,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            race: crate::game::mob::MobRace::Formless,
            mob_type: crate::game::mob::MobType::Normal,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior,
            skills: Vec::new(),
            skill_cooldowns: parking_lot::RwLock::new(std::collections::HashMap::new()),
            sight_range: 10,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
            spawn_x: position.0,
            spawn_y: position.1,
            spawn_map: "test_map".to_string(),
            death_time: parking_lot::RwLock::new(None),
            drops: vec![],
            base_exp: 10,
            job_exp: 5,
            zeny: Some(100),
            drops_processed: parking_lot::RwLock::new(false),
            path_manager: parking_lot::RwLock::new(MobPathManager::new()),
            damage_log: parking_lot::RwLock::new(std::collections::HashMap::new()),
            dmglog: parking_lot::RwLock::new(std::collections::HashMap::new()),
            flee_from: parking_lot::RwLock::new(None),
        })
    }

    #[test]
    fn assist_mob_chases_attacker_of_nearby_mob() {
        // 创建 Assist 怪物在 (100, 100)
        let assist_mob = create_assist_mob((100, 100), crate::game::mob::MobBehavior::Assist);
        // 创建被攻击的怪物在 (105, 105)，距离 = 7.07 < sight_range (10)
        let other_mob = create_test_mob((105, 105), 5);
        let player = create_test_player((106, 106), 10);

        // 模拟 other_mob 正在追击该玩家
        *other_mob.ai_state.write() = MobAIState::Chase;
        *other_mob.target_id.write() = Some(player.id);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 使用带 spawn_manager 的 AI，注册 other_mob
        let spawn_manager = Arc::new(MobSpawnManager::new());
        spawn_manager.register_mob("test_map", other_mob.clone());

        let ai = MobAI::new(
            spawn_manager,
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(vec![10])),
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        );

        ai.update(&assist_mob, &map_state);

        // Assist 怪物应该追击攻击者
        assert_eq!(*assist_mob.ai_state.read(), MobAIState::Chase);
        assert_eq!(*assist_mob.target_id.read(), Some(player.id));
    }

    #[test]
    fn assist_mob_ignores_distant_mob() {
        // Assist 怪物在 (100, 100)，远处怪物在 (200, 200)
        let assist_mob = create_assist_mob((100, 100), crate::game::mob::MobBehavior::Assist);
        let other_mob = create_test_mob((200, 200), 5);
        let player = create_test_player((201, 201), 10);

        *other_mob.ai_state.write() = MobAIState::Chase;
        *other_mob.target_id.write() = Some(player.id);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        let spawn_manager = Arc::new(MobSpawnManager::new());
        spawn_manager.register_mob("test_map", other_mob.clone());

        let ai = MobAI::new(
            spawn_manager,
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(vec![10])),
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        );

        ai.update(&assist_mob, &map_state);

        // 距离太远，不应追击
        assert_eq!(*assist_mob.ai_state.read(), MobAIState::Idle);
    }

    #[test]
    fn passive_assist_mob_chases_after_being_attacked() {
        // PassiveAssist 怪物被攻击后才会协助
        let assist_mob = create_assist_mob((100, 100), crate::game::mob::MobBehavior::PassiveAssist);
        let other_mob = create_test_mob((105, 105), 5);
        let player = create_test_player((106, 106), 10);

        *other_mob.ai_state.write() = MobAIState::Chase;
        *other_mob.target_id.write() = Some(player.id);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 模拟 assist_mob 被该玩家攻击过
        assist_mob.damage_log.write().insert(player.id, 50);

        let spawn_manager = Arc::new(MobSpawnManager::new());
        spawn_manager.register_mob("test_map", other_mob.clone());

        let ai = MobAI::new(
            spawn_manager,
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(vec![10])),
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        );

        ai.update(&assist_mob, &map_state);

        // 被攻击过，应协助追击
        assert_eq!(*assist_mob.ai_state.read(), MobAIState::Chase);
        assert_eq!(*assist_mob.target_id.read(), Some(player.id));
    }

    #[test]
    fn passive_assist_mob_stays_idle_without_damage() {
        // PassiveAssist 怪物未被攻击时不应协助
        let assist_mob = create_assist_mob((100, 100), crate::game::mob::MobBehavior::PassiveAssist);
        let other_mob = create_test_mob((105, 105), 5);
        let player = create_test_player((106, 106), 10);

        *other_mob.ai_state.write() = MobAIState::Chase;
        *other_mob.target_id.write() = Some(player.id);

        let map_state = create_test_map_state();
        map_state.add_player((*player).clone());

        // 未被攻击（damage_log 为空）

        let spawn_manager = Arc::new(MobSpawnManager::new());
        spawn_manager.register_mob("test_map", other_mob.clone());

        let ai = MobAI::new(
            spawn_manager,
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(MapDatabase::new()),
            Arc::new(MockRng::new(vec![10])),
            Arc::new(BattleHandler::new(Arc::new(MockRng::new(vec![50])))),
            Arc::new(SkillDatabase::new()),
        );

        ai.update(&assist_mob, &map_state);

        // 未被攻击，不应协助
        assert_eq!(*assist_mob.ai_state.read(), MobAIState::Idle);
    }
}
