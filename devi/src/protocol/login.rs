/// 登录请求包 (0x0064 - CALogin)
///
/// 客户端发送给登录服务器的认证请求
#[derive(Debug, Clone)]
pub struct LoginRequest {
    /// 客户端版本号
    pub version: u32,
    /// 用户名（最大 23 字符）
    pub username: String,
    /// 密码（最大 23 字符）
    pub password: String,
}

/// 登录响应包 (0x0069 - ACAceptLogin)
///
/// 登录服务器返回的认证结果，包含角色服务器连接信息
#[derive(Debug, Clone)]
pub struct LoginResponse {
    /// 账号 ID
    pub account_id: u32,
    /// 登录验证 ID 1
    pub login_id1: u32,
    /// 登录验证 ID 2
    pub login_id2: u32,
    /// 性别（0=女, 1=男）
    pub sex: u8,
    /// 角色服务器 IP 地址（大端序，4 字节）
    pub char_ip: [u8; 4],
    /// 角色服务器端口
    pub char_port: u16,
    /// 服务器名称
    pub server_name: String,
    /// 在线用户数
    pub user_count: u16,
    /// 服务器类型
    pub server_type: u8,
    /// 新服标志
    pub new_flag: u16,
}
