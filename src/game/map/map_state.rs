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

    /// 获取指定位置一定范围内的所有玩家
    pub fn get_players_near(&self, map_name: &str, x: u16, y: u16, radius: u16) -> Vec<Player> {
        let players = self.get_players_on_map(map_name);
        players.into_iter()
            .filter(|p| {
                let (px, py) = p.get_position();
                let dx = (px as i32 - x as i32).unsigned_abs() as u16;
                let dy = (py as i32 - y as i32).unsigned_abs() as u16;
                dx <= radius && dy <= radius
            })
            .collect()
    }

    /// 检查位置是否可通行（简化版本，始终返回 true）
    pub fn is_walkable(&self, _map_name: &str, _x: u16, _y: u16) -> bool {
        true // TODO: 实现实际碰撞检测
    }
}

impl Default for MapState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use crate::game::item::Equipment;

    fn create_test_player(x: u16, y: u16, map: &str) -> Player {
        Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            pos_x: RwLock::new(x),
            pos_y: RwLock::new(y),
            map_name: map.to_string(),
            hp: RwLock::new(100),
            max_hp: RwLock::new(100),
            sp: RwLock::new(100),
            max_sp: RwLock::new(100),
            base_level: RwLock::new(1),
            job_level: RwLock::new(1),
            str: RwLock::new(1),
            agi: RwLock::new(1),
            vit: RwLock::new(1),
            int: RwLock::new(1),
            dex: RwLock::new(1),
            luk: RwLock::new(1),
            walk_speed: RwLock::new(150),
            zeny: RwLock::new(0),
            current_weight: RwLock::new(0),
            max_weight: RwLock::new(20000 + 200),
            equipment: RwLock::new(Equipment::new()),
        }
    }

    #[test]
    fn test_get_players_near() {
        let state = MapState::new();
        let map = "test_map";

        // 添加不同位置的测试玩家
        // 中心点 (100, 100)，半径 100
        let player1 = create_test_player(100, 100, map);  // 距离 0，在范围内
        let player2 = create_test_player(150, 100, map);  // 距离 50，在范围内
        let player3 = create_test_player(250, 250, map);  // 距离约 212，超出范围
        let player4 = create_test_player(100, 100, "other_map");  // 不同地图

        state.add_player(player1.clone());
        state.add_player(player2.clone());
        state.add_player(player3.clone());
        state.add_player(player4.clone());

        // 测试：中心点 (100, 100)，半径 100
        let nearby = state.get_players_near(map, 100, 100, 100);
        assert_eq!(nearby.len(), 2); // player1 和 player2 在范围内
        assert!(nearby.iter().any(|p| p.id == player1.id));
        assert!(nearby.iter().any(|p| p.id == player2.id));

        // 测试：更大半径
        let nearby2 = state.get_players_near(map, 100, 100, 300);
        assert_eq!(nearby2.len(), 3); // 所有 test_map 上的玩家都在范围内

        // 测试：空地图
        let empty = state.get_players_near("nonexistent_map", 0, 0, 100);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_is_walkable() {
        let state = MapState::new();
        // 当前简化实现始终返回 true
        assert!(state.is_walkable("test_map", 100, 100));
        assert!(state.is_walkable("any_map", 0, 0));
    }
}
