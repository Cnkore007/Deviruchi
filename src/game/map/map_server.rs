//! MapServer - 地图服务器核心，处理客户端数据包

use std::sync::Arc;
use uuid::Uuid;
use crate::network::session::Session;
use crate::game::token::TokenStore;
use crate::game::map::MapState;
use crate::game::map::channel::{ChannelBus, GameEvent, ChatType};
use crate::game::map::drop_item::DropManager;
use crate::game::party::PartyManager;
use crate::protocol::char_packets::{CZRequestMove, CZUseSkill};
use crate::protocol::map_packets::{CZRequestAction, CZUseItem, CZRequestPickupItem, CZContactNpc};
use crate::protocol::party_packets::{CZMakeParty, CZReqPartyInvite, CZReqPartyJoin, CZPartyChat, CZChatMessage};
use crate::protocol::packet_builder::Packed;

pub struct MapServer {
    pub db: Arc<crate::storage::Database>,
    pub token_store: Arc<TokenStore>,
    pub map_state: Arc<MapState>,
    pub channel_bus: Arc<ChannelBus>,
    pub drop_manager: Arc<DropManager>,
    pub party_manager: Arc<PartyManager>,
    pub death_drop_items: bool,
}

impl MapServer {
    pub fn new(
        db: Arc<crate::storage::Database>,
        token_store: Arc<TokenStore>,
        map_state: Arc<MapState>,
        channel_bus: Arc<ChannelBus>,
        drop_manager: Arc<DropManager>,
        party_manager: Arc<PartyManager>,
        death_drop_items: bool,
    ) -> Self {
        Self {
            db,
            token_store,
            map_state,
            channel_bus,
            drop_manager,
            party_manager,
            death_drop_items,
        }
    }

    /// Handle incoming packet
    pub fn handle_packet(&self, packet_id: u16, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        match packet_id {
            0x007C => self.handle_enter(data, session),
            0x0085 => self.handle_move(data, session),
            0x0112 => self.handle_use_skill(data, session),
            0x0089 => self.handle_attack(data, session),
            0x009B => self.handle_use_item(data, session),
            0x0090 => self.handle_pickup_item(data, session),
            0x0190 => self.handle_npc_interact(data, session),
            0x0100 => self.handle_party_create(data, session),
            0x0101 => self.handle_party_invite(data, session),
            0x0102 => self.handle_party_reply(data, session),
            0x0103 => self.handle_party_leave(session),
            0x0109 => self.handle_party_chat(data, session),
            0x010C => self.handle_chat(data, session),
            _ => None,
        }
    }

    /// Handle player enter map (0x007C)
    /// Simplified: expects data to contain account_id and char_id and token
    fn handle_enter(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        if data.len() < 8 {
            return None;
        }
        let account_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let char_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let token_len = if data.len() > 8 { (data.len() - 8).min(32) } else { 0 };
        let token = String::from_utf8_lossy(&data[8..8 + token_len]).to_string();

        // Verify token
        if !self.token_store.verify(&token, account_id, char_id) {
            return None;
        }

        // Load character from DB
        let character = self.db.get_character_by_id(char_id).ok()??;

        // Create player
        let mut player = crate::game::map::Player::from_character(character);
        player.account_id = account_id;

        let player_id = player.id;
        let pos_x = *player.pos_x.read();
        let pos_y = *player.pos_y.read();
        let map_name = player.map_name.clone();

        // Add to map state
        self.map_state.add_player(player);

        // Update session
        session.player_id = Some(player_id);

        // Subscribe to map channel
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let channel_name = format!("map:{}", map_name);
        self.channel_bus.subscribe(&channel_name, player_id, tx, pos_x, pos_y);

        // Return accept packet (simplified)
        Some(vec![0x2D, 0xD3, 0x00, 0x00])
    }

    /// Handle player move (0x0085)
    fn handle_move(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let player = self.map_state.get_player(&player_id)?;

        let move_pkt = CZRequestMove::from_slice(data)?;

        let from_x = *player.pos_x.read();
        let from_y = *player.pos_y.read();
        player.move_to(move_pkt.pos_x, move_pkt.pos_y);

        // Update channel position
        let channel_name = format!("map:{}", player.map_name);
        self.channel_bus.update_position(&channel_name, &player_id, move_pkt.pos_x, move_pkt.pos_y);

        None
    }

    /// Handle use skill (0x0112)
    fn handle_use_skill(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let skill_pkt = CZUseSkill::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        // Publish skill event
        let channel_name = format!("map:{}", player.map_name);
        let event = GameEvent::PlayerUseSkill {
            caster_id: player_id,
            skill_id: skill_pkt.skill_id as u32,
            target_id: None,
            x: skill_pkt.target_x,
            y: skill_pkt.target_y,
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        None
    }

    /// Handle attack (0x0089)
    fn handle_attack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let action_pkt = CZRequestAction::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        let channel_name = format!("map:{}", player.map_name);
        let event = GameEvent::PlayerAttack {
            attacker_id: player_id,
            target_id: Uuid::from_u128(action_pkt.target_id as u128),
            damage: 10,
            is_crit: false,
            killed: false,
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        None
    }

    /// Handle use item (0x009B)
    fn handle_use_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let _player_id = session.player_id?;
        let _item_pkt = CZUseItem::from_slice(data)?;
        // Simplified - actual item use logic handled elsewhere
        None
    }

    /// Handle pickup item (0x0090)
    fn handle_pickup_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pickup_pkt = CZRequestPickupItem::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        // Find drop at position
        if let Some(drop) = self.drop_manager.find_at_position(pickup_pkt.x, pickup_pkt.y, &player.map_name) {
            self.drop_manager.pickup(&drop.id);

            let channel_name = format!("map:{}", player.map_name);
            let event = GameEvent::ItemPickup {
                player_id,
                item_id: drop.item_id,
                amount: drop.amount,
            };
            self.channel_bus.publish(&channel_name, &event, vec![]);
        }

        None
    }

    /// Handle NPC interact (0x0190)
    fn handle_npc_interact(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let _player_id = session.player_id?;
        let _npc_pkt = CZContactNpc::from_slice(data)?;
        // Simplified - NPC interaction handled elsewhere
        None
    }

    /// Handle create party (0x0100)
    fn handle_party_create(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZMakeParty::from_slice(data)?;

        // Check if already in a party
        if self.party_manager.get_player_party(&player_id).is_some() {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        let party = self.party_manager.create_party(&pkt.party_name, player_id, player.name.clone());

        // Subscribe to party channel
        let channel_name = format!("party:{}", party.id);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let (x, y) = player.get_position();
        self.channel_bus.subscribe(&channel_name, player_id, tx, x, y);

        None
    }

    /// Handle party invite (0x0101)
    fn handle_party_invite(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let _player_id = session.player_id?;
        let _pkt = CZReqPartyInvite::from_slice(data)?;
        // Simplified - party invite logic handled elsewhere
        None
    }

    /// Handle party reply (0x0102)
    fn handle_party_reply(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZReqPartyJoin::from_slice(data)?;

        if !pkt.accept {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        let party_id = Uuid::from_u128(pkt.party_id as u128);

        self.party_manager.join_party(&party_id, player_id, player.name.clone())?;

        // Subscribe to party channel
        let channel_name = format!("party:{}", party_id);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let (x, y) = player.get_position();
        self.channel_bus.subscribe(&channel_name, player_id, tx, x, y);

        None
    }

    /// Handle party leave (0x0103)
    fn handle_party_leave(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        self.party_manager.leave_party(&player_id);
        None
    }

    /// Handle party chat (0x0109)
    fn handle_party_chat(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZPartyChat::from_slice(data)?;

        if let Some(party) = self.party_manager.get_player_party(&player_id) {
            let player = self.map_state.get_player(&player_id)?;
            let channel_name = format!("party:{}", party.id);

            let event = GameEvent::PlayerChat {
                player_id,
                message: pkt.message,
                chat_type: ChatType::Party,
            };
            self.channel_bus.publish(&channel_name, &event, vec![]);
        }

        None
    }

    /// Handle map chat (0x010C)
    fn handle_chat(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZChatMessage::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;
        let channel_name = format!("map:{}", player.map_name);

        let event = GameEvent::PlayerChat {
            player_id,
            message: pkt.message,
            chat_type: ChatType::Map,
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_server_handles_unknown_packet() {
        let db = Arc::new(crate::storage::Database::open_memory().unwrap());
        let server = MapServer::new(
            db,
            Arc::new(TokenStore::new()),
            Arc::new(MapState::new()),
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            false,
        );
        let mut session = Session::new();
        let result = server.handle_packet(0xFFFF, &[], &mut session);
        assert!(result.is_none());
    }
}
