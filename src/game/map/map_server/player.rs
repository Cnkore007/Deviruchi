//! 玩家基础操作 handler：进入地图、移动、攻击、技能、使用物品、拾取

use super::MapServer;
use crate::game::item::ItemUseResult;
use crate::game::map::channel::GameEvent;
use crate::network::packet::id::*;
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

        // 发送视野内实体快照给新进入的玩家
        // 遍历同地图视野范围内的其他玩家和怪物，逐个构建 0x0078 包并发送
        {
            const VISION_RADIUS: u16 = 14;

            // 视野内的其他玩家（排除自己）
            let nearby_players = self.map_state.get_players_near(&map_name, pos_x, pos_y, VISION_RADIUS);
            for other in &nearby_players {
                if other.id == player_id {
                    continue;
                }
                let other_x = other.pos_x();
                let other_y = other.pos_y();
                let other_dir = other.direction() as u8;
                let other_eid = crate::game::map::channel::uuid_to_entity_id(&other.id);
                let appear_pkt = crate::game::map::channel::build_entity_appear_packet(
                    other_eid, 0, other_x, other_y, 0, other_dir,
                );
                self.channel_bus.send_to_player(&player_id, appear_pkt);
            }

            // 视野内的怪物
            let nearby_mobs = self.spawn_manager.get_active_mobs(&map_name);
            for mob in &nearby_mobs {
                if mob.is_dead() {
                    continue;
                }
                let mob_pos = mob.pos.read();
                let dx = (mob_pos.x as i32 - pos_x as i32).unsigned_abs() as u16;
                let dy = (mob_pos.y as i32 - pos_y as i32).unsigned_abs() as u16;
                if dx > VISION_RADIUS || dy > VISION_RADIUS {
                    continue;
                }
                let mob_eid = mob.get_entity_id();
                let appear_pkt = crate::game::map::channel::build_entity_appear_packet(
                    mob_eid, 6, mob_pos.x, mob_pos.y, mob.mob_id, 0,
                );
                self.channel_bus.send_to_player(&player_id, appear_pkt);
            }
        }

        // 构建 ZC_ACCEPT_ENTER (0x0073) 包
        // 格式: start_time(u32) + pos_x(u16) + pos_y(u16) + dir(u16) + font(u16)
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        let mut response = Vec::with_capacity(16);
        response.extend_from_slice(&16u16.to_le_bytes()); // length
        response.extend_from_slice(&0x0073u16.to_le_bytes()); // packet_id
        response.extend_from_slice(&start_time.to_le_bytes());
        response.extend_from_slice(&pos_x.to_le_bytes());
        response.extend_from_slice(&pos_y.to_le_bytes());
        response.extend_from_slice(&0u16.to_le_bytes()); // direction
        response.extend_from_slice(&0u16.to_le_bytes()); // font
        Some(response)
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

        // Broadcast movement to other players on the same map
        let event = GameEvent::PlayerMove {
            player_id,
            from_x,
            from_y,
            to_x: move_pkt.pos_x,
            to_y: move_pkt.pos_y,
        };
        let packet = event.to_packet_bytes();
        self.channel_bus.publish(&channel_name, &event, packet);

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
        // target_id 来自客户端是 account_id，需查找实际 player UUID
        let channel_name = format!("map:{}", player.map_name);
        let target_uuid = if skill_pkt.target_id != 0 {
            self.map_state
                .find_player_by_account_id(skill_pkt.target_id)
                .map(|p| p.id)
        } else {
            None
        };
        let event = GameEvent::PlayerUseSkill {
            caster_id: player_id,
            skill_id: skill_pkt.skill_id as u32,
            target_id: target_uuid,
            x: skill_pkt.target_x,
            y: skill_pkt.target_y,
        };
        let packet = event.to_packet_bytes();
        self.channel_bus.publish(&channel_name, &event, packet);

        None
    }

    /// Handle attack (0x0089)
    pub(super) fn handle_attack(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let action_pkt = CZRequestAction::from_slice(data)?;

        let player = self.map_state.get_player(&player_id)?;

        // 从 spawn_manager 通过客户端实体 ID 查找目标怪物
        let mob = self
            .spawn_manager
            .find_mob_by_entity_id(&player.map_name, action_pkt.target_id)?;
        let target_id = mob.id;

        // 已死亡的怪物不能重复攻击
        if mob.is_dead() {
            return None;
        }

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
        let packet = event.to_packet_bytes();
        self.channel_bus.publish(&channel_name, &event, packet);

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
            let packet = event.to_packet_bytes();
            self.channel_bus.publish(&channel_name, &event, packet);
        }

        None
    }

    /// 处理状态点分配请求 (CZ_STATUS_CHANGE, 0x014D)
    ///
    /// 客户端发送格式：packet_id(2) + status_id(2) + amount(1)
    /// rAthena status_id 映射：
    /// - 13 = STR, 14 = AGI, 15 = VIT, 16 = INT, 17 = DEX, 18 = LUK
    ///
    /// 处理逻辑：
    /// 1. 解析 status_id 和 amount
    /// 2. 验证 amount > 0 且 status_id 合法
    /// 3. 检查玩家是否有足够的状态点（status_point）
    /// 4. 增加对应属性
    /// 5. 消耗状态点
    /// 6. 返回 ZC_STATUS_CHANGE_ACK (0x00BC)
    pub(super) fn handle_status_change(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // 包体格式：status_id(2) + amount(1) = 3 字节
        if data.len() < 3 {
            return None;
        }

        let status_id = u16::from_le_bytes([data[0], data[1]]);
        let amount = data[2];

        // 验证 amount > 0
        if amount == 0 {
            return None;
        }

        // 验证 status_id 范围
        if !(13..=18).contains(&status_id) {
            if let Some(player) = self.map_state.get_player(&player_id) {
                tracing::warn!(
                    player = %player.name,
                    status_id = status_id,
                    "无效的 status_id"
                );
            }
            return None;
        }

        let player = self.map_state.get_player(&player_id)?;

        // 检查是否有足够的状态点
        let available_points = player.status_point();
        if available_points < amount as u16 {
            tracing::warn!(
                player = %player.name,
                requested = amount,
                available = available_points,
                "状态点不足，分配请求被拒绝"
            );
            return Some(build_status_change_ack(status_id, &player));
        }

        // 通过 MapState 直接修改存储的玩家属性（内部可变性）
        if !self.map_state.allocate_player_stat(&player_id, status_id, amount as u16) {
            return Some(build_status_change_ack(status_id, &player));
        }

        // 重新获取修改后的玩家数据用于构建 ACK
        let updated_player = self.map_state.get_player(&player_id)?;

        tracing::info!(
            player = %updated_player.name,
            status_id = status_id,
            amount = amount,
            "状态点分配成功"
        );

        // 返回 ZC_STATUS_CHANGE_ACK
        Some(build_status_change_ack(status_id, &updated_player))
    }

    /// 处理技能点分配请求 (CZ_SKILL_UP, 0x010B)
    ///
    /// 客户端发送格式：packet_id(2) + skill_id(2)
    ///
    /// 处理逻辑：
    /// 1. 解析 skill_id
    /// 2. 验证玩家有剩余技能点 > 0
    /// 3. 验证技能已学习且等级 < MAX_SINGLE_SKILL_LEVEL
    /// 4. 增加技能等级，消耗技能点
    /// 5. 返回 ZC_SKILLINFO_UPDATE (0x010E)
    pub(super) fn handle_skill_up(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;

        // 包体格式：skill_id(2) = 2 字节
        if data.len() < 2 {
            return None;
        }

        let skill_id = u16::from_le_bytes([data[0], data[1]]);

        let player = self.map_state.get_player(&player_id)?;

        // 从数据库获取当前技能等级
        let current_level = match self.db.get_skill_level(player.char_id, skill_id) {
            Ok(lv) => lv,
            Err(e) => {
                tracing::error!("查询技能等级失败: {}", e);
                return None;
            }
        };

        // 验证技能已学习（等级 > 0）
        if current_level == 0 {
            tracing::warn!(
                player = %player.name,
                skill_id = skill_id,
                "技能未学习，无法升级"
            );
            return Some(build_skill_info_update(skill_id, 0));
        }

        // 检查是否有剩余技能点
        let skill_points = player.skill_point();
        if skill_points == 0 {
            tracing::warn!(
                player = %player.name,
                skill_id = skill_id,
                "技能点不足，分配请求被拒绝"
            );
            return Some(build_skill_info_update(skill_id, current_level));
        }

        // 检查技能是否已达上限
        if current_level >= crate::game::constants::MAX_SINGLE_SKILL_LEVEL {
            tracing::warn!(
                player = %player.name,
                skill_id = skill_id,
                current_level = current_level,
                "技能已达最大等级"
            );
            return Some(build_skill_info_update(skill_id, current_level));
        }

        // 增加技能等级
        let new_level = current_level + 1;
        if let Err(e) = self.db.set_skill_level(player.char_id, skill_id, new_level) {
            tracing::error!("保存技能等级失败: {}", e);
            return None;
        }

        // 消耗技能点
        self.map_state.allocate_player_skill_point(&player_id);

        tracing::info!(
            player = %player.name,
            skill_id = skill_id,
            old_level = current_level,
            new_level = new_level,
            "技能点分配成功"
        );

        Some(build_skill_info_update(skill_id, new_level))
    }
}

/// 构建 ZC_STATUS_CHANGE_ACK (0x00BC) 包
///
/// rAthena 格式：length(2) + packet_id(2) + status_id(2) + value(2) + status_point(2)
/// 成功时 value 为新属性值，失败时 value 为当前值（未变化），客户端通过比较判断结果
fn build_status_change_ack(status_id: u16, player: &crate::game::map::Player) -> Vec<u8> {
    let value = match status_id {
        13 => player.str(),
        14 => player.agi(),
        15 => player.vit(),
        16 => player.int(),
        17 => player.dex(),
        18 => player.luk(),
        _ => 0,
    };

    let status_point = player.status_point();

    let mut pkt = Vec::with_capacity(10);
    pkt.extend_from_slice(&10u16.to_le_bytes());
    pkt.extend_from_slice(&ZC_STATUS_CHANGE_ACK.to_le_bytes());
    pkt.extend_from_slice(&status_id.to_le_bytes());
    pkt.extend_from_slice(&value.to_le_bytes());
    pkt.extend_from_slice(&status_point.to_le_bytes());
    pkt
}

/// 构建 ZC_SKILLINFO_UPDATE (0x010E) 包
///
/// rAthena 格式：length(2) + packet_id(2) + skill_id(2) + level(2)
fn build_skill_info_update(skill_id: u16, level: u8) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(8);
    pkt.extend_from_slice(&8u16.to_le_bytes());
    pkt.extend_from_slice(&ZC_SKILLINFO_UPDATE.to_le_bytes());
    pkt.extend_from_slice(&skill_id.to_le_bytes());
    pkt.extend_from_slice(&(level as u16).to_le_bytes());
    pkt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::channel::ChannelBus;
    use crate::game::map::player::{
        Attributes, CombatStats, Economy, LevelStats, PlayerState, Position, SavePoint,
    };
    use crate::game::map::MapState;
    use crate::game::status::PlayerStatus;
    use crate::game::item::Equipment;
    use crate::game::constants;
    use parking_lot::RwLock;
    use std::sync::Arc;
    use uuid::Uuid;

    /// 创建测试用 MapServer（简化版）
    fn make_test_server(map_state: Arc<MapState>, channel_bus: Arc<ChannelBus>) -> MapServer {
        let db = Arc::new(crate::storage::Database::open_memory().unwrap());
        let teleport_manager = Arc::new(RwLock::new(
            crate::game::map::teleport::TeleportManager::new(),
        ));
        let save_point_manager = Arc::new(RwLock::new(
            crate::game::map::teleport::SavePointManager::new(),
        ));
        let warp_service = Arc::new(crate::game::map::teleport::WarpService::new(
            teleport_manager.clone(),
            save_point_manager.clone(),
            db.clone(),
        ));

        MapServer::new(
            db,
            Arc::new(crate::game::token::TokenStore::new()),
            map_state,
            channel_bus,
            Arc::new(crate::game::map::drop_item::DropManager::new()),
            Arc::new(crate::game::party::PartyManager::new()),
            Arc::new(crate::game::guild::GuildManager::new()),
            Arc::new(crate::game::storage::StorageManager::new()),
            Arc::new(crate::game::trade::TradeManager::new()),
            teleport_manager,
            warp_service,
            false,
            Arc::new(crate::game::battle::BattleHandler::default()),
            Arc::new(crate::game::mob::MobSpawnManager::new()),
        )
    }

    /// 创建测试用 Player
    fn make_test_player(status_points: u16) -> crate::game::map::Player {
        make_test_player_full(status_points, 0)
    }

    /// 创建测试用 Player（完整参数版本）
    fn make_test_player_full(status_points: u16, skill_points: u16) -> crate::game::map::Player {
        crate::game::map::Player {
            id: Uuid::new_v4(),
            char_id: 1,
            account_id: 1,
            name: "TestPlayer".to_string(),
            map_name: "test_map".to_string(),
            combat: RwLock::new(CombatStats {
                hp: 100,
                max_hp: 100,
                sp: 50,
                max_sp: 50,
                state: PlayerState::Alive,
                in_combat: false,
                is_sitting: false,
                walk_speed: constants::DEFAULT_WALK_SPEED,
                direction: 0,
            }),
            pos: RwLock::new(Position { x: 100, y: 100 }),
            level: RwLock::new(LevelStats {
                base_level: 10,
                job_level: 5,
                base_exp: 5000,
                job_exp: 3000,
                status_point: status_points,
                skill_point: skill_points,
            }),
            attrs: RwLock::new(Attributes {
                str: 1,
                agi: 1,
                vit: 1,
                int: 1,
                dex: 1,
                luk: 1,
            }),
            economy: RwLock::new(Economy {
                zeny: 0,
                current_weight: 0,
                max_weight: constants::BASE_MAX_WEIGHT,
                job: 0,
                shop_id: None,
                group_id: 0,
            }),
            save_point: RwLock::new(SavePoint {
                map: "test_map".to_string(),
                x: 50,
                y: 50,
            }),
            equipment: RwLock::new(Equipment::new()),
            status: PlayerStatus::new(Uuid::new_v4()),
            inventory: RwLock::new(Vec::new()),
            hotkeys: RwLock::new(Vec::new()),
        }
    }

    /// 构建 CZ_STATUS_CHANGE 数据包体
    fn build_status_change_packet(status_id: u16, amount: u8) -> Vec<u8> {
        let mut data = Vec::with_capacity(3);
        data.extend_from_slice(&status_id.to_le_bytes());
        data.push(amount);
        data
    }

    // ==================== 状态点分配测试 ====================

    #[test]
    fn test_status_change_str_success() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 分配 5 点到 STR (status_id = 13)
        let data = build_status_change_packet(13, 5);
        let result = server.handle_status_change(&data, &mut session);

        // 应返回 ZC_STATUS_CHANGE_ACK
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt.len(), 10);
        // 验证 packet_id = 0x00BC
        assert_eq!(pkt[2], 0xBC);
        assert_eq!(pkt[3], 0x00);

        // 验证属性已增加
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.str(), 6); // 原始 1 + 分配 5

        // 验证状态点已消耗
        assert_eq!(player.status_point(), 95); // 100 - 5
    }

    #[test]
    fn test_status_change_all_stats() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(60);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 分配 10 点到每个属性
        let stat_ids = [13, 14, 15, 16, 17, 18]; // STR, AGI, VIT, INT, DEX, LUK
        for &stat_id in &stat_ids {
            let data = build_status_change_packet(stat_id, 10);
            let result = server.handle_status_change(&data, &mut session);
            assert!(result.is_some());
        }

        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.str(), 11);  // 1 + 10
        assert_eq!(player.agi(), 11);
        assert_eq!(player.vit(), 11);
        assert_eq!(player.int(), 11);
        assert_eq!(player.dex(), 11);
        assert_eq!(player.luk(), 11);
        assert_eq!(player.status_point(), 0); // 60 - 60
    }

    #[test]
    fn test_status_change_insufficient_points() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(3); // 只有 3 点
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 尝试分配 5 点（不足）
        let data = build_status_change_packet(13, 5);
        let result = server.handle_status_change(&data, &mut session);

        // 应返回失败的 ACK
        assert!(result.is_some());

        // 属性不应改变
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.str(), 1);
        assert_eq!(player.status_point(), 3);
    }

    #[test]
    fn test_status_change_rejects_zero_amount() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        let data = build_status_change_packet(13, 0);
        let result = server.handle_status_change(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_status_change_rejects_invalid_status_id() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 无效的 status_id (99)
        let data = build_status_change_packet(99, 5);
        let result = server.handle_status_change(&data, &mut session);
        assert!(result.is_none());

        // 状态点不应改变
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.status_point(), 100);
    }

    #[test]
    fn test_status_change_rejects_no_session() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let server = make_test_server(map_state, channel_bus);

        let mut session = Session::new();
        let data = build_status_change_packet(13, 5);
        let result = server.handle_status_change(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_status_change_rejects_short_data() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 数据太短（只有 2 字节，需要 3 字节）
        let data = vec![13, 0];
        let result = server.handle_status_change(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_status_change_updates_max_weight() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;

        let initial_max_weight = player.max_weight();
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 分配 10 点到 STR
        let data = build_status_change_packet(13, 10);
        server.handle_status_change(&data, &mut session);

        // 最大负重应增加（每点 STR 增加 WEIGHT_PER_STR）
        let player = map_state.get_player(&player_id).unwrap();
        let new_max_weight = player.max_weight();
        assert!(new_max_weight > initial_max_weight);
    }

    // ==================== 属性上限测试 ====================

    #[test]
    fn test_status_change_stat_cap_at_99() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;
        // 设置 STR 为 95
        player.attributes_mut().str = 95;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 尝试分配 10 点到 STR（95 + 10 = 105 > 99），应失败
        let data = build_status_change_packet(13, 10);
        let result = server.handle_status_change(&data, &mut session);
        assert!(result.is_some());

        // 属性不应改变
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.str(), 95);
        assert_eq!(player.status_point(), 100);
    }

    #[test]
    fn test_status_change_stat_cap_exact_99() {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player(100);
        let player_id = player.id;
        // 设置 STR 为 95
        player.attributes_mut().str = 95;
        map_state.add_player(player);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 分配 4 点到 STR（95 + 4 = 99，刚好等于上限），应成功
        let data = build_status_change_packet(13, 4);
        let result = server.handle_status_change(&data, &mut session);
        assert!(result.is_some());

        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.str(), 99);
        assert_eq!(player.status_point(), 96);
    }

    // ==================== 技能点分配测试 ====================

    /// 构建 CZ_SKILL_UP 数据包体
    fn build_skill_up_packet(skill_id: u16) -> Vec<u8> {
        let mut data = Vec::with_capacity(2);
        data.extend_from_slice(&skill_id.to_le_bytes());
        data
    }

    /// 创建带数据库技能的测试环境
    fn setup_skill_test(skill_points: u16, skill_id: u16, skill_level: u8) -> (MapServer, Arc<MapState>, Uuid) {
        let map_state = Arc::new(MapState::new());
        let channel_bus = Arc::new(ChannelBus::new());
        let player = make_test_player_full(0, skill_points);
        let player_id = player.id;
        map_state.add_player(player);

        let server = make_test_server(map_state.clone(), channel_bus);

        // 初始化数据库 schema（包含 skills 表）
        crate::storage::schema::init_schema(&server.db).unwrap();
        // 创建测试账户和角色（满足 FOREIGN KEY 约束）
        server.db.create_account("test", "hash", 0).unwrap();
        server.db.create_character(1, 0, "TestChar", 1, 1, 1, 1, 1, 1, 1, 0).unwrap();

        // 在数据库中初始化技能
        if skill_level > 0 {
            server.db.set_skill_level(1, skill_id, skill_level).unwrap();
        }

        (server, map_state, player_id)
    }

    #[test]
    fn test_skill_up_success() {
        let (server, map_state, player_id) = setup_skill_test(5, 1, 3);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        // 升级技能 1 从等级 3 到 4
        let data = build_skill_up_packet(1);
        let result = server.handle_skill_up(&data, &mut session);

        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt.len(), 8);
        // 验证 packet_id = 0x010E
        assert_eq!(pkt[2], 0x0E);
        assert_eq!(pkt[3], 0x01);
        // 验证新等级 = 4
        assert_eq!(pkt[6], 4);

        // 验证技能点已消耗
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.skill_point(), 4);

        // 验证数据库中技能等级已更新
        let level = server.db.get_skill_level(1, 1).unwrap();
        assert_eq!(level, 4);
    }

    #[test]
    fn test_skill_up_no_skill_points() {
        let (server, _, player_id) = setup_skill_test(0, 1, 3);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let data = build_skill_up_packet(1);
        let result = server.handle_skill_up(&data, &mut session);

        // 应返回失败的 ACK（等级不变）
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[6], 3); // 等级仍为 3
    }

    #[test]
    fn test_skill_up_not_learned() {
        let (server, _, player_id) = setup_skill_test(5, 99, 0);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        // 技能 99 未学习（等级 0）
        let data = build_skill_up_packet(99);
        let result = server.handle_skill_up(&data, &mut session);

        // 应返回失败的 ACK（等级为 0）
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[6], 0);
    }

    #[test]
    fn test_skill_up_max_level() {
        let (server, map_state, player_id) = setup_skill_test(5, 1, constants::MAX_SINGLE_SKILL_LEVEL);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        let data = build_skill_up_packet(1);
        let result = server.handle_skill_up(&data, &mut session);

        // 应返回当前等级（未变化）
        assert!(result.is_some());
        let pkt = result.unwrap();
        assert_eq!(pkt[6], constants::MAX_SINGLE_SKILL_LEVEL);

        // 技能点不应消耗
        let player = map_state.get_player(&player_id).unwrap();
        assert_eq!(player.skill_point(), 5);
    }

    #[test]
    fn test_skill_up_rejects_short_data() {
        let (server, _, player_id) = setup_skill_test(5, 1, 3);

        let mut session = Session::new();
        session.player_id = Some(player_id);

        // 数据太短（只有 1 字节，需要 2 字节）
        let data = vec![1];
        let result = server.handle_skill_up(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_skill_up_rejects_no_session() {
        let (server, _, _) = setup_skill_test(5, 1, 3);

        let mut session = Session::new();
        let data = build_skill_up_packet(1);
        let result = server.handle_skill_up(&data, &mut session);
        assert!(result.is_none());
    }
}
