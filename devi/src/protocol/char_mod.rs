/// 选服请求包
#[derive(Debug, Clone)]
pub struct CharSelectRequest {
    pub server_index: u16,
}

/// 角色列表响应包
#[derive(Debug, Clone)]
pub struct CharListResponse {
    pub chars: Vec<CharInfo>,
}

/// 角色信息
#[derive(Debug, Clone)]
pub struct CharInfo {
    pub char_id: u32,
    pub base_level: u32,
    pub job_level: u32,
    pub name: String,
    pub job_id: u16,
    pub map_name: String,
}

/// 创建角色请求包
#[derive(Debug, Clone)]
pub struct CharCreateRequest {
    pub name: String,
    pub job_id: u16,
    pub hair_style: u8,
    pub hair_color: u8,
}

/// 创建角色响应包
#[derive(Debug, Clone)]
pub enum CharCreateResponse {
    /// 创建成功，返回角色信息
    Success(CharInfo),
    /// 创建失败，返回错误码
    Failure { error_code: u8 },
}

/// 删除角色请求包
#[derive(Debug, Clone)]
pub struct CharDeleteRequest {
    pub char_id: u32,
    pub email: String,
}

/// 删除角色响应包
#[derive(Debug, Clone)]
pub enum CharDeleteResponse {
    /// 删除成功
    Success,
    /// 删除失败，返回错误码
    Failure { error_code: u8 },
}

/// 进入角色请求包
#[derive(Debug, Clone)]
pub struct CharEnterRequest {
    pub char_id: u32,
}

/// 进入角色响应包
#[derive(Debug, Clone)]
pub enum CharEnterResponse {
    /// 进入成功，返回地图服务器地址和会话密钥
    Success {
        map_server_ip: String,
        map_server_port: u16,
        char_id: u32,
        session_key: [u8; 16],
    },
    /// 进入失败，返回错误码
    Failure { error_code: u8 },
}
