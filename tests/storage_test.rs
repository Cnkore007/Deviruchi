use deviruchi::storage::{Database, init_schema};

#[test]
fn test_database_memory() {
    let db = Database::open_memory().unwrap();
    init_schema(&db).unwrap();

    // 测试插入账户
    db.execute(
        "INSERT INTO accounts (user_id, password_hash, sex, created_at)
         VALUES ('test', 'hash', 0, 1234567890)"
    ).unwrap();

    // 测试查询
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM accounts",
        |row| row.get(0)
    ).unwrap();

    assert_eq!(count, 1);
}
