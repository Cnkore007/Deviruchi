use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStage {
    Login,
    Char,
    Map,
}

#[derive(Clone)]
pub struct Session {
    pub id: Uuid,
    pub account_id: Option<u32>,
    pub char_id: Option<u32>,
    pub authenticated: bool,
    pub version: u32,
    pub client_type: u8,
    pub stage: SessionStage,
    pub player_id: Option<Uuid>,
    /// Channel sender for game events pushed to client (connected to ChannelBus)
    pub map_event_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id: None,
            char_id: None,
            authenticated: false,
            version: 0,
            client_type: 0,
            stage: SessionStage::Login,
            player_id: None,
            map_event_tx: None,
        }
    }

    pub fn authenticate(&mut self, account_id: u32) {
        self.account_id = Some(account_id);
        self.authenticated = true;
    }
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    addr_to_session: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            addr_to_session: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add(&self, addr: String, session: Session) -> Uuid {
        let id = session.id;
        self.sessions.write().insert(id, session);
        self.addr_to_session.write().insert(addr, id);
        id
    }

    pub fn remove(&self, id: &Uuid) {
        self.sessions.write().remove(id);
    }

    pub fn get(&self, id: &Uuid) -> Option<Session> {
        self.sessions.read().get(id).cloned()
    }

    pub fn update(&self, id: &Uuid, session: Session) {
        self.sessions.write().insert(*id, session);
    }

    pub fn count(&self) -> usize {
        self.sessions.read().len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_starts_at_login_stage() {
        let session = Session::new();
        assert_eq!(session.stage, SessionStage::Login);
    }

    #[test]
    fn player_id_is_none_by_default() {
        let session = Session::new();
        assert!(session.player_id.is_none());
    }

    #[test]
    fn session_stage_can_be_set_to_char_and_map() {
        let mut session = Session::new();
        session.stage = SessionStage::Char;
        assert_eq!(session.stage, SessionStage::Char);
        session.stage = SessionStage::Map;
        assert_eq!(session.stage, SessionStage::Map);
    }
}
