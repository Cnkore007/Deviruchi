//! 社交与经济 handler：队伍、聊天、仓库、交易

use super::MapServer;
use crate::game::map::channel::{ChatType, GameEvent};
use crate::game::trade::TradeItem;
use crate::network::session::Session;
use crate::protocol::packet_builder::Packed;
use crate::protocol::party_packets::{
    CZChatMessage, CZMakeParty, CZPartyChat, CZReqPartyInvite, CZReqPartyJoin,
};
use crate::protocol::storage_packets::*;
use crate::protocol::trade_packets::*;

impl MapServer {
    /// Handle create party (0x0100)
    pub(super) fn handle_party_create(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZMakeParty::from_slice(data)?;

        // Check if already in a party
        if self.party_manager.get_player_party(&player_id).is_some() {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        let party =
            self.party_manager
                .create_party(&pkt.party_name, player_id, player.name.clone());

        // Subscribe to party channel using session's event sender
        if let Some(tx) = &session.map_event_tx {
            let channel_name = format!("party:{}", party.id);
            let (x, y) = player.get_position();
            self.channel_bus
                .subscribe(&channel_name, player_id, tx.clone(), x, y);
        }

        None
    }

    /// Handle party invite (0x0101)
    pub(super) fn handle_party_invite(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZReqPartyInvite::from_slice(data)?;

        // Check inviter is in a party
        let party = self.party_manager.get_player_party(&player_id)?;
        let inviter = self.map_state.get_player(&player_id)?;

        // Find target player
        let target = self.map_state.find_player_by_account_id(pkt.target_account_id)?;

        tracing::info!(
            "Player {} ({}) invited player {} ({}) to party {} ({})",
            inviter.name,
            player_id,
            target.name,
            pkt.target_account_id,
            party.name,
            party.id
        );

        // Build invite packet: opcode(2) + party_id(4) + party_name(24)
        // Use opcode 0x00dc (ZC_PARTY_JOIN_REQ) as placeholder
        let mut packet = Vec::with_capacity(30);
        packet.extend_from_slice(&0x00dcu16.to_le_bytes());
        packet.extend_from_slice(&party.id.as_bytes()[0..4]); // first 4 bytes of UUID as party_id
        let name_bytes = party.name.as_bytes();
        let mut name_buf = [0u8; 24];
        let copy_len = name_bytes.len().min(24);
        name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
        packet.extend_from_slice(&name_buf);

        self.channel_bus.send_to_player(&target.id, packet);

        None
    }

    /// Handle party reply (0x0102)
    pub(super) fn handle_party_reply(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZReqPartyJoin::from_slice(data)?;

        if !pkt.accept {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        let party = self.party_manager.find_party_by_short_id(pkt.party_id)?;

        self.party_manager
            .join_party(&party.id, player_id, player.name.clone())?;

        // Subscribe to party channel using session's event sender
        if let Some(tx) = &session.map_event_tx {
            let channel_name = format!("party:{}", party.id);
            let (x, y) = player.get_position();
            self.channel_bus
                .subscribe(&channel_name, player_id, tx.clone(), x, y);
        }

        None
    }

    /// Handle party leave (0x0103)
    pub(super) fn handle_party_leave(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let party = self.party_manager.get_player_party(&player_id)?;
        let channel_name = format!("party:{}", party.id);
        self.party_manager.leave_party(&player_id);
        self.channel_bus.unsubscribe(&channel_name, &player_id);
        None
    }

    /// Handle party chat (0x0109)
    pub(super) fn handle_party_chat(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZPartyChat::from_slice(data)?;

        if let Some(party) = self.party_manager.get_player_party(&player_id) {
            let _player = self.map_state.get_player(&player_id)?;
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
    pub(super) fn handle_chat(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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

    /// Handle storage open request (0x0213)
    pub(super) fn handle_storage_open(&self, session: &Session) -> Option<Vec<u8>> {
        let char_id = session.char_id?;

        // Get or create storage
        let storage = self.storage_manager.get_or_create(char_id, 100);
        let storage = storage.read();

        // Build item list
        let items: Vec<_> = storage
            .slots()
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| StorageItem {
                index: s.index,
                item_id: s.item_id,
                amount: s.amount,
                identified: s.identified,
            })
            .collect();

        // Send storage items packet
        let items_packet = ZCStorageItems {
            count: items.len() as u16,
            items,
        }
        .to_packet();

        Some(items_packet)
    }

    /// Handle storage close request (0x0214)
    pub(super) fn handle_storage_close(&self, session: &Session) -> Option<Vec<u8>> {
        // Save storage to database
        if let Some(char_id) = session.char_id
            && let Some(storage) = self.storage_manager.get(char_id)
        {
            let storage = storage.read();
            if let Err(e) = self.db.save_storage(&storage) {
                tracing::error!("Failed to save storage for char {}: {}", char_id, e);
            }
        }

        Some(ZCStorageClose.to_packet())
    }

    /// Handle storage move item (0x0215)
    pub(super) fn handle_storage_move_item(
        &self,
        data: &[u8],
        session: &Session,
    ) -> Option<Vec<u8>> {
        let req = CZReqStorageMoveItem::from_packet(data)?;
        let char_id = session.char_id?;

        let storage = self.storage_manager.get_or_create(char_id, 100);

        if req.is_to_storage {
            // Simplified: just add to storage
            let mut s = storage.write();
            if s.add_item(req.from_index, req.amount) {
                // Find the slot
                for slot in s.slots() {
                    if slot.item_id == req.from_index {
                        return Some(
                            ZCStorageItemAdd {
                                index: slot.index,
                                item_id: slot.item_id,
                                amount: slot.amount,
                                identified: slot.identified,
                            }
                            .to_packet(),
                        );
                    }
                }
            }
        } else {
            // From storage
            let mut s = storage.write();
            if s.remove_item(req.from_index, req.amount) {
                // Simplified: just acknowledge
                return Some(
                    ZCStorageItemRemove {
                        index: req.from_index,
                        amount: req.amount,
                    }
                    .to_packet(),
                );
            }
        }

        None
    }

    /// Handle trade request (0x00E4)
    /// Player requests to trade with a target player. Creates a trade session
    /// and sends the request notification to the target.
    pub(super) fn handle_trade_request(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZTradeRequest::from_packet(data)?;

        // Don't allow trading with yourself
        let requester = self.map_state.get_player(&player_id)?;
        if requester.account_id == pkt.target_account_id {
            return None;
        }

        // Find target player by account_id
        let target = self
            .map_state
            .find_player_by_account_id(pkt.target_account_id)?;
        let target_id = target.id;

        // Don't allow if either player is already in a trade
        if self.trade_manager.find_session_for_player(player_id).is_some() {
            return None;
        }
        if self
            .trade_manager
            .find_session_for_player(target_id)
            .is_some()
        {
            return None;
        }

        // Create trade session via TradeManager
        let _session_id = self.trade_manager.request_trade(player_id, target_id);

        // Send ZCTradeRequest notification to the target player
        let notify = ZCTradeRequest {
            requester_id: requester.account_id,
            requester_name: requester.name.clone(),
        }
        .to_packet();

        // Deliver notification to target via channel bus
        self.channel_bus.send_to_player(&target_id, notify);

        // Return None to the requester (no direct response needed)
        None
    }

    /// Handle trade acknowledge (accept/reject) (0x00E6)
    /// Target player accepts or rejects the trade request.
    /// If accepted, transitions the session to Trading state and notifies the requester.
    pub(super) fn handle_trade_ack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZTradeAck::from_packet(data)?;

        // Find the trade session for this player
        let session_id = self.trade_manager.find_session_for_player(player_id)?;

        if pkt.accept {
            // Start the trade (Requesting -> Trading)
            if !self.trade_manager.start_trade(session_id) {
                return None;
            }
        } else {
            // Reject: cancel and clean up the trade session
            self.trade_manager.cancel_trade(session_id);
            self.trade_manager.end_trade(session_id);
        }

        // Get the partner to send them the ack
        let trade_session = self.trade_manager.get_session(session_id)?;
        let partner_id = trade_session.get_partner_id(player_id)?;

        // Send ZCTradeAck to the requester (partner)
        let response = ZCTradeAck { accept: pkt.accept }.to_packet();
        self.channel_bus.send_to_player(&partner_id, response);

        None
    }

    /// Handle trade add item (0x00B0)
    /// Player adds an item from their inventory to the trade window.
    /// Resolves the item from inventory, adds it to the session, and notifies the partner.
    pub(super) fn handle_trade_add_item(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZTradeAddItem::from_packet(data)?;

        // Find the trade session
        let session_id = self.trade_manager.find_session_for_player(player_id)?;

        // Resolve item from player inventory
        let player = self.map_state.get_player(&player_id)?;
        let inventory = player.inventory.read();
        let inv_index = pkt.inventory_index as usize;
        let inv_item = inventory.get(inv_index)?;

        let trade_item = TradeItem {
            inventory_index: pkt.inventory_index,
            item_id: inv_item.item_id,
            amount: pkt.amount as u16,
        };

        // Add item to the trade session
        if !self
            .trade_manager
            .add_item_to_session(session_id, player_id, trade_item)
        {
            return None;
        }

        // Get partner and notify them about the added item
        let trade_session = self.trade_manager.get_session(session_id)?;
        let partner_id = trade_session.get_partner_id(player_id)?;

        let notify = ZCTradeAddItem {
            amount: pkt.amount,
            item_id: inv_item.item_id,
            identified: true,
            damaged: false,
            refine: 0,
            cards: [0; 4],
        }
        .to_packet();

        self.channel_bus.send_to_player(&partner_id, notify);
        None
    }

    /// Handle trade add zeny (0x00B1)
    /// Player sets the zeny amount for the trade.
    pub(super) fn handle_trade_add_zeny(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZTradeAddZeny::from_packet(data)?;

        // Find the trade session
        let session_id = self.trade_manager.find_session_for_player(player_id)?;

        // Set zeny in the trade session
        if !self
            .trade_manager
            .set_zeny_in_session(session_id, player_id, pkt.amount)
        {
            return None;
        }

        // Get partner and notify them about the added zeny
        let trade_session = self.trade_manager.get_session(session_id)?;
        let partner_id = trade_session.get_partner_id(player_id)?;

        let notify = ZCTradeAddZeny { amount: pkt.amount }.to_packet();
        self.channel_bus.send_to_player(&partner_id, notify);
        None
    }

    /// Handle trade lock (0x00EF)
    /// Player clicks OK to lock their side of the trade.
    /// When both players have locked, the trade is ready to execute.
    pub(super) fn handle_trade_lock(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // Find the trade session
        let session_id = self.trade_manager.find_session_for_player(player_id)?;

        // Lock the trade for this player; returns true if both players are now locked
        let both_locked = self.trade_manager.lock_trade(session_id, player_id);

        // Get partner and notify them about the lock
        let trade_session = self.trade_manager.get_session(session_id)?;
        let partner_id = trade_session.get_partner_id(player_id)?;

        if both_locked {
            // Both sides locked - send commit notification to both players
            let commit_pkt = ZCTradeCommit.to_packet();
            self.channel_bus
                .send_to_player(&partner_id, commit_pkt.clone());
            // Clean up the trade session
            self.trade_manager.end_trade(session_id);
        } else {
            // Only this side locked - notify partner
            let lock_pkt = ZCTradeLock.to_packet();
            self.channel_bus.send_to_player(&partner_id, lock_pkt);
        }

        None
    }
}
