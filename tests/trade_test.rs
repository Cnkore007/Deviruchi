use deviruchi::game::trade::*;
use uuid::Uuid;

#[test]
fn test_trade_session_new() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);

    assert_eq!(session.player1_id, player1);
    assert_eq!(session.player2_id, player2);
    assert_eq!(*session.state.read(), TradeState::Requesting);
    assert_eq!(*session.zeny1.read(), 0);
    assert_eq!(*session.zeny2.read(), 0);
    assert!(!*session.locked1.read());
    assert!(!*session.locked2.read());
}

#[test]
fn test_trade_session_add_item() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);
    session.start();

    let item = TradeItem {
        inventory_index: 0,
        item_id: 501,
        amount: 5,
    };
    assert!(session.add_item(player1, item));

    let items = session.items1.read();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_id, 501);
    assert_eq!(items[0].amount, 5);
}

#[test]
fn test_trade_session_set_zeny() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);
    session.start();

    assert!(session.set_zeny(player1, 1000));
    assert_eq!(*session.zeny1.read(), 1000);
}

#[test]
fn test_trade_session_lock() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);
    session.start();

    assert!(session.lock(player1));
    assert!(*session.locked1.read());
    assert!(!*session.locked2.read());
    assert!(!session.is_fully_locked());

    assert!(session.lock(player2));
    assert!(*session.locked2.read());
    assert!(session.is_fully_locked());
}

#[test]
fn test_trade_session_cancel() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);
    session.start();

    session.cancel();
    assert_eq!(*session.state.read(), TradeState::Cancelled);
}

#[test]
fn test_trade_session_both_players_add_items() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);
    session.start();

    // Player1 添加物品
    let item = TradeItem {
        inventory_index: 0,
        item_id: 501,
        amount: 5,
    };
    assert!(session.add_item(player1, item));
    assert_eq!(session.items1.read().len(), 1);

    // Player2 添加物品
    let item2 = TradeItem {
        inventory_index: 0,
        item_id: 502,
        amount: 3,
    };
    assert!(session.add_item(player2, item2));
    assert_eq!(session.items2.read().len(), 1);
    assert_eq!(session.items2.read()[0].item_id, 502);
}
