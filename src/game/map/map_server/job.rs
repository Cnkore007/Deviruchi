//! 转职系统 handler
//!
//! 处理转职请求（CZ_REQ_CHANGEJOB 和 GM 命令 @jobchange），
//! 校验转职条件，应用转职效果并持久化。

use super::MapServer;
use crate::game::job::{check_job_change_requirements, JobType};
use crate::network::packet::id::*;
use crate::network::session::Session;

impl MapServer {
    /// 处理转职请求 (CZ_REQ_CHANGEJOB, 0x019D)
    ///
    /// 客户端发送格式：packet_id(2) + target_job(2)
    ///
    /// 处理流程：
    /// 1. 解析目标职业 ID
    /// 2. 校验转职条件（等级、职业前置）
    /// 3. 应用转职效果（更新职业、重置 JobLv/JobExp、重算 HP/SP）
    /// 4. 保存到数据库
    /// 5. 返回 ZC_ACK_CHANGEJOB (0x019E)
    pub(super) fn handle_job_change(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // 包体格式：target_job(2) = 2 字节
        if data.len() < 2 {
            return None;
        }

        let target_job = u16::from_le_bytes([data[0], data[1]]);

        // 获取玩家（克隆用于条件检查）
        let player = self.map_state.get_player(&player_id)?;

        let current_job = player.job();
        let base_level = player.base_level();
        let player_name = player.name.clone();

        tracing::info!(
            player = %player_name,
            current_job = current_job,
            target_job = target_job,
            "收到转职请求"
        );

        // 校验转职条件
        if let Err(e) = check_job_change_requirements(current_job, target_job, base_level) {
            tracing::warn!(
                player = %player_name,
                error = %e,
                "转职条件不满足"
            );
            // 返回当前职业不变的 ACK（客户端通过对比 job_id 判断是否成功）
            return Some(build_ack_changejob(current_job));
        }

        // 应用转职效果（通过 MapState 直接修改原始玩家）
        self.apply_job_change(&player_id, target_job, &player_name);

        tracing::info!(
            player = %player_name,
            old_job = current_job,
            new_job = target_job,
            job_name = JobType::from_u16(target_job).map(|j| j.name()).unwrap_or("未知"),
            "转职成功"
        );

        // 返回转职成功 ACK
        Some(build_ack_changejob(target_job))
    }

    /// GM 命令处理转职（跳过部分条件检查）
    ///
    /// 与协议转职不同，GM 命令可以：
    /// - 跳过等级要求
    /// - 跳过职业前置检查
    ///
    /// # 参数
    /// - `player_id`: 目标玩家 UUID
    /// - `target_job_id`: 目标职业 ID
    ///
    /// # 返回
    /// - `Ok(String)` 成功消息
    /// - `Err(String)` 失败原因
    pub fn gm_change_job(
        &self,
        player_id: &uuid::Uuid,
        target_job_id: u16,
    ) -> Result<String, String> {
        let target_job = JobType::from_u16(target_job_id)
            .ok_or_else(|| format!("无效的职业 ID: {}", target_job_id))?;

        let player = self.map_state.get_player(player_id)
            .ok_or_else(|| "找不到目标玩家".to_string())?;

        let current_job = player.job();
        let player_name = player.name.clone();

        tracing::info!(
            player = %player_name,
            current_job = current_job,
            target_job = target_job_id,
            "GM 命令转职"
        );

        // 应用转职效果（GM 命令跳过条件检查，通过 MapState 直接修改原始玩家）
        self.apply_job_change(player_id, target_job_id, &player_name);

        Ok(format!(
            "转职成功: {} -> {} ({})",
            JobType::from_u16(current_job).map(|j| j.name()).unwrap_or("未知"),
            target_job.name(),
            target_job_id
        ))
    }

    /// 应用转职效果到玩家
    ///
    /// 通过 MapState 直接修改存储的原始玩家（而非克隆），
    /// 确保转职效果对后续操作可见。
    ///
    /// 转职会触发以下变更：
    /// 1. 更新职业 ID（economy.job）
    /// 2. 重置 Job 等级为 1
    /// 3. 重置 Job 经验为 0
    /// 4. 重算最大 HP/SP（根据新职业的基础值）
    /// 5. 恢复 HP/SP 到最大值
    /// 6. 更新最大负重
    /// 7. 保存到数据库
    fn apply_job_change(&self, player_id: &uuid::Uuid, new_job: u16, player_name: &str) {
        // 通过 MapState 直接修改原始玩家
        self.map_state.change_player_job(player_id, new_job);

        // 保存到数据库（重新获取修改后的玩家数据）
        if let Some(updated_player) = self.map_state.get_player(player_id)
            && let Err(e) = updated_player.save_to_db(&self.db) {
                tracing::error!(
                    player = %player_name,
                    error = %e,
                    "转职后保存玩家数据失败"
                );
            }
    }
}

/// 构建 ZC_ACK_CHANGEJOB (0x019E) 包
///
/// rAthena 格式：length(2) + packet_id(2) + job_id(2)
/// - job_id: 当前职业 ID（成功时为新职业，失败时为原职业）
fn build_ack_changejob(job_id: u16) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(6);
    pkt.extend_from_slice(&6u16.to_le_bytes());          // length
    pkt.extend_from_slice(&ZC_ACK_CHANGEJOB.to_le_bytes()); // packet_id
    pkt.extend_from_slice(&job_id.to_le_bytes());         // job_id
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::channel::ChannelBus;
    use crate::game::map::player::{
        Attributes, CombatStats, Economy, LevelStats, PlayerState, Position, SavePoint,
    };
    use crate::game::map::MapState;
    use crate::game::status::PlayerStatus;
    use crate::game::item::Equipment;
    use crate::game::constants;
    use parking_lot::RwLock;
    use std::sync::Arc;
    use uuid::Uuid;

    /// 创建测试用 MapServer（简化版）
    fn make_test_server(map_state: Arc<MapState>, channel_bus: Arc<ChannelBus>) -> MapServer {
        let db = Arc::new(crate::storage::Database::open_memory().unwrap());
        let teleport_manager = Arc::new(RwLock::new(
            crate::game::map::teleport::TeleportManager::new(),
        ));
        let save_point_manager = Arc::new(RwLock::new(
            crate::game::map::teleport::SavePointManager::new(),
        ));
        let warp_service = Arc::new(crate::game::map::teleport::WarpService::new(
            teleport_manager.clone(),
            save_point_manager.clone(),
            db.clone(),
        ));

        MapServer::new(
            db,
            Arc::new(crate::game::token::TokenStore::new()),
            map_state,
            channel_bus,
            Arc::new(crate::game::map::drop_item::DropManager::new()),
            Arc::new(crate::game::party::PartyManager::new()),
            Arc::new(crate::game::guild::GuildManager::new()),
            Arc::new(crate::game::storage::StorageManager::new()),
            Arc::new(crate::game::trade::TradeManager::new()),
            teleport_manager,
            warp_service,
            false,
            Arc::new(crate::game::battle::BattleHandler::default()),
            Arc::new(crate::game::mob::MobSpawnManager::new()),
        )
    }

    /// 创建测试用 Player（指定职业和等级）
    fn make_test_player_with_job(job: u16, base_level: u16, job_level: u16) -> crate::game::map::Player {
        crate::game::map::Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            map_name: "test_map".to_string(),
            combat: RwLock::new(CombatStats {
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
            pos: RwLock::new(Position { x: 100, y: 100 }),
            level: RwLock::new(LevelStats {
                base_level,
                job_level,
                base_exp: 5000,
                job_exp: 3000,
                status_point: 0,
                skill_point: 0,
            }),
            attrs: RwLock::new(Attributes {
                str: 10,
                agi: 10,
                vit: 10,
                int: 10,
                dex: 10,
                luk: 10,
            }),
            economy: RwLock::new(Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job,
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
        }
    }

    /// 构建 CZ_REQ_CHANGEJOB 数据包体
    fn build_job_change_packet(target_job: u16) -> Vec<u8> {
        let mut data = Vec::with_capacity(2);
        data.extend_from_slice(&target_job.to_le_bytes());
        data
    }

    // ==================== 转职处理器测试 ====================

    #[test]
    fn test_job_change_novice_to_swordman_success() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 10, 1); // Novice, BaseLv 10
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 初始化数据库（转职后需要保存）
        crate::storage::schema::init_schema(&server.db).unwrap();
        server.db.create_account("test", "hash", 0).unwrap();
        server.db.create_character(1, 0, "TestPlayer", 1, 1, 1, 1, 1, 1, 1, 0).unwrap();

        let data = build_job_change_packet(1); // Swordman
        let result = server.handle_job_change(&data, &mut session);

        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt.len(), 6);
        // 验证 packet_id = 0x019E
        assert_eq!(pkt[2], 0x9E);
        assert_eq!(pkt[3], 0x01);
        // 验证 job_id = 1 (Swordman)
        assert_eq!(pkt[4], 1);
        assert_eq!(pkt[5], 0);

        // 验证玩家状态
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.job(), 1); // Swordman
        assert_eq!(player.job_level(), 1); // 重置为 1
        assert_eq!(player.job_exp(), 0); // 重置为 0
    }

    #[test]
    fn test_job_change_insufficient_level() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 5, 1); // Novice, BaseLv 5（不足 10）
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        let data = build_job_change_packet(1); // Swordman
        let result = server.handle_job_change(&data, &mut session);

        // 应返回失败的 ACK（job_id 不变）
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[4], 0); // job_id 仍为 0 (Novice)

        // 职业不应改变
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.job(), 0); // 仍为 Novice
    }

    #[test]
    fn test_job_change_wrong_prerequisite() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(2, 10, 1); // Mage, BaseLv 10
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        let data = build_job_change_packet(7); // Knight（需要 Swordman 前置）
        let result = server.handle_job_change(&data, &mut session);

        // 应返回失败的 ACK
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[4], 2); // job_id 仍为 2 (Mage)

        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.job(), 2); // 仍为 Mage
    }

    #[test]
    fn test_job_change_swordman_to_knight_success() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(1, 40, 40); // Swordman, BaseLv 40, JobLv 40
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        crate::storage::schema::init_schema(&server.db).unwrap();
        server.db.create_account("test", "hash", 0).unwrap();
        server.db.create_character(1, 0, "TestPlayer", 1, 1, 1, 1, 1, 1, 1, 0).unwrap();

        let data = build_job_change_packet(7); // Knight
        let result = server.handle_job_change(&data, &mut session);

        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[4], 7); // Knight

        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.job(), 7);
        assert_eq!(player.job_level(), 1); // 重置
        assert_eq!(player.job_exp(), 0);
    }

    #[test]
    fn test_job_change_invalid_target_job() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 10, 1);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        let data = build_job_change_packet(99); // 无效职业
        let result = server.handle_job_change(&data, &mut session);

        // 应返回失败的 ACK
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[4], 0); // job_id 不变
    }

    #[test]
    fn test_job_change_rejects_no_session() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let server = make_test_server(map_state, channel_bus);

        let mut session = Session::new();
        let data = build_job_change_packet(1);
        let result = server.handle_job_change(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_job_change_rejects_short_data() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 10, 1);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state, channel_bus);

        // 数据太短（只有 1 字节，需要 2 字节）
        let data = vec![1];
        let result = server.handle_job_change(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_job_change_hp_sp_recalculated() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 20, 1); // Novice, BaseLv 20
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        crate::storage::schema::init_schema(&server.db).unwrap();
        server.db.create_account("test", "hash", 0).unwrap();
        server.db.create_character(1, 0, "TestPlayer", 1, 1, 1, 1, 1, 1, 1, 0).unwrap();

        // 转职为 Swordman（base_hp=200, hp_per_level=30）
        let data = build_job_change_packet(1);
        let result = server.handle_job_change(&data, &mut session);
        assert!(result.is_some());

        let player = map_state.get_player(&player_id).unwrap();
        // max_hp = 200 + (20-1) * 30 = 200 + 570 = 770
        assert_eq!(player.max_hp(), 770);
        assert_eq!(player.hp(), 770); // HP 应恢复到最大值
    }

    #[test]
    fn test_job_change_to_same_job_resets() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(1, 20, 30); // Swordman, JobLv 30
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        crate::storage::schema::init_schema(&server.db).unwrap();
        server.db.create_account("test", "hash", 0).unwrap();
        server.db.create_character(1, 0, "TestPlayer", 1, 1, 1, 1, 1, 1, 1, 0).unwrap();

        // 转职为同一职业（Swordman -> Swordman），应重置 JobLv
        let data = build_job_change_packet(1);
        let result = server.handle_job_change(&data, &mut session);
        assert!(result.is_some());

        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.job(), 1);
        assert_eq!(player.job_level(), 1); // 重置
        assert_eq!(player.job_exp(), 0);
    }

    // ==================== GM 命令转职测试 ====================

    #[test]
    fn test_gm_change_job_success() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 1, 1); // Novice, BaseLv 1
        let player_id = player.id;
        map_state.add_player(player);

        let server = make_test_server(map_state.clone(), channel_bus);

        crate::storage::schema::init_schema(&server.db).unwrap();
        server.db.create_account("test", "hash", 0).unwrap();
        server.db.create_character(1, 0, "TestPlayer", 1, 1, 1, 1, 1, 1, 1, 0).unwrap();

        // GM 命令可以跳过等级检查
        let result = server.gm_change_job(&player_id, 7); // Knight
        assert!(result.is_ok());
        assert!(result.unwrap().contains("骑士"));

        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.job(), 7);
    }

    #[test]
    fn test_gm_change_job_invalid_id() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_with_job(0, 1, 1);
        let player_id = player.id;
        map_state.add_player(player);

        let server = make_test_server(map_state, channel_bus);

        let result = server.gm_change_job(&player_id, 99);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效"));
    }

    #[test]
    fn test_gm_change_job_player_not_found() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let server = make_test_server(map_state, channel_bus);

        let fake_id = Uuid::new_v4();
        let result = server.gm_change_job(&fake_id, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("找不到"));
    }

    // ==================== ACK 包构建测试 ====================

    #[test]
    fn test_build_ack_changejob() {
        let pkt = build_ack_changejob(7); // Knight
        assert_eq!(pkt.len(), 6);
        // length = 6
        assert_eq!(pkt[0], 6);
        assert_eq!(pkt[1], 0);
        // packet_id = 0x019E
        assert_eq!(pkt[2], 0x9E);
        assert_eq!(pkt[3], 0x01);
        // job_id = 7
        assert_eq!(pkt[4], 7);
        assert_eq!(pkt[5], 0);
    }
}
