use deviruchi::network::session::{Session, SessionStage};

#[test]
fn test_session_stage_transitions() {
    let mut session = Session::new();

    // Initially Login stage
    assert!(matches!(session.stage, SessionStage::Login));
    assert!(session.player_id.is_none());

    // Advance through stages
    session.stage = SessionStage::Char;
    assert!(matches!(session.stage, SessionStage::Char));

    session.stage = SessionStage::Map;
    assert!(matches!(session.stage, SessionStage::Map));

    // Can set player_id in Map stage
    session.player_id = Some(uuid::Uuid::new_v4());
    assert!(session.player_id.is_some());
}

#[test]
fn test_session_authentication() {
    let mut session = Session::new();

    // Initially not authenticated
    assert!(!session.authenticated);
    assert!(session.account_id.is_none());

    // Authenticate
    session.authenticate(12345);
    assert!(session.authenticated);
    assert_eq!(session.account_id, Some(12345));
}

#[test]
fn test_session_manager_operations() {
    use deviruchi::network::session::SessionManager;

    let manager = SessionManager::new();

    // Initially no sessions
    assert_eq!(manager.count(), 0);

    // Add a session
    let session = Session::new();
    let session_id = manager.add("127.0.0.1:12345".to_string(), session);

    // Now has one session
    assert_eq!(manager.count(), 1);

    // Can retrieve session
    let retrieved = manager.get(&session_id);
    assert!(retrieved.is_some());

    // Remove session
    manager.remove(&session_id);
    assert_eq!(manager.count(), 0);
}
