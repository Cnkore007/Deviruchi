//! 社交与经济 handler：队伍、聊天、仓库、交易

use super::MapServer;
use crate::game::item::ItemDatabase;
use crate::game::map::channel::{ChatType, GameEvent};
use crate::game::map::Player;
use crate::game::trade::TradeItem;
use crate::game::zeny::ZenyManager;
use crate::network::session::Session;
use crate::protocol::packet_builder::Packed;
use crate::protocol::party_packets::{
    CZChatMessage, CZMakeParty, CZPartyChat, CZReqPartyInvite, CZReqPartyJoin,
};
use crate::protocol::storage_packets::*;
use crate::protocol::trade_packets::*;
use crate::storage::character::CharacterInventoryData;
use uuid::Uuid;

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
            let packet = event.to_packet_bytes();
            self.channel_bus.publish(&channel_name, &event, packet);
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
        let packet = event.to_packet_bytes();
        self.channel_bus.publish(&channel_name, &event, packet);

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

        // 校验：请求的数量必须 > 0 且不超过实际持有量
        let requested = pkt.amount as u16;
        if requested == 0 || requested > inv_item.amount {
            tracing::warn!(
                player_id = %player_id,
                "Trade add item rejected: requested {} but have {}",
                requested, inv_item.amount
            );
            return None;
        }

        let trade_item = TradeItem {
            inventory_index: pkt.inventory_index,
            item_id: inv_item.item_id,
            amount: requested,
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

    /// 取消交易并向双方发送取消通知
    fn cancel_trade_session(&self, session_id: Uuid, player1_id: Uuid, player2_id: Uuid) {
        let cancel_pkt = ZCTradeCancel { reason: 0 }.to_packet();
        self.channel_bus
            .send_to_player(&player1_id, cancel_pkt.clone());
        self.channel_bus
            .send_to_player(&player2_id, cancel_pkt);
        self.trade_manager.end_trade(session_id);
    }

    /// Handle trade lock (0x00EF)
    /// Player clicks OK to lock their side of the trade.
    /// When both players have locked, validates and executes the trade,
    /// transferring items and zeny between players.
    pub(super) fn handle_trade_lock(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // Find the trade session
        let session_id = self.trade_manager.find_session_for_player(player_id)?;

        // Lock the trade for this player; returns true if both players are now locked
        let both_locked = self.trade_manager.lock_trade(session_id, player_id);

        // Get the cloned session to read trade state
        let trade_session = self.trade_manager.get_session(session_id)?;
        let partner_id = trade_session.get_partner_id(player_id)?;

        if both_locked {
            // Both sides locked - validate and execute the trade
            let player1 = self.map_state.get_player(&trade_session.player1_id)?;
            let player2 = self.map_state.get_player(&trade_session.player2_id)?;
            let item_db = self.item_integration_handler.item_db();

            // Validate: check weight and zeny constraints
            if let Err(e) = trade_session.validate(
                &player1,
                &crate::game::item::Inventory::from_character_inventory(
                    &player1.inventory.read(),
                    item_db.clone(),
                ),
                &player2,
                &crate::game::item::Inventory::from_character_inventory(
                    &player2.inventory.read(),
                    item_db.clone(),
                ),
                &*item_db,
            ) {
                tracing::warn!(
                    "Trade validation failed between {} and {}: {:?}",
                    player1.name,
                    player2.name,
                    e
                );
                self.cancel_trade_session(
                    session_id,
                    trade_session.player1_id,
                    trade_session.player2_id,
                );
                return None;
            }

            // Execute: mark trade as completed and get transfer data
            let execution = match trade_session.execute() {
                Ok(exec) => exec,
                Err(e) => {
                    tracing::warn!("Trade execution failed: {:?}", e);
                    self.cancel_trade_session(
                        session_id,
                        trade_session.player1_id,
                        trade_session.player2_id,
                    );
                    return None;
                }
            };

            // Transfer zeny first (validate before moving items to avoid partial-failure corruption)
            // Transfer zeny: player1 -> player2
            if execution.zeny_from_player1 > 0 {
                if !ZenyManager::sub(&player1, execution.zeny_from_player1) {
                    tracing::warn!(
                        "Failed to subtract {} zeny from {}",
                        execution.zeny_from_player1,
                        player1.name
                    );
                    self.cancel_trade_session(
                        session_id,
                        trade_session.player1_id,
                        trade_session.player2_id,
                    );
                    return None;
                }
                ZenyManager::add(&player2, execution.zeny_from_player1);
            }

            // Transfer zeny: player2 -> player1
            if execution.zeny_from_player2 > 0 {
                if !ZenyManager::sub(&player2, execution.zeny_from_player2) {
                    tracing::warn!(
                        "Failed to subtract {} zeny from {}",
                        execution.zeny_from_player2,
                        player2.name
                    );
                    // Rollback: restore player1's zeny
                    if execution.zeny_from_player1 > 0 {
                        ZenyManager::sub(&player2, execution.zeny_from_player1);
                        ZenyManager::add(&player1, execution.zeny_from_player1);
                    }
                    self.cancel_trade_session(
                        session_id,
                        trade_session.player1_id,
                        trade_session.player2_id,
                    );
                    return None;
                }
                ZenyManager::add(&player1, execution.zeny_from_player2);
            }

            // Transfer items (zeny already validated, so this cannot leave partial state)
            Self::transfer_items(
                &player1,
                &player2,
                &execution.items_for_player2,
            );

            Self::transfer_items(
                &player2,
                &player1,
                &execution.items_for_player1,
            );

            // 物品转移后重新计算双方负重
            Self::recalc_inventory_weight(&player1, &item_db);
            Self::recalc_inventory_weight(&player2, &item_db);

            // Send commit notification to both players
            let commit_pkt = ZCTradeCommit.to_packet();
            self.channel_bus
                .send_to_player(&trade_session.player1_id, commit_pkt.clone());
            self.channel_bus
                .send_to_player(&trade_session.player2_id, commit_pkt);

            // Clean up the trade session
            self.trade_manager.end_trade(session_id);
        } else {
            // Only this side locked - notify partner
            let lock_pkt = ZCTradeLock.to_packet();
            self.channel_bus.send_to_player(&partner_id, lock_pkt);
        }

        None
    }

    /// 重新计算并更新玩家的背包负重
    fn recalc_inventory_weight(player: &Player, item_db: &ItemDatabase) {
        let inv = player.inventory.read();
        let weight: u32 = inv
            .iter()
            .map(|slot| {
                let amount = slot.amount as u32;
                item_db
                    .get(slot.item_id)
                    .map(|i| (i.weight as u32) * amount)
                    .unwrap_or(0)
            })
            .sum();
        drop(inv);
        player.economy.write().current_weight = weight;
    }

    /// 将物品从发送者转移到接收者的背包
    fn transfer_items(
        sender: &Player,
        receiver: &Player,
        items: &[TradeItem],
    ) {
        for item in items {
            // 从发送者背包中移除物品
            {
                let mut sender_inv = sender.inventory.write();
                if let Some(pos) = sender_inv
                    .iter()
                    .position(|i| i.index == item.inventory_index as u8)
                {
                    if sender_inv[pos].amount <= item.amount {
                        sender_inv.remove(pos);
                    } else {
                        sender_inv[pos].amount -= item.amount;
                    }
                }
            }

            // 将物品添加到接收者背包
            {
                let mut receiver_inv = receiver.inventory.write();
                // 尝试与已有同类物品堆叠
                let mut stacked = false;
                for slot in receiver_inv.iter_mut() {
                    if slot.item_id == item.item_id {
                        slot.amount += item.amount;
                        stacked = true;
                        break;
                    }
                }
                if !stacked {
                    // 寻找第一个空闲索引
                    let mut used: Vec<u8> = receiver_inv.iter().map(|i| i.index).collect();
                    used.sort_unstable();
                    let next_index = used
                        .iter()
                        .enumerate()
                        .find(|(i, idx)| **idx != *i as u8)
                        .map(|(i, _)| i as u8)
                        .unwrap_or(receiver_inv.len() as u8);

                    receiver_inv.push(CharacterInventoryData {
                        index: next_index,
                        item_id: item.item_id,
                        amount: item.amount,
                        identified: true,
                        refine: 0,
                        cards: [0; 4],
                    });
                }
            }
        }
    }
}
