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
}
