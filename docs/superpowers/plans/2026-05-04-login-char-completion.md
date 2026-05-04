# 登录/角色流程补全实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复登录和角色选择流程中的关键 BUG，补全缺失的功能（角色删除、名称校验、封禁过期检查等），并为所有逻辑添加测试覆盖。

**Architecture:** 登录服务器（LoginServer）处理账号认证和会话建立，角色服务器（CharServer）处理角色 CRUD 和进入游戏流程。两者共享 Database 实例和 SessionManager。协议层通过 Packed trait 实现二进制序列化/反序列化，网络层通过 packet_id 路由到对应 handler。

**Tech Stack:** Rust, Tokio, SQLite (rusqlite), parking_lot, Argon2, uuid

---

## Task 1: 修复 login.rs 账号不存在时无响应的 BUG

**问题分析:** `src/game/login.rs` 第 64 行 `self.db.get_account_by_userid(&login.username).ok()??` 存在双重 `?` 操作符问题。`.ok()` 将 `Result<Option<Account>>` 转为 `Option<Option<Account>>`，第一个 `?` 解包外层 Option（数据库错误时返回 None），第二个 `?` 解包内层 Option（账号不存在时返回 None）。当账号不存在时，函数直接返回 `None`，客户端收不到任何响应。

**Files:**
- Modify: `src/game/login.rs`

- [ ] **Step 1: 修复 handle_ca_login 中的账号查询逻辑**

将第 64 行的 `.ok()??` 改为显式匹配，区分数据库错误和账号不存在两种情况。

```rust
// src/game/login.rs 第 54-109 行，替换整个 handle_ca_login 方法
fn handle_ca_login(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
    // 解析登录包
    let login = CALogin::from_slice(data)?;

    info!(
        "Login attempt: user={}, version={}",
        login.username, login.version
    );

    // 查询账户 —— 区分数据库错误和账号不存在
    let account = match self.db.get_account_by_userid(&login.username) {
        Ok(Some(account)) => account,
        Ok(None) => {
            warn!("Login failed: account not found, user={}", login.username);
            return Some(ACRefuseLogin { error_code: 0 }.to_packet());
        }
        Err(e) => {
            error!("Database error during login for user={}: {}", login.username, e);
            return Some(ACRefuseLogin { error_code: 0 }.to_packet());
        }
    };

    // 验证密码 (Argon2 哈希验证)
    if !crate::storage::password::verify_password(&login.password, &account.password_hash) {
        warn!("Login failed: invalid password for user={}", login.username);
        return Some(ACRefuseLogin { error_code: 0 }.to_packet());
    }

    // 检查账户状态
    if account.state != 0 {
        warn!(
            "Login failed: account banned or suspended, user={}",
            login.username
        );
        return Some(ACRefuseLogin { error_code: 3 }.to_packet());
    }

    // 更新最后登录时间
    if let Err(e) = self.db.update_last_login(account.account_id) {
        error!("Failed to update last_login: {}", e);
    }

    // 获取登录 ID
    let login_id1 = *self.login_id1.read();
    let login_id2 = *self.login_id2.read();

    // 更新 session
    session.account_id = Some(account.account_id);
    session.authenticated = true;

    info!(
        "Login success: account_id={}, user={}",
        account.account_id, login.username
    );

    // 返回成功响应
    Some(
        ACAceptLogin {
            account_id: account.account_id,
            login_id1,
            login_id2,
            sex: account.sex,
        }
        .to_packet(),
    )
}
```

- [ ] **Step 2: 运行现有测试确认无回归**

```bash
cargo test --lib game::login
```

- [ ] **Step 3: 提交**

```
fix(login): 修复账号不存在时客户端无响应的问题
```

---

## Task 2: 为 login.rs 添加测试

**Files:**
- Modify: `src/game/login.rs`

- [ ] **Step 1: 在 login.rs 底部添加测试模块**

在 `src/game/login.rs` 文件末尾添加 `#[cfg(test)] mod tests` 块，包含以下测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::login_packets::CALogin;
    use crate::protocol::packet_builder::Packed;
    use crate::storage::{init_schema, Database};

    /// 创建测试用的 LoginServer，内部包含内存数据库和已初始化的 schema
    fn create_test_server() -> (LoginServer, Arc<Database>) {
        let db = Arc::new(Database::open_memory().unwrap());
        init_schema(&db).unwrap();
        let session_manager = Arc::new(SessionManager::new());
        let server = LoginServer::new(db.clone(), session_manager);
        (server, db)
    }

    /// 构造一个合法的 CALogin 包的原始字节
    fn make_login_packet(username: &str, password: &str, version: u32) -> Vec<u8> {
        CALogin {
            version,
            username: username.to_string(),
            password: password.to_string(),
        }
        .to_packet()
    }

    #[test]
    fn test_login_success() {
        let (server, db) = create_test_server();
        // 创建测试账户，密码为 "password123"
        db.create_account("testuser", "password123", 1).unwrap();

        let packet = make_login_packet("testuser", "password123", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session);
        assert!(result.is_some(), "成功登录应返回响应包");

        let response = result.unwrap();
        // ACAceptLogin 包头: 2字节长度 + 2字节 packet_id (0x0069)
        assert_eq!(response.len(), 4 + 4 + 4 + 4 + 1, "ACAceptLogin 包长度应为 17 字节");
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x0069, "应返回 ACAceptLogin (0x0069)");

        // 验证 session 已更新
        assert!(session.authenticated, "登录成功后 session 应标记为已认证");
        assert!(session.account_id.is_some(), "登录成功后 session 应有 account_id");
    }

    #[test]
    fn test_login_account_not_found() {
        let (server, _db) = create_test_server();
        // 不创建任何账户

        let packet = make_login_packet("nonexistent", "password", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session);
        assert!(result.is_some(), "账号不存在时应返回拒绝包而非 None");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin (0x006A)");

        // 验证 session 未被修改
        assert!(!session.authenticated, "登录失败后 session 不应标记为已认证");
        assert!(session.account_id.is_none(), "登录失败后 session 不应有 account_id");
    }

    #[test]
    fn test_login_wrong_password() {
        let (server, db) = create_test_server();
        db.create_account("testuser", "correct_password", 1).unwrap();

        let packet = make_login_packet("testuser", "wrong_password", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session);
        assert!(result.is_some(), "密码错误时应返回拒绝包");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin (0x006A)");
        assert!(!session.authenticated, "密码错误后 session 不应标记为已认证");
    }

    #[test]
    fn test_login_banned_account() {
        let (server, db) = create_test_server();
        // 创建账户
        let account_id = db.create_account("banned_user", "password", 1).unwrap();
        // 将账户状态设为封禁 (state != 0)
        db.execute_with_params(
            "UPDATE accounts SET state = 5 WHERE account_id = ?1",
            rusqlite::params![account_id],
        ).unwrap();

        let packet = make_login_packet("banned_user", "password", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session);
        assert!(result.is_some(), "封禁账户应返回拒绝包");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin (0x006A)");
        // error_code = 3 表示封禁/暂停
        assert_eq!(response[4], 3, "封禁账户的 error_code 应为 3");
    }

    #[test]
    fn test_login_packet_dispatch() {
        let (server, db) = create_test_server();
        db.create_account("user", "pass", 0).unwrap();

        let packet = make_login_packet("user", "pass", 20);
        let mut session = Session::new();

        // 通过 handle_packet 分发
        let result = server.handle_packet(0x0064, &packet, &mut session);
        assert!(result.is_some(), "通过 handle_packet 分发应正常工作");

        // 未知包 ID
        let result = server.handle_packet(0xFFFF, &[], &mut session);
        assert!(result.is_none(), "未知包 ID 应返回 None");
    }

    #[test]
    fn test_login_truncated_packet() {
        let (server, _db) = create_test_server();
        let mut session = Session::new();

        // 发送截断的数据包（长度不足）
        let truncated = vec![0u8; 10];
        let result = server.handle_ca_login(&truncated, &mut session);
        assert!(result.is_none(), "截断的包应返回 None（CALogin::from_slice 失败）");
    }

    #[test]
    fn test_login_sets_login_ids() {
        let (server, db) = create_test_server();
        db.create_account("user", "pass", 0).unwrap();

        // 设置自定义 login_id
        server.set_login_ids(12345, 67890);

        let packet = make_login_packet("user", "pass", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session).unwrap();
        // ACAceptLogin 结构: [len:2][id:2][account_id:4][login_id1:4][login_id2:4][sex:1]
        let login_id1 = u32::from_le_bytes([result[6], result[7], result[8], result[9]]);
        let login_id2 = u32::from_le_bytes([result[10], result[11], result[12], result[13]]);
        assert_eq!(login_id1, 12345, "login_id1 应匹配设置的值");
        assert_eq!(login_id2, 67890, "login_id2 应匹配设置的值");
    }
}
```

- [ ] **Step 2: 运行测试确认全部通过**

```bash
cargo test --lib game::login::tests
```

- [ ] **Step 3: 提交**

```
test(login): 添加登录模块测试覆盖（成功/失败/封禁/截断等场景）
```

---

## Task 3: 修复 char.rs 测试 BUG

**问题分析:** `test_handle_make_char_success` 中使用 str=10 创建角色，超出 MAX_SINGLE_STAT=9（`src/game/constants.rs` 第 14 行），handler 会返回 `vec![0x00]`（失败）。测试断言 `assert_eq!(packet_data, vec![0])` 恰好匹配失败响应，所以测试通过但实际上测试的是"创建失败"而非"创建成功"。

**Files:**
- Modify: `src/game/char.rs`

- [ ] **Step 1: 修复 test_handle_make_char_success 测试**

将 `src/game/char.rs` 第 381-403 行的 `test_handle_make_char_success` 替换为：

```rust
#[test]
fn test_handle_make_char_success() {
    let server = create_test_server();
    let mut session = create_session_with_account(1);

    // 属性值在合法范围内 (1-9)，总和 <= 30
    let data = CHMakeChar {
        name: "NewChar".to_string(),
        str: 5,
        agi: 5,
        vit: 5,
        int: 5,
        dex: 5,
        luk: 5,
        hair_color: 0,
        hair: 1,
    }
    .to_packet();

    let result = server.handle_make_char(&data, &mut session);
    assert!(result.is_some());
    // 成功时返回 vec![0x01]
    let packet_data = result.unwrap();
    assert_eq!(packet_data, vec![0x01], "角色创建成功应返回 0x01");
}
```

- [ ] **Step 2: 运行测试确认通过**

```bash
cargo test --lib game::char::tests::test_handle_make_char_success
```

- [ ] **Step 3: 提交**

```
test(char): 修复 test_handle_make_char_success 测试使用合法属性值
```

---

## Task 4: 添加 Account 结构缺失字段并实现封禁过期检查

**问题分析:** 数据库 schema (`src/storage/schema.rs` 第 15-16 行) 中 `accounts` 表有 `unban_time` 和 `expiration_time` 字段，但 `Account` 结构体 (`src/storage/account.rs`) 未包含这两个字段，查询 SQL 也未读取它们。登录时无法检查封禁是否已过期。

**Files:**
- Modify: `src/storage/account.rs`

- [ ] **Step 1: 扩展 Account 结构体，添加缺失字段**

在 `src/storage/account.rs` 第 6-18 行，将 `Account` 结构体修改为：

```rust
#[derive(Debug, Clone)]
pub struct Account {
    pub account_id: u32,
    pub user_id: String,
    pub password_hash: String,
    pub sex: u8,
    pub email: Option<String>,
    pub group_id: i32,
    pub state: i32,
    pub unban_time: i64,
    pub expiration_time: i64,
    pub logcount: i32,
    pub last_login: Option<i64>,
    pub created_at: i64,
}
```

- [ ] **Step 2: 更新 get_account_by_userid 查询**

在 `src/storage/account.rs` 第 33-54 行，更新 SQL 和 row 映射：

```rust
pub fn get_account_by_userid(&self, user_id: &str) -> Result<Option<Account>> {
    self.query_row_optional(
        "SELECT account_id, user_id, password_hash, sex, email, group_id,
                state, unban_time, expiration_time, logcount, last_login, created_at
         FROM accounts WHERE user_id = ?1",
        params![user_id],
        |row| {
            Ok(Account {
                account_id: row.get(0)?,
                user_id: row.get(1)?,
                password_hash: row.get(2)?,
                sex: row.get(3)?,
                email: row.get(4)?,
                group_id: row.get(5)?,
                state: row.get(6)?,
                unban_time: row.get(7)?,
                expiration_time: row.get(8)?,
                logcount: row.get(9)?,
                last_login: row.get(10)?,
                created_at: row.get(11)?,
            })
        },
    )
}
```

- [ ] **Step 3: 更新 get_account_by_id 查询**

在 `src/storage/account.rs` 第 65-86 行，同样更新 SQL 和 row 映射：

```rust
pub fn get_account_by_id(&self, account_id: u32) -> Result<Option<Account>> {
    self.query_row_optional(
        "SELECT account_id, user_id, password_hash, sex, email, group_id,
                state, unban_time, expiration_time, logcount, last_login, created_at
         FROM accounts WHERE account_id = ?1",
        params![account_id],
        |row| {
            Ok(Account {
                account_id: row.get(0)?,
                user_id: row.get(1)?,
                password_hash: row.get(2)?,
                sex: row.get(3)?,
                email: row.get(4)?,
                group_id: row.get(5)?,
                state: row.get(6)?,
                unban_time: row.get(7)?,
                expiration_time: row.get(8)?,
                logcount: row.get(9)?,
                last_login: row.get(10)?,
                created_at: row.get(11)?,
            })
        },
    )
}
```

- [ ] **Step 4: 添加封禁过期检查方法**

在 `src/storage/account.rs` 的 `impl Database` 块末尾添加：

```rust
/// 检查封禁是否已过期，如果已过期则自动解除
pub fn check_and_clear_ban(&self, account: &mut Account) -> Result<bool> {
    if account.state == 0 {
        return Ok(true); // 未被封禁
    }

    let now = crate::storage::chrono_now();

    // 检查 unban_time：> 0 表示有时间限制的封禁
    if account.unban_time > 0 && now >= account.unban_time {
        // 封禁已过期，自动解除
        self.execute_with_params(
            "UPDATE accounts SET state = 0, unban_time = 0 WHERE account_id = ?1",
            params![account.account_id],
        )?;
        account.state = 0;
        account.unban_time = 0;
        tracing::info!("Account {} ban expired, auto-unbanned", account.account_id);
        return Ok(true);
    }

    // 检查 expiration_time：> 0 表示账号有过期时间
    if account.expiration_time > 0 && now >= account.expiration_time {
        // 账号已过期
        return Ok(false);
    }

    Ok(false) // 仍在封禁中
}
```

- [ ] **Step 5: 运行编译确认无错误**

```bash
cargo check
```

- [ ] **Step 6: 提交**

```
feat(storage): Account 结构体添加 unban_time/expiration_time 字段及封禁过期检查
```

---

## Task 5: 在 login.rs 中集成封禁过期检查

**Files:**
- Modify: `src/game/login.rs`

- [ ] **Step 1: 在 handle_ca_login 中添加封禁过期检查**

在 `src/game/login.rs` 的 `handle_ca_login` 方法中，在"检查账户状态"（`if account.state != 0`）之前插入封禁过期检查逻辑。将 Task 1 中已修改的"检查账户状态"部分替换为：

```rust
    // 检查封禁过期（自动解除已过期的封禁）
    let mut account = account; // 使 account 可变
    if account.state != 0 {
        let is_allowed = match self.db.check_and_clear_ban(&mut account) {
            Ok(allowed) => allowed,
            Err(e) => {
                error!("Failed to check ban status for user={}: {}", login.username, e);
                return Some(ACRefuseLogin { error_code: 0 }.to_packet());
            }
        };

        if !is_allowed {
            warn!(
                "Login failed: account banned or suspended, user={}",
                login.username
            );
            let error_code = if account.expiration_time > 0 { 5 } else { 3 };
            return Some(ACRefuseLogin { error_code }.to_packet());
        }
    }
```

注意：此步骤需要在 Task 1 完成后执行，因为它修改的是 Task 1 已修改的代码区域。完整的方法应为 Task 1 的代码 + 此步骤的替换。

- [ ] **Step 2: 添加封禁过期自动解除的测试**

在 `src/game/login.rs` 的测试模块中添加：

```rust
    #[test]
    fn test_login_ban_expired_auto_unban() {
        let (server, db) = create_test_server();
        let account_id = db.create_account("tempban", "pass", 1).unwrap();

        // 设置封禁状态，但 unban_time 为过去的时间
        let past_time = crate::storage::chrono_now() - 3600; // 1 小时前
        db.execute_with_params(
            "UPDATE accounts SET state = 5, unban_time = ?1 WHERE account_id = ?2",
            rusqlite::params![past_time, account_id],
        ).unwrap();

        let packet = make_login_packet("tempban", "pass", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session);
        assert!(result.is_some(), "封禁已过期时应允许登录");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x0069, "封禁已过期应返回 ACAceptLogin");
        assert!(session.authenticated, "封禁过期后应认证成功");
    }

    #[test]
    fn test_login_account_expired() {
        let (server, db) = create_test_server();
        let account_id = db.create_account("expired", "pass", 1).unwrap();

        // 设置账号过期时间为过去
        let past_time = crate::storage::chrono_now() - 3600;
        db.execute_with_params(
            "UPDATE accounts SET state = 5, expiration_time = ?1 WHERE account_id = ?2",
            rusqlite::params![past_time, account_id],
        ).unwrap();

        let packet = make_login_packet("expired", "pass", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet, &mut session);
        assert!(result.is_some(), "账号过期应返回拒绝包");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin");
        assert_eq!(response[4], 5, "账号过期的 error_code 应为 5");
    }
```

- [ ] **Step 3: 运行测试确认通过**

```bash
cargo test --lib game::login::tests
```

- [ ] **Step 4: 提交**

```
feat(login): 集成封禁过期检查，支持临时封禁自动解除和账号过期
```

---

## Task 6: 添加角色删除协议包定义

**Files:**
- Modify: `src/protocol/map_packets.rs`

- [ ] **Step 1: 添加 CHDeleteChar 和 HCDeleteCharOk 包定义**

在 `src/protocol/map_packets.rs` 的 `CHMakeChar` 结构体之后（约第 168 行后）添加：

```rust
/// 客户端请求删除角色 (0x0068)
#[derive(Debug, Clone)]
pub struct CHDeleteChar {
    pub char_id: u32,
    pub email: String,
}

impl Packed for CHDeleteChar {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0068)
            .put_u32(self.char_id)
            .put_fixed_str(&self.email, 40)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let mut offset = 4;
        let email = parse_fixed_string(slice, &mut offset, 40)?;
        Some(Self { char_id, email })
    }
}

/// 服务器确认角色删除已安排 (0x006C)
#[derive(Debug, Clone)]
pub struct HCDeleteCharOk {
    pub char_id: u32,
}

impl Packed for HCDeleteCharOk {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x006C).put_u32(self.char_id).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端取消角色删除 (0x01F8)
#[derive(Debug, Clone)]
pub struct CHCancelDelete {
    pub char_id: u32,
}

impl Packed for CHCancelDelete {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01F8).put_u32(self.char_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { char_id })
    }
}

/// 服务器确认取消删除 (0x006D)
#[derive(Debug, Clone)]
pub struct HCCancelDeleteOk {
    pub char_id: u32,
}

impl Packed for HCCancelDeleteOk {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x006D).put_u32(self.char_id).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}
```

- [ ] **Step 2: 添加包解析测试**

在 `src/protocol/map_packets.rs` 的测试模块中添加：

```rust
    #[test]
    fn test_ch_delete_char_parse() {
        let mut data = vec![0u8; 4 + 40];
        // char_id = 42
        data[0] = 42;
        // email = "test@email.com" + null padding
        let email_bytes = b"test@email.com";
        data[4..4 + email_bytes.len()].copy_from_slice(email_bytes);

        let pkt = CHDeleteChar::from_slice(&data).unwrap();
        assert_eq!(pkt.char_id, 42);
        assert_eq!(pkt.email, "test@email.com");
    }

    #[test]
    fn test_ch_delete_char_truncated() {
        let data = vec![0u8; 2];
        assert!(CHDeleteChar::from_slice(&data).is_none());
    }

    #[test]
    fn test_ch_cancel_delete_parse() {
        let data = vec![100, 0, 0, 0]; // char_id = 100
        let pkt = CHCancelDelete::from_slice(&data).unwrap();
        assert_eq!(pkt.char_id, 100);
    }

    #[test]
    fn test_ch_cancel_delete_truncated() {
        let data = vec![0u8; 2];
        assert!(CHCancelDelete::from_slice(&data).is_none());
    }
```

- [ ] **Step 3: 运行测试确认通过**

```bash
cargo test --lib protocol::map_packets::tests
```

- [ ] **Step 4: 提交**

```
feat(protocol): 添加角色删除(0x0068)和取消删除(0x01F8)协议包定义
```

---

## Task 7: 添加数据库层角色删除/取消删除方法

**Files:**
- Modify: `src/storage/character.rs`

- [ ] **Step 1: 添加设置删除定时器方法**

在 `src/storage/character.rs` 的 `impl Database` 块中（`cleanup_deleted_characters` 方法之后），添加：

```rust
    /// 设置角色删除定时器（标记删除）
    /// delete_after_secs: 从现在起多少秒后删除（rAthena 默认 86400 = 24小时）
    pub fn mark_character_for_deletion(
        &self,
        char_id: u32,
        account_id: u32,
        delete_after_secs: i64,
    ) -> Result<bool> {
        let now = chrono_now();
        let delete_timer = (now + delete_after_secs) as u32;

        // 验证角色属于该账户
        let affected = self.execute_with_params(
            "UPDATE characters SET delete_timer = ?1
             WHERE char_id = ?2 AND account_id = ?3 AND (delete_timer = 0 OR delete_timer IS NULL)",
            params![delete_timer, char_id, account_id],
        )?;

        if affected > 0 {
            tracing::info!(
                "Character {} marked for deletion at {} (in {} seconds)",
                char_id, delete_timer, delete_after_secs
            );
            Ok(true)
        } else {
            tracing::warn!(
                "Failed to mark character {} for deletion (not found or already marked)",
                char_id
            );
            Ok(false)
        }
    }

    /// 取消角色删除标记
    pub fn cancel_character_deletion(
        &self,
        char_id: u32,
        account_id: u32,
    ) -> Result<bool> {
        let affected = self.execute_with_params(
            "UPDATE characters SET delete_timer = 0
             WHERE char_id = ?1 AND account_id = ?2 AND delete_timer > 0",
            params![char_id, account_id],
        )?;

        if affected > 0 {
            tracing::info!("Character {} deletion cancelled", char_id);
            Ok(true)
        } else {
            tracing::warn!(
                "Failed to cancel deletion for character {} (not found or not marked)",
                char_id
            );
            Ok(false)
        }
    }

    /// 按名称查找角色（用于名称重复检查）
    pub fn get_character_by_name(&self, name: &str) -> Result<Option<Character>> {
        self.query_row_optional(
            "SELECT char_id, char_num, name, class, base_level, job_level,
                    base_exp, job_exp, zeny, str, agi, vit, int, dex, luk,
                    hp, max_hp, sp, max_sp,
                    hair, hair_color, clothes_color,
                    weapon, shield, head_top, head_mid, head_bottom,
                    last_map, last_x, last_y, save_map, save_x, save_y,
                    delete_timer, created_at, updated_at
             FROM characters WHERE name = ?1",
            params![name],
            |row| {
                Ok(Character {
                    char_id: row.get(0)?,
                    char_num: row.get(1)?,
                    name: row.get(2)?,
                    class: row.get(3)?,
                    base_level: row.get(4)?,
                    job_level: row.get(5)?,
                    base_exp: row.get(6)?,
                    job_exp: row.get(7)?,
                    zeny: row.get(8)?,
                    str: row.get(9)?,
                    agi: row.get(10)?,
                    vit: row.get(11)?,
                    int: row.get(12)?,
                    dex: row.get(13)?,
                    luk: row.get(14)?,
                    hp: row.get(15)?,
                    max_hp: row.get(16)?,
                    sp: row.get(17)?,
                    max_sp: row.get(18)?,
                    hair: row.get(19)?,
                    hair_color: row.get(20)?,
                    clothes_color: row.get(21)?,
                    weapon: row.get(22)?,
                    shield: row.get(23)?,
                    head_top: row.get(24)?,
                    head_mid: row.get(25)?,
                    head_bottom: row.get(26)?,
                    last_map: row.get(27)?,
                    last_x: row.get(28)?,
                    last_y: row.get(29)?,
                    save_map: row.get(30)?,
                    save_x: row.get(31)?,
                    save_y: row.get(32)?,
                    delete_timer: row.get(33)?,
                    created_at: row.get(34)?,
                    updated_at: row.get(35)?,
                })
            },
        )
    }
```

- [ ] **Step 2: 添加数据库层测试**

在 `src/storage/character.rs` 的测试模块中添加：

```rust
    #[test]
    fn test_mark_and_cancel_character_deletion() {
        let db = Database::open_memory().unwrap();
        init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        let char_id = db
            .create_character(account_id, 0, "DeleteMe", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 标记删除（24小时后）
        let result = db.mark_character_for_deletion(char_id, account_id, 86400).unwrap();
        assert!(result, "标记删除应成功");

        // 验证 delete_timer 已设置
        let char = db.get_character_by_id(char_id).unwrap().unwrap();
        assert!(char.delete_timer > 0, "delete_timer 应大于 0");

        // 取消删除
        let result = db.cancel_character_deletion(char_id, account_id).unwrap();
        assert!(result, "取消删除应成功");

        // 验证 delete_timer 已重置
        let char = db.get_character_by_id(char_id).unwrap().unwrap();
        assert_eq!(char.delete_timer, 0, "delete_timer 应重置为 0");
    }

    #[test]
    fn test_mark_deletion_wrong_account() {
        let db = Database::open_memory().unwrap();
        init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        let char_id = db
            .create_character(account_id, 0, "MyChar", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 用错误的 account_id 尝试标记删除
        let result = db.mark_character_for_deletion(char_id, 9999, 86400).unwrap();
        assert!(!result, "错误账户标记删除应失败");

        // 验证角色未被标记
        let char = db.get_character_by_id(char_id).unwrap().unwrap();
        assert_eq!(char.delete_timer, 0, "角色不应被错误账户标记删除");
    }

    #[test]
    fn test_get_character_by_name() {
        let db = Database::open_memory().unwrap();
        init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        db.create_character(account_id, 0, "UniqueName", 5, 5, 5, 5, 5, 5, 1, 0).unwrap();

        // 查找存在的角色
        let result = db.get_character_by_name("UniqueName").unwrap();
        assert!(result.is_some(), "应能找到已创建的角色");
        assert_eq!(result.unwrap().name, "UniqueName");

        // 查找不存在的角色
        let result = db.get_character_by_name("NonExistent").unwrap();
        assert!(result.is_none(), "不存在的角色应返回 None");
    }

    #[test]
    fn test_double_mark_deletion_fails() {
        let db = Database::open_memory().unwrap();
        init_schema(&db).unwrap();
        let account_id = setup_test_account(&db);

        let char_id = db
            .create_character(account_id, 0, "DoubleDel", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 第一次标记成功
        let result = db.mark_character_for_deletion(char_id, account_id, 86400).unwrap();
        assert!(result);

        // 第二次标记应失败（已标记）
        let result = db.mark_character_for_deletion(char_id, account_id, 86400).unwrap();
        assert!(!result, "重复标记删除应返回 false");
    }
```

- [ ] **Step 3: 运行测试确认通过**

```bash
cargo test --lib storage::character::tests
```

- [ ] **Step 4: 提交**

```
feat(storage): 添加角色删除/取消删除/名称查询数据库方法
```

---

## Task 8: 在 CharServer 中实现角色删除和取消删除 handler

**Files:**
- Modify: `src/game/char.rs`

- [ ] **Step 1: 在 handle_packet 中注册新包 ID**

在 `src/game/char.rs` 第 62-69 行的 `handle_packet` 方法中，添加 0x0068 和 0x01F8 的路由：

```rust
    pub fn handle_packet(
        &self,
        packet_id: u16,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        match packet_id {
            0x0066 => self.handle_request_char_list(session),
            0x0067 => self.handle_make_char(data, session),
            0x0065 => self.handle_select_char(data, session),
            0x0068 => self.handle_delete_char(data, session),
            0x01F8 => self.handle_cancel_delete(data, session),
            _ => {
                warn!("Unknown char packet id: 0x{:04X}", packet_id);
                None
            }
        }
    }
```

- [ ] **Step 2: 添加 import**

在 `src/game/char.rs` 第 9 行的 import 中添加新包类型：

```rust
use crate::protocol::map_packets::{
    CHEnter, CHDeleteChar, CHCancelDelete, CHMakeChar, CharInfo,
    HCNotifyZoneServer, HCDeleteCharOk, HCCancelDeleteOk, SCCharList,
};
```

- [ ] **Step 3: 添加 handle_delete_char 方法**

在 `handle_select_char` 方法之后（约第 242 行后）添加：

```rust
    /// 处理请求删除角色 (0x0068)
    /// rAthena 协议：发送 delete_timer 标记，客户端收到后显示倒计时
    /// 默认删除延迟 86400 秒 (24 小时)
    fn handle_delete_char(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        let delete_req = CHDeleteChar::from_slice(data)?;

        info!(
            "Delete char request: char_id={}, account_id={}",
            delete_req.char_id, account_id
        );

        // 验证角色是否属于该账户
        let characters = self.db.get_characters_by_account(account_id).ok()?;
        let char_exists = characters.iter().any(|c| c.char_id == delete_req.char_id);

        if !char_exists {
            warn!(
                "Delete char rejected: char_id={} not owned by account_id={}",
                delete_req.char_id, account_id
            );
            return Some(vec![0x00]); // 失败响应
        }

        // 标记角色删除（24小时后删除）
        match self.db.mark_character_for_deletion(delete_req.char_id, account_id, 86400) {
            Ok(true) => {
                info!(
                    "Character {} marked for deletion (account_id={})",
                    delete_req.char_id, account_id
                );
                Some(
                    HCDeleteCharOk {
                        char_id: delete_req.char_id,
                    }
                    .to_packet(),
                )
            }
            Ok(false) => {
                warn!(
                    "Failed to mark character {} for deletion (already marked or not found)",
                    delete_req.char_id
                );
                Some(vec![0x00]) // 失败响应
            }
            Err(e) => {
                error!("Database error deleting character {}: {}", delete_req.char_id, e);
                Some(vec![0x00]) // 失败响应
            }
        }
    }
```

- [ ] **Step 4: 添加 handle_cancel_delete 方法**

在 `handle_delete_char` 方法之后添加：

```rust
    /// 处理取消删除角色 (0x01F8)
    fn handle_cancel_delete(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        let cancel_req = CHCancelDelete::from_slice(data)?;

        info!(
            "Cancel delete request: char_id={}, account_id={}",
            cancel_req.char_id, account_id
        );

        // 验证角色是否属于该账户
        let characters = self.db.get_characters_by_account(account_id).ok()?;
        let char_exists = characters.iter().any(|c| c.char_id == cancel_req.char_id);

        if !char_exists {
            warn!(
                "Cancel delete rejected: char_id={} not owned by account_id={}",
                cancel_req.char_id, account_id
            );
            return Some(vec![0x00]); // 失败响应
        }

        match self.db.cancel_character_deletion(cancel_req.char_id, account_id) {
            Ok(true) => {
                info!(
                    "Character {} deletion cancelled (account_id={})",
                    cancel_req.char_id, account_id
                );
                Some(
                    HCCancelDeleteOk {
                        char_id: cancel_req.char_id,
                    }
                    .to_packet(),
                )
            }
            Ok(false) => {
                warn!(
                    "Failed to cancel deletion for character {} (not marked)",
                    cancel_req.char_id
                );
                Some(vec![0x00]) // 失败响应
            }
            Err(e) => {
                error!(
                    "Database error cancelling deletion for {}: {}",
                    cancel_req.char_id, e
                );
                Some(vec![0x00]) // 失败响应
            }
        }
    }
```

- [ ] **Step 5: 添加删除/取消删除测试**

在 `src/game/char.rs` 的测试模块中添加 import 和测试：

首先更新测试模块中的 import：

```rust
    use crate::protocol::map_packets::{CHEnter, CHDeleteChar, CHCancelDelete, CHMakeChar};
```

然后添加测试：

```rust
    #[test]
    fn test_handle_delete_char_success() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 先创建一个角色
        let char_id = server
            .db
            .create_character(1, 0, "DeleteMe", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 请求删除
        let data = CHDeleteChar {
            char_id,
            email: String::new(),
        }
        .to_packet();

        let result = server.handle_delete_char(&data, &mut session);
        assert!(result.is_some(), "删除请求应返回响应");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006C, "应返回 HCDeleteCharOk (0x006C)");
    }

    #[test]
    fn test_handle_delete_char_wrong_account() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 创建角色属于 account 1
        let char_id = server
            .db
            .create_character(1, 0, "Owned", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 用 account 2 的 session 尝试删除
        let mut wrong_session = create_session_with_account(2);
        // account 2 也需要存在
        server.db.create_account("other", "pass", 0).unwrap();

        let data = CHDeleteChar {
            char_id,
            email: String::new(),
        }
        .to_packet();

        let result = server.handle_delete_char(&data, &mut wrong_session);
        assert!(result.is_some(), "错误账户删除应返回失败响应");
        assert_eq!(result.unwrap(), vec![0x00], "应返回失败字节");
    }

    #[test]
    fn test_handle_cancel_delete_success() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 创建角色
        let char_id = server
            .db
            .create_character(1, 0, "CancelDel", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 先标记删除
        server.db.mark_character_for_deletion(char_id, 1, 86400).unwrap();

        // 取消删除
        let data = CHCancelDelete { char_id }.to_packet();
        let result = server.handle_cancel_delete(&data, &mut session);
        assert!(result.is_some(), "取消删除应返回响应");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006D, "应返回 HCCancelDeleteOk (0x006D)");
    }

    #[test]
    fn test_handle_cancel_delete_not_marked() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 创建角色但不标记删除
        let char_id = server
            .db
            .create_character(1, 0, "NoMark", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        let data = CHCancelDelete { char_id }.to_packet();
        let result = server.handle_cancel_delete(&data, &mut session);
        assert!(result.is_some(), "未标记删除时取消应返回失败响应");
        assert_eq!(result.unwrap(), vec![0x00], "应返回失败字节");
    }

    #[test]
    fn test_delete_char_requires_account_id() {
        let server = create_test_server();
        let mut session = Session::new(); // 无 account_id

        let data = CHDeleteChar {
            char_id: 1,
            email: String::new(),
        }
        .to_packet();

        let result = server.handle_delete_char(&data, &mut session);
        assert!(result.is_none(), "无 account_id 时应返回 None");
    }

    #[test]
    fn test_cancel_delete_requires_account_id() {
        let server = create_test_server();
        let mut session = Session::new();

        let data = CHCancelDelete { char_id: 1 }.to_packet();
        let result = server.handle_cancel_delete(&data, &mut session);
        assert!(result.is_none(), "无 account_id 时应返回 None");
    }
```

- [ ] **Step 6: 运行测试确认通过**

```bash
cargo test --lib game::char::tests
```

- [ ] **Step 7: 提交**

```
feat(char): 实现角色删除(0x0068)和取消删除(0x01F8)处理逻辑
```

---

## Task 9: 添加角色名验证（重复检查 + 特殊字符过滤）

**Files:**
- Modify: `src/game/char.rs`

- [ ] **Step 1: 添加名称验证辅助方法**

在 `src/game/char.rs` 的 `CharServer` 的 `impl` 块中（`find_empty_slot` 方法之后），添加：

```rust
    /// 验证角色名称是否合法
    /// 返回 Ok(()) 表示合法，Err(String) 包含错误信息
    fn validate_character_name(&self, name: &str) -> Result<(), String> {
        // 检查名称长度
        let trimmed = name.trim_matches('\0');
        if trimmed.is_empty() {
            return Err("角色名不能为空".to_string());
        }
        if trimmed.len() > 24 {
            return Err("角色名过长（最大24字节）".to_string());
        }

        // 检查最小长度（rAthena 最少 4 个字符）
        if trimmed.len() < 4 {
            return Err("角色名过短（最少4字节）".to_string());
        }

        // 检查特殊字符：只允许字母、数字、中文、韩文、日文等
        // rAthena 默认不允许以下字符
        for ch in trimmed.chars() {
            match ch {
                // 允许：英文字母、数字、中文(CJK统一汉字)、韩文、日文平假名/片假名
                'a'..='z' | 'A'..='Z' | '0'..='9' => {}
                '\u{4E00}'..='\u{9FFF}'   // CJK统一汉字
                | '\u{3400}'..='\u{4DBF}' // CJK扩展A
                | '\u{AC00}'..='\u{D7AF}' // 韩文音节
                | '\u{3040}'..='\u{309F}' // 日文平假名
                | '\u{30A0}'..='\u{30FF}' // 日文片假名
                | '\u{FF66}'..='\u{FF9F}' // 半角片假名
                => {}
                _ => {
                    return Err(format!("角色名包含不允许的字符: '{}'", ch));
                }
            }
        }

        // 检查名称是否已存在（含已标记删除但未过期的角色）
        match self.db.get_character_by_name(trimmed) {
            Ok(Some(_)) => {
                // 检查该角色是否已标记删除且已过期
                // 如果已过期，cleanup 会清理掉，此处当作已存在
                Err("角色名已被使用".to_string())
            }
            Ok(None) => Ok(()),
            Err(e) => {
                error!("Database error checking name: {}", e);
                Err("服务器内部错误".to_string())
            }
        }
    }
```

- [ ] **Step 2: 在 handle_make_char 中集成名称验证**

在 `src/game/char.rs` 的 `handle_make_char` 方法中，在"校验名称长度"部分（约第 155-163 行）之后、"创建角色"部分之前，插入名称验证调用。替换现有的名称长度检查为：

```rust
        // 校验角色名称（长度 + 特殊字符 + 重复检查）
        if let Err(err_msg) = self.validate_character_name(&make_char.name) {
            warn!(
                "Character creation rejected: {} (account_id={})",
                err_msg, account_id
            );
            return Some(vec![0x00]);
        }
        let name = make_char.name.trim_matches('\0');
```

注意：移除原有的 `name.is_empty() || name.len() > 24` 检查，因为 `validate_character_name` 已包含。

- [ ] **Step 3: 添加名称验证测试**

在 `src/game/char.rs` 的测试模块中添加：

```rust
    #[test]
    fn test_handle_make_char_duplicate_name() {
        let server = create_test_server();
        let mut session1 = create_session_with_account(1);

        // 创建第一个角色
        let data1 = CHMakeChar {
            name: "TakenName".to_string(),
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();
        let result1 = server.handle_make_char(&data1, &mut session1);
        assert_eq!(result1.unwrap(), vec![0x01], "第一个角色应创建成功");

        // 尝试创建同名角色
        let mut session2 = create_session_with_account(1);
        let data2 = CHMakeChar {
            name: "TakenName".to_string(),
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();
        let result2 = server.handle_make_char(&data2, &mut session2);
        assert_eq!(result2.unwrap(), vec![0x00], "重复名称应返回失败");
    }

    #[test]
    fn test_handle_make_char_name_too_short() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHMakeChar {
            name: "Ab".to_string(), // 只有 2 字节，少于最少 4 字节
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data, &mut session);
        assert_eq!(result.unwrap(), vec![0x00], "过短名称应返回失败");
    }

    #[test]
    fn test_handle_make_char_special_characters() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHMakeChar {
            name: "Test@#$".to_string(), // 包含特殊字符
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data, &mut session);
        assert_eq!(result.unwrap(), vec![0x00], "含特殊字符的名称应返回失败");
    }

    #[test]
    fn test_handle_make_char_chinese_name_allowed() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHMakeChar {
            name: "测试角色".to_string(), // 中文名称应被允许
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data, &mut session);
        assert_eq!(result.unwrap(), vec![0x01], "中文名称应创建成功");
    }

    #[test]
    fn test_validate_name_boundary() {
        let server = create_test_server();

        // 空名称
        assert!(server.validate_character_name("").is_err());
        assert!(server.validate_character_name("\0\0\0").is_err());

        // 3 字节（少于 4）
        assert!(server.validate_character_name("abc").is_err());

        // 4 字节（刚好）
        assert!(server.validate_character_name("abcd").is_ok());

        // 24 字节（刚好）
        let name_24 = "a".repeat(24);
        assert!(server.validate_character_name(&name_24).is_ok());

        // 25 字节（超出）
        let name_25 = "a".repeat(25);
        assert!(server.validate_character_name(&name_25).is_err());
    }
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --lib game::char::tests
```

- [ ] **Step 5: 提交**

```
feat(char): 添加角色名验证（重复检查、特殊字符过滤、长度校验）
```

---

## Task 10: 最终集成验证

**Files:** 无新文件（仅运行测试）

- [ ] **Step 1: 运行全量测试**

```bash
cargo test --lib 2>&1 | tail -20
```

- [ ] **Step 2: 确认编译无警告**

```bash
cargo check 2>&1 | grep -E "warning|error" | head -20
```

- [ ] **Step 3: 运行 login 和 char 模块的完整测试**

```bash
cargo test --lib game::login::tests -- --nocapture
cargo test --lib game::char::tests -- --nocapture
cargo test --lib storage::character::tests -- --nocapture
cargo test --lib protocol::map_packets::tests -- --nocapture
```

- [ ] **Step 4: 如有测试失败，修复后重新运行**

---

## 总结

| Task | 描述 | 涉及文件 |
|------|------|----------|
| 1 | 修复 login.rs 账号不存在无响应 BUG | `src/game/login.rs` |
| 2 | 添加 login.rs 测试 | `src/game/login.rs` |
| 3 | 修复 char.rs 测试 BUG | `src/game/char.rs` |
| 4 | Account 结构体添加封禁字段 + 过期检查 | `src/storage/account.rs` |
| 5 | login.rs 集成封禁过期检查 | `src/game/login.rs` |
| 6 | 添加角色删除协议包定义 | `src/protocol/map_packets.rs` |
| 7 | 数据库层角色删除/取消删除方法 | `src/storage/character.rs` |
| 8 | CharServer 角色删除/取消删除 handler | `src/game/char.rs` |
| 9 | 角色名验证（重复+特殊字符+长度） | `src/game/char.rs` |
| 10 | 最终集成验证 | - |

**任务依赖关系:**
- Task 1 -> Task 2（先修复 BUG，再加测试）
- Task 4 -> Task 5（先扩展 Account 结构，再集成到 login）
- Task 6 -> Task 7 -> Task 8（协议包 -> 数据库方法 -> handler 实现，顺序依赖）
- Task 3 和 Task 9 独立，可与其他任务并行
- Task 10 必须在所有任务完成后执行
