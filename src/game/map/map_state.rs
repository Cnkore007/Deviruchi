use super::data::MapDatabase;
use super::player::Player;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct MapState {
    players: RwLock<HashMap<Uuid, Player>>,
    players_by_map: RwLock<HashMap<String, Vec<Uuid>>>,
    /// 地图数据库引用（用于碰撞检测）
    map_database: Arc<MapDatabase>,
}

impl MapState {
    /// 创建 MapState，使用默认地图数据库
    pub fn new() -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
            players_by_map: RwLock::new(HashMap::new()),
            map_database: Arc::new(MapDatabase::new()),
        }
    }

    /// 创建 MapState，使用指定的地图数据库
    pub fn with_map_database(map_database: Arc<MapDatabase>) -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
            players_by_map: RwLock::new(HashMap::new()),
            map_database,
        }
    }

    /// 添加玩家到地图
    pub fn add_player(&self, player: Player) {
        let player_id = player.id;
        let map_name = player.map_name.clone();

        self.players.write().insert(player_id, player);

        let mut by_map = self.players_by_map.write();
        by_map.entry(map_name).or_default().push(player_id);
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
    /// 锁顺序：players → players_by_map（与 add_player/remove_player 一致，避免 ABBA 死锁）
    pub fn get_players_on_map(&self, map_name: &str) -> Vec<Player> {
        let players = self.players.read();
        let by_map = self.players_by_map.read();

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

    /// 设置玩家朝向方向（原地修改，不返回克隆）
    pub fn set_player_direction(&self, player_id: &Uuid, direction: u16) -> bool {
        let players = self.players.read();
        if let Some(player) = players.get(player_id) {
            player.set_direction(direction);
            true
        } else {
            false
        }
    }

    /// 给玩家增加属性点（原地修改），返回是否成功
    ///
    /// status_id: 13=STR, 14=AGI, 15=VIT, 16=INT, 17=DEX, 18=LUK
    pub fn allocate_player_stat(
        &self,
        player_id: &Uuid,
        status_id: u16,
        amount: u16,
    ) -> bool {
        let players = self.players.read();
        let Some(player) = players.get(player_id) else {
            return false;
        };

        // 检查状态点是否充足
        let available = player.level.read().status_point;
        if available < amount {
            return false;
        }

        // 检查属性值是否超过上限（99）
        {
            let attrs = player.attrs.read();
            let current = match status_id {
                13 => attrs.str,
                14 => attrs.agi,
                15 => attrs.vit,
                16 => attrs.int,
                17 => attrs.dex,
                18 => attrs.luk,
                _ => return false,
            };
            if current + amount > crate::game::constants::MAX_STAT {
                return false;
            }
        }

        // 修改属性
        {
            let mut attrs = player.attrs.write();
            match status_id {
                13 => attrs.str += amount,
                14 => attrs.agi += amount,
                15 => attrs.vit += amount,
                16 => attrs.int += amount,
                17 => attrs.dex += amount,
                18 => attrs.luk += amount,
                _ => return false,
            }
        }

        // 消耗状态点
        player.level.write().status_point -= amount;

        // 更新最大负重（STR 变化影响负重）
        player.update_max_weight();

        true
    }

    /// 消耗一个技能点（原地修改），返回是否成功
    pub fn allocate_player_skill_point(&self, player_id: &Uuid) -> bool {
        let players = self.players.read();
        let Some(player) = players.get(player_id) else {
            return false;
        };

        let mut lvl = player.level.write();
        if lvl.skill_point == 0 {
            return false;
        }
        lvl.skill_point -= 1;
        true
    }

    /// 变更玩家职业（原地修改）
    ///
    /// 转职效果：
    /// 1. 更新职业 ID（economy.job）
    /// 2. 重置 Job 等级为 1，Job 经验为 0
    /// 3. 重算最大 HP/SP（根据新职业的基础值）
    /// 4. 恢复 HP/SP 到最大值
    /// 5. 更新最大负重
    ///
    /// # 返回
    /// - `true` 转职成功
    /// - `false` 找不到玩家
    pub fn change_player_job(&self, player_id: &Uuid, new_job: u16) -> bool {
        let players = self.players.read();
        let Some(player) = players.get(player_id) else {
            return false;
        };

        let target_job = crate::game::job::JobType::from_u16(new_job);

        // 1. 更新职业 ID
        player.economy.write().job = new_job;

        // 2. 重置 Job 等级和经验
        {
            let mut lvl = player.level.write();
            lvl.job_level = 1;
            lvl.job_exp = 0;
        }

        // 3-4. 重算最大 HP/SP 并恢复
        if let Some(job_type) = target_job {
            let base_hp = job_type.base_hp();
            let base_sp = job_type.base_sp();
            let base_level = player.level.read().base_level;

            let hp_per_level = match job_type {
                crate::game::job::JobType::Swordman
                | crate::game::job::JobType::Knight
                | crate::game::job::JobType::LordKnight
                | crate::game::job::JobType::Paladin => 30,
                crate::game::job::JobType::Mage
                | crate::game::job::JobType::Wizard
                | crate::game::job::JobType::HighWizard => 15,
                crate::game::job::JobType::Archer
                | crate::game::job::JobType::Hunter
                | crate::game::job::JobType::Sniper => 20,
                crate::game::job::JobType::Acolyte
                | crate::game::job::JobType::Priest
                | crate::game::job::JobType::HighPriest => 25,
                crate::game::job::JobType::Merchant
                | crate::game::job::JobType::Blacksmith
                | crate::game::job::JobType::Whitesmith => 25,
                crate::game::job::JobType::Thief
                | crate::game::job::JobType::Assassin
                | crate::game::job::JobType::AssassinCross => 25,
                crate::game::job::JobType::Novice => 10,
            };

            let sp_per_level = match job_type {
                crate::game::job::JobType::Mage
                | crate::game::job::JobType::Wizard
                | crate::game::job::JobType::HighWizard => 8,
                crate::game::job::JobType::Acolyte
                | crate::game::job::JobType::Priest
                | crate::game::job::JobType::HighPriest => 5,
                crate::game::job::JobType::Archer
                | crate::game::job::JobType::Hunter
                | crate::game::job::JobType::Sniper => 3,
                crate::game::job::JobType::Merchant
                | crate::game::job::JobType::Blacksmith
                | crate::game::job::JobType::Whitesmith => 3,
                _ => 2,
            };

            let new_max_hp = base_hp + (base_level.saturating_sub(1) as u32) * hp_per_level;
            let new_max_sp = base_sp + (base_level.saturating_sub(1) as u32) * sp_per_level;

            let mut combat = player.combat.write();
            combat.max_hp = new_max_hp;
            combat.max_sp = new_max_sp;
            combat.hp = new_max_hp;
            combat.sp = new_max_sp;
        }

        // 5. 更新最大负重
        player.update_max_weight();

        true
    }

    /// 根据 account_id 查找玩家
    pub fn find_player_by_account_id(&self, account_id: u32) -> Option<Player> {
        let players = self.players.read();
        players.values().find(|p| p.account_id == account_id).cloned()
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
            let list = by_map.entry(new_map.to_string()).or_default();
            if !list.contains(player_id) {
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
        let list = by_map.entry(new_map.to_string()).or_default();
        if !list.contains(player_id) {
            list.push(*player_id);
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

    /// 检查位置是否可通行（基于地图 cell 数据的真实碰撞检测）
    pub fn is_walkable(&self, map_name: &str, x: u16, y: u16) -> bool {
        self.map_database
            .get(map_name)
            .map(|map| map.is_walkable(x, y))
            .unwrap_or(false)
    }

    /// 获取地图数据库引用
    pub fn map_database(&self) -> &Arc<MapDatabase> {
        &self.map_database
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
    use crate::game::constants;
    use crate::game::item::Equipment;
    use crate::game::map::player::PlayerState;
    use parking_lot::RwLock;

    fn create_test_player(x: u16, y: u16, map: &str) -> Player {
        Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            map_name: map.to_string(),
            combat: RwLock::new(crate::game::map::player::CombatStats {
                hp: 100,
                max_hp: 100,
                sp: 100,
                max_sp: 100,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: RwLock::new(crate::game::map::player::Position { x, y }),
            level: RwLock::new(crate::game::map::player::LevelStats {
                base_level: 1,
                job_level: 1,
                base_exp: 0,
                job_exp: 0,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: RwLock::new(crate::game::map::player::Attributes {
                str: 1,
                agi: 1,
                vit: 1,
                int: 1,
                dex: 1,
                luk: 1,
            }),
            economy: RwLock::new(crate::game::map::player::Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT + constants::WEIGHT_PER_STR,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(crate::game::map::player::SavePoint {
                map: map.to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
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

        // new_1-1.gat 有硬编码的边界墙和水域
        // 角落是墙
        assert!(!state.is_walkable("new_1-1.gat", 0, 0));
        // 中心区域应该是可行走的（避开水域 80-120 范围）
        assert!(state.is_walkable("new_1-1.gat", 50, 50));
        // 水域不可行走
        assert!(!state.is_walkable("new_1-1.gat", 90, 90));
        // 不存在的地图返回 false
        assert!(!state.is_walkable("nonexistent.gat", 0, 0));
    }

    #[test]
    fn test_is_walkable_boundary() {
        let state = MapState::new();

        // 超出地图范围
        assert!(!state.is_walkable("new_1-1.gat", 200, 200));
        assert!(!state.is_walkable("new_1-1.gat", 999, 999));
    }
}
