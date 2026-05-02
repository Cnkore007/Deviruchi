use std::sync::Arc;
use tracing::warn;
use crate::storage::Database;
use crate::network::{Session, SessionManager, PacketId};
use crate::network::session::SessionStage;
use crate::game::token::TokenStore;
use crate::game::map::{MapState, ChannelBus, DropManager, MapServer};
use crate::game::party::PartyManager;

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
    ) -> Self {
        let map_server = Arc::new(MapServer::new(
            db.clone(),
            token_store.clone(),
            map_state,
            channel_bus,
            drop_manager,
            party_manager,
            false, // death_drop_items
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

    pub fn handle(&self, session: &mut Session, packet_id: PacketId, data: &[u8]) -> Option<Vec<u8>> {
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
                if matches!(packet_id, 0x0065 | 0x0066 | 0x0067 | 0x0068) {
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
            SessionStage::Map => {
                self.map_server.handle_packet(packet_id, data, session)
            }
        }
    }
}
