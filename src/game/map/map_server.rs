//! MapServer - 地图服务器核心，处理客户端数据包

use crate::game::guild::GuildManager;
use crate::game::item::{ItemDatabase, ItemEffectDatabase, ItemIntegrationHandler, ItemUseResult};
use crate::game::map::MapState;
use crate::game::map::channel::{ChannelBus, ChatType, GameEvent};
use crate::game::map::drop_item::DropManager;
use crate::game::map::teleport::{TeleportAction, TeleportManager, WarpService};
use crate::game::party::PartyManager;
use crate::game::skill::SkillHandler;
use crate::game::storage::StorageManager;
use crate::game::token::TokenStore;
use crate::game::trade::TradeManager;
use crate::network::packet::id::*;
use crate::network::session::Session;
use crate::protocol::char_packets::{CZRequestMove, CZUseSkill};
use crate::protocol::guild_packets::*;
use crate::protocol::map_packets::{CZContactNpc, CZRequestAction, CZRequestPickupItem, CZUseItem};
use crate::protocol::packet_builder::Packed;
use crate::protocol::party_packets::{
    CZChatMessage, CZMakeParty, CZPartyChat, CZReqPartyInvite, CZReqPartyJoin,
};
use crate::protocol::storage_packets::*;
use crate::protocol::teleport_packets::*;
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

pub struct MapServer {
    pub db: Arc<crate::storage::Database>,
    pub token_store: Arc<TokenStore>,
    pub map_state: Arc<MapState>,
    pub channel_bus: Arc<ChannelBus>,
    pub drop_manager: Arc<DropManager>,
    pub party_manager: Arc<PartyManager>,
    pub guild_manager: Arc<GuildManager>,
    pub storage_manager: Arc<StorageManager>,
    pub trade_manager: Arc<TradeManager>,
    pub teleport_manager: Arc<RwLock<TeleportManager>>,
    pub warp_service: Arc<WarpService>,
    pub death_drop_items: bool,
    pub map_server_id: u32,
    pub skill_handler: Arc<SkillHandler>,
    pub item_integration_handler: Arc<ItemIntegrationHandler>,
}

impl MapServer {
    pub fn new(
        db: Arc<crate::storage::Database>,
        token_store: Arc<TokenStore>,
        map_state: Arc<MapState>,
        channel_bus: Arc<ChannelBus>,
        drop_manager: Arc<DropManager>,
        party_manager: Arc<PartyManager>,
        guild_manager: Arc<GuildManager>,
        storage_manager: Arc<StorageManager>,
        trade_manager: Arc<TradeManager>,
        teleport_manager: Arc<RwLock<TeleportManager>>,
        warp_service: Arc<WarpService>,
        death_drop_items: bool,
    ) -> Self {
        // 初始化物品和技能系统
        let effect_db = Arc::new(ItemEffectDatabase::new());
        let item_db = Arc::new(ItemDatabase::new());
        let skill_handler = Arc::new(SkillHandler::new());
        let item_integration_handler = Arc::new(ItemIntegrationHandler::new(effect_db, item_db));

        Self {
            db,
            token_store,
            map_state,
            channel_bus,
            drop_manager,
            party_manager,
            guild_manager,
            storage_manager,
            trade_manager,
            teleport_manager,
            warp_service,
            death_drop_items,
            map_server_id: 1,
            skill_handler,
            item_integration_handler,
        }
    }

    /// 设置 Map Server ID
    #[allow(dead_code)]
    pub fn with_server_id(mut self, server_id: u32) -> Self {
        self.map_server_id = server_id;
        self
    }

    /// Handle incoming packet
    pub fn handle_packet(
        &self,
        packet_id: u16,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
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
            CZ_REQ_STORAGE_OPEN => self.handle_storage_open(session),
            CZ_REQ_STORAGE_CLOSE => self.handle_storage_close(session),
            CZ_REQ_STORAGE_MOVE_ITEM => self.handle_storage_move_item(data, session),
            CZ_TRADE_REQUEST => self.handle_trade_request(data, session),
            CZ_TRADE_ACK => self.handle_trade_ack(data, session),
            CZ_TRADE_ADD_ITEM => self.handle_trade_add_item(data, session),
            CZ_TRADE_ADD_ZENY => self.handle_trade_add_zeny(data, session),
            CZ_TRADE_LOCK => self.handle_trade_lock(session),
            0x0119 => self.handle_use_return(session),
            0x0138 => self.handle_gm_warp(data, session),
            0x013A => self.handle_gm_goto(data, session),
            0x013B => self.handle_gm_summon(data, session),
            0x013C => self.handle_gm_savepoint(session),
            0x01B8 => self.handle_set_savepoint(session),
            0x0165 => self.handle_guild_create(data, session),
            0x0168 => self.handle_guild_invite(data, session),
            0x0169 => self.handle_guild_join(data, session),
            0x016B => self.handle_guild_leave(session),
            0x016C => self.handle_guild_expel(data, session),
            0x0183 => self.handle_guild_change_notice(data, session),
            0x01B7 => self.handle_guild_request_info(data, session),
            0x01EC => self.handle_guild_chat(data, session),
            0x00B2 => self.handle_restart(session),
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
        let token_len = if data.len() > 8 {
            (data.len() - 8).min(32)
        } else {
            0
        };
        let token = String::from_utf8_lossy(&data[8..8 + token_len]).to_string();

        // Verify token and get the expected map server ID
        // Token verification includes map_server_id check
        if !self
            .token_store
            .verify(&token, account_id, char_id, self.map_server_id)
        {
            tracing::warn!(
                "Token verification failed for account_id={}, char_id={}, map_server_id={}",
                account_id,
                char_id,
                self.map_server_id
            );
            return None;
        }

        // Load character from DB
        let character = self.db.get_character_by_id(char_id).ok()??;

        // Create player
        let mut player = crate::game::map::Player::from_character(character);
        player.account_id = account_id;

        // Load account group_id for permission checks
        if let Ok(Some(account)) = self.db.get_account_by_id(account_id) {
            *player.group_id.write() = account.group_id;
        }

        let player_id = player.id;
        let pos_x = *player.pos_x.read();
        let pos_y = *player.pos_y.read();
        let map_name = player.map_name.clone();

        // Add to map state
        self.map_state.add_player(player);

        // Update session
        session.player_id = Some(player_id);

        // Subscribe to map channel using session's event sender
        if let Some(tx) = &session.map_event_tx {
            let channel_name = format!("map:{}", map_name);
            self.channel_bus
                .subscribe(&channel_name, player_id, tx.clone(), pos_x, pos_y);
        }

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

        // Validate coordinates are within map bounds
        if move_pkt.pos_x >= 4000 || move_pkt.pos_y >= 4000 {
            tracing::warn!(
                player_id = %player_id,
                "Move rejected: out-of-bounds coordinates ({}, {})",
                move_pkt.pos_x, move_pkt.pos_y
            );
            return None;
        }

        // Validate step distance (squared Euclidean distance)
        let dx = move_pkt.pos_x as i32 - from_x as i32;
        let dy = move_pkt.pos_y as i32 - from_y as i32;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq > 225 {
            tracing::warn!(
                player_id = %player_id,
                from_x = from_x,
                from_y = from_y,
                to_x = move_pkt.pos_x,
                to_y = move_pkt.pos_y,
                dist_sq = dist_sq,
                "Move rejected: distance too large (possible speed hack)"
            );
            return None;
        }

        player.move_to(move_pkt.pos_x, move_pkt.pos_y);

        // Update channel position
        let channel_name = format!("map:{}", player.map_name);
        self.channel_bus
            .update_position(&channel_name, &player_id, move_pkt.pos_x, move_pkt.pos_y);

        // Check for warp trigger
        if let Some(warp_action) = self.warp_service.handle_move_with_warp_on_map(
            session,
            &player.map_name,
            move_pkt.pos_x,
            move_pkt.pos_y,
        ) {
            // Execute the warp
            if let Err(e) = self.warp_service.execute_warp(session, warp_action) {
                tracing::error!("Warp execution failed for session={}: {}", session.id, e);
            }
        }

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
        let player_id = session.player_id?;
        let item_pkt = CZUseItem::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        // 创建临时物品栏用于处理物品使用
        let item_db = self.item_integration_handler.item_db();
        let inv_data = player.inventory.read().clone();
        let mut inventory =
            crate::game::item::Inventory::from_character_inventory(&inv_data, item_db);

        // 使用 ItemIntegrationHandler 处理物品使用
        let result = self.item_integration_handler.use_item(
            &player,
            &mut inventory,
            item_pkt.item_id as u16,
            &self.warp_service,
            &self.skill_handler,
            &self.map_state,
        );

        match result {
            ItemUseResult::Success(msg) => {
                tracing::info!(
                    "Player {} used item {}: {}",
                    player.name,
                    item_pkt.item_id,
                    msg
                );
                // 更新玩家物品栏数据
                *player.inventory.write() = inventory.to_character_inventory();
            }
            ItemUseResult::Failure(msg) => {
                tracing::warn!(
                    "Player {} failed to use item {}: {}",
                    player.name,
                    item_pkt.item_id,
                    msg
                );
            }
            ItemUseResult::Teleport { map, x, y } => {
                // 执行传送
                tracing::info!(
                    "Player {} teleporting to {} ({}, {})",
                    player.name,
                    map,
                    x,
                    y
                );
                // 更新玩家物品栏数据
                *player.inventory.write() = inventory.to_character_inventory();
            }
            ItemUseResult::SkillUsed { skill_id } => {
                // 触发技能
                tracing::info!("Player {} used skill {} from item", player.name, skill_id);
                // 更新玩家物品栏数据
                *player.inventory.write() = inventory.to_character_inventory();
            }
            ItemUseResult::CooldownActive { remaining_ms } => {
                tracing::debug!(
                    "Player {} item on cooldown: {}ms remaining",
                    player.name,
                    remaining_ms
                );
            }
            ItemUseResult::RequirementsNotMet(reason) => {
                tracing::debug!(
                    "Player {} item requirements not met: {}",
                    player.name,
                    reason
                );
            }
            _ => {}
        }

        None
    }

    /// Handle pickup item (0x0090)
    fn handle_pickup_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pickup_pkt = CZRequestPickupItem::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        // Find drop at position
        if let Some(drop) =
            self.drop_manager
                .find_at_position(pickup_pkt.x, pickup_pkt.y, &player.map_name)
        {
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

        self.party_manager
            .join_party(&party_id, player_id, player.name.clone())?;

        // Subscribe to party channel using session's event sender
        if let Some(tx) = &session.map_event_tx {
            let channel_name = format!("party:{}", party_id);
            let (x, y) = player.get_position();
            self.channel_bus
                .subscribe(&channel_name, player_id, tx.clone(), x, y);
        }

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

    /// Handle storage open request (0x0213)
    fn handle_storage_open(&self, session: &Session) -> Option<Vec<u8>> {
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
    fn handle_storage_close(&self, session: &Session) -> Option<Vec<u8>> {
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
    fn handle_storage_move_item(&self, data: &[u8], session: &Session) -> Option<Vec<u8>> {
        let req = CZReqStorageMoveItem::from_packet(data)?;
        let char_id = session.char_id?;

        let storage = self.storage_manager.get_or_create(char_id, 100);

        if req.is_to_storage {
            // Simplified: just add to storage
            // In real implementation, this should:
            // 1. Remove from inventory
            // 2. Add to storage
            // For now, just simulate
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
                // In real implementation, add to inventory
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
    fn handle_trade_request(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::CZTradeRequest;
        use crate::protocol::trade_packets::ZCTradeRequest;

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
    fn handle_trade_ack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::{CZTradeAck, ZCTradeAck};

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
    fn handle_trade_add_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::game::trade::TradeItem;
        use crate::protocol::trade_packets::{CZTradeAddItem, ZCTradeAddItem};

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
    fn handle_trade_add_zeny(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::{CZTradeAddZeny, ZCTradeAddZeny};

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
    fn handle_trade_lock(&self, session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::{ZCTradeCommit, ZCTradeLock};

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
            self.channel_bus.send_to_player(&partner_id, commit_pkt.clone());
            // Clean up the trade session
            self.trade_manager.end_trade(session_id);
        } else {
            // Only this side locked - notify partner
            let lock_pkt = ZCTradeLock.to_packet();
            self.channel_bus.send_to_player(&partner_id, lock_pkt);
        }

        None
    }

    /// Handle use return (0x0119)
    /// Player uses butterfly wing / return scroll to go back to save point
    fn handle_use_return(&self, session: &mut Session) -> Option<Vec<u8>> {
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

    /// 检查 GM 权限
    fn check_gm_permission(player: &crate::game::map::Player, min_level: i32) -> bool {
        *player.group_id.read() >= min_level
    }

    /// Handle GM warp command (0x0138)
    /// @warp <map_name> <x> <y>
    fn handle_gm_warp(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_gm_goto(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_gm_summon(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmSummon::from_slice(data)?;
        let player_id = session.player_id?;

        // Check GM permissions
        let player = self.map_state.get_player(&player_id)?;
        if !Self::check_gm_permission(&player, 10) {
            tracing::warn!("Player {} attempted GM summon without permission", player.name);
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
    fn handle_gm_savepoint(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let char_id = session.char_id?;

        // Check GM permissions
        let player = self.map_state.get_player(&player_id)?;
        if !Self::check_gm_permission(&player, 10) {
            tracing::warn!("Player {} attempted GM savepoint without permission", player.name);
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

    /// Handle restart (0x00B2) - 玩家死亡后重生到存储点
    fn handle_restart(&self, session: &mut Session) -> Option<Vec<u8>> {
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
            .unwrap_or_else(|| crate::game::map::teleport::SavePoint::new("new_1-1.gat", 53, 111));

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
        self.channel_bus
            .publish(&new_channel, &revive_event, vec![]);

        // 更新数据库位置（best effort）
        if let Err(e) = self.db.execute_with_params(
            "UPDATE characters SET last_map = ?1, last_x = ?2, last_y = ?3, hp = ?4, sp = ?5 WHERE char_id = ?6",
            rusqlite::params![save_point.map_name, save_point.x as i32, save_point.y as i32, *player.max_hp.read(), *player.max_sp.read(), char_id],
        ) {
            tracing::warn!("Failed to update character position in DB: {}", e);
        }

        None
    }

    /// Handle guild create (0x0165)
    fn handle_guild_create(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_guild_invite(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_guild_join(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildJoin::from_slice(data)?;

        if !pkt.accept {
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;
        let guild_id = uuid::Uuid::from_u128(pkt.guild_id as u128);

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
    fn handle_guild_leave(&self, session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        if self.guild_manager.leave_guild(player_id) {
            Some(ZCGuildLeaveResult { result: 0 }.to_packet())
        } else {
            Some(ZCGuildLeaveResult { result: 1 }.to_packet())
        }
    }

    /// Handle guild expel (0x016C)
    fn handle_guild_expel(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_guild_change_notice(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_guild_request_info(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    fn handle_guild_chat(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildChat::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;
        let guild = self.guild_manager.get_player_guild(&player_id)?;

        let channel_name = format!("guild:{}", guild.id);
        let event = GameEvent::PlayerChat {
            player_id,
            message: pkt.message.clone(),
            chat_type: ChatType::Party, // 复用Party聊天类型
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        Some(
            ZCGuildChat {
                sender_name: player.name.clone(),
                message: pkt.message,
            }
            .to_packet(),
        )
    }

    /// Handle set savepoint (0x01B8)
    /// NPC or item that sets the save point
    fn handle_set_savepoint(&self, session: &mut Session) -> Option<Vec<u8>> {
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

    /// ==================== 玩家数据持久化 ====================
    /// 保存指定玩家到数据库
    pub fn save_player(&self, player_id: &Uuid) -> Result<(), String> {
        let player = match self.map_state.get_player(player_id) {
            Some(p) => p,
            None => return Err(format!("Player {} not found", player_id)),
        };

        player.save_to_db(&self.db).map_err(|e| {
            tracing::error!("Failed to save player {}: {}", player.name, e);
            e.to_string()
        })
    }

    /// 保存所有在线玩家到数据库
    pub fn save_all_players(&self) -> Result<usize, String> {
        let player_ids = self.map_state.get_all_player_ids();
        let mut saved_count = 0;

        for player_id in player_ids {
            match self.save_player(&player_id) {
                Ok(_) => saved_count += 1,
                Err(e) => tracing::warn!("Failed to save player {}: {}", player_id, e),
            }
        }

        tracing::info!("Saved {} players to database", saved_count);
        Ok(saved_count)
    }

    /// 处理玩家断开连接：保存玩家数据并从地图移除
    pub fn handle_player_disconnect(&self, player_id: &Uuid) {
        // 保存玩家数据
        if let Err(e) = self.save_player(player_id) {
            tracing::error!("Failed to save player on disconnect: {}", e);
        }

        // 从地图移除玩家
        self.map_state.remove_player(player_id);

        tracing::info!("Player {} disconnected and saved", player_id);
    }

    /// 定期保存定时任务调用（每5分钟调用一次）
    /// 返回保存的玩家数量
    pub fn periodic_save(&self) -> usize {
        match self.save_all_players() {
            Ok(count) => count,
            Err(e) => {
                tracing::error!("Periodic save failed: {}", e);
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_server_handles_unknown_packet() {
        use crate::game::guild::GuildManager;
        use crate::game::map::teleport::{SavePointManager, TeleportManager, WarpService};
        use crate::game::trade::TradeManager;

        let db = Arc::new(crate::storage::Database::open_memory().unwrap());
        let teleport_manager = Arc::new(RwLock::new(TeleportManager::new()));
        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let warp_service = Arc::new(WarpService::new(
            teleport_manager.clone(),
            save_point_manager.clone(),
            db.clone(),
        ));

        let server = MapServer::new(
            db,
            Arc::new(TokenStore::new()),
            Arc::new(MapState::new()),
            Arc::new(ChannelBus::new()),
            Arc::new(DropManager::new()),
            Arc::new(PartyManager::new()),
            Arc::new(GuildManager::new()),
            Arc::new(StorageManager::new()),
            Arc::new(TradeManager::new()),
            teleport_manager,
            warp_service,
            false,
        );
        let mut session = Session::new();
        let result = server.handle_packet(0xFFFF, &[], &mut session);
        assert!(result.is_none());
    }
}
