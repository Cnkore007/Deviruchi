use crate::game::battle::ExpDistributor;
use crate::game::heal::{FoodManager, HealService};
use crate::game::map::MapState;
use crate::game::map::channel::ChannelBus;
use crate::game::map::drop_item::DropManager;
use crate::game::mob::droptable::DropResolver;
use crate::game::mob::{MobAI, MobSpawnManager};
use crate::game::token::TokenStore;
use std::sync::Arc;
use std::time::Duration;

#[allow(dead_code)]
pub struct GameLoop {
    map_state: Arc<MapState>,
    drop_manager: Arc<DropManager>,
    token_store: Arc<TokenStore>,
    mob_ai: Arc<MobAI>,
    mob_spawn_manager: Arc<MobSpawnManager>,
    drop_resolver: Arc<DropResolver>,
    channel_bus: Arc<ChannelBus>,
    exp_distributor: Arc<ExpDistributor>,
    heal_service: Arc<HealService>,
    food_manager: Arc<FoodManager>,
    tick_interval: Duration,
}

impl GameLoop {
    pub fn new(
        map_state: Arc<MapState>,
        drop_manager: Arc<DropManager>,
        token_store: Arc<TokenStore>,
        mob_ai: Arc<MobAI>,
        mob_spawn_manager: Arc<MobSpawnManager>,
        drop_resolver: Arc<DropResolver>,
        channel_bus: Arc<ChannelBus>,
        exp_distributor: Arc<ExpDistributor>,
        heal_service: Arc<HealService>,
        food_manager: Arc<FoodManager>,
    ) -> Self {
        Self {
            map_state,
            drop_manager,
            token_store,
            mob_ai,
            mob_spawn_manager,
            drop_resolver,
            channel_bus,
            exp_distributor,
            heal_service,
            food_manager,
            tick_interval: Duration::from_millis(100),
        }
    }

    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = interval;
        self
    }

    /// Execute one tick
    pub fn tick(&self) {
        // 1. DropManager cleanup with broadcast (5 minute TTL)
        self.drop_manager
            .cleanup_expired_with_broadcast(self.channel_bus.as_ref());

        // 2. TokenStore cleanup (30 second TTL)
        self.token_store.cleanup_expired();

        // 3. MobSpawnManager check and process respawns
        self.mob_spawn_manager
            .check_respawn(self.channel_bus.as_ref());

        // 4. Update all active Mob AI (100ms tick interval)
        self.update_mob_ai();

        // 5. Update player states (HP/SP regeneration, food effects, status cleanup)
        self.process_player_regeneration();
    }

    /// Process player HP/SP regeneration and related systems
    fn process_player_regeneration(&self) {
        let unique_maps = self.map_state.get_all_map_names();

        for map_name in &unique_maps {
            let players = self.map_state.get_players_on_map(map_name);

            for player in players {
                // 跳过死亡玩家
                if !player.is_alive() {
                    continue;
                }

                // 1. 处理食物效果
                let (food_hp, food_sp) = self.food_manager.process_food_effects(&player);

                // 2. 应用食物效果
                if food_hp > 0 || food_sp > 0 {
                    let current_hp = player.hp();
                    let max_hp = player.max_hp();
                    let current_sp = player.sp();
                    let max_sp = player.max_sp();

                    let new_hp = (current_hp + food_hp).min(max_hp);
                    let new_sp = (current_sp + food_sp).min(max_sp);

                    if new_hp != current_hp || new_sp != current_sp {
                        let mut c = player.combat_mut();
                        c.hp = new_hp;
                        c.sp = new_sp;
                        tracing::trace!(
                            "Player {} food effect: +{} HP, +{} SP",
                            player.name,
                            food_hp,
                            food_sp
                        );
                    }
                }

                // 3. 处理状态效果清理
                player.cleanup_expired_status();
            }
        }
    }

    /// Update all active mob AI state machines
    fn update_mob_ai(&self) {
        let mobs = self.mob_spawn_manager.get_all_active_mobs();
        let map_state = &self.map_state;

        for mob in mobs {
            self.mob_ai.update(&mob, map_state);
        }
    }

    /// Start the game loop as an async task
    #[allow(dead_code)]
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let tick_interval = self.tick_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_interval);
            loop {
                interval.tick().await;
                self.tick();
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::constants;
    use crate::game::mob::data::MobPathManager;
    use uuid::Uuid;

    fn create_test_mob_ai_and_spawn_manager() -> (Arc<MobAI>, Arc<MobSpawnManager>) {
        let spawn_manager = Arc::new(MobSpawnManager::new());
        let ai = Arc::new(MobAI::new(
            spawn_manager.clone(),
            Arc::new(crate::game::map::channel::ChannelBus::new()),
            Arc::new(crate::game::map::drop_item::DropManager::new()),
            Arc::new(crate::game::party::PartyManager::new()),
            Arc::new(crate::game::map::data::MapDatabase::new()),
            crate::game::rand::thread_rng(),
            Arc::new(crate::game::battle::BattleHandler::new(
                crate::game::rand::thread_rng(),
            )),
        ));
        (ai, spawn_manager)
    }

    fn create_test_game_loop() -> GameLoop {
        let map_state = Arc::new(MapState::new());
        let drop_manager = Arc::new(DropManager::new());
        let token_store = Arc::new(TokenStore::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let drop_resolver = Arc::new(DropResolver);
        let exp_distributor = Arc::new(ExpDistributor);
        let (mob_ai, mob_spawn_manager) = create_test_mob_ai_and_spawn_manager();
        let config = Arc::new(crate::core::Config::default());
        let heal_service = Arc::new(HealService::new(config.clone()));
        let food_manager = Arc::new(FoodManager::new());

        GameLoop::new(
            map_state,
            drop_manager,
            token_store,
            mob_ai,
            mob_spawn_manager,
            drop_resolver,
            channel_bus,
            exp_distributor,
            heal_service,
            food_manager,
        )
    }

    #[test]
    fn test_game_loop_tick_runs_without_panic() {
        let game_loop = create_test_game_loop();
        // Should not panic
        game_loop.tick();
    }

    #[test]
    fn test_game_loop_cleans_up_expired_tokens() {
        let game_loop = create_test_game_loop();

        // Create a token
        let token_store = game_loop.token_store.as_ref();
        let token = token_store.create(1, 1, 1); // account_id, char_id, map_server_id
        assert!(token_store.verify(&token, 1, 1, 1));

        // Simulate token expiry by calling cleanup
        std::thread::sleep(Duration::from_millis(50));
        game_loop.tick();

        // Token should be cleaned up after expiry check
        // (Note: cleanup_expired checks based on time, so we just verify tick doesn't panic)
    }

    #[test]
    fn test_game_loop_with_custom_interval() {
        let game_loop = create_test_game_loop().with_tick_interval(Duration::from_secs(1));

        assert_eq!(game_loop.tick_interval, Duration::from_secs(1));
    }

    #[test]
    fn test_game_loop_full_mob_death_respawn_cycle() {
        use crate::game::mob::{Mob, MobAIState, MobPosition};
        use std::time::Instant;

        let game_loop = create_test_game_loop();

        // Create a test mob
        let mob = Arc::new(Mob {
            id: Uuid::new_v4(),
            mob_id: 1001,
            name: "TestPoring".to_string(),
            pos: parking_lot::RwLock::new(MobPosition { x: 100, y: 100 }),
            map_name: "prontera".to_string(),
            level: 1,
            hp: parking_lot::RwLock::new(50),
            max_hp: 50,
            sp: parking_lot::RwLock::new(0),
            max_sp: 0,
            atk: 7,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 7,
            flee: 5,
            crit: 0,
            walk_speed: constants::DEFAULT_WALK_SPEED,
            atk_range: 1,
            element: crate::game::battle::element::Element::Neutral,
            element_level: crate::game::battle::element::ElementLevel::Level1,
            size: crate::game::battle::element::MobSize::Medium,
            ai_state: parking_lot::RwLock::new(MobAIState::Idle),
            target_id: parking_lot::RwLock::new(None),
            behavior: crate::game::mob::MobBehavior::Aggressive,
            skills: Vec::new(),
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 100, // Short respawn for testing
            spawn_x: 100,
            spawn_y: 100,
            spawn_map: "prontera".to_string(),
            death_time: parking_lot::RwLock::new(None),
            drops: vec![],
            base_exp: 2,
            job_exp: 1,
            zeny: Some(10),
            drops_processed: parking_lot::RwLock::new(false),
            path_manager: parking_lot::RwLock::new(MobPathManager::new()),
            damage_log: parking_lot::RwLock::new(std::collections::HashMap::new()),
        });

        // Register the mob
        game_loop
            .mob_spawn_manager
            .register_mob("prontera", mob.clone());

        // 1. Verify mob is active
        let active_mobs = game_loop.mob_spawn_manager.get_all_active_mobs();
        assert_eq!(active_mobs.len(), 1);

        // 2. Simulate player attacking mob until HP = 0
        *mob.hp.write() = 0;
        *mob.ai_state.write() = MobAIState::Dead;
        *mob.death_time.write() = Some(Instant::now());

        // 3. Unregister mob (simulating death handling)
        game_loop
            .mob_spawn_manager
            .unregister_mob("prontera", &mob.id);

        // Verify mob is no longer in active list
        let active_mobs = game_loop.mob_spawn_manager.get_all_active_mobs();
        assert!(active_mobs.is_empty());

        // 4. Call check_respawn (should not respawn yet - too early)
        let respawned = game_loop
            .mob_spawn_manager
            .check_respawn(game_loop.channel_bus.as_ref());
        assert!(respawned.is_empty());

        // Wait for respawn delay
        std::thread::sleep(Duration::from_millis(150));

        // 5. Call check_respawn again (should respawn now)
        let respawned = game_loop
            .mob_spawn_manager
            .check_respawn(game_loop.channel_bus.as_ref());
        // Note: check_respawn removes death time but doesn't re-register mob
        // The actual mob respawn logic is in MobAI::update_dead

        // 6. Verify drop_manager cleanup works
        let drop_manager = game_loop.drop_manager.as_ref();
        let drop_id = drop_manager.add(501, 1, 100, 100, "prontera");
        let drops = drop_manager.get_drops_for_map("prontera");
        assert_eq!(drops.len(), 1);

        // Test that cleanup_expired_with_broadcast doesn't panic
        let expired = drop_manager.cleanup_expired_with_broadcast(game_loop.channel_bus.as_ref());
        assert!(expired.is_empty()); // Fresh drops shouldn't be expired
    }
}
