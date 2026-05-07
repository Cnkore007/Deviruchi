/// 进入地图请求包
#[derive(Debug, Clone)]
pub struct MapEnterRequest {
    pub char_id: u32,
    pub login_id: u32,
    pub client_tick: u32,
    pub gender: u8,
}

/// 进入地图响应包
#[derive(Debug, Clone)]
pub struct MapEnteredResponse {
    pub char_id: u32,
    pub map_name: String,
    pub pos_x: u16,
    pub pos_y: u16,
}

/// 玩家移动请求包
#[derive(Debug, Clone)]
pub struct PlayerMoveRequest {
    pub dest_x: u16,
    pub dest_y: u16,
}

/// 实体移动通知包
#[derive(Debug, Clone)]
pub struct EntityMoveNotify {
    pub entity_id: u32,
    pub from_x: u16,
    pub from_y: u16,
    pub dest_x: u16,
    pub dest_y: u16,
    pub speed: u16,
}

/// 聊天消息包
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender_id: u32,
    pub sender_name: String,
    pub message: String,
}

/// 实体出现通知包
#[derive(Debug, Clone)]
pub struct EntityAppearNotify {
    pub entity_id: u32,
    pub entity_type: u8,
    pub pos_x: u16,
    pub pos_y: u16,
    pub direction: u8,
    pub look: u16,
}

/// 实体消失通知包
#[derive(Debug, Clone)]
pub struct EntityDisappearNotify {
    pub entity_id: u32,
    pub reason: u8,
}
