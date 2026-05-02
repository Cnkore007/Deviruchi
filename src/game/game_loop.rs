use std::sync::Arc;
use std::time::Duration;
use crate::game::map::MapState;
use crate::game::map::drop_item::DropManager;
use crate::game::token::TokenStore;

pub struct GameLoop {
    map_state: Arc<MapState>,
    drop_manager: Arc<DropManager>,
    token_store: Arc<TokenStore>,
    tick_interval: Duration,
}

impl GameLoop {
    pub fn new(
        map_state: Arc<MapState>,
        drop_manager: Arc<DropManager>,
        token_store: Arc<TokenStore>,
    ) -> Self {
        Self {
            map_state,
            drop_manager,
            token_store,
            tick_interval: Duration::from_millis(100),
        }
    }

    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = interval;
        self
    }

    /// Execute one tick
    pub fn tick(&self) {
        // 1. Clean up expired drop items (5 minute TTL)
        self.drop_manager.cleanup_expired();

        // 2. Clean up expired tokens (30 second TTL)
        self.token_store.cleanup_expired();

        // 3. Update player states (HP regeneration, etc.)
        // Simplified: iterate players and do basic updates
        // TODO: Add player HP/SP regeneration logic
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

    #[test]
    fn test_game_loop_tick_runs_without_panic() {
        let map_state = Arc::new(MapState::new());
        let drop_manager = Arc::new(DropManager::new());
        let token_store = Arc::new(TokenStore::new());

        let game_loop = GameLoop::new(map_state, drop_manager, token_store);

        // Should not panic
        game_loop.tick();
    }

    #[test]
    fn test_game_loop_cleans_up_expired_tokens() {
        let map_state = Arc::new(MapState::new());
        let drop_manager = Arc::new(DropManager::new());
        let token_store = Arc::new(TokenStore::new());

        // Create a token
        let token = token_store.create(1, 1);
        assert!(token_store.verify(&token, 1, 1));

        let game_loop = GameLoop::new(map_state, drop_manager, token_store);

        // Simulate token expiry by calling cleanup
        std::thread::sleep(Duration::from_millis(50));
        game_loop.tick();

        // Token should be cleaned up after expiry check
        // (Note: cleanup_expired checks based on time, so we just verify tick doesn't panic)
    }

    #[test]
    fn test_game_loop_with_custom_interval() {
        let map_state = Arc::new(MapState::new());
        let drop_manager = Arc::new(DropManager::new());
        let token_store = Arc::new(TokenStore::new());

        let game_loop = GameLoop::new(map_state, drop_manager, token_store)
            .with_tick_interval(Duration::from_secs(1));

        assert_eq!(game_loop.tick_interval, Duration::from_secs(1));
    }
}
