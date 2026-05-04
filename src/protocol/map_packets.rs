use super::packet_builder::{Packed, PacketBuilder, parse_fixed_string, parse_string};

const NAME_LENGTH: usize = 24;

/// 服务器发送角色列表 (0x006B)
#[derive(Debug, Clone)]
pub struct SCCharList {
    pub characters: Vec<CharInfo>,
}

#[derive(Debug, Clone)]
pub struct CharInfo {
    pub char_id: u32,
    pub exp: u32,
    pub gold: u32,
    pub job_exp: u32,
    pub job_level: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub virtue: i16,
    pub honor: i16,
    pub job: u16,
    pub hair: u16,
    pub hair_color: u16,
    pub clothes_color: u16,
    pub body: u16,
    pub weapon: u16,
    pub head_bottom: u16,
    pub shield: u16,
    pub head_top: u16,
    pub head_mid: u16,
    pub hair_color2: u16,
    pub clothes_color2: u16,
    pub name: String,
    pub base_level: u16,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub slot: u8,
    pub delete_timer: u32,
    pub rename: u8,
    pub map_name: String,
}

impl Packed for SCCharList {
    fn to_packet(&self) -> Vec<u8> {
        let count = self.characters.len() as u8;
        let mut ctx = PacketBuilder::new(0x006B);
        ctx = ctx.put_u8(count);

        for char_info in &self.characters {
            ctx = ctx
                .put_u32(char_info.char_id)
                .put_u32(char_info.exp)
                .put_u32(char_info.gold)
                .put_u32(char_info.job_exp)
                .put_u16(char_info.job_level)
                .put_u16(char_info.body_state)
                .put_u16(char_info.health_state)
                .put_u32(char_info.effect_state)
                .put_i16(char_info.virtue)
                .put_i16(char_info.honor)
                .put_u16(char_info.job)
                .put_u16(char_info.hair)
                .put_u16(char_info.hair_color)
                .put_u16(char_info.clothes_color)
                .put_u16(char_info.body)
                .put_u16(char_info.weapon)
                .put_u16(char_info.head_bottom)
                .put_u16(char_info.shield)
                .put_u16(char_info.head_top)
                .put_u16(char_info.head_mid)
                .put_u16(char_info.hair_color2)
                .put_u16(char_info.clothes_color2)
                .put_fixed_str(&char_info.name, NAME_LENGTH)
                .put_u16(char_info.base_level)
                .put_u16(char_info.str)
                .put_u16(char_info.agi)
                .put_u16(char_info.vit)
                .put_u16(char_info.int)
                .put_u16(char_info.dex)
                .put_u16(char_info.luk)
                .put_u8(char_info.slot)
                .put_u32(char_info.delete_timer)
                .put_u8(char_info.rename)
                .put_fixed_str(&char_info.map_name, NAME_LENGTH);
        }

        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端选择角色进入游戏 (0x0065)
#[derive(Debug, Clone)]
pub struct CHEnter {
    pub char_id: u32,
}

impl Packed for CHEnter {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0065).put_u32(self.char_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { char_id })
    }
}

/// 客户端创建角色 (0x0067)
#[derive(Debug, Clone)]
pub struct CHMakeChar {
    pub name: String,
    pub str: u8,
    pub agi: u8,
    pub vit: u8,
    pub int: u8,
    pub dex: u8,
    pub luk: u8,
    pub hair_color: u16,
    pub hair: u16,
}

impl Packed for CHMakeChar {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0067)
            .put_fixed_str(&self.name, NAME_LENGTH)
            .put_u8(self.str)
            .put_u8(self.agi)
            .put_u8(self.vit)
            .put_u8(self.int)
            .put_u8(self.dex)
            .put_u8(self.luk)
            .put_u16(self.hair_color)
            .put_u16(self.hair)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let name = parse_fixed_string(slice, &mut offset, NAME_LENGTH)?;
        if slice.len() < offset + 10 {
            return None;
        }
        Some(Self {
            name,
            str: slice[offset],
            agi: slice[offset + 1],
            vit: slice[offset + 2],
            int: slice[offset + 3],
            dex: slice[offset + 4],
            luk: slice[offset + 5],
            hair_color: u16::from_le_bytes([slice[offset + 6], slice[offset + 7]]),
            hair: u16::from_le_bytes([slice[offset + 8], slice[offset + 9]]),
        })
    }
}

/// 客户端请求删除角色 (0x0068)
#[derive(Debug, Clone)]
pub struct CHDeleteChar {
    pub char_id: u32,
    pub email: String,
}

impl Packed for CHDeleteChar {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0068)
            .put_u32(self.char_id)
            .put_fixed_str(&self.email, 40)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        // 注意: PacketBuilder::put_u32 使用大端序（bytes crate 默认行为）
        let char_id = u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let mut offset = 4;
        let email = parse_fixed_string(slice, &mut offset, 40)?;
        Some(Self { char_id, email })
    }
}

/// 服务器确认角色删除已安排 (0x006C)
#[derive(Debug, Clone)]
pub struct HCDeleteCharOk {
    pub char_id: u32,
}

impl Packed for HCDeleteCharOk {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x006C).put_u32(self.char_id).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端取消角色删除 (0x01F8)
#[derive(Debug, Clone)]
pub struct CHCancelDelete {
    pub char_id: u32,
}

impl Packed for CHCancelDelete {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x01F8).put_u32(self.char_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        // 注意: PacketBuilder::put_u32 使用大端序（bytes crate 默认行为）
        let char_id = u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { char_id })
    }
}

/// 服务器确认取消删除 (0x006D)
#[derive(Debug, Clone)]
pub struct HCCancelDeleteOk {
    pub char_id: u32,
}

impl Packed for HCCancelDeleteOk {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x006D).put_u32(self.char_id).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// Char Server 通知客户端连接 Map Server (0x0083)
#[derive(Debug, Clone)]
pub struct HCNotifyZoneServer {
    pub map_ip: String,
    pub map_port: u16,
    pub token: String,
}

impl Packed for HCNotifyZoneServer {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0083)
            .put_fixed_str(&self.map_ip, 16)
            .put_u16(self.map_port)
            .put_fixed_str(&self.token, 32)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端请求攻击/动作 (0x0089)
#[derive(Debug, Clone)]
pub struct CZRequestAction {
    pub account_id: u32,
    pub target_id: u32,
    pub action_type: u8,
}

impl Packed for CZRequestAction {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0089)
            .put_u32(self.account_id)
            .put_u32(self.target_id)
            .put_u8(self.action_type)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 9 {
            return None;
        }
        let account_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let target_id = u32::from_le_bytes([slice[4], slice[5], slice[6], slice[7]]);
        let action_type = slice[8];
        Some(Self {
            account_id,
            target_id,
            action_type,
        })
    }
}

/// 客户端使用物品 (0x009B)
#[derive(Debug, Clone)]
pub struct CZUseItem {
    pub index: u16,
    pub item_id: u32,
}

impl Packed for CZUseItem {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x009B)
            .put_u16(self.index)
            .put_u32(self.item_id)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 6 {
            return None;
        }
        let index = u16::from_le_bytes([slice[0], slice[1]]);
        let item_id = u32::from_le_bytes([slice[2], slice[3], slice[4], slice[5]]);
        Some(Self { index, item_id })
    }
}

/// 客户端拾取物品 (0x0090)
#[derive(Debug, Clone)]
pub struct CZRequestPickupItem {
    pub x: u16,
    pub y: u16,
}

impl Packed for CZRequestPickupItem {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0090)
            .put_u16(self.x)
            .put_u16(self.y)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let x = u16::from_le_bytes([slice[0], slice[1]]);
        let y = u16::from_le_bytes([slice[2], slice[3]]);
        Some(Self { x, y })
    }
}

/// 客户端交互 NPC (0x0190)
#[derive(Debug, Clone)]
pub struct CZContactNpc {
    pub npc_id: u32,
    pub action: u8,
}

impl Packed for CZContactNpc {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0190)
            .put_u32(self.npc_id)
            .put_u8(self.action)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let action = slice[4];
        Some(Self { npc_id, action })
    }
}

// ─── NPC 对话相关数据包 ───────────────────────────────────────────

/// 服务器发送 NPC 对话消息 (0x00B4)
#[derive(Debug, Clone)]
pub struct ZcSayDialog {
    pub npc_id: u32,
    pub message: String,
}

impl Packed for ZcSayDialog {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00B4)
            .put_u32(self.npc_id)
            .put_str(&self.message)
            .put_u8(0)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let mut offset = 4;
        let message = parse_string(slice, &mut offset)?;
        Some(Self { npc_id, message })
    }
}

/// 服务器发送 NPC 等待对话 (0x00B5)
#[derive(Debug, Clone)]
pub struct ZcWaitDialog {
    pub npc_id: u32,
}

impl Packed for ZcWaitDialog {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00B5).put_u32(self.npc_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { npc_id })
    }
}

/// 服务器关闭 NPC 对话 (0x00B6)
#[derive(Debug, Clone)]
pub struct ZcCloseDialog {
    pub npc_id: u32,
}

impl Packed for ZcCloseDialog {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00B6).put_u32(self.npc_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { npc_id })
    }
}

/// 服务器发送 NPC 菜单列表 (0x00B7)
#[derive(Debug, Clone)]
pub struct ZcMenuList {
    pub npc_id: u32,
    pub menu_text: String,
}

impl Packed for ZcMenuList {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00B7)
            .put_u32(self.npc_id)
            .put_str(&self.menu_text)
            .put_u8(0)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let mut offset = 4;
        let menu_text = parse_string(slice, &mut offset)?;
        Some(Self { npc_id, menu_text })
    }
}

/// 客户端确认下一句对话 (0x00B9)
#[derive(Debug, Clone)]
pub struct CzAckNextDialog {
    pub npc_id: u32,
}

impl Packed for CzAckNextDialog {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00B9).put_u32(self.npc_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { npc_id })
    }
}

/// 客户端选择菜单项 (0x00B8)
#[derive(Debug, Clone)]
pub struct CzAckSelectMenu {
    pub npc_id: u32,
    pub select: u8,
}

impl Packed for CzAckSelectMenu {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x00B8)
            .put_u32(self.npc_id)
            .put_u8(self.select)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let select = slice[4];
        Some(Self { npc_id, select })
    }
}

/// 客户端关闭 NPC 对话 (0x0146)
#[derive(Debug, Clone)]
pub struct CzAckCloseDialog {
    pub npc_id: u32,
}

impl Packed for CzAckCloseDialog {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0146).put_u32(self.npc_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let npc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { npc_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hc_notify_zone_server_packet_id() {
        let pkt = HCNotifyZoneServer {
            map_ip: "127.0.0.1".to_string(),
            map_port: 6121,
            token: "test_token".to_string(),
        };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x0083);
    }

    #[test]
    fn test_cz_request_action_parse() {
        let data = vec![1, 0, 0, 0, 2, 0, 0, 0, 7];
        let pkt = CZRequestAction::from_slice(&data).unwrap();
        assert_eq!(pkt.account_id, 1);
        assert_eq!(pkt.target_id, 2);
        assert_eq!(pkt.action_type, 7);
    }

    #[test]
    fn test_ch_make_char_truncated_returns_none() {
        // NAME_LENGTH bytes for name + 6 stat bytes, but missing the two u16 fields (4 bytes).
        // Before the fix this would panic; now it should return None.
        let data = vec![0u8; NAME_LENGTH + 6];
        assert!(CHMakeChar::from_slice(&data).is_none());
    }

    #[test]
    fn test_ch_make_char_exact_minimum_parses() {
        // NAME_LENGTH bytes for name + 6 u8 stats + 2 u16 fields = NAME_LENGTH + 10 bytes.
        let mut data = vec![0u8; NAME_LENGTH + 10];
        // Set hair_color (u16 LE at offset+6) and hair (u16 LE at offset+8).
        data[NAME_LENGTH + 6] = 0x0A;
        data[NAME_LENGTH + 7] = 0x00;
        data[NAME_LENGTH + 8] = 0x14;
        data[NAME_LENGTH + 9] = 0x00;
        let pkt = CHMakeChar::from_slice(&data).unwrap();
        assert_eq!(pkt.hair_color, 0x000A);
        assert_eq!(pkt.hair, 0x0014);
    }

    #[test]
    fn test_ch_delete_char_parse() {
        let mut data = vec![0u8; 4 + 40];
        // char_id = 42，大端序写入（与 put_u32 一致）
        data[3] = 42;
        // email = "test@email.com" + null padding
        let email_bytes = b"test@email.com";
        data[4..4 + email_bytes.len()].copy_from_slice(email_bytes);

        let pkt = CHDeleteChar::from_slice(&data).unwrap();
        assert_eq!(pkt.char_id, 42);
        assert_eq!(pkt.email, "test@email.com");
    }

    #[test]
    fn test_ch_delete_char_truncated() {
        let data = vec![0u8; 2];
        assert!(CHDeleteChar::from_slice(&data).is_none());
    }

    #[test]
    fn test_ch_cancel_delete_parse() {
        // char_id = 100，大端序写入
        let data = vec![0, 0, 0, 100];
        let pkt = CHCancelDelete::from_slice(&data).unwrap();
        assert_eq!(pkt.char_id, 100);
    }

    #[test]
    fn test_ch_cancel_delete_truncated() {
        let data = vec![0u8; 2];
        assert!(CHCancelDelete::from_slice(&data).is_none());
    }
}

/// NPC 对话相关数据包 ID 常量
pub mod id {
    // 服务器 -> 客户端
    #[allow(dead_code)]
    pub const ZC_SAY_DIALOG: u16 = 0x00B4;
    #[allow(dead_code)]
    pub const ZC_WAIT_DIALOG: u16 = 0x00B5;
    #[allow(dead_code)]
    pub const ZC_CLOSE_DIALOG: u16 = 0x00B6;
    #[allow(dead_code)]
    pub const ZC_MENU_LIST: u16 = 0x00B7;

    // 客户端 -> 服务器
    #[allow(dead_code)]
    pub const CZ_ACK_SELECT_MENU: u16 = 0x00B8;
    #[allow(dead_code)]
    pub const CZ_ACK_NEXT_DIALOG: u16 = 0x00B9;
    #[allow(dead_code)]
    pub const CZ_ACK_CLOSE_DIALOG: u16 = 0x0146;
}
