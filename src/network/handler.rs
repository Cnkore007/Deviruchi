use std::sync::Arc;
use crate::storage::Database;
use crate::network::{Session, SessionManager, PacketId};
use crate::game::token::TokenStore;

pub struct PacketHandler {
    login_server: Arc<crate::game::login::LoginServer>,
    char_server: Arc<crate::game::char::CharServer>,
}

impl PacketHandler {
    pub fn new(
        db: Arc<Database>,
        session_manager: Arc<SessionManager>,
        token_store: Arc<TokenStore>,
    ) -> Self {
        Self {
            login_server: Arc::new(crate::game::login::LoginServer::new(
                db.clone(),
                session_manager.clone()
            )),
            char_server: Arc::new(crate::game::char::CharServer::new(
                db,
                session_manager,
                token_store,
            )),
        }
    }

    pub fn handle(&self, session: &mut Session, packet_id: PacketId, data: &[u8]) -> Option<Vec<u8>> {
        // Login Server packet range: 0x0064
        if packet_id == 0x0064 {
            return self.login_server.handle_packet(packet_id, data, session);
        }

        // Char Server packet range: 0x0065 - 0x0068
        if matches!(packet_id, 0x0065 | 0x0066 | 0x0067 | 0x0068) {
            return self.char_server.handle_packet(packet_id, data, session);
        }

        None
    }
}
