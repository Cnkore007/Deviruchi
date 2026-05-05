//! 方向改变与私聊 handler

use super::MapServer;
use crate::game::map::channel::GameEvent;
use crate::network::packet::id::*;
use crate::network::session::Session;
use uuid::Uuid;

impl MapServer {
    /// 处理方向改变请求 (CZ_REQUEST_CHANGE_DIRECTION, 0x00D9)
    ///
    /// 客户端发送格式：packet_id(2) + direction(2)
    /// direction 取值 0-7，表示8个方向
    ///
    /// 处理逻辑：
    /// 1. 验证 direction 范围（0-7）
    /// 2. 更新玩家朝向
    /// 3. 向同地图其他玩家广播方向变化事件
    pub(super) fn handle_change_direction(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // 解析方向值（u16, little-endian）
        if data.len() < 2 {
            return None;
        }
        let direction = u16::from_le_bytes([data[0], data[1]]);

        // 验证方向范围
        if direction > 7 {
            tracing::warn!(
                player_id = %player_id,
                direction = direction,
                "方向改变请求被拒绝：direction 超出范围 0-7"
            );
            return None;
        }

        // 通过 MapState 直接更新存储的玩家朝向
        self.map_state.set_player_direction(&player_id, direction);

        let player = self.map_state.get_player(&player_id)?;

        // 向同地图其他玩家广播方向变化
        let channel_name = format!("map:{}", player.map_name);
        let event = GameEvent::PlayerDirectionChange {
            player_id,
            direction,
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        tracing::debug!(
            player = %player.name,
            direction = direction,
            "玩家方向已更新"
        );

        None
    }

    /// 处理私聊请求 (CZ_WHISPER, 0x00F7)
    ///
    /// 客户端发送格式：packet_id(2) + length(2) + name(24) + message(variable)
    /// - length: 整个包的长度
    /// - name: 目标玩家名字，24字节，null 终止
    /// - message: 消息内容，变长
    ///
    /// 处理逻辑：
    /// 1. 解析目标玩家名字和消息内容
    /// 2. 查找目标玩家是否在线
    /// 3. 在线：通过 ChannelBus 发送 ZC_WHISPER (0x0097) 给目标
    /// 4. 离线：发送 ZC_ACK_WHISPER (0x0098) status=1 给发送者
    pub(super) fn handle_whisper(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // 包体最小长度：length(2) + name(24) = 26 字节
        if data.len() < 26 {
            return None;
        }

        // 解析长度字段（跳过，因为我们用 data 长度）
        let _pkt_length = u16::from_le_bytes([data[0], data[1]]);

        // 解析目标玩家名字（24 字节，null 终止）
        let name_raw = &data[2..26];
        let name_end = name_raw.iter().position(|&b| b == 0).unwrap_or(24);
        let target_name = match std::str::from_utf8(&name_raw[..name_end]) {
            Ok(s) => s.to_string(),
            Err(_) => return None,
        };

        // 解析消息内容（从第 26 字节到包末尾）
        let message = if data.len() > 26 {
            match std::str::from_utf8(&data[26..]) {
                Ok(s) => s.trim_end_matches('\0').to_string(),
                Err(_) => return None,
            }
        } else {
            String::new()
        };

        let sender = self.map_state.get_player(&player_id)?;

        tracing::info!(
            sender = %sender.name,
            target = %target_name,
            message = %message,
            "私聊消息"
        );

        // 查找目标玩家
        match self.map_state.find_player_by_name(&target_name) {
            Some(target) => {
                // 目标在线，构建 ZC_WHISPER 包发送给目标
                let sender_name = &sender.name;
                let msg_bytes = message.as_bytes();

                // ZC_WHISPER 格式：length(2) + packet_id(2) + sender_name(24) + message(variable)
                let pkt_len = 2 + 2 + 24 + msg_bytes.len();
                let mut whisper_pkt = Vec::with_capacity(pkt_len);
                whisper_pkt.extend_from_slice(&(pkt_len as u16).to_le_bytes());
                whisper_pkt.extend_from_slice(&ZC_WHISPER.to_le_bytes());

                // 发送者名字（24 字节，null 填充）
                let mut name_buf = [0u8; 24];
                let copy_len = sender_name.len().min(24);
                name_buf[..copy_len].copy_from_slice(&sender_name.as_bytes()[..copy_len]);
                whisper_pkt.extend_from_slice(&name_buf);

                // 消息内容
                whisper_pkt.extend_from_slice(msg_bytes);

                self.channel_bus.send_to_player(&target.id, whisper_pkt);
            }
            None => {
                // 目标离线，发送 ZC_ACK_WHISPER 给发送者
                // ZC_ACK_WHISPER 格式：length(2) + packet_id(2) + name(24) + status(1)
                let mut ack_pkt = Vec::with_capacity(29);
                ack_pkt.extend_from_slice(&29u16.to_le_bytes()); // length
                ack_pkt.extend_from_slice(&ZC_ACK_WHISPER.to_le_bytes());

                // 目标名字（24 字节，null 填充）
                let mut name_buf = [0u8; 24];
                let copy_len = target_name.len().min(24);
                name_buf[..copy_len].copy_from_slice(&target_name.as_bytes()[..copy_len]);
                ack_pkt.extend_from_slice(&name_buf);

                // status = 1 表示目标不在线
                ack_pkt.push(1);

                return Some(ack_pkt);
            }
        }

        None
    }
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
    use tokio::sync::mpsc;
    use uuid::Uuid;

    /// 创建测试用 Player
    fn make_test_player(name: &str, map: &str, x: u16, y: u16) -> crate::game::map::Player {
        crate::game::map::Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: name.to_string(),
            map_name: map.to_string(),
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
            pos: RwLock::new(Position { x, y }),
            level: RwLock::new(LevelStats {
                base_level: 10,
                job_level: 5,
                base_exp: 5000,
                job_exp: 3000,
                status_point: 100,
            }),
            attrs: RwLock::new(Attributes {
                str: 1,
                agi: 1,
                vit: 1,
                int: 1,
                dex: 1,
                luk: 1,
            }),
            economy: RwLock::new(Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: map.to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        }
    }

    /// 构建 CZ_REQUEST_CHANGE_DIRECTION 数据包体（不含 packet_id）
    fn build_direction_packet(direction: u16) -> Vec<u8> {
        direction.to_le_bytes().to_vec()
    }

    /// 构建 CZ_WHISPER 数据包体（不含 packet_id）
    fn build_whisper_packet(target_name: &str, message: &str) -> Vec<u8> {
        let total_len = 2 + 24 + message.len(); // length(2) + name(24) + message
        let mut data = Vec::with_capacity(total_len);
        data.extend_from_slice(&(total_len as u16).to_le_bytes());

        // 名字 24 字节，null 填充
        let mut name_buf = [0u8; 24];
        let name_bytes = target_name.as_bytes();
        let copy_len = name_bytes.len().min(24);
        name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        data.extend_from_slice(&name_buf);

        // 消息
        data.extend_from_slice(message.as_bytes());
        data
    }

    // ==================== 方向改变测试 ====================

    #[test]
    fn test_direction_change_updates_player_direction() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player("TestPlayer", "test_map", 100, 100);
        let player_id = player.id;

        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        // 构建 MapServer（使用简化的构造方式）
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

        let server = MapServer::new(
            db,
            Arc::new(crate::game::token::TokenStore::new()),
            map_state.clone(),
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
        );

        // 发送方向改变请求（direction = 3）
        let data = build_direction_packet(3);
        let result = server.handle_change_direction(&data, &mut session);
        assert!(result.is_none()); // 方向改变不返回响应包

        // 验证方向已更新
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.direction(), 3);
    }

    #[test]
    fn test_direction_change_rejects_invalid_direction() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player("TestPlayer", "test_map", 100, 100);
        let player_id = player.id;

        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

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

        let server = MapServer::new(
            db,
            Arc::new(crate::game::token::TokenStore::new()),
            map_state.clone(),
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
        );

        // 发送无效方向（direction = 8，超出 0-7 范围）
        let data = build_direction_packet(8);
        let result = server.handle_change_direction(&data, &mut session);
        assert!(result.is_none());

        // 验证方向未改变（仍为初始值 0）
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.direction(), 0);
    }

    #[test]
    fn test_direction_change_rejects_no_session() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());

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

        let server = MapServer::new(
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
        );

        let mut session = Session::new(); // 无 player_id
        let data = build_direction_packet(3);
        let result = server.handle_change_direction(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_direction_change_rejects_empty_data() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player("TestPlayer", "test_map", 100, 100);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

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

        let server = MapServer::new(
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
        );

        // 空数据
        let result = server.handle_change_direction(&[], &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_direction_change_boundary_values() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player("TestPlayer", "test_map", 100, 100);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

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

        let server = MapServer::new(
            db,
            Arc::new(crate::game::token::TokenStore::new()),
            map_state.clone(),
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
        );

        // 测试边界值：direction = 0 和 direction = 7 都应被接受
        let data = build_direction_packet(0);
        server.handle_change_direction(&data, &mut session);
        assert_eq!(map_state.get_player(&player_id).unwrap().direction(), 0);

        let data = build_direction_packet(7);
        server.handle_change_direction(&data, &mut session);
        assert_eq!(map_state.get_player(&player_id).unwrap().direction(), 7);
    }

    // ==================== 私聊测试 ====================

    #[test]
    fn test_whisper_sends_to_online_target() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());

        let sender = make_test_player("Sender", "test_map", 100, 100);
        let target = make_test_player("Target", "test_map", 110, 110);
        let sender_id = sender.id;
        let target_id = target.id;

        map_state.add_player(sender);
        map_state.add_player(target);

        // 为目标订阅到 channel bus
        let (tx, _rx) = mpsc::unbounded_channel();
        channel_bus.subscribe("map:test_map", target_id, tx, 110, 110);

        let mut session = Session::new();
        session.player_id = Some(sender_id);

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

        let server = MapServer::new(
            db,
            Arc::new(crate::game::token::TokenStore::new()),
            map_state,
            channel_bus.clone(),
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
        );

        let data = build_whisper_packet("Target", "Hello there!");
        let result = server.handle_whisper(&data, &mut session);
        // 在线目标不返回响应包（消息通过 ChannelBus 发送）
        assert!(result.is_none());
    }

    #[test]
    fn test_whisper_returns_offline_ack_for_missing_target() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());

        let sender = make_test_player("Sender", "test_map", 100, 100);
        let sender_id = sender.id;
        map_state.add_player(sender);

        let mut session = Session::new();
        session.player_id = Some(sender_id);

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

        let server = MapServer::new(
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
        );

        // 发送给不存在的玩家
        let data = build_whisper_packet("NonExistent", "Hello?");
        let result = server.handle_whisper(&data, &mut session);

        // 应返回 ZC_ACK_WHISPER（status=1）
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt.len(), 29); // length(2) + packet_id(2) + name(24) + status(1)
        // 验证 packet_id = 0x0098
        assert_eq!(pkt[2], 0x98);
        assert_eq!(pkt[3], 0x00);
        // 验证 status = 1（离线）
        assert_eq!(pkt[28], 1);
    }

    #[test]
    fn test_whisper_rejects_no_session() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());

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

        let server = MapServer::new(
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
        );

        let mut session = Session::new();
        let data = build_whisper_packet("Target", "Hello");
        let result = server.handle_whisper(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_whisper_rejects_short_data() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let sender = make_test_player("Sender", "test_map", 100, 100);
        let sender_id = sender.id;
        map_state.add_player(sender);

        let mut session = Session::new();
        session.player_id = Some(sender_id);

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

        let server = MapServer::new(
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
        );

        // 数据太短
        let data = vec![0u8; 10]; // 小于 26 字节
        let result = server.handle_whisper(&data, &mut session);
        assert!(result.is_none());
    }
}
