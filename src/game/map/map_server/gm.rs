//! GM 命令与传送 handler：warp/goto/summon/savepoint、return 传送、重生

use super::MapServer;
use crate::game::map::channel::GameEvent;
use crate::game::map::teleport::{SavePoint, TeleportAction};
use crate::network::session::Session;
use crate::protocol::packet_builder::Packed;
use crate::protocol::teleport_packets::*;

impl MapServer {
    /// 检查 GM 权限
    pub(super) fn check_gm_permission(
        player: &crate::game::map::Player,
        min_level: i32,
    ) -> bool {
        player.group_id() >= min_level
    }

    /// Handle GM warp command (0x0138)
    /// @warp <map_name> <x> <y>
    pub(super) fn handle_gm_warp(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmWarp::from_slice(data)?;
        let player_id = session.player_id?;

        // Check GM permissions
        if let Some(player) = self.map_state.get_player(&player_id) {
            if !Self::check_gm_permission(&player, 10) {
                tracing::warn!("Player {} attempted GM warp without permission", player.name);
                return None;
            }
        }

        // Check warp cooldown
        {
            let tm = self.teleport_manager.read();
            if !tm.can_warp(player_id) {
                return Some(
                    ZCWarpError {
                        error_code: ZCWarpError::COOLDOWN,
                    }
                    .to_packet(),
                );
            }
        }

        let player = self.map_state.get_player(&player_id)?;

        let warp_action = TeleportAction {
            from_map: player.map_name.clone(),
            to_map: pkt.map_name,
            from_pos: player.get_position(),
            to_pos: (pkt.x, pkt.y),
        };

        match self.warp_service.execute_warp(session, warp_action) {
            Ok(_) => Some(ZCWarpAck { warp_type: 3 }.to_packet()),
            Err(_) => Some(
                ZCWarpError {
                    error_code: ZCWarpError::INVALID_MAP,
                }
                .to_packet(),
            ),
        }
    }

    /// Handle GM goto command (0x013A)
    /// @goto <player_name> - teleport to another player
    pub(super) fn handle_gm_goto(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmGoto::from_slice(data)?;
        let player_id = session.player_id?;

        // Check GM permissions
        if let Some(player) = self.map_state.get_player(&player_id) {
            if !Self::check_gm_permission(&player, 10) {
                tracing::warn!("Player {} attempted GM goto without permission", player.name);
                return None;
            }
        }

        // Check warp cooldown
        {
            let tm = self.teleport_manager.read();
            if !tm.can_warp(player_id) {
                return Some(
                    ZCWarpError {
                        error_code: ZCWarpError::COOLDOWN,
                    }
                    .to_packet(),
                );
            }
        }

        let player = self.map_state.get_player(&player_id)?;

        // Find target player by name
        let target = self.map_state.find_player_by_name(&pkt.target_name);

        if let Some(target) = target {
            let target_pos = target.get_position();
            let warp_action = TeleportAction {
                from_map: player.map_name.clone(),
                to_map: target.map_name.clone(),
                from_pos: player.get_position(),
                to_pos: target_pos,
            };

            match self.warp_service.execute_warp(session, warp_action) {
                Ok(_) => Some(ZCWarpAck { warp_type: 3 }.to_packet()),
                Err(_) => Some(
                    ZCWarpError {
                        error_code: ZCWarpError::INVALID_MAP,
                    }
                    .to_packet(),
                ),
            }
        } else {
            Some(
                ZCWarpError {
                    error_code: ZCWarpError::TARGET_NOT_FOUND,
                }
                .to_packet(),
            )
        }
    }

    /// Handle GM summon command (0x013B)
    /// @summon <player_name> - bring another player to current location
    pub(super) fn handle_gm_summon(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmSummon::from_slice(data)?;
        let player_id = session.player_id?;

        // Check GM permissions
        let player = self.map_state.get_player(&player_id)?;
        if !Self::check_gm_permission(&player, 10) {
            tracing::warn!(
                "Player {} attempted GM summon without permission",
                player.name
            );
            return None;
        }
        let _player_pos = player.get_position();
        let _player_map = player.map_name.clone();

        // Find target player by name
        let target = self.map_state.find_player_by_name(&pkt.target_name);

        if let Some(target) = target {
            let target_id = target.id;

            // Check if target can warp
            {
                let tm = self.teleport_manager.read();
                if !tm.can_warp(target_id) {
                    return Some(
                        ZCWarpError {
                            error_code: ZCWarpError::COOLDOWN,
                        }
                        .to_packet(),
                    );
                }
            }

            // Note: In a full implementation, this would need to communicate
            // with the target player's session to warp them
            // For now, return success acknowledgment
            Some(ZCWarpAck { warp_type: 3 }.to_packet())
        } else {
            Some(
                ZCWarpError {
                    error_code: ZCWarpError::TARGET_NOT_FOUND,
                }
                .to_packet(),
            )
        }
    }

    /// Handle GM savepoint command (0x013C)
    /// @savepoint - set current location as save point
    pub(super) fn handle_gm_savepoint(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let char_id = session.char_id?;

        // Check GM permissions
        let player = self.map_state.get_player(&player_id)?;
        if !Self::check_gm_permission(&player, 10) {
            tracing::warn!(
                "Player {} attempted GM savepoint without permission",
                player.name
            );
            return None;
        }

        let (x, y) = player.get_position();

        // Set save point
        {
            let warp_service = &self.warp_service;
            warp_service.set_save_point(char_id, &player.map_name, x, y);
        }

        // Notify player
        Some(
            ZCSavePointSet {
                map_name: player.map_name.clone(),
                x,
                y,
            }
            .to_packet(),
        )
    }

    /// Handle use return (0x0119)
    /// Player uses butterfly wing / return scroll to go back to save point
    pub(super) fn handle_use_return(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let char_id = session.char_id?;

        // Check warp cooldown
        {
            let tm = self.teleport_manager.read();
            if !tm.can_warp(player_id) {
                return Some(
                    ZCWarpError {
                        error_code: ZCWarpError::COOLDOWN,
                    }
                    .to_packet(),
                );
            }
        }

        // Get player and their save point
        let player = self.map_state.get_player(&player_id)?;
        let save_point = {
            let warp_service = &self.warp_service;
            warp_service.get_save_point(char_id)?.clone()
        };

        // Execute the return warp
        let warp_action = TeleportAction {
            from_map: player.map_name.clone(),
            to_map: save_point.map_name.clone(),
            from_pos: (player.get_position()),
            to_pos: (save_point.x, save_point.y),
        };

        match self.warp_service.execute_warp(session, warp_action) {
            Ok(_) => {
                // Send warp acknowledgment
                Some(ZCWarpAck { warp_type: 2 }.to_packet())
            }
            Err(_) => Some(
                ZCWarpError {
                    error_code: ZCWarpError::INVALID_MAP,
                }
                .to_packet(),
            ),
        }
    }

    /// Handle set savepoint (0x01B8)
    /// NPC or item that sets the save point
    pub(super) fn handle_set_savepoint(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let char_id = session.char_id?;

        let player = self.map_state.get_player(&player_id)?;
        let (x, y) = player.get_position();

        // Set save point
        {
            let warp_service = &self.warp_service;
            warp_service.set_save_point(char_id, &player.map_name, x, y);
        }

        // Notify player
        Some(
            ZCSavePointSet {
                map_name: player.map_name.clone(),
                x,
                y,
            }
            .to_packet(),
        )
    }

    /// Handle restart (0x00B2) - 玩家死亡后重生到存储点
    pub(super) fn handle_restart(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let char_id = session.char_id?;
        let player = self.map_state.get_player(&player_id)?;

        // 只有死亡状态才能重生
        if !player.is_dead() {
            return None;
        }

        // 获取存储点（默认回到 new_1-1.gat 的出生点）
        let save_point = self
            .warp_service
            .get_save_point(char_id)
            .unwrap_or_else(|| SavePoint::new("new_1-1.gat", 53, 111));

        // 更新玩家运行时状态（通过 MapState 原子更新）
        self.map_state
            .respawn_player(&player_id, save_point.x, save_point.y, &save_point.map_name);

        // 更新 ChannelBus 中的位置
        let new_channel = format!("map:{}", save_point.map_name);
        self.channel_bus
            .update_position(&new_channel, &player_id, save_point.x, save_point.y);

        // 发布重生事件
        let revive_event = GameEvent::PlayerRevive {
            player_id,
            x: save_point.x,
            y: save_point.y,
        };
        let packet = revive_event.to_packet_bytes();
        self.channel_bus
            .publish(&new_channel, &revive_event, packet);

        // 更新数据库位置（best effort）
        if let Err(e) = self.db.execute_params(
            "UPDATE characters SET last_map = ?1, last_x = ?2, last_y = ?3, hp = ?4, sp = ?5 WHERE char_id = ?6",
            &[
                &save_point.map_name as &dyn crate::storage::backend::IntoValue,
                &(save_point.x as i32) as &dyn crate::storage::backend::IntoValue,
                &(save_point.y as i32) as &dyn crate::storage::backend::IntoValue,
                &(player.max_hp() as i32) as &dyn crate::storage::backend::IntoValue,
                &(player.max_sp() as i32) as &dyn crate::storage::backend::IntoValue,
                &(char_id as i32) as &dyn crate::storage::backend::IntoValue,
            ],
        ) {
            tracing::warn!("Failed to update character position in DB: {}", e);
        }

        None
    }

    /// Handle heartbeat/time sync (0x00A7)
    /// 客户端定期发送，服务器返回当前时间戳
    pub(super) fn handle_request_time(&self) -> Option<Vec<u8>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        // ZC_ACK_TIME (0x007F): packet_id(2) + timestamp(4) = 6 bytes
        let mut response = Vec::with_capacity(6);
        response.extend_from_slice(&6u16.to_le_bytes()); // length
        response.extend_from_slice(&0x007Fu16.to_le_bytes()); // packet_id
        response.extend_from_slice(&now.to_le_bytes()); // timestamp
        Some(response)
    }

    /// Handle quit request (0x00F3)
    /// 保存玩家数据并断开连接
    pub(super) fn handle_request_quit(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // 保存玩家数据到数据库
        if let Err(e) = self.save_player(&player_id) {
            tracing::error!("保存玩家数据失败 (quit): {}", e);
        }

        // 从地图移除玩家
        if let Some(player) = self.map_state.get_player(&player_id) {
            let map_name = player.map_name.clone();
            let channel_name = format!("map:{}", map_name);
            self.channel_bus.unsubscribe(&channel_name, &player_id);
            self.map_state.remove_player(&player_id);
        }

        // 返回断开确认包 ZC_ACK_REQ_DISCONNECT (0x018A)
        let mut response = Vec::with_capacity(4);
        response.extend_from_slice(&4u16.to_le_bytes()); // length
        response.extend_from_slice(&0x018Au16.to_le_bytes()); // packet_id
        Some(response)
    }
}
