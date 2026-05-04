//! 玩家基础操作 handler：进入地图、移动、攻击、技能、使用物品、拾取

use super::MapServer;
use crate::game::item::ItemUseResult;
use crate::game::map::channel::GameEvent;
use crate::network::session::Session;
use crate::protocol::char_packets::{CZRequestMove, CZUseSkill};
use crate::protocol::map_packets::{CZRequestAction, CZRequestPickupItem, CZUseItem};
use crate::protocol::packet_builder::Packed;
use std::sync::Arc;
use uuid::Uuid;

impl MapServer {
    /// Handle player enter map (0x007C)
    /// Simplified: expects data to contain account_id and char_id and token
    pub(super) fn handle_enter(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
            session.authenticated = false;
            return None;
        }

        // Load character from DB
        let character = self.db.get_character_by_id(char_id).ok()??;

        // Create player
        let mut player = crate::game::map::Player::from_character(character);
        player.account_id = account_id;

        // Load account group_id for permission checks
        if let Ok(Some(account)) = self.db.get_account_by_id(account_id) {
            player.economy_mut().group_id = account.group_id;
        }

        let player_id = player.id;
        let pos_x = player.pos_x();
        let pos_y = player.pos_y();
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
    pub(super) fn handle_move(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let player = self.map_state.get_player(&player_id)?;

        let move_pkt = CZRequestMove::from_slice(data)?;

        let from_x = player.pos_x();
        let from_y = player.pos_y();

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
    pub(super) fn handle_use_skill(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let skill_pkt = CZUseSkill::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        // 调用 SkillHandler 执行技能逻辑
        let result = self.skill_handler.use_skill(
            Arc::new(player.clone()),
            skill_pkt.skill_id as u16,
            1, // 默认技能等级 1
            skill_pkt.target_id,
            &self.map_state,
        );

        match result {
            Ok(skill_result) => {
                tracing::info!(
                    "Player {} used skill {} successfully: {:?}",
                    player.name,
                    skill_pkt.skill_id,
                    skill_result
                );
            }
            Err(err) => {
                tracing::warn!(
                    "Player {} failed to use skill {}: {:?}",
                    player.name,
                    skill_pkt.skill_id,
                    err
                );
                return None;
            }
        }

        // 发布技能使用事件（仅在成功后）
        let channel_name = format!("map:{}", player.map_name);
        let event = GameEvent::PlayerUseSkill {
            caster_id: player_id,
            skill_id: skill_pkt.skill_id as u32,
            target_id: if skill_pkt.target_id != 0 {
                Some(Uuid::from_u128(skill_pkt.target_id as u128))
            } else {
                None
            },
            x: skill_pkt.target_x,
            y: skill_pkt.target_y,
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        None
    }

    /// Handle attack (0x0089)
    pub(super) fn handle_attack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let action_pkt = CZRequestAction::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;
        let target_id = Uuid::from_u128(action_pkt.target_id as u128);

        // 从 spawn_manager 查找目标怪物
        let mob = self
            .spawn_manager
            .find_mob_by_id(&player.map_name, &target_id)?;

        // 调用 BattleHandler 计算真实伤害
        let result = self.battle_handler.normal_attack(&player, &mob);

        let (damage, is_crit, killed) = match result {
            crate::game::battle::AttackResult::Hit {
                damage,
                is_crit,
                killed,
            } => (damage.max(0) as u32, is_crit, killed),
            crate::game::battle::AttackResult::Miss => (0, false, false),
            crate::game::battle::AttackResult::Blocked
            | crate::game::battle::AttackResult::Immune => (0, false, false),
        };

        let channel_name = format!("map:{}", player.map_name);
        let event = GameEvent::PlayerAttack {
            attacker_id: player_id,
            target_id,
            damage,
            is_crit,
            killed,
        };
        self.channel_bus.publish(&channel_name, &event, vec![]);

        None
    }

    /// Handle use item (0x009B)
    pub(super) fn handle_use_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
    pub(super) fn handle_pickup_item(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
}
