use deviruchi::storage::{Database, init_schema};

#[test]
fn test_create_and_get_account() {
    let db = Database::open_memory().unwrap();
    init_schema(&db).unwrap();

    let account_id = db.create_account("testuser", "hash123", 0).unwrap();
    assert!(account_id > 0);

    let account = db.get_account_by_userid("testuser").unwrap().unwrap();
    assert_eq!(account.user_id, "testuser");
    assert_eq!(account.sex, 0);
}

#[test]
fn test_create_and_get_character() {
    let db = Database::open_memory().unwrap();
    init_schema(&db).unwrap();

    let account_id = db.create_account("testuser", "hash123", 0).unwrap();

    let char_id = db.create_character(
        account_id, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0
    ).unwrap();
    assert!(char_id > 0);

    let characters = db.get_characters_by_account(account_id).unwrap();
    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0].name, "TestChar");
}
