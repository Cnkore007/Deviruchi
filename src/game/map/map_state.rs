use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;
use super::player::Player;

pub struct MapState {
    players: RwLock<HashMap<Uuid, Player>>,
    players_by_map: RwLock<HashMap<String, Vec<Uuid>>>,
}

impl MapState {
    pub fn new() -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
            players_by_map: RwLock::new(HashMap::new()),
        }
    }

    /// 添加玩家到地图
    pub fn add_player(&self, player: Player) {
        let player_id = player.id;
        let map_name = player.map_name.clone();

        self.players.write().insert(player_id, player);

        let mut by_map = self.players_by_map.write();
        by_map.entry(map_name.clone()).or_default();
        if let Some(players) = by_map.get_mut(&map_name) {
            players.push(player_id);
        }
    }

    /// 从地图移除玩家
    pub fn remove_player(&self, player_id: &Uuid) {
        if let Some(player) = self.players.write().remove(player_id) {
            let mut by_map = self.players_by_map.write();
            if let Some(players) = by_map.get_mut(&player.map_name) {
                players.retain(|id| id != player_id);
            }
        }
    }

    /// 获取玩家
    pub fn get_player(&self, player_id: &Uuid) -> Option<Player> {
        self.players.read().get(player_id).cloned()
    }

    /// 获取地图上的所有玩家
    pub fn get_players_on_map(&self, map_name: &str) -> Vec<Player> {
        let by_map = self.players_by_map.read();
        let players = self.players.read();

        by_map
            .get(map_name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| players.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取在线玩家数量
    pub fn player_count(&self) -> usize {
        self.players.read().len()
    }
}

impl Default for MapState {
    fn default() -> Self {
        Self::new()
    }
}
