//! 登录服务器业务逻辑

use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, error};

use crate::protocol::login_packets::{CALogin, ACAceptLogin, ACRefuseLogin};
use crate::protocol::packet_builder::Packed;
use crate::storage::Database;
use crate::network::session::{SessionManager, Session};

/// 登录服务器
pub struct LoginServer {
    db: Arc<Database>,
    session_manager: Arc<SessionManager>,
    login_id1: Arc<RwLock<u32>>,
    login_id2: Arc<RwLock<u32>>,
}

impl LoginServer {
    pub fn new(db: Arc<Database>, session_manager: Arc<SessionManager>) -> Self {
        Self {
            db,
            session_manager,
            login_id1: Arc::new(RwLock::new(0)),
            login_id2: Arc::new(RwLock::new(0)),
        }
    }

    /// 设置登录 ID
    pub fn set_login_ids(&self, id1: u32, id2: u32) {
        *self.login_id1.write() = id1;
        *self.login_id2.write() = id2;
    }

    /// 根据 packet_id 分发处理
    pub fn handle_packet(&self, packet_id: u16, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
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
        // 解析登录包
        let login = CALogin::from_slice(data)?;

        info!(
            "Login attempt: user={}, version={}",
            login.username, login.version
        );

        // 查询账户
        let account = self.db.get_account_by_userid(&login.username).ok()??;

        // 验证密码 (简单比较)
        if login.password != account.password_hash {
            warn!("Login failed: invalid password for user={}", login.username);
            return Some(ACRefuseLogin { error_code: 0 }.to_packet());
        }

        // 检查账户状态
        if account.state != 0 {
            warn!("Login failed: account banned or suspended, user={}", login.username);
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
}
