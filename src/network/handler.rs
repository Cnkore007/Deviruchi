use crate::game::battle::BattleHandler;
use crate::game::guild::GuildManager;
use crate::game::map::teleport::{SavePointManager, TeleportManager, WarpService};
use crate::game::map::{ChannelBus, DropManager, MapServer, MapState, map_channel_name};
use crate::game::mob::MobSpawnManager;
use crate::game::party::PartyManager;
use crate::game::storage::StorageManager;
use crate::game::token::TokenStore;
use crate::game::trade::TradeManager;
use crate::network::session::SessionStage;
use crate::network::{PacketId, Session, SessionManager};
use crate::storage::Database;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::warn;

/// 服务器运行模式，决定 PacketHandler 包含哪些子服务器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    Login,
    Char,
    Map,
    All,
}

pub struct PacketHandler {
    #[allow(dead_code)]
    mode: ServerMode,
    login_server: Option<Arc<crate::game::login::LoginServer>>,
    char_server: Option<Arc<crate::game::char::CharServer>>,
    map_server: Option<Arc<MapServer>>,
}

impl PacketHandler {
    /// Login-only 模式：仅创建 LoginServer
    pub fn new_login(
        db: Arc<Database>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self {
            mode: ServerMode::Login,
            login_server: Some(Arc::new(crate::game::login::LoginServer::new(
                db,
                session_manager,
            ))),
            char_server: None,
            map_server: None,
        }
    }

    /// Char-only 模式：仅创建 CharServer
    pub fn new_char(
        db: Arc<Database>,
        session_manager: Arc<SessionManager>,
        token_store: Arc<TokenStore>,
        _inter_comm: Arc<crate::game::inter_server::InterServerComm>,
    ) -> Self {
        Self {
            mode: ServerMode::Char,
            login_server: None,
            char_server: Some(Arc::new(crate::game::char::CharServer::new(
                db,
                session_manager,
                token_store,
            ))),
            map_server: None,
        }
    }

    /// Map-only 模式：仅创建 MapServer 及其依赖
    #[allow(clippy::too_many_arguments)]
    pub fn new_map(
        db: Arc<Database>,
        _session_manager: Arc<SessionManager>,
        token_store: Arc<TokenStore>,
        map_state: Arc<MapState>,
        channel_bus: Arc<ChannelBus>,
        drop_manager: Arc<DropManager>,
        party_manager: Arc<PartyManager>,
        battle_handler: Arc<BattleHandler>,
        spawn_manager: Arc<MobSpawnManager>,
        death_drop_items: bool,
        guild_manager: Arc<GuildManager>,
        inter_comm: Arc<crate::game::inter_server::InterServerComm>,
    ) -> Self {
        let storage_manager = Arc::new(StorageManager::new());
        let trade_manager = Arc::new(TradeManager::new());

        let teleport_manager = Arc::new(RwLock::new(TeleportManager::new()));
        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let warp_service = Arc::new(WarpService::new(
            teleport_manager.clone(),
            save_point_manager.clone(),
            db.clone(),
        ));

        let map_server = Arc::new(MapServer::new(
            db.clone(),
            token_store.clone(),
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
            battle_handler,
            spawn_manager,
        ).with_inter_comm(inter_comm));

        Self {
            mode: ServerMode::Map,
            login_server: None,
            char_server: None,
            map_server: Some(map_server),
        }
    }

    /// 全模式：创建所有子服务器（向后兼容）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Arc<Database>,
        session_manager: Arc<SessionManager>,
        token_store: Arc<TokenStore>,
        map_state: Arc<MapState>,
        channel_bus: Arc<ChannelBus>,
        drop_manager: Arc<DropManager>,
        party_manager: Arc<PartyManager>,
        battle_handler: Arc<BattleHandler>,
        spawn_manager: Arc<MobSpawnManager>,
        death_drop_items: bool,
        guild_manager: Arc<GuildManager>,
    ) -> Self {
        let storage_manager = Arc::new(StorageManager::new());
        let trade_manager = Arc::new(TradeManager::new());

        let teleport_manager = Arc::new(RwLock::new(TeleportManager::new()));
        let save_point_manager = Arc::new(RwLock::new(SavePointManager::new()));
        let warp_service = Arc::new(WarpService::new(
            teleport_manager.clone(),
            save_point_manager.clone(),
            db.clone(),
        ));

        let map_server = Arc::new(MapServer::new(
            db.clone(),
            token_store.clone(),
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
            battle_handler,
            spawn_manager,
        ));

        Self {
            mode: ServerMode::All,
            login_server: Some(Arc::new(crate::game::login::LoginServer::new(
                db.clone(),
                session_manager.clone(),
            ))),
            char_server: Some(Arc::new(crate::game::char::CharServer::new(
                db.clone(),
                session_manager.clone(),
                token_store.clone(),
            ))),
            map_server: Some(map_server),
        }
    }

    pub fn handle(
        &self,
        session: &mut Session,
        packet_id: PacketId,
        data: &[u8],
    ) -> Option<Vec<u8>> {
        match session.stage {
            SessionStage::Login => {
                if packet_id == 0x0064 {
                    let result = self.login_server.as_ref()?.handle_packet(packet_id, data, session);
                    if result.is_some() && session.authenticated {
                        session.stage = SessionStage::Char;
                    }
                    result
                } else {
                    warn!("Invalid packet 0x{:04X} at Login stage", packet_id);
                    None
                }
            }
            SessionStage::Char => {
                if matches!(packet_id, 0x0065..=0x0068 | 0x01F8) {
                    let result = self.char_server.as_ref()?.handle_packet(packet_id, data, session);
                    if packet_id == 0x0066 && result.is_some() && session.char_id.is_some() {
                        session.stage = SessionStage::Map;
                    }
                    result
                } else {
                    warn!("Invalid packet 0x{:04X} at Char stage", packet_id);
                    None
                }
            }
            SessionStage::Map => {
                self.map_server.as_ref()?.handle_packet(packet_id, data, session)
            }
        }
    }

    /// 处理玩家断开连接：保存数据并从地图移除
    pub fn handle_disconnect(&self, session: &Session) {
        if let Some(player_id) = session.player_id
            && let Some(ref map_server) = self.map_server
        {
            if let Err(e) = map_server.save_player(&player_id) {
                tracing::error!("断连保存玩家数据失败: {}", e);
            }
            if let Some(player) = map_server.map_state.get_player(&player_id) {
                let map_name = player.map_name.clone();
                let channel_name = map_channel_name(&map_name);
                map_server.channel_bus.unsubscribe(&channel_name, &player_id);
                map_server.map_state.remove_player(&player_id);
            }
        }
    }
}
