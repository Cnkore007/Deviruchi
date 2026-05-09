/// 请求角色列表 (0x0066)
///
/// 客户端请求当前账号下的角色列表，无包体数据
#[derive(Debug, Clone)]
pub struct CharListRequest;

/// 选择角色进入 (0x0065 - CHEnter)
///
/// 客户端选择一个角色进入游戏
#[derive(Debug, Clone)]
pub struct CharEnterRequest {
    /// 角色 ID
    pub char_id: u32,
}

/// 角色列表响应包 (0x006b - SCCharList)
///
/// 服务器返回当前账号下的所有角色信息
#[derive(Debug, Clone)]
pub struct CharListResponse {
    /// 角色列表
    pub chars: Vec<CharInfo>,
}

/// 角色信息（108 字节定长结构）
///
/// 对应服务端 SCCharList 中的单条角色数据
#[derive(Debug, Clone)]
pub struct CharInfo {
    /// 角色 ID
    pub char_id: u32,
    /// 经验值
    pub exp: u32,
    /// 金币
    pub gold: u32,
    /// 职业经验值
    pub job_exp: u32,
    /// 职业等级
    pub job_level: u16,
    /// 身体状态
    pub body_state: u16,
    /// 健康状态
    pub health_state: u16,
    /// 效果状态
    pub effect_state: u32,
    /// 道德值
    pub virtue: i16,
    /// 荣誉值
    pub honor: i16,
    /// 职业 ID
    pub job: u16,
    /// 发型
    pub hair: u16,
    /// 发色
    pub hair_color: u16,
    /// 衣服颜色
    pub clothes_color: u16,
    /// 身体
    pub body: u16,
    /// 武器
    pub weapon: u16,
    /// 头部下装饰
    pub head_bottom: u16,
    /// 盾牌
    pub shield: u16,
    /// 头部上装饰
    pub head_top: u16,
    /// 头部中装饰
    pub head_mid: u16,
    /// 发色 2
    pub hair_color2: u16,
    /// 衣服颜色 2
    pub clothes_color2: u16,
    /// 角色名（最大 24 字节）
    pub name: String,
    /// 基础等级
    pub base_level: u16,
    /// 力量
    pub str: u16,
    /// 敏捷
    pub agi: u16,
    /// 体力
    pub vit: u16,
    /// 智力
    pub int: u16,
    /// 灵巧
    pub dex: u16,
    /// 幸运
    pub luk: u16,
    /// 角色栏位
    pub slot: u8,
    /// 删除倒计时（秒）
    pub delete_timer: u32,
    /// 改名标志
    pub rename: u8,
    /// 所在地图名（最大 24 字节）
    pub map_name: String,
}

/// 创建角色请求包 (0x0067 - CHMakeChar)
///
/// 客户端请求创建新角色
#[derive(Debug, Clone)]
pub struct CharCreateRequest {
    /// 角色名（最大 24 字节）
    pub name: String,
    /// 力量初始值
    pub str: u8,
    /// 敏捷初始值
    pub agi: u8,
    /// 体力初始值
    pub vit: u8,
    /// 智力初始值
    pub int: u8,
    /// 灵巧初始值
    pub dex: u8,
    /// 幸运初始值
    pub luk: u8,
    /// 发色
    pub hair_color: u16,
    /// 发型
    pub hair: u16,
}

/// 创建角色响应包 (0x006d)
///
/// 服务器返回角色创建结果
#[derive(Debug, Clone)]
pub enum CharCreateResponse {
    /// 创建成功，返回角色信息
    Success(CharInfo),
    /// 创建失败，返回错误码
    Failure { error_code: u8 },
}

/// 删除角色请求包 (0x0068 - CHDeleteChar)
///
/// 客户端请求删除指定角色
#[derive(Debug, Clone)]
pub struct CharDeleteRequest {
    /// 要删除的角色 ID
    pub char_id: u32,
    /// 注册邮箱（用于验证，最大 40 字节）
    pub email: String,
}

/// 删除角色响应包 (0x006e - HCDeleteCharOk)
///
/// 服务器确认角色删除已安排
#[derive(Debug, Clone)]
pub struct CharDeleteResponse {
    /// 被删除的角色 ID
    pub char_id: u32,
}

/// 进入角色响应包 (0x0071 - HCNotifyZoneServer)
///
/// 服务器返回地图服务器连接信息，客户端需切换到地图服务器
#[derive(Debug, Clone)]
pub struct CharEnterResponse {
    /// 地图服务器 IP 地址（16 字节定长字符串）
    pub map_ip: String,
    /// 地图服务器端口
    pub map_port: u16,
    /// 会话令牌（32 字节定长字符串）
    pub token: String,
}
