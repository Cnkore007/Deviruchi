use super::player::Player;
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

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
        players
            .into_iter()
            .filter(|p| {
                let (px, py) = p.get_position();
                let dx = (px as i32 - x as i32).unsigned_abs() as u16;
                let dy = (py as i32 - y as i32).unsigned_abs() as u16;
                dx <= radius && dy <= radius
            })
            .collect()
    }

    /// 根据玩家名称查找玩家
    pub fn find_player_by_name(&self, name: &str) -> Option<Player> {
        let players = self.players.read();
        players.values().find(|p| p.name == name).cloned()
    }

    /// 重生玩家：更新位置、地图、HP/SP、状态为 Alive
    pub fn respawn_player(&self, player_id: &Uuid, x: u16, y: u16, new_map: &str) -> bool {
        let mut players = self.players.write();
        let Some(player) = players.get_mut(player_id) else {
            return false;
        };

        let old_map = player.map_name.clone();
        player.respawn(x, y);

        // 如果地图改变，更新地图索引
        if old_map != new_map {
            player.map_name = new_map.to_string();
            drop(players);
            let mut by_map = self.players_by_map.write();
            if let Some(list) = by_map.get_mut(&old_map) {
                list.retain(|id| id != player_id);
            }
            by_map.entry(new_map.to_string()).or_default();
            if let Some(list) = by_map.get_mut(new_map)
                && !list.contains(player_id)
            {
                list.push(*player_id);
            }
        }

        true
    }

    /// 更新玩家的地图归属（玩家切换地图时调用）
    pub fn update_player_map(&self, player_id: &Uuid, old_map: &str, new_map: &str) {
        // 从旧地图移除
        let mut by_map = self.players_by_map.write();
        if let Some(players) = by_map.get_mut(old_map) {
            players.retain(|id| id != player_id);
        }
        // 添加到新地图
        by_map.entry(new_map.to_string()).or_default();
        if let Some(players) = by_map.get_mut(new_map)
            && !players.contains(player_id)
        {
            players.push(*player_id);
        }
    }

    /// 给玩家增加基础经验（原地修改）
    pub fn add_player_base_exp(&self, player_id: &Uuid, exp: u64) -> bool {
        let players = self.players.read();
        if let Some(player) = players.get(player_id) {
            player.add_base_exp(exp);
            true
        } else {
            false
        }
    }

    /// 给玩家增加职业经验（原地修改）
    pub fn add_player_job_exp(&self, player_id: &Uuid, exp: u64) -> bool {
        let players = self.players.read();
        if let Some(player) = players.get(player_id) {
            player.add_job_exp(exp);
            true
        } else {
            false
        }
    }

    /// 给玩家增加 Zeny（原地修改）
    pub fn add_player_zeny(&self, player_id: &Uuid, zeny: u64) -> bool {
        let players = self.players.read();
        if let Some(player) = players.get(player_id) {
            player.add_zeny(zeny);
            true
        } else {
            false
        }
    }

    /// 检查位置是否可通行（简化版本，始终返回 true）
    pub fn is_walkable(&self, _map_name: &str, _x: u16, _y: u16) -> bool {
        tracing::debug!("is_walkable check not yet implemented, returning true");
        true
    }

    /// 获取所有唯一地图名称
    pub fn get_all_map_names(&self) -> Vec<String> {
        self.players_by_map.read().keys().cloned().collect()
    }

    /// 获取所有在线玩家的 ID 列表
    pub fn get_all_player_ids(&self) -> Vec<Uuid> {
        self.players.read().keys().cloned().collect()
    }

    /// 获取所有玩家的克隆
    pub fn get_all_players(&self) -> Vec<Player> {
        self.players.read().values().cloned().collect()
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
    use crate::game::item::Equipment;
    use crate::game::map::player::PlayerState;
    use parking_lot::RwLock;

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
            base_exp: RwLock::new(0),
            job_exp: RwLock::new(0),
            state: RwLock::new(PlayerState::Alive),
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
            is_sitting: RwLock::new(false),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            shop_id: RwLock::new(None),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            save_map: RwLock::new(map.to_string()),
            save_x: RwLock::new(50),
            save_y: RwLock::new(50),
            job: RwLock::new(0),
            in_combat: RwLock::new(false),
            group_id: RwLock::new(0),
        }
    }

    #[test]
    fn test_get_players_near() {
        let state = MapState::new();
        let map = "test_map";

        // 添加不同位置的测试玩家
        // 中心点 (100, 100)，半径 100
        let player1 = create_test_player(100, 100, map); // 距离 0，在范围内
        let player2 = create_test_player(150, 100, map); // 距离 50，在范围内
        let player3 = create_test_player(250, 250, map); // 距离约 212，超出范围
        let player4 = create_test_player(100, 100, "other_map"); // 不同地图

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
