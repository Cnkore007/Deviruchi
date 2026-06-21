//! 集成测试
//!
//! 测试 Login→Char→Map 完整流程

use std::sync::Arc;

// 导入 Deviruchi 模块
use deviruchi::storage::Database;
use deviruchi::storage::schema::init_schema;

/// 测试账号创建和登录流程
#[test]
fn test_account_creation_and_login() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    // 创建账号
    let account_id = db.create_account("test_user", "password_hash", 1).unwrap();
    assert!(account_id > 0, "账号 ID 应该大于 0");

    // 验证账号存在
    let account = db.get_account_by_userid("test_user").unwrap();
    assert!(account.is_some(), "账号应该存在");
}

/// 测试角色创建流程
#[test]
fn test_character_creation() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    // 创建账号
    let account_id = db.create_account("test_user", "password_hash", 1).unwrap();

    // 创建角色
    let result = db.create_character(
        account_id, 0, // slot
        "TestChar", 1, // str
        1, // agi
        1, // vit
        1, // int
        1, // dex
        1, // luk
        0, // hair
        0, // hair_color
    );
    assert!(result.is_ok(), "角色创建应该成功");

    let char_id = result.unwrap();
    let character = db.get_character_by_id(char_id).unwrap();
    assert!(character.is_some(), "角色应该存在");
    assert_eq!(character.unwrap().name, "TestChar");
}

/// 测试完整的游戏流程
#[test]
fn test_full_game_flow() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    // 1. 创建账号
    let account_id = db.create_account("player1", "pass123", 1).unwrap();
    assert!(account_id > 0, "账号 ID 应该大于 0");

    // 2. 创建角色
    let char_id = db
        .create_character(account_id, 0, "Hero", 1, 1, 1, 1, 1, 1, 0, 0)
        .unwrap();
    assert!(char_id > 0, "角色 ID 应该大于 0");

    // 3. 验证角色数据
    let character = db.get_character_by_id(char_id).unwrap().unwrap();
    assert_eq!(character.name, "Hero");

    // 4. 测试角色删除
    let delete_result = db.mark_character_for_deletion(char_id, account_id, 86400);
    assert!(delete_result.is_ok(), "角色删除标记应该成功");

    // 5. 测试取消删除
    let cancel_result = db.cancel_character_deletion(char_id, account_id);
    assert!(cancel_result.is_ok(), "取消删除应该成功");
}

/// 测试数据库初始化
#[test]
fn test_database_initialization() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    // 验证数据库初始化成功
    let account_id = db.create_account("init_test", "pass", 1);
    assert!(account_id.is_ok(), "数据库应该初始化成功");
}

/// 测试并发访问
#[test]
fn test_concurrent_access() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    let mut handles = vec![];

    // 创建多个并发任务
    for i in 0..10 {
        let db_clone = db.clone();
        let handle = std::thread::spawn(move || {
            let account_id = db_clone
                .create_account(&format!("user_{}", i), "password", 1)
                .unwrap();

            let char_id = db_clone
                .create_character(
                    account_id,
                    0,
                    &format!("Char_{}", i),
                    1,
                    1,
                    1,
                    1,
                    1,
                    1,
                    0,
                    0,
                )
                .unwrap();

            (account_id, char_id)
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        let (account_id, char_id) = handle.join().unwrap();
        assert!(account_id > 0);
        assert!(char_id > 0);
    }
}

/// 测试角色名称验证
#[test]
fn test_character_name_validation() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    let account_id = db.create_account("test_user", "pass", 1).unwrap();

    // 测试有效名称
    let result = db.create_character(account_id, 0, "ValidName", 1, 1, 1, 1, 1, 1, 0, 0);
    assert!(result.is_ok(), "有效名称应该成功");

    // 测试重复名称
    let result = db.create_character(account_id, 1, "ValidName", 1, 1, 1, 1, 1, 1, 0, 0);
    assert!(result.is_ok(), "数据库允许重复名称（应用层检查）");
}

/// 测试角色属性
#[test]
fn test_character_stats() {
    let db = Arc::new(Database::open_memory().unwrap());
    init_schema(&db).unwrap();

    let account_id = db.create_account("test_user", "pass", 1).unwrap();
    let char_id = db
        .create_character(
            account_id, 0, "StatTest", 5, // str
            3, // agi
            4, // vit
            2, // int
            6, // dex
            1, // luk
            0, 0,
        )
        .unwrap();

    let character = db.get_character_by_id(char_id).unwrap().unwrap();
    assert_eq!(character.str, 5);
    assert_eq!(character.agi, 3);
    assert_eq!(character.vit, 4);
    assert_eq!(character.int, 2);
    assert_eq!(character.dex, 6);
    assert_eq!(character.luk, 1);
}
