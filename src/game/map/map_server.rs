//! MapServer - 地图服务器核心，处理客户端数据包

use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;
use crate::network::session::Session;
use crate::game::token::TokenStore;
use crate::game::map::MapState;
use crate::game::map::channel::{ChannelBus, GameEvent, ChatType};
use crate::game::map::drop_item::DropManager;
use crate::game::party::PartyManager;
use crate::game::trade::TradeManager;
use crate::game::guild::GuildManager;
use crate::game::map::teleport::{TeleportManager, WarpService, TeleportAction};
use crate::protocol::char_packets::{CZRequestMove, CZUseSkill};
use crate::protocol::map_packets::{CZRequestAction, CZUseItem, CZRequestPickupItem, CZContactNpc};
use crate::protocol::party_packets::{CZMakeParty, CZReqPartyInvite, CZReqPartyJoin, CZPartyChat, CZChatMessage};
use crate::protocol::storage_packets::*;
use crate::protocol::teleport_packets::*;
use crate::protocol::guild_packets::*;
use crate::protocol::packet_builder::Packed;
use crate::network::packet::id::*;
use crate::game::storage::{StorageManager, Storage};
use crate::game::battle::BattleHandler;
use crate::game::mob::MobSpawnManager;

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
    pub spawn_manager: Arc<MobSpawnManager>,
    pub death_drop_items: bool,
    battle_handler: BattleHandler,
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
        spawn_manager: Arc<MobSpawnManager>,
        death_drop_items: bool,
    ) -> Self {
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
            spawn_manager,
            death_drop_items,
            battle_handler: BattleHandler::new(),
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

        // Check for warp trigger
        if let Some(warp_action) = self.warp_service.handle_move_with_warp_on_map(
            session,
            &player.map_name,
            move_pkt.pos_x,
            move_pkt.pos_y,
        ) {
            // Execute the warp
            let _ = self.warp_service.execute_warp(session, warp_action);
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
    /// 参考 rAthena: CZ_REQUEST_ACT → unit_attack → battle_attack
    fn handle_attack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let action_pkt = CZRequestAction::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;
        let target_mob_id = Uuid::from_u128(action_pkt.target_id as u128);

        // 从 MobSpawnManager 查找目标怪物
        let mob = self.spawn_manager.get_mob(&target_mob_id)?;

        // 必须是同地图
        if mob.map_name != player.map_name {
            return None;
        }

        // 调用 BattleHandler 处理伤害
        let result = self.battle_handler.normal_attack(&player, &mob);

        match result {
            crate::game::battle::AttackResult::Miss => None,
            crate::game::battle::AttackResult::Hit { damage, is_crit, killed } => {
                // 记录伤害（用于血条同步）
                mob.add_damage(player_id, damage as u32);

                // 广播 0x8d (ZC_NOTIFY_ACT) 给周围玩家
                let channel_name = format!("map:{}", player.map_name);
                let src_gid = player_id.as_u128() as u32;
                let dst_gid = target_mob_id.as_u128() as u32;
                let action_type = if is_crit { 5 } else { 0 };

                use crate::protocol::map_packets::ZCNotifyAct;
                let damage_packet = ZCNotifyAct {
                    src_id: src_gid,
                    dst_id: dst_gid,
                    damage: damage as u32,
                    action: action_type,
                    left_damage: 0,
                }.to_packet();

                let event = GameEvent::MobDamage {
                    mob_id: target_mob_id,
                    attacker_id: player_id,
                    damage: damage as u32,
                    is_crit,
                };
                self.channel_bus.publish(&channel_name, &event, damage_packet);

                // 如果击杀，发布 MobDeath 事件
                if killed {
                    let killer_id = player_id;
                    let event = GameEvent::MobDeath {
                        mob_id: target_mob_id,
                        killer_id,
                    };
                    self.channel_bus.publish(&channel_name, &event, vec![]);
                } else {
                    // 广播 0x977 给 dmglog 中的玩家
                    self.broadcast_mob_hp_bar(&mob, &channel_name);
                }

                None
            }
            crate::game::battle::AttackResult::Blocked | crate::game::battle::AttackResult::Immune => None,
        }
    }

    /// 广播怪物血条给 dmglog 中的玩家（参考 rAthena mob_damage）
    fn broadcast_mob_hp_bar(&self, mob: &Arc<crate::game::mob::Mob>, channel_name: &str) {
        let dmglog = mob.dmglog.read();
        if dmglog.is_empty() {
            return;
        }

        let hp = *mob.hp.read();
        let max_hp = mob.max_hp;
        let mob_gid = mob.id.as_u128() as u32;

        use crate::protocol::map_packets::ZCMonsterHpBar;
        let hp_packet = ZCMonsterHpBar {
            mob_id: mob_gid,
            hp,
            max_hp,
        }.to_packet();

        let event = GameEvent::MobHpUpdate {
            mob_id: mob.id,
            hp,
            max_hp,
        };
        self.channel_bus.publish(channel_name, &event, hp_packet);
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

    /// Handle storage open request (0x0213)
    fn handle_storage_open(&self, session: &Session) -> Option<Vec<u8>> {
        let char_id = session.char_id?;

        // Get or create storage
        let storage = self.storage_manager.get_or_create(char_id, 100);
        let storage = storage.read();

        // Build item list
        let items: Vec<_> = storage.slots()
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
        }.to_packet();

        Some(items_packet)
    }

    /// Handle storage close request (0x0214)
    fn handle_storage_close(&self, session: &Session) -> Option<Vec<u8>> {
        // Save storage to database
        if let Some(char_id) = session.char_id {
            if let Some(storage) = self.storage_manager.get(char_id) {
                let storage = storage.read();
                if let Err(e) = self.db.save_storage(&storage) {
                    tracing::error!("Failed to save storage for char {}: {}", char_id, e);
                }
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
            if s.add_item(req.from_index as u16, req.amount) {
                // Find the slot
                for slot in s.slots() {
                    if slot.item_id == req.from_index as u16 {
                        return Some(ZCStorageItemAdd {
                            index: slot.index,
                            item_id: slot.item_id,
                            amount: slot.amount,
                            identified: slot.identified,
                        }.to_packet());
                    }
                }
            }
        } else {
            // From storage
            let mut s = storage.write();
            if s.remove_item(req.from_index, req.amount) {
                // Simplified: just acknowledge
                // In real implementation, add to inventory
                return Some(ZCStorageItemRemove {
                    index: req.from_index,
                    amount: req.amount,
                }.to_packet());
            }
        }

        None
    }

    /// Handle trade request (0x00E4)
    fn handle_trade_request(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::CZTradeRequest;
        use crate::protocol::trade_packets::ZCTradeRequest;

        let player_id = session.player_id?;
        let pkt = CZTradeRequest::from_packet(data)?;

        // Get requester info
        let player = self.map_state.get_player(&player_id)?;
        let requester_name = player.name.clone();

        // Create trade request notification for target
        let notify = ZCTradeRequest {
            requester_id: pkt.target_account_id,
            requester_name,
        }.to_packet();

        Some(notify)
    }

    /// Handle trade acknowledge (accept/reject) (0x00E6)
    fn handle_trade_ack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::{CZTradeAck, ZCTradeAck};

        let _player_id = session.player_id?;
        let pkt = CZTradeAck::from_packet(data)?;

        // Send acknowledgement response
        let response = ZCTradeAck {
            accept: pkt.accept,
        }.to_packet();

        Some(response)
    }

    /// Handle trade add item (0x00B0)
    fn handle_trade_add_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::{CZTradeAddItem, ZCTradeAddItem};

        let _player_id = session.player_id?;
        let pkt = CZTradeAddItem::from_packet(data)?;

        // Notify partner about added item
        let notify = ZCTradeAddItem {
            amount: pkt.amount,
            item_id: 0, // Would be resolved from inventory
            identified: true,
            damaged: false,
            refine: 0,
            cards: [0; 4],
        }.to_packet();

        Some(notify)
    }

    /// Handle trade add zeny (0x00B1)
    fn handle_trade_add_zeny(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::{CZTradeAddZeny, ZCTradeAddZeny};

        let _player_id = session.player_id?;
        let pkt = CZTradeAddZeny::from_packet(data)?;

        // Notify partner about added zeny
        let notify = ZCTradeAddZeny {
            amount: pkt.amount,
        }.to_packet();

        Some(notify)
    }

    /// Handle trade lock (0x00EF)
    fn handle_trade_lock(&self, _session: &mut Session) -> Option<Vec<u8>> {
        use crate::protocol::trade_packets::ZCTradeLock;

        // Notify partner that player has locked
        let notify = ZCTradeLock.to_packet();

        Some(notify)
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
                return Some(ZCWarpError { error_code: ZCWarpError::COOLDOWN }.to_packet());
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
            Err(_) => {
                Some(ZCWarpError { error_code: ZCWarpError::INVALID_MAP }.to_packet())
            }
        }
    }

    /// Handle GM warp command (0x0138)
    /// @warp <map_name> <x> <y>
    fn handle_gm_warp(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmWarp::from_slice(data)?;
        let player_id = session.player_id?;

        // Check GM permissions (simplified - in real implementation check account level)
        // For now, allow all for testing

        // Check warp cooldown
        {
            let tm = self.teleport_manager.read();
            if !tm.can_warp(player_id) {
                return Some(ZCWarpError { error_code: ZCWarpError::COOLDOWN }.to_packet());
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
            Ok(_) => {
                Some(ZCWarpAck { warp_type: 3 }.to_packet())
            }
            Err(_) => {
                Some(ZCWarpError { error_code: ZCWarpError::INVALID_MAP }.to_packet())
            }
        }
    }

    /// Handle GM goto command (0x013A)
    /// @goto <player_name> - teleport to another player
    fn handle_gm_goto(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmGoto::from_slice(data)?;
        let player_id = session.player_id?;

        // Check warp cooldown
        {
            let tm = self.teleport_manager.read();
            if !tm.can_warp(player_id) {
                return Some(ZCWarpError { error_code: ZCWarpError::COOLDOWN }.to_packet());
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
                Ok(_) => {
                    Some(ZCWarpAck { warp_type: 3 }.to_packet())
                }
                Err(_) => {
                    Some(ZCWarpError { error_code: ZCWarpError::INVALID_MAP }.to_packet())
                }
            }
        } else {
            Some(ZCWarpError { error_code: ZCWarpError::TARGET_NOT_FOUND }.to_packet())
        }
    }

    /// Handle GM summon command (0x013B)
    /// @summon <player_name> - bring another player to current location
    fn handle_gm_summon(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let pkt = CZGmSummon::from_slice(data)?;
        let player_id = session.player_id?;

        let player = self.map_state.get_player(&player_id)?;
        let player_pos = player.get_position();
        let player_map = player.map_name.clone();

        // Find target player by name
        let target = self.map_state.find_player_by_name(&pkt.target_name);

        if let Some(target) = target {
            let target_id = target.id;

            // Check if target can warp
            {
                let tm = self.teleport_manager.read();
                if !tm.can_warp(target_id) {
                    return Some(ZCWarpError { error_code: ZCWarpError::COOLDOWN }.to_packet());
                }
            }

            // Note: In a full implementation, this would need to communicate
            // with the target player's session to warp them
            // For now, return success acknowledgment
            Some(ZCWarpAck { warp_type: 3 }.to_packet())
        } else {
            Some(ZCWarpError { error_code: ZCWarpError::TARGET_NOT_FOUND }.to_packet())
        }
    }

    /// Handle GM savepoint command (0x013C)
    /// @savepoint - set current location as save point
    fn handle_gm_savepoint(&self, session: &mut Session) -> Option<Vec<u8>> {
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
        Some(ZCSavePointSet {
            map_name: player.map_name.clone(),
            x,
            y,
        }.to_packet())
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
        let save_point = self.warp_service.get_save_point(char_id)
            .unwrap_or_else(|| {
                crate::game::map::teleport::SavePoint::new("new_1-1.gat", 53, 111)
            });

        // 更新玩家运行时状态（通过 MapState 原子更新）
        self.map_state.respawn_player(&player_id, save_point.x, save_point.y, &save_point.map_name);

        // 更新 ChannelBus 中的位置
        let new_channel = format!("map:{}", save_point.map_name);
        self.channel_bus.update_position(&new_channel, &player_id, save_point.x, save_point.y);

        // 发布重生事件
        let revive_event = GameEvent::PlayerRevive {
            player_id,
            x: save_point.x,
            y: save_point.y,
        };
        self.channel_bus.publish(&new_channel, &revive_event, vec![]);

        // 更新数据库位置（best effort）
        let _ = self.db.execute_with_params(
            "UPDATE characters SET last_map = ?1, last_x = ?2, last_y = ?3, hp = ?4, sp = ?5 WHERE char_id = ?6",
            rusqlite::params![save_point.map_name, save_point.x as i32, save_point.y as i32, *player.max_hp.read(), *player.max_sp.read(), char_id],
        );

        None
    }

    /// Handle guild create (0x0165)
    fn handle_guild_create(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildCreate::from_slice(data)?;

        if self.guild_manager.get_player_guild(&player_id).is_some() {
            return Some(ZCGuildCreated { result: 2, guild_id: 0 }.to_packet());
        }

        let player = self.map_state.get_player(&player_id)?;
        match self.guild_manager.create_guild(pkt.name.clone(), player.name.clone()) {
            Some(guild_id) => {
                self.guild_manager.join_guild(guild_id, player_id, player.name.clone());
                self.guild_manager.set_member_position_direct(&guild_id, &player_id, 0);
                Some(ZCGuildCreated { result: 0, guild_id: 0 }.to_packet())
            }
            None => Some(ZCGuildCreated { result: 1, guild_id: 0 }.to_packet()),
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
        Some(ZCGuildInvite {
            guild_id: 0,
            guild_name: guild.name.clone(),
            inviter_name: player.name.clone(),
        }.to_packet())
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

        if self.guild_manager.join_guild(guild_id, player_id, player.name.clone()) {
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

        if self.guild_manager.expel_member(guild_id, &player_id, &target_id) {
            Some(ZCGuildExpelResult {
                result: 0,
                target_name: pkt.target_name,
                reason: pkt.reason,
            }.to_packet())
        } else {
            Some(ZCGuildExpelResult {
                result: 1,
                target_name: pkt.target_name,
                reason: String::new(),
            }.to_packet())
        }
    }

    /// Handle guild change notice (0x0183)
    fn handle_guild_change_notice(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CZGuildChangeNotice::from_slice(data)?;

        let guild_id = self.guild_manager.get_player_guild_id(&player_id)?;
        let guild = self.guild_manager.get_guild(&guild_id)?;

        if guild.has_permission(&player_id, crate::game::guild::GuildPermission::Expel) {
            self.guild_manager.update_notice(&guild_id, pkt.notice.clone());
            return Some(ZCGuildNotice { notice: pkt.notice }.to_packet());
        }

        None
    }

    /// Handle guild request info (0x01B7)
    fn handle_guild_request_info(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let guild = self.guild_manager.get_player_guild(&player_id)?;

        Some(ZCGuildInfo {
            guild_id: 0,
            level: guild.level,
            member_count: guild.member_count,
            max_members: guild.max_members,
            average_level: guild.average_level,
            exp: guild.exp,
            max_exp: guild.max_exp,
            notice: guild.notice.clone(),
        }.to_packet())
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

        Some(ZCGuildChat {
            sender_name: player.name.clone(),
            message: pkt.message,
        }.to_packet())
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
        Some(ZCSavePointSet {
            map_name: player.map_name.clone(),
            x,
            y,
        }.to_packet())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_server_handles_unknown_packet() {
        use crate::game::trade::TradeManager;
        use crate::game::guild::GuildManager;
        use crate::game::map::teleport::{TeleportManager, WarpService, SavePointManager};

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
            Arc::new(MobSpawnManager::new()),
            false,
        );
        let mut session = Session::new();
        let result = server.handle_packet(0xFFFF, &[], &mut session);
        assert!(result.is_none());
    }
}
