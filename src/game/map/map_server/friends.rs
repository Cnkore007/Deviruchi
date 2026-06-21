//! 好友系统 handler

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::friend_packets::{CzFriendsListAdd, CzFriendsListRemove, CzFriendsListReply};

impl MapServer {
    /// 处理添加好友请求 (0x0201)
    pub(super) fn handle_friends_list_add(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzFriendsListAdd::from_slice(data)?;

        tracing::info!("Player {} 请求添加好友: char_id={}", player_id, pkt.char_id);

        // 简化实现：记录日志
        // 完整实现需要发送好友请求给目标玩家
        tracing::info!("Player {} 发送好友请求", player_id);

        None
    }

    /// 处理删除好友请求 (0x0203)
    pub(super) fn handle_friends_list_remove(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzFriendsListRemove::from_slice(data)?;

        tracing::info!("Player {} 请求删除好友: char_id={}", player_id, pkt.char_id);

        // 简化实现：记录日志
        tracing::info!("Player {} 删除好友", player_id);

        None
    }

    /// 处理好友请求回复 (0x0208)
    pub(super) fn handle_friends_list_reply(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzFriendsListReply::from_slice(data)?;

        tracing::info!(
            "Player {} 回复好友请求: char_id={}, reply={}",
            player_id,
            pkt.char_id,
            pkt.reply
        );

        // 简化实现：记录日志
        if pkt.reply == 1 {
            tracing::info!("Player {} 接受好友请求", player_id);
        } else {
            tracing::info!("Player {} 拒绝好友请求", player_id);
        }

        None
    }
}
