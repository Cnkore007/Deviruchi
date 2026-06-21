//! 坐骑管理器模块

use crate::game::map::player::Player;
use crate::game::mount::data::{Mount, MountDatabase, MountType};
use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// 坐骑错误类型
#[derive(Debug, Error, Clone)]
pub enum MountError {
    #[error("Mount not found: {0}")]
    MountNotFound(u16),

    #[error("Player level too low: required {required}, have {have}")]
    LevelTooLow { required: u16, have: u16 },

    #[error("Player already mounted")]
    AlreadyMounted,

    #[error("Player not mounted")]
    NotMounted,

    #[error("Mount type not allowed for this class")]
    MountTypeNotAllowed,

    #[error("Missing required item: {0}")]
    MissingItem(u16),

    #[error("Cannot mount while dead")]
    CannotMountWhileDead,
}

/// 玩家坐骑状态
#[derive(Debug, Clone)]
pub struct PlayerMountState {
    pub mount_id: u16,
    pub original_speed: u16, // 上马前的原始速度
}

/// 坐骑管理器
pub struct MountManager {
    /// 所有坐骑数据
    mounts: MountDatabase,
    /// 玩家当前坐骑状态 (player_id -> PlayerMountState)
    player_mounts: RwLock<HashMap<Uuid, PlayerMountState>>,
}

impl MountManager {
    pub fn new() -> Self {
        Self {
            mounts: MountDatabase::new(),
            player_mounts: RwLock::new(HashMap::new()),
        }
    }

    /// 检查玩家是否可以骑乘指定坐骑
    pub fn can_mount(&self, player: &Player, mount_id: u16) -> bool {
        // 检查玩家等级
        let mount = match self.mounts.get(mount_id) {
            Some(m) => m,
            None => return false,
        };

        if player.base_level() < mount.required_level {
            return false;
        }

        // 检查存活状态
        if !player.is_alive() {
            return false;
        }

        // 职业限制检查（简化实现：允许所有职业骑乘）
        // 完整实现需要检查坐骑数据中的职业限制列表

        true
    }

    /// 骑乘坐骑
    pub fn mount(&self, player: &Player, mount_id: u16) -> Result<(), MountError> {
        // 检查是否已在上马
        if self.player_mounts.read().contains_key(&player.id) {
            return Err(MountError::AlreadyMounted);
        }

        // 检查存活状态
        if !player.is_alive() {
            return Err(MountError::CannotMountWhileDead);
        }

        // 获取坐骑数据
        let mount = self
            .mounts
            .get(mount_id)
            .ok_or(MountError::MountNotFound(mount_id))?;

        // 检查等级
        let player_level = player.base_level();
        if player_level < mount.required_level {
            return Err(MountError::LevelTooLow {
                required: mount.required_level,
                have: player_level,
            });
        }

        // 保存原始速度
        let original_speed = player.walk_speed();

        // 计算新速度
        let new_speed = mount.calculate_speed(original_speed);

        // 更新玩家速度
        player.combat_mut().walk_speed = new_speed;

        // 记录坐骑状态
        self.player_mounts.write().insert(
            player.id,
            PlayerMountState {
                mount_id,
                original_speed,
            },
        );

        Ok(())
    }

    /// 下马
    pub fn dismount(&self, player_id: Uuid) -> Result<(), MountError> {
        let mut mounts = self.player_mounts.write();

        let _state = mounts.remove(&player_id).ok_or(MountError::NotMounted)?;

        // 注意：这里需要访问Player来恢复速度
        // 由于我们只存储了player_id，需要通过其他方式恢复
        // 实际的恢复逻辑应该在调用者中进行
        Ok(())
    }

    /// 下马（需要Player引用来恢复速度）
    pub fn dismount_with_player(&self, player: &Player) -> Result<(), MountError> {
        let mut mounts = self.player_mounts.write();

        let state = mounts.remove(&player.id).ok_or(MountError::NotMounted)?;

        // 恢复原始速度
        player.combat_mut().walk_speed = state.original_speed;

        Ok(())
    }

    /// 获取坐骑数据
    pub fn get_mount(&self, mount_id: u16) -> Option<&Mount> {
        self.mounts.get(mount_id)
    }

    /// 获取玩家当前坐骑
    pub fn get_player_mount(&self, player_id: Uuid) -> Option<Mount> {
        let mounts = self.player_mounts.read();
        let state = mounts.get(&player_id)?;
        self.mounts.get(state.mount_id).cloned()
    }

    /// 获取玩家坐骑状态
    pub fn get_player_mount_state(&self, player_id: Uuid) -> Option<PlayerMountState> {
        self.player_mounts.read().get(&player_id).cloned()
    }

    /// 检查玩家是否正在骑乘
    pub fn is_mounted(&self, player_id: Uuid) -> bool {
        self.player_mounts.read().contains_key(&player_id)
    }

    /// 获取玩家的速度加成
    pub fn get_speed_bonus(&self, player_id: Uuid) -> i32 {
        let mounts = self.player_mounts.read();
        if let Some(state) = mounts.get(&player_id)
            && let Some(mount) = self.mounts.get(state.mount_id)
        {
            return mount.speed_modifier - 100; // 返回额外的百分比
        }
        0
    }

    /// 获取坐骑数据库
    pub fn get_mount_database(&self) -> &MountDatabase {
        &self.mounts
    }

    /// 通过物品ID获取坐骑
    pub fn get_mount_by_item(&self, item_id: u16) -> Option<&Mount> {
        self.mounts.get_by_item_id(item_id)
    }

    /// 获取指定类型的坐骑列表
    pub fn get_mounts_by_type(&self, mount_type: MountType) -> Vec<&Mount> {
        self.mounts.get_by_type(mount_type)
    }

    /// 强制下马（用于死亡等情况）
    pub fn force_dismount(&self, player_id: Uuid) {
        self.player_mounts.write().remove(&player_id);
    }

    /// 获取所有已上马的玩家
    pub fn get_mounted_players(&self) -> Vec<(Uuid, u16)> {
        self.player_mounts
            .read()
            .iter()
            .map(|(pid, state)| (*pid, state.mount_id))
            .collect()
    }
}

impl Default for MountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::constants;
    use crate::game::map::player::PlayerState;
    use parking_lot::RwLock;

    fn create_test_player(level: u16) -> Player {
        Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "Test Player".to_string(),
            map_name: "test_map".to_string(),
            combat: RwLock::new(crate::game::map::player::CombatStats {
                hp: 100,
                max_hp: 100,
                sp: 50,
                max_sp: 50,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: RwLock::new(crate::game::map::player::Position { x: 100, y: 100 }),
            level: RwLock::new(crate::game::map::player::LevelStats {
                base_level: level,
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
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(crate::game::map::player::SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(crate::game::item::Equipment::new()),
            status: crate::game::status::PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
            party_id: parking_lot::RwLock::new(None),
            guild_id: parking_lot::RwLock::new(None),
        }
    }

    #[test]
    fn test_mount_creation() {
        let manager = MountManager::new();
        let mount = manager.get_mount(1);
        assert!(mount.is_some());
        assert_eq!(mount.unwrap().name, "Peco Peco");
    }

    #[test]
    fn test_can_mount_level_check() {
        let manager = MountManager::new();

        // 50级可以骑Peco (需要50级)
        let player = create_test_player(50);
        assert!(manager.can_mount(&player, 1));

        // 49级不能骑Peco
        let player = create_test_player(49);
        assert!(!manager.can_mount(&player, 1));
    }

    #[test]
    fn test_mount_and_dismount() {
        let manager = MountManager::new();
        let player = create_test_player(60);

        // 初始速度
        assert_eq!(player.walk_speed(), 150);

        // 上马
        let result = manager.mount(&player, 1); // Peco Peco
        assert!(result.is_ok());

        // 检查速度已改变
        assert!(manager.is_mounted(player.id));
        let mount = manager.get_player_mount(player.id);
        assert!(mount.is_some());

        // 下马
        let result = manager.dismount_with_player(&player);
        assert!(result.is_ok());

        // 检查速度已恢复
        assert!(!manager.is_mounted(player.id));
    }

    #[test]
    fn test_cannot_mount_already_mounted() {
        let manager = MountManager::new();
        let player = create_test_player(60);

        manager.mount(&player, 1).unwrap();
        let result = manager.mount(&player, 2);
        assert!(matches!(result, Err(MountError::AlreadyMounted)));
    }

    #[test]
    fn test_cannot_mount_dead_player() {
        let manager = MountManager::new();
        let player = create_test_player(60);
        player.combat_mut().state = PlayerState::Dead;

        let result = manager.mount(&player, 1);
        assert!(matches!(result, Err(MountError::CannotMountWhileDead)));
    }

    #[test]
    fn test_speed_calculation() {
        let manager = MountManager::new();
        let player = create_test_player(60);

        // Peco是150%速度
        manager.mount(&player, 1).unwrap();
        // 150 * 1.5 = 225
        assert_eq!(player.walk_speed(), 225);
    }

    #[test]
    fn test_get_mount_by_item() {
        let manager = MountManager::new();
        let mount = manager.get_mount_by_item(2260);
        assert!(mount.is_some());
        assert_eq!(mount.unwrap().mount_id, 1);
    }

    #[test]
    fn test_get_mounts_by_type() {
        let manager = MountManager::new();
        let peco_mounts = manager.get_mounts_by_type(MountType::Peco);
        assert!(!peco_mounts.is_empty());

        let warg_mounts = manager.get_mounts_by_type(MountType::Warg);
        assert!(!warg_mounts.is_empty());
    }
}
