/// 登录请求包
#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应包
#[derive(Debug, Clone)]
pub enum LoginResponse {
    /// 登录成功，返回 login_id、account_id 和 session_key
    Success {
        login_id: u32,
        account_id: u32,
        session_key: [u8; 16],
    },
    /// 登录失败，返回错误码
    Failure { error_code: u8 },
}
