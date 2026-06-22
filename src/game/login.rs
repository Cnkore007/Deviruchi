//! 登录服务器业务逻辑

use std::sync::Arc;
use tracing::{error, info, warn};

use crate::game::ipban::IpBanManager;
use crate::network::session::{Session, SessionManager};
use crate::protocol::login_packets::{
    ACAceptLogin20220406, ACRefuseLogin, CALogin, CharacterServerEntry,
};
use crate::protocol::packet_builder::Packed;
use crate::storage::Database;

/// Char Server 连接信息
#[derive(Debug, Clone)]
pub struct CharServerInfo {
    pub ip: u32,
    pub port: u16,
    pub name: String,
}

/// 登录服务器
#[allow(dead_code)]
pub struct LoginServer {
    db: Arc<Database>,
    session_manager: Arc<SessionManager>,
    char_server: CharServerInfo,
    /// IP 封禁管理器，用于暴力破解防护
    ip_ban_manager: Arc<IpBanManager>,
}

impl LoginServer {
    pub fn new(db: Arc<Database>, session_manager: Arc<SessionManager>) -> Self {
        Self {
            db,
            session_manager,
            char_server: CharServerInfo {
                ip: 0x7F000001, // 127.0.0.1
                port: 6000,
                name: "Deviruchi".to_string(),
            },
            ip_ban_manager: Arc::new(IpBanManager::default()),
        }
    }

    /// 设置 IP 封禁管理器
    pub fn with_ip_ban_manager(mut self, manager: Arc<IpBanManager>) -> Self {
        self.ip_ban_manager = manager;
        self
    }

    /// 设置 Char Server 连接信息
    pub fn with_char_server(mut self, info: CharServerInfo) -> Self {
        self.char_server = info;
        self
    }

    /// 根据 packet_id 分发处理
    pub fn handle_packet(
        &self,
        packet_id: u16,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        match packet_id {
            0x0064 => self.handle_ca_login(data, session),
            _ => {
                warn!("Unknown packet id: 0x{:04X}", packet_id);
                None
            }
        }
    }

    /// 处理 CALogin (0x0064)
    fn handle_ca_login(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        // IP 封禁检查：拦截被封禁 IP 的登录请求
        if let Some(ref addr) = session.client_addr
            && self.ip_ban_manager.is_banned(addr)
        {
            warn!("Login rejected: IP {} is banned", addr);
            return Some(ACRefuseLogin { error_code: 0 }.to_packet());
        }

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
                error!(
                    "Database error during login for user={}: {}",
                    login.username, e
                );
                return Some(ACRefuseLogin { error_code: 0 }.to_packet());
            }
        };

        // 验证密码 (Argon2 哈希验证)
        if !crate::storage::password::verify_password(&login.password, &account.password_hash) {
            warn!("Login failed: invalid password for user={}", login.username);
            // 暴力破解检测：记录失败尝试，超过阈值自动封禁 IP
            if let Some(ref addr) = session.client_addr
                && self.ip_ban_manager.record_attempt(addr)
            {
                error!("IP {} auto-banned due to brute force attempts", addr);
            }
            return Some(ACRefuseLogin { error_code: 0 }.to_packet());
        }

        // 登录成功，重置该 IP 的失败计数
        if let Some(ref addr) = session.client_addr {
            self.ip_ban_manager.reset_attempts(addr);
        }

        // 检查封禁过期（自动解除已过期的封禁）
        let mut account = account; // 使 account 可变
        if account.state != 0 {
            let is_allowed = match self.db.check_and_clear_ban(&mut account) {
                Ok(allowed) => allowed,
                Err(e) => {
                    error!(
                        "Failed to check ban status for user={}: {}",
                        login.username, e
                    );
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

        // 更新最后登录时间
        if let Err(e) = self.db.update_last_login(account.account_id) {
            error!("Failed to update last_login: {}", e);
        }

        // 生成每会话唯一的 login_id
        let login_id1 = rand::random::<u32>();
        let login_id2 = rand::random::<u32>();

        // 更新 session
        session.account_id = Some(account.account_id);
        session.authenticated = true;
        session.login_id1 = login_id1;
        session.login_id2 = login_id2;

        info!(
            "Login success: account_id={}, user={}",
            account.account_id, login.username
        );

        // 返回成功响应（包含 char server 地址）
        // 默认使用 PACKETVER 20220406 格式 (0x0AC4)
        let mut name = [0u8; 24];
        let name_bytes = self.char_server.name.as_bytes();
        let copy_len = name_bytes.len().min(24);
        name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

        let char_ip = self.char_server.ip.to_be_bytes();
        Some(
            ACAceptLogin20220406 {
                login_id1,
                account_id: account.account_id,
                login_id2,
                ip_address: 0,
                name,
                unknown: 0,
                sex: account.sex,
                auth_token: [0u8; 17],
                servers: vec![CharacterServerEntry {
                    ip: char_ip,
                    port: self.char_server.port,
                    name: self.char_server.name.clone(),
                    user_count: 0,
                    server_type: 0,
                    display_new: 0,
                }],
            }
            .to_packet(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::login_packets::CALogin;
    use crate::protocol::packet_builder::Packed;
    use crate::storage::{Database, init_schema};

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

        let result = server.handle_ca_login(&packet[4..], &mut session);
        assert!(result.is_some(), "成功登录应返回响应包");

        let response = result.unwrap();
        // PACKETVER 20220406 使用 0x0AC4，含固定头 + 1 个 char server 条目
        // len(2) + id(2) + login_id1(4) + account_id(4) + login_id2(4) + ip(4) + name(24) + unknown(2) + sex(1) + auth_token(17) + server(156)
        assert_eq!(response.len(), 224, "ACAceptLogin20220406 包长度应为 224 字节");
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x0AC4, "应返回 ACAceptLogin20220406 (0x0AC4)");

        // 验证 session 已更新
        assert!(session.authenticated, "登录成功后 session 应标记为已认证");
        assert!(
            session.account_id.is_some(),
            "登录成功后 session 应有 account_id"
        );
    }

    #[test]
    fn test_login_account_not_found() {
        let (server, _db) = create_test_server();
        // 不创建任何账户

        let packet = make_login_packet("nonexistent", "password", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet[4..], &mut session);
        assert!(result.is_some(), "账号不存在时应返回拒绝包而非 None");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin (0x006A)");

        // 验证 session 未被修改
        assert!(
            !session.authenticated,
            "登录失败后 session 不应标记为已认证"
        );
        assert!(
            session.account_id.is_none(),
            "登录失败后 session 不应有 account_id"
        );
    }

    #[test]
    fn test_login_wrong_password() {
        let (server, db) = create_test_server();
        db.create_account("testuser", "correct_password", 1)
            .unwrap();

        let packet = make_login_packet("testuser", "wrong_password", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet[4..], &mut session);
        assert!(result.is_some(), "密码错误时应返回拒绝包");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin (0x006A)");
        assert!(
            !session.authenticated,
            "密码错误后 session 不应标记为已认证"
        );
    }

    #[test]
    fn test_login_banned_account() {
        let (server, db) = create_test_server();
        // 创建账户
        let account_id = db.create_account("banned_user", "password", 1).unwrap();
        // 将账户状态设为封禁 (state != 0)
        db.execute_params(
            "UPDATE accounts SET state = 5 WHERE account_id = ?1",
            &[&(account_id as i32) as &dyn crate::storage::backend::IntoValue],
        )
        .unwrap();

        let packet = make_login_packet("banned_user", "password", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet[4..], &mut session);
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
        let result = server.handle_packet(0x0064, &packet[4..], &mut session);
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
        assert!(
            result.is_none(),
            "截断的包应返回 None（CALogin::from_slice 失败）"
        );
    }

    #[test]
    fn test_login_sets_login_ids() {
        let (server, db) = create_test_server();
        db.create_account("user", "pass", 0).unwrap();

        let packet = make_login_packet("user", "pass", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet[4..], &mut session).unwrap();
        // ACAceptLogin20220406 结构: [len:2LE][id:2LE][login_id1:4LE][account_id:4LE][login_id2:4LE]...
        let login_id1 = u32::from_le_bytes([result[4], result[5], result[6], result[7]]);
        let login_id2 = u32::from_le_bytes([result[12], result[13], result[14], result[15]]);
        // 验证 login_id 被正确设置到 session 中，且与响应包一致
        assert_eq!(
            session.login_id1, login_id1,
            "session.login_id1 应与响应包一致"
        );
        assert_eq!(
            session.login_id2, login_id2,
            "session.login_id2 应与响应包一致"
        );
        // login_id 应为非零随机值（极小概率为 0，可接受）
        assert!(
            session.login_id1 != 0 || session.login_id2 != 0,
            "login_id 不应全为 0"
        );
    }

    #[test]
    fn test_login_ban_expired_auto_unban() {
        let (server, db) = create_test_server();
        let account_id = db.create_account("tempban", "pass", 1).unwrap();

        // 设置封禁状态，但 unban_time 为过去的时间
        let past_time = crate::storage::chrono_now() - 3600; // 1 小时前
        db.execute_params(
            "UPDATE accounts SET state = 5, unban_time = ?1 WHERE account_id = ?2",
            &[
                &past_time as &dyn crate::storage::backend::IntoValue,
                &(account_id as i32) as &dyn crate::storage::backend::IntoValue,
            ],
        )
        .unwrap();

        let packet = make_login_packet("tempban", "pass", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet[4..], &mut session);
        assert!(result.is_some(), "封禁已过期时应允许登录");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x0AC4, "封禁已过期应返回 ACAceptLogin20220406");
        assert!(session.authenticated, "封禁过期后应认证成功");
    }

    #[test]
    fn test_login_account_expired() {
        let (server, db) = create_test_server();
        let account_id = db.create_account("expired", "pass", 1).unwrap();

        // 设置账号过期时间为过去
        let past_time = crate::storage::chrono_now() - 3600;
        db.execute_params(
            "UPDATE accounts SET state = 5, expiration_time = ?1 WHERE account_id = ?2",
            &[
                &past_time as &dyn crate::storage::backend::IntoValue,
                &(account_id as i32) as &dyn crate::storage::backend::IntoValue,
            ],
        )
        .unwrap();

        let packet = make_login_packet("expired", "pass", 20);
        let mut session = Session::new();

        let result = server.handle_ca_login(&packet[4..], &mut session);
        assert!(result.is_some(), "账号过期应返回拒绝包");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006A, "应返回 ACRefuseLogin");
        assert_eq!(response[4], 5, "账号过期的 error_code 应为 5");
    }
}
