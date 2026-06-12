//! 公会 handler：创建、邀请、加入、离开、踢人、公告、信息、聊天

use super::MapServer;
use crate::game::map::channel::{ChatType, GameEvent, guild_channel_name};
use crate::network::session::Session;
use crate::protocol::guild_packets::*;
use crate::protocol::packet_builder::Packed;
use uuid::Uuid;

impl MapServer {
    /// Handle guild create (0x0165)
    pub(super) fn handle_guild_create(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildCreate::from_slice(data)?;

        if self.guild_manager.get_player_guild(&player_id).is_some() {
            return Some(
                ZCGuildCreated {
                    result: 2,
                    guild_id: 0,
                }
                .to_packet(),
            );
        }

        let player = self.map_state.get_player(&player_id)?;
        match self
            .guild_manager
            .create_guild(pkt.name.clone(), player.name.clone())
        {
            Some(guild_id) => {
                self.guild_manager
                    .join_guild(guild_id, player_id, player.name.clone());
                self.guild_manager
                    .set_member_position_direct(&guild_id, &player_id, 0);
                Some(
                    ZCGuildCreated {
                        result: 0,
                        guild_id: 0,
                    }
                    .to_packet(),
                )
            }
            None => Some(
                ZCGuildCreated {
                    result: 1,
                    guild_id: 0,
                }
                .to_packet(),
            ),
        }
    }

    /// Handle guild invite (0x0168)
    pub(super) fn handle_guild_invite(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildInvite::from_slice(data)?;

        let guild = self.guild_manager.get_player_guild(&player_id)?;
        if !guild.has_permission(&player_id, crate::game::guild::GuildPermission::Invite) {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        // 查找目标玩家
        let target = self.map_state.find_player_by_name(&pkt.target_name)?;
        let target_id = target.id;

        if self.guild_manager.get_player_guild(&target_id).is_some() {
            return None; // 目标已在公会中
        }

        // 发送邀请通知给目标 (简化实现，直接返回ack)
        Some(
            ZCGuildInvite {
                guild_id: 0,
                guild_name: guild.name.clone(),
                inviter_name: player.name.clone(),
            }
            .to_packet(),
        )
    }

    /// Handle guild join reply (0x0169)
    pub(super) fn handle_guild_join(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildJoin::from_slice(data)?;

        if !pkt.accept {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        let guild_id = Uuid::from_u128(pkt.guild_id as u128);

        if self
            .guild_manager
            .join_guild(guild_id, player_id, player.name.clone())
        {
            Some(ZCGuildLeaveResult { result: 0 }.to_packet())
        } else {
            Some(ZCGuildLeaveResult { result: 1 }.to_packet())
        }
    }

    /// Handle guild leave (0x016B)
    pub(super) fn handle_guild_leave(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        if self.guild_manager.leave_guild(player_id) {
            Some(ZCGuildLeaveResult { result: 0 }.to_packet())
        } else {
            Some(ZCGuildLeaveResult { result: 1 }.to_packet())
        }
    }

    /// Handle guild expel (0x016C)
    pub(super) fn handle_guild_expel(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildExpel::from_slice(data)?;

        let guild = self.guild_manager.get_player_guild(&player_id)?;
        let guild_id = guild.id;

        let target = self.map_state.find_player_by_name(&pkt.target_name)?;
        let target_id = target.id;

        if self
            .guild_manager
            .expel_member(guild_id, &player_id, &target_id)
        {
            Some(
                ZCGuildExpelResult {
                    result: 0,
                    target_name: pkt.target_name,
                    reason: pkt.reason,
                }
                .to_packet(),
            )
        } else {
            Some(
                ZCGuildExpelResult {
                    result: 1,
                    target_name: pkt.target_name,
                    reason: String::new(),
                }
                .to_packet(),
            )
        }
    }

    /// Handle guild change notice (0x0183)
    pub(super) fn handle_guild_change_notice(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildChangeNotice::from_slice(data)?;

        let guild_id = self.guild_manager.get_player_guild_id(&player_id)?;
        let guild = self.guild_manager.get_guild(&guild_id)?;

        if guild.has_permission(&player_id, crate::game::guild::GuildPermission::Expel) {
            self.guild_manager
                .update_notice(&guild_id, pkt.notice.clone());
            return Some(ZCGuildNotice { notice: pkt.notice }.to_packet());
        }

        None
    }

    /// Handle guild request info (0x01B7)
    pub(super) fn handle_guild_request_info(
        &self,
        _data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let guild = self.guild_manager.get_player_guild(&player_id)?;

        Some(
            ZCGuildInfo {
                guild_id: 0,
                level: guild.level,
                member_count: guild.member_count,
                max_members: guild.max_members,
                average_level: guild.average_level,
                exp: guild.exp,
                max_exp: guild.max_exp,
                notice: guild.notice.clone(),
            }
            .to_packet(),
        )
    }

    /// Handle guild chat (0x01EC)
    pub(super) fn handle_guild_chat(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildChat::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;
        let guild = self.guild_manager.get_player_guild(&player_id)?;

        let channel_name = guild_channel_name(guild.id);
        let event = GameEvent::PlayerChat {
            player_id,
            message: pkt.message.clone(),
            chat_type: ChatType::Party, // 复用Party聊天类型
        };
        let packet = event.to_packet_bytes();
        self.channel_bus.publish(&channel_name, &event, packet);

        Some(
            ZCGuildChat {
                sender_name: player.name.clone(),
                message: pkt.message,
            }
            .to_packet(),
        )
    }
}
