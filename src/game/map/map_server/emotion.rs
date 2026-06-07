//! 表情 handler

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{CzEmotion, ZcEmotion};
use crate::protocol::packet_builder::Packed;

impl MapServer {
    /// 处理表情请求 (0x00BF)
    pub(super) fn handle_emotion(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzEmotion::from_slice(data)?;

        tracing::info!(
            "Player {} 发送表情: {}",
            player_id, pkt.emotion
        );

        // 广播表情给周围玩家（简化实现）
        // TODO: 广播表情给周围玩家（当前仅发回发送者）
        let _player = self.map_state.get_player(&player_id)?;

        // 使用 UUID 的前 4 字节作为实体 ID
        let entity_id = u32::from_le_bytes([
            player_id.as_bytes()[0],
            player_id.as_bytes()[1],
            player_id.as_bytes()[2],
            player_id.as_bytes()[3],
        ]);

        Some(ZcEmotion {
            entity_id,
            emotion: pkt.emotion,
        }.to_packet())
    }
}
