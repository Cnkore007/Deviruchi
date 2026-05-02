use deviruchi::game::trade::data::*;
use uuid::Uuid;

#[test]
fn test_trade_item_new() {
    let item = TradeItem::new(1, 501, 10, false, 0, [0; 4]);
    assert_eq!(item.index, 1);
    assert_eq!(item.item_id, 501);
    assert_eq!(item.amount, 10);
    assert!(!item.identified);
    assert_eq!(item.refine, 0);
    assert_eq!(item.cards, [0; 4]);
}

#[test]
fn test_trade_session_new() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let session = TradeSession::new(player1, player2);

    assert_eq!(session.player1_id, player1);
    assert_eq!(session.player2_id, player2);
    assert_eq!(session.state, TradeState::Requesting);
    assert_eq!(session.player1_zeny, 0);
    assert_eq!(session.player2_zeny, 0);
    assert!(!session.player1_locked);
    assert!(!session.player2_locked);
}

#[test]
fn test_trade_session_add_item() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;

    let item = TradeItem::new(0, 501, 5, true, 0, [0; 4]);
    assert!(session.add_item(player1, item));

    assert_eq!(session.player1_items.len(), 1);
    assert_eq!(session.player1_items[0].item_id, 501);
    assert_eq!(session.player1_items[0].amount, 5);
}

#[test]
fn test_trade_session_add_zeny() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;

    assert!(session.add_zeny(player1, 1000));
    assert_eq!(session.player1_zeny, 1000);
}

#[test]
fn test_trade_session_lock() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;

    assert!(session.lock(player1));
    assert!(session.player1_locked);
    assert!(!session.player2_locked);
    assert!(!session.is_fully_locked());

    assert!(session.lock(player2));
    assert!(session.player2_locked);
    assert!(session.is_fully_locked());
}

#[test]
fn test_trade_session_cancel() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;

    session.cancel();
    assert_eq!(session.state, TradeState::Cancelled);
}

#[test]
fn test_trade_session_get_partner_items() {
    let player1 = Uuid::new_v4();
    let player2 = Uuid::new_v4();

    let mut session = TradeSession::new(player1, player2);
    session.state = TradeState::Trading;

    // Player1 添加物品
    let item = TradeItem::new(0, 501, 5, true, 0, [0; 4]);
    session.add_item(player1, item);

    // Player1 查看对方物品（应该是 player2 的物品，现在为空）
    let partner_items = session.get_partner_items(player1);
    assert!(partner_items.is_empty());

    // Player2 添加物品
    let item2 = TradeItem::new(0, 502, 3, true, 0, [0; 4]);
    session.add_item(player2, item2);

    // Player1 查看对方物品
    let partner_items = session.get_partner_items(player1);
    assert_eq!(partner_items.len(), 1);
    assert_eq!(partner_items[0].item_id, 502);
}
