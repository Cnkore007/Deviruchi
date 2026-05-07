pub mod login;
pub mod char_mod;
pub mod map;

/// 所有协议包的统一枚举
#[derive(Debug, Clone)]
pub enum Packet {
    LoginRequest(login::LoginRequest),
    LoginResponse(login::LoginResponse),
    CharSelectRequest(char_mod::CharSelectRequest),
    CharListResponse(char_mod::CharListResponse),
    CharCreateRequest(char_mod::CharCreateRequest),
    CharCreateResponse(char_mod::CharCreateResponse),
    CharDeleteRequest(char_mod::CharDeleteRequest),
    CharDeleteResponse(char_mod::CharDeleteResponse),
    CharEnterRequest(char_mod::CharEnterRequest),
    CharEnterResponse(char_mod::CharEnterResponse),
    MapEnter(map::MapEnterRequest),
    MapEntered(map::MapEnteredResponse),
    PlayerMove(map::PlayerMoveRequest),
    EntityMove(map::EntityMoveNotify),
    ChatMessage(map::ChatMessage),
    EntityAppear(map::EntityAppearNotify),
    EntityDisappear(map::EntityDisappearNotify),
}

impl Packet {
    /// 获取协议包 ID
    pub fn packet_id(&self) -> u16 {
        match self {
            Packet::LoginRequest(_) => 0x0064,
            Packet::LoginResponse(_) => 0x0069,
            Packet::CharSelectRequest(_) => 0x0065,
            Packet::CharListResponse(_) => 0x006b,
            Packet::CharCreateRequest(_) => 0x0067,
            Packet::CharCreateResponse(_) => 0x006d,
            Packet::CharDeleteRequest(_) => 0x0068,
            Packet::CharDeleteResponse(_) => 0x006e,
            Packet::CharEnterRequest(_) => 0x0066,
            Packet::CharEnterResponse(_) => 0x0071,
            Packet::MapEnter(_) => 0x0072,
            Packet::MapEntered(_) => 0x0073,
            Packet::PlayerMove(_) => 0x0085,
            Packet::EntityMove(_) => 0x0086,
            Packet::ChatMessage(_) => 0x008c,
            Packet::EntityAppear(_) => 0x0078,
            Packet::EntityDisappear(_) => 0x007a,
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
            Packet::CharSelectRequest(_)
                | Packet::CharListResponse(_)
                | Packet::CharCreateRequest(_)
                | Packet::CharCreateResponse(_)
                | Packet::CharDeleteRequest(_)
                | Packet::CharDeleteResponse(_)
                | Packet::CharEnterRequest(_)
                | Packet::CharEnterResponse(_)
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
                | Packet::EntityAppear(_)
                | Packet::EntityDisappear(_)
        )
    }
}
