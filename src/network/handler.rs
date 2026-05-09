use crate::game::battle::BattleHandler;
use crate::game::guild::GuildManager;
use crate::game::map::teleport::{SavePointManager, TeleportManager, WarpService};
use crate::game::map::{ChannelBus, DropManager, MapServer, MapState};
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

pub struct PacketHandler {
    login_server: Arc<crate::game::login::LoginServer>,
    char_server: Arc<crate::game::char::CharServer>,
    map_server: Arc<MapServer>,
}

impl PacketHandler {
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

        // Create teleport manager, save point manager and warp service
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
            login_server: Arc::new(crate::game::login::LoginServer::new(
                db.clone(),
                session_manager.clone(),
            )),
            char_server: Arc::new(crate::game::char::CharServer::new(
                db.clone(),
                session_manager.clone(),
                token_store.clone(),
            )),
            map_server,
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
                    let result = self.login_server.handle_packet(packet_id, data, session);
                    // Advance to Char stage on successful login
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
                    let result = self.char_server.handle_packet(packet_id, data, session);
                    // Advance to Map stage on successful char selection
                    if packet_id == 0x0065 && result.is_some() && session.char_id.is_some() {
                        session.stage = SessionStage::Map;
                    }
                    result
                } else {
                    warn!("Invalid packet 0x{:04X} at Char stage", packet_id);
                    None
                }
            }
            SessionStage::Map => self.map_server.handle_packet(packet_id, data, session),
        }
    }

    /// 处理玩家断开连接：保存数据并从地图移除
    pub fn handle_disconnect(&self, session: &Session) {
        if let Some(player_id) = session.player_id {
            // 保存玩家数据
            if let Err(e) = self.map_server.save_player(&player_id) {
                tracing::error!("断连保存玩家数据失败: {}", e);
            }
            // 从地图移除
            if let Some(player) = self.map_server.map_state.get_player(&player_id) {
                let map_name = player.map_name.clone();
                let channel_name = format!("map:{}", map_name);
                self.map_server.channel_bus.unsubscribe(&channel_name, &player_id);
                self.map_server.map_state.remove_player(&player_id);
            }
        }
    }
}
