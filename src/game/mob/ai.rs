use crate::game::battle::{BattleHandler, ExpDistributor};
use crate::game::map::data::MapDatabase;
use crate::game::map::{ChannelBus, DropManager, GameEvent, MapState};
use crate::game::mob::droptable::{DropResolver, MVPResolver, MobDropTable};
use crate::game::mob::{Mob, MobAIState, MobBehavior, MobSpawnManager};
use crate::game::party::PartyManager;
use crate::game::rand::GameRng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

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
    ) -> Self {
        Self {
            spawn_manager,
            channel_bus,
            drop_manager,
            party_manager,
            map_database,
            rng,
            battle_handler,
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
                // 低血量逃跑：暂不实现逃跑逻辑，当前与被动行为一致
            }
            MobBehavior::Assist | MobBehavior::PassiveAssist => {
                // 协助：暂不实现，当前与被动行为一致
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
        let target_id = *mob.target_id.read();

        if let Some(target_id) = target_id {
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
                            self.channel_bus.publish(&channel_name, &event, vec![]);

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

    fn update_return(&self, mob: &Arc<Mob>) {
        *mob.ai_state.write() = MobAIState::Idle;
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
            self.channel_bus.publish(&channel_name, &event, vec![]);

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
                self.channel_bus.publish(&channel_name, &drop_event, vec![]);
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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::{MapState, Player};
    use crate::game::mob::data::MobPathManager;
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
            mob_id: 1001,
            name: "TestMob".to_string(),
            pos_x: parking_lot::RwLock::new(position.0),
            pos_y: parking_lot::RwLock::new(position.1),
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
            walk_speed: 150,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Aggressive,
            skills: Vec::new(),
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
        })
    }

    fn create_test_player(position: (u16, u16), level: u16) -> Arc<Player> {
        Arc::new(Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            pos_x: parking_lot::RwLock::new(position.0),
            pos_y: parking_lot::RwLock::new(position.1),
            map_name: "test_map".to_string(),
            hp: parking_lot::RwLock::new(1000),
            max_hp: parking_lot::RwLock::new(1000),
            sp: parking_lot::RwLock::new(100),
            max_sp: parking_lot::RwLock::new(100),
            base_level: parking_lot::RwLock::new(level),
            job_level: parking_lot::RwLock::new(5),
            base_exp: parking_lot::RwLock::new(0),
            job_exp: parking_lot::RwLock::new(0),
            state: parking_lot::RwLock::new(crate::game::map::player::PlayerState::Alive),
            str: parking_lot::RwLock::new(10),
            agi: parking_lot::RwLock::new(10),
            vit: parking_lot::RwLock::new(10),
            int: parking_lot::RwLock::new(10),
            dex: parking_lot::RwLock::new(10),
            luk: parking_lot::RwLock::new(10),
            walk_speed: parking_lot::RwLock::new(150),
            zeny: parking_lot::RwLock::new(0),
            current_weight: parking_lot::RwLock::new(0),
            max_weight: parking_lot::RwLock::new(20000),
            equipment: parking_lot::RwLock::new(crate::game::item::Equipment::new()),
            is_sitting: parking_lot::RwLock::new(false),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            shop_id: parking_lot::RwLock::new(None),
            inventory: parking_lot::RwLock::new(Vec::new()),
            hotkeys: parking_lot::RwLock::new(Vec::new()),
            save_map: parking_lot::RwLock::new("test_map".to_string()),
            save_x: parking_lot::RwLock::new(50),
            save_y: parking_lot::RwLock::new(50),
            job: parking_lot::RwLock::new(0),
            in_combat: parking_lot::RwLock::new(false),
            group_id: parking_lot::RwLock::new(0),
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
            mob_id: 1001,
            name: "PassiveMob".to_string(),
            pos_x: parking_lot::RwLock::new(position.0),
            pos_y: parking_lot::RwLock::new(position.1),
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
            walk_speed: 150,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Passive,
            skills: Vec::new(),
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
            pos_x: parking_lot::RwLock::new(105),
            pos_y: parking_lot::RwLock::new(105),
            map_name: "other_map".to_string(), // Different map
            hp: parking_lot::RwLock::new(1000),
            max_hp: parking_lot::RwLock::new(1000),
            sp: parking_lot::RwLock::new(100),
            max_sp: parking_lot::RwLock::new(100),
            base_level: parking_lot::RwLock::new(10),
            job_level: parking_lot::RwLock::new(5),
            base_exp: parking_lot::RwLock::new(0),
            job_exp: parking_lot::RwLock::new(0),
            state: parking_lot::RwLock::new(crate::game::map::player::PlayerState::Alive),
            str: parking_lot::RwLock::new(10),
            agi: parking_lot::RwLock::new(10),
            vit: parking_lot::RwLock::new(10),
            int: parking_lot::RwLock::new(10),
            dex: parking_lot::RwLock::new(10),
            luk: parking_lot::RwLock::new(10),
            walk_speed: parking_lot::RwLock::new(150),
            zeny: parking_lot::RwLock::new(0),
            current_weight: parking_lot::RwLock::new(0),
            max_weight: parking_lot::RwLock::new(20000),
            equipment: parking_lot::RwLock::new(crate::game::item::Equipment::new()),
            is_sitting: parking_lot::RwLock::new(false),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            shop_id: parking_lot::RwLock::new(None),
            inventory: parking_lot::RwLock::new(Vec::new()),
            hotkeys: parking_lot::RwLock::new(Vec::new()),
            save_map: parking_lot::RwLock::new("other_map".to_string()),
            save_x: parking_lot::RwLock::new(50),
            save_y: parking_lot::RwLock::new(50),
            job: parking_lot::RwLock::new(0),
            in_combat: parking_lot::RwLock::new(false),
            group_id: parking_lot::RwLock::new(0),
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
}
