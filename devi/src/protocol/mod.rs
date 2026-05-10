pub mod login;
pub mod char_mod;
pub mod map;

/// 所有协议包的统一枚举
#[derive(Debug, Clone)]
pub enum Packet {
    // ===== 登录阶段 =====
    LoginRequest(login::LoginRequest),
    LoginResponse(login::LoginResponse),

    // ===== 角色选择阶段 =====
    /// 请求角色列表 (0x0066)
    CharListRequest,
    /// 角色列表响应 (0x006b)
    CharListResponse(char_mod::CharListResponse),
    /// 选择角色进入 (0x0065)
    CharEnterRequest(char_mod::CharEnterRequest),
    /// 进入角色响应，含地图服务器信息 (0x0071)
    CharEnterResponse(char_mod::CharEnterResponse),
    /// 创建角色请求 (0x0067)
    CharCreateRequest(char_mod::CharCreateRequest),
    /// 创建角色响应 (0x006d)
    CharCreateResponse(char_mod::CharCreateResponse),
    /// 删除角色请求 (0x0068)
    CharDeleteRequest(char_mod::CharDeleteRequest),
    /// 删除角色响应 (0x006e)
    CharDeleteResponse(char_mod::CharDeleteResponse),

    // ===== 地图阶段 =====
    /// 进入地图请求 (0x0072)
    MapEnter(map::MapEnterRequest),
    /// 进入地图响应 (0x0073)
    MapEntered(map::MapEnteredResponse),
    /// 玩家移动请求 (0x0085)
    PlayerMove(map::PlayerMoveRequest),
    /// 实体移动通知 (0x0086)
    EntityMove(map::EntityMoveNotify),
    /// 聊天消息 (0x008c)
    ChatMessage(map::ChatMessage),
    /// 发送聊天消息请求 (0x008c)
    ChatSendRequest(map::ChatSendRequest),
    /// 实体出现通知 (0x0078)
    EntityAppear(map::EntityAppearNotify),
    /// 实体消失通知 (0x007a)
    EntityDisappear(map::EntityDisappearNotify),
    /// 攻击请求 (0x0089)
    AttackRequest(map::AttackRequest),
    /// 攻击通知 (0x008a)
    AttackNotify(map::AttackNotify),
}

impl Packet {
    /// 获取协议包 ID
    pub fn packet_id(&self) -> u16 {
        match self {
            // 登录
            Packet::LoginRequest(_) => 0x0064,
            Packet::LoginResponse(_) => 0x0069,
            // 角色
            Packet::CharListRequest => 0x0066,
            Packet::CharListResponse(_) => 0x006b,
            Packet::CharEnterRequest(_) => 0x0065,
            Packet::CharEnterResponse(_) => 0x0071,
            Packet::CharCreateRequest(_) => 0x0067,
            Packet::CharCreateResponse(_) => 0x006d,
            Packet::CharDeleteRequest(_) => 0x0068,
            Packet::CharDeleteResponse(_) => 0x006e,
            // 地图
            Packet::MapEnter(_) => 0x0072,
            Packet::MapEntered(_) => 0x0073,
            Packet::PlayerMove(_) => 0x0085,
            Packet::EntityMove(_) => 0x0086,
            Packet::ChatMessage(_) => 0x008c,
            Packet::ChatSendRequest(_) => 0x008c,
            Packet::EntityAppear(_) => 0x0078,
            Packet::EntityDisappear(_) => 0x007a,
            Packet::AttackRequest(_) => 0x0089,
            Packet::AttackNotify(_) => 0x008a,
        }
    }

    /// 判断是否为登录相关包
    pub fn is_login_packet(&self) -> bool {
        matches!(self, Packet::LoginRequest(_) | Packet::LoginResponse(_))
    }

    /// 判断是否为选角色相关包
    pub fn is_char_packet(&self) -> bool {
        matches!(
            self,
            Packet::CharListRequest
                | Packet::CharListResponse(_)
                | Packet::CharEnterRequest(_)
                | Packet::CharEnterResponse(_)
                | Packet::CharCreateRequest(_)
                | Packet::CharCreateResponse(_)
                | Packet::CharDeleteRequest(_)
                | Packet::CharDeleteResponse(_)
        )
    }

    /// 判断是否为地图相关包
    pub fn is_map_packet(&self) -> bool {
        matches!(
            self,
            Packet::MapEnter(_)
                | Packet::MapEntered(_)
                | Packet::PlayerMove(_)
                | Packet::EntityMove(_)
                | Packet::ChatMessage(_)
                | Packet::ChatSendRequest(_)
                | Packet::EntityAppear(_)
                | Packet::EntityDisappear(_)
                | Packet::AttackRequest(_)
                | Packet::AttackNotify(_)
        )
    }
}
