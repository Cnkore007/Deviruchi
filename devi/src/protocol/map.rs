/// 进入地图请求包 (0x0072 - CZ_ENTER)
///
/// 客户端通知地图服务器准备进入
#[derive(Debug, Clone)]
pub struct MapEnterRequest {
    /// 角色 ID
    pub char_id: u32,
    /// 登录验证 ID
    pub login_id: u32,
    /// 客户端 tick
    pub client_tick: u32,
    /// 性别
    pub gender: u8,
}

/// 进入地图响应包 (0x0073 - ZC_ACCEPT_ENTER)
///
/// 地图服务器确认进入，返回初始位置信息
#[derive(Debug, Clone)]
pub struct MapEnteredResponse {
    /// 服务器启动时间
    pub start_time: u32,
    /// 初始 X 坐标
    pub pos_x: u16,
    /// 初始 Y 坐标
    pub pos_y: u16,
    /// 朝向
    pub direction: u16,
    /// 字体（保留字段）
    pub font: u16,
}

/// 玩家移动请求包 (0x0085 - CZ_REQUEST_MOVE)
///
/// 客户端请求移动到目标坐标
#[derive(Debug, Clone)]
pub struct PlayerMoveRequest {
    /// 目标 X 坐标
    pub dest_x: u16,
    /// 目标 Y 坐标
    pub dest_y: u16,
}

/// 实体移动通知包 (0x0086 - ZC_MOVE)
///
/// 服务器广播实体移动信息
#[derive(Debug, Clone)]
pub struct EntityMoveNotify {
    /// 实体 ID
    pub entity_id: u32,
    /// 起始 X 坐标
    pub from_x: u16,
    /// 起始 Y 坐标
    pub from_y: u16,
    /// 目标 X 坐标
    pub dest_x: u16,
    /// 目标 Y 坐标
    pub dest_y: u16,
    /// 移动速度
    pub speed: u16,
}

/// 聊天消息包 (0x008c - CZ_REQUEST_CHAT / ZC_NOTIFY_CHAT)
///
/// 客户端发送或服务器广播的聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// 发送者 ID（服务器广播时使用）
    pub sender_id: u32,
    /// 发送者名称
    pub sender_name: String,
    /// 消息内容
    pub message: String,
}

/// 发送聊天消息请求包 (0x008c - CZ_REQUEST_CHAT)
///
/// 客户端向服务器发送聊天消息
#[derive(Debug, Clone)]
pub struct ChatSendRequest {
    /// 消息内容
    pub message: String,
}

/// 实体出现通知包 (0x0078 - ZC_NOTIFY_STANDENTRY)
///
/// 服务器通知客户端一个实体出现在视野中
#[derive(Debug, Clone)]
pub struct EntityAppearNotify {
    /// 实体 ID
    pub entity_id: u32,
    /// 实体类型（0=玩家, 5=NPC, 6=怪物等）
    pub entity_type: u8,
    /// X 坐标
    pub pos_x: u16,
    /// Y 坐标
    pub pos_y: u16,
    /// 朝向
    pub direction: u8,
    /// 外观 ID
    pub look: u16,
}

/// 实体消失通知包 (0x007a - ZC_NOTIFY_VANISH)
///
/// 服务器通知客户端一个实体从视野中消失
#[derive(Debug, Clone)]
pub struct EntityDisappearNotify {
    /// 实体 ID
    pub entity_id: u32,
    /// 消失原因（0=走出视野, 1=死亡等）
    pub reason: u8,
}

/// 攻击请求包 (0x0089 - CZ_REQUEST_ACT)
///
/// 客户端向服务器发起攻击请求
#[derive(Debug, Clone)]
pub struct AttackRequest {
    /// 目标实体 ID
    pub target_id: u32,
    /// 动作类型（0=攻击）
    pub action: u8,
}

/// 攻击通知包 (0x008a - ZC_NOTIFY_ACT)
///
/// 服务器广播攻击动作结果，包含攻击者、目标、伤害等信息
#[derive(Debug, Clone)]
pub struct AttackNotify {
    /// 攻击者实体 ID
    pub src_id: u32,
    /// 目标实体 ID
    pub dst_id: u32,
    /// 服务器时间戳
    pub start_time: u32,
    /// 攻击速度
    pub attack_speed: u32,
    /// 伤害值
    pub damage: u32,
    /// 命中数（多段攻击时 > 1）
    pub count: u16,
    /// 动作类型（0=普通攻击, 8=技能攻击等）
    pub action: u8,
    /// 伤害类型（0=普通, 3=暴击等）
    pub damage_type: u8,
}
