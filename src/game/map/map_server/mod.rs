//! MapServer - 地图服务器核心，处理客户端数据包
//!
//! 按功能领域拆分为子模块：
//! - `player` — 进入、移动、攻击、技能、物品使用、拾取
//! - `npc`    — NPC 交互与对话状态机
//! - `social` — 队伍、聊天、仓库、交易
//! - `gm`     — GM 命令、传送、重生
//! - `guild`  — 公会操作

mod chatroom;
mod emotion;
mod equip;
mod friends;
mod gm;
mod guild;
mod homunculus_mercenary;
mod item_ops;
mod job;
mod mail_bank_shop;
mod movement;
mod npc;
mod pet;
mod player;
mod quest_achievement_pvp;
mod shop;
mod social;

use crate::game::battle::BattleHandler;
use crate::game::guild::GuildManager;
use crate::game::item::{ItemDatabase, ItemEffectDatabase, ItemIntegrationHandler};
use crate::game::map::MapState;
use crate::game::map::channel::ChannelBus;
use crate::game::map::drop_item::DropManager;
use crate::game::map::teleport::{TeleportManager, WarpService};
use crate::game::mob::MobSpawnManager;
use crate::game::npc::handler::NpcHandler;
use crate::game::party::PartyManager;
use crate::game::script::dialogue::NpcDialogueState;
use crate::game::skill::SkillHandler;
use crate::game::storage::StorageManager;
use crate::game::token::TokenStore;
use crate::game::trade::TradeManager;
use crate::network::packet::id::*;
use crate::network::session::Session;
use parking_lot::RwLock;
use std::collections::HashMap;
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
    pub npc_handler: Arc<NpcHandler>,
    pub active_dialogues: RwLock<HashMap<Uuid, NpcDialogueState>>,
    pub battle_handler: Arc<BattleHandler>,
    pub spawn_manager: Arc<MobSpawnManager>,
}

impl MapServer {
    #[allow(clippy::too_many_arguments)]
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
        battle_handler: Arc<BattleHandler>,
        spawn_manager: Arc<MobSpawnManager>,
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
            npc_handler: Arc::new(NpcHandler::new()),
            active_dialogues: RwLock::new(HashMap::new()),
            battle_handler,
            spawn_manager,
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
            0x00A7 => self.handle_use_item(data, session),
            0x0090 => self.handle_pickup_item(data, session),
            0x0190 => self.handle_npc_interact(data, session),
            0x00B9 => self.handle_npc_next(data, session),
            0x00B8 => self.handle_npc_select(data, session),
            0x0146 => self.handle_npc_close(data, session),
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
            PACKET_CZ_REQUEST_TIME => self.handle_request_time(),
            PACKET_CZ_REQUEST_QUIT => self.handle_request_quit(session),
            CZ_REQUEST_CHANGE_DIRECTION => self.handle_change_direction(data, session),
            CZ_WHISPER => self.handle_whisper(data, session),
            CZ_STATUS_CHANGE => self.handle_status_change(data, session),
            CZ_SKILL_UP => self.handle_skill_up(data, session),
            CZ_REQ_CHANGEJOB => self.handle_job_change(data, session),
            // 装备系统
            0x00A9 => self.handle_equip_item(data, session),
            0x00AB => self.handle_unequip_item(data, session),
            // NPC商店
            0x00C8 => self.handle_npc_buy(data, session),
            0x00C9 => self.handle_npc_sell(data, session),
            // 物品操作
            0x017C => self.handle_insert_card(data, session),
            0x01DD => self.handle_item_identify(data, session),
            0x0222 => self.handle_weapon_refine(data, session),
            // 表情
            0x00BF => self.handle_emotion(data, session),
            // 宠物
            0x019F => self.handle_catch_pet(data, session),
            0x01A9 => self.handle_pet_menu(data, session),
            0x01A7 => self.handle_select_egg(data, session),
            // 半魔娘/佣兵
            0x022D => self.handle_homunculus_menu(data, session),
            0x022F => self.handle_mercenary_action(data, session),
            // 聊天室
            0x00D5 => self.handle_create_chat_room(data, session),
            0x00D9 => self.handle_chat_add_member(data, session),
            0x00E0 => self.handle_chat_leave(data, session),
            // 好友
            0x0201 => self.handle_friends_list_add(data, session),
            0x0203 => self.handle_friends_list_remove(data, session),
            0x0208 => self.handle_friends_list_reply(data, session),
            // 邮件
            0x0260 => self.handle_mail_open(data, session),
            0x0261 => self.handle_mail_send(data, session),
            // 银行
            0x09B7 => self.handle_bank_open(data, session),
            0x09B8 => self.handle_bank_close(data, session),
            0x09B9 => self.handle_bank_deposit(data, session),
            0x09BA => self.handle_bank_withdraw(data, session),
            // 商城
            0x0845 => self.handle_cash_shop_open(data, session),
            0x0848 => self.handle_cash_shop_buy(data, session),
            0x084A => self.handle_cash_shop_close(data, session),
            // 任务/成就/PVP
            0x02B5 => self.handle_quest_state(data, session),
            0x0224 => self.handle_achievement_reward(data, session),
            0x0237 => self.handle_pvp_info(data, session),
            // 坐骑/技能
            0x019C => self.handle_change_cart(data, session),
            0x0A35 => self.handle_skill_select_menu(data, session),
            0x01CF => self.handle_auto_spell(data, session),
            unknown_id => {
                tracing::warn!(
                    "Unknown packet ID 0x{:04X} from session {}",
                    unknown_id,
                    session.id
                );
                None
            }
        }
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
        use crate::game::battle::BattleHandler;
        use crate::game::guild::GuildManager;
        use crate::game::map::teleport::{SavePointManager, TeleportManager, WarpService};
        use crate::game::mob::MobSpawnManager;
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
            Arc::new(BattleHandler::default()),
            Arc::new(MobSpawnManager::new()),
        );
        let mut session = Session::new();
        let result = server.handle_packet(0xFFFF, &[], &mut session);
        assert!(result.is_none());
    }
}
