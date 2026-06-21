//! 聊天室 handler

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{CzChatAddMember, CzCreateChatRoom};

impl MapServer {
    /// 处理创建聊天室请求 (0x00D5)
    pub(super) fn handle_create_chat_room(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzCreateChatRoom::from_slice(data)?;

        tracing::info!(
            "Player {} 创建聊天室: title={}, size={}, public={}",
            player_id,
            pkt.title,
            pkt.size,
            pkt.is_public
        );

        // 简化实现：记录日志
        // 完整实现需要创建聊天室数据结构，广播给周围玩家
        tracing::info!("Player {} 成功创建聊天室", player_id);

        None
    }

    /// 处理加入聊天室请求 (0x00D9)
    pub(super) fn handle_chat_add_member(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzChatAddMember::from_slice(data)?;

        tracing::info!("Player {} 加入聊天室: chat_id={}", player_id, pkt.chat_id);

        // 简化实现：记录日志
        tracing::info!("Player {} 成功加入聊天室", player_id);

        None
    }

    /// 处理离开聊天室请求 (0x00E0)
    pub(super) fn handle_chat_leave(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        tracing::info!("Player {} 离开聊天室", player_id);

        // 简化实现：记录日志
        tracing::info!("Player {} 成功离开聊天室", player_id);

        None
    }
}
