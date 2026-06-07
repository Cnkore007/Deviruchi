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

/// 客户端进入角色服务器 (0x0065)
/// 连接 Char Server 后发送的第一个包，用于身份验证
#[derive(Debug, Clone)]
pub struct CHEnterCharServer {
    pub account_id: u32,
    pub login_id1: u32,
    pub login_id2: u32,
    pub sex: u8,
}

impl Packed for CHEnterCharServer {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0065)
            .put_u32(self.account_id)
            .put_u32(self.login_id1)
            .put_u32(self.login_id2)
            .put_u8(self.sex)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 13 {
            return None;
        }
        let account_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let login_id1 = u32::from_le_bytes([slice[4], slice[5], slice[6], slice[7]]);
        let login_id2 = u32::from_le_bytes([slice[8], slice[9], slice[10], slice[11]]);
        let sex = slice[12];
        Some(Self { account_id, login_id1, login_id2, sex })
    }
}

/// 客户端选择角色 (0x0066)
#[derive(Debug, Clone)]
pub struct CHSelectChar {
    pub char_id: u32,
}

impl Packed for CHSelectChar {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0066).put_u32(self.char_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { char_id })
    }
}

/// 向后兼容别名
pub type CHEnter = CHSelectChar;

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
        let char_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
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
        let char_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
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

/// 服务器通知动作/伤害 (0x008D)
/// 参考 rAthena: clif_damage() in clif.cpp
#[derive(Debug, Clone)]
pub struct ZCNotifyAct {
    pub src_id: u32,       // 攻击者 GID
    pub dst_id: u32,       // 目标 GID
    pub damage: u32,       // 伤害值
    pub action: u8,         // 0=damage, 5=critical, 14=pickup
    pub left_damage: u32,  // 左侧伤害（分身后用）
}

impl Packed for ZCNotifyAct {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x008D)
            .put_u32(self.src_id)
            .put_u32(self.dst_id)
            .put_u32(self.damage)
            .put_u8(self.action)
            .put_u32(self.left_damage)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 怪物血条更新 (0x0977)
/// 参考 rAthena: clif_monster_hp_bar() in clif.cpp
/// 只发送给 dmglog 中的玩家（攻击过该怪物的玩家）
#[derive(Debug, Clone)]
pub struct ZCMonsterHpBar {
    pub mob_id: u32,
    pub hp: u32,
    pub max_hp: u32,
}

impl Packed for ZCMonsterHpBar {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0977)
            .put_u32(self.mob_id)
            .put_u32(self.hp)
            .put_u32(self.max_hp)
            .build()
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
        // char_id = 42，小端序写入
        data[0] = 42;
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
        // char_id = 100，小端序写入
        let data = vec![100, 0, 0, 0];
        let pkt = CHCancelDelete::from_slice(&data).unwrap();
        assert_eq!(pkt.char_id, 100);
    }

    #[test]
    fn test_ch_cancel_delete_truncated() {
        let data = vec![0u8; 2];
        assert!(CHCancelDelete::from_slice(&data).is_none());
    }

    #[test]
    fn test_zc_notify_act_packet_id() {
        let pkt = ZCNotifyAct {
            src_id: 1,
            dst_id: 2,
            damage: 50,
            action: 0,
            left_damage: 0,
        };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x008D);
    }

    #[test]
    fn test_zc_monster_hp_bar_packet_id() {
        let pkt = ZCMonsterHpBar {
            mob_id: 100,
            hp: 30,
            max_hp: 100,
        };
        let bytes = pkt.to_packet();
        let packet_id = u16::from_le_bytes([bytes[2], bytes[3]]);
        assert_eq!(packet_id, 0x0977);
    }

    #[test]
    fn test_zc_notify_act_content() {
        let pkt = ZCNotifyAct {
            src_id: 12345,
            dst_id: 67890,
            damage: 999,
            action: 5,
            left_damage: 0,
        };
        let bytes = pkt.to_packet();
        // PacketBuilder 使用小端序 (put_u32_le)，所以用 LE 读取
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 12345);
        assert_eq!(u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 67890);
        assert_eq!(u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]), 999);
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

/// 客户端请求穿戴装备 (0x00A9)
#[derive(Debug, Clone)]
pub struct CzReqWearEquip {
    /// 物品栏索引
    pub index: u16,
    /// 装备位置掩码
    pub position: u16,
}

impl CzReqWearEquip {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let index = u16::from_le_bytes([data[0], data[1]]);
        let position = u16::from_le_bytes([data[2], data[3]]);
        Some(Self { index, position })
    }
}

/// 客户端请求卸下装备 (0x00AB)
#[derive(Debug, Clone)]
pub struct CzReqTakeoffEquip {
    /// 装备位置掩码
    pub position: u16,
}

impl CzReqTakeoffEquip {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let position = u16::from_le_bytes([data[0], data[1]]);
        Some(Self { position })
    }
}

/// 服务器响应穿戴装备结果 (0x00AA)
#[derive(Debug, Clone)]
pub struct ZcReqWearEquipAck {
    /// 物品栏索引
    pub index: u16,
    /// 装备位置掩码
    pub position: u16,
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcReqWearEquipAck {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x00AA);
        ctx = ctx
            .put_u16(self.index)
            .put_u16(self.position)
            .put_u8(self.result);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器响应卸下装备结果 (0x00AC)
#[derive(Debug, Clone)]
pub struct ZcReqTakeoffEquipAck {
    /// 装备位置掩码
    pub position: u16,
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcReqTakeoffEquipAck {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x00AC);
        ctx = ctx
            .put_u16(self.position)
            .put_u8(self.result);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端请求购买NPC商店物品 (0x00C8)
#[derive(Debug, Clone)]
pub struct CzNpcBuyListSend {
    /// 购买数量
    pub count: u16,
    /// 物品ID列表
    pub item_ids: Vec<u16>,
}

impl CzNpcBuyListSend {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let count = u16::from_le_bytes([data[0], data[1]]);
        let mut item_ids = Vec::new();
        for i in 0..count as usize {
            let offset = 2 + i * 2;
            if offset + 2 <= data.len() {
                let item_id = u16::from_le_bytes([data[offset], data[offset + 1]]);
                item_ids.push(item_id);
            }
        }
        Some(Self { count, item_ids })
    }
}

/// 客户端请求出售物品给NPC (0x00C9)
#[derive(Debug, Clone)]
pub struct CzNpcSellListSend {
    /// 出售数量
    pub count: u16,
    /// 物品信息列表（索引 + 数量）
    pub items: Vec<(u16, u16)>,
}

impl CzNpcSellListSend {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let count = u16::from_le_bytes([data[0], data[1]]);
        let mut items = Vec::new();
        for i in 0..count as usize {
            let offset = 2 + i * 4;
            if offset + 4 <= data.len() {
                let index = u16::from_le_bytes([data[offset], data[offset + 1]]);
                let amount = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
                items.push((index, amount));
            }
        }
        Some(Self { count, items })
    }
}

/// 服务器响应购买结果 (0x00CA)
#[derive(Debug, Clone)]
pub struct ZcPcPurchaseResult {
    /// 结果：0=成功, 1=失败, 2=zeny不足, 3=超重
    pub result: u8,
}

impl Packed for ZcPcPurchaseResult {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x00CA);
        ctx = ctx.put_u8(self.result);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 服务器响应出售结果 (0x00CB)
#[derive(Debug, Clone)]
pub struct ZcPcSellResult {
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcPcSellResult {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x00CB);
        ctx = ctx.put_u8(self.result);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端请求插入卡片 (0x017C)
#[derive(Debug, Clone)]
pub struct CzInsertCard {
    /// 卡片索引
    pub card_index: u16,
    /// 装备索引
    pub equip_index: u16,
}

impl CzInsertCard {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let card_index = u16::from_le_bytes([data[0], data[1]]);
        let equip_index = u16::from_le_bytes([data[2], data[3]]);
        Some(Self { card_index, equip_index })
    }
}

/// 客户端请求鉴定物品 (0x01DD)
#[derive(Debug, Clone)]
pub struct CzItemIdentify {
    /// 物品索引
    pub index: u16,
}

impl CzItemIdentify {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let index = u16::from_le_bytes([data[0], data[1]]);
        Some(Self { index })
    }
}

/// 服务器响应鉴定结果 (0x01DC)
#[derive(Debug, Clone)]
pub struct ZcItemIdentifyAck {
    /// 物品索引
    pub index: u16,
    /// 结果：0=成功, 1=失败
    pub result: u8,
}

impl Packed for ZcItemIdentifyAck {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x01DC);
        ctx = ctx
            .put_u16(self.index)
            .put_u8(self.result);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端请求精炼武器 (0x0222)
#[derive(Debug, Clone)]
pub struct CzWeaponRefine {
    /// 物品索引
    pub index: u16,
}

impl CzWeaponRefine {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let index = u16::from_le_bytes([data[0], data[1]]);
        Some(Self { index })
    }
}

/// 服务器响应精炼结果 (0x0223)
#[derive(Debug, Clone)]
pub struct ZcWeaponRefineAck {
    /// 结果：0=成功, 1=失败, 2=已达上限
    pub result: u8,
    /// 物品索引
    pub index: u16,
}

impl Packed for ZcWeaponRefineAck {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x0223);
        ctx = ctx
            .put_u8(self.result)
            .put_u16(self.index);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端发送表情 (0x00BF)
#[derive(Debug, Clone)]
pub struct CzEmotion {
    /// 表情ID
    pub emotion: u8,
}

impl CzEmotion {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { emotion: data[0] })
    }
}

/// 服务器广播表情 (0x00C0)
#[derive(Debug, Clone)]
pub struct ZcEmotion {
    /// 实体ID
    pub entity_id: u32,
    /// 表情ID
    pub emotion: u8,
}

impl Packed for ZcEmotion {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x00C0);
        ctx = ctx
            .put_u32(self.entity_id)
            .put_u8(self.emotion);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端请求捕捉宠物 (0x019F)
#[derive(Debug, Clone)]
pub struct CzCatchPet {
    /// 怪物实体ID
    pub mob_id: u32,
}

impl CzCatchPet {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let mob_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { mob_id })
    }
}

/// 客户端请求宠物菜单 (0x01A9)
#[derive(Debug, Clone)]
pub struct CzPetMenu {
    /// 操作类型：0=信息, 1=喂食, 2=放生, 3=召回
    pub action: u8,
}

impl CzPetMenu {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { action: data[0] })
    }
}

/// 客户端选择宠物蛋 (0x01A7)
#[derive(Debug, Clone)]
pub struct CzSelectEgg {
    /// 蛋索引
    pub egg_index: u8,
}

impl CzSelectEgg {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { egg_index: data[0] })
    }
}

/// 客户端半魔娘操作 (0x022D)
#[derive(Debug, Clone)]
pub struct CzHomMenu {
    /// 操作类型：0=信息, 1=喂食, 2=放生, 3=召回, 4=攻击, 5=移动
    pub action: u8,
}

impl CzHomMenu {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { action: data[0] })
    }
}

/// 客户端佣兵操作 (0x022F)
#[derive(Debug, Clone)]
pub struct CzMercenaryAction {
    /// 操作类型：0=信息, 1=召回, 2=放生
    pub action: u8,
}

impl CzMercenaryAction {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { action: data[0] })
    }
}

/// 客户端创建聊天室 (0x00D5)
#[derive(Debug, Clone)]
pub struct CzCreateChatRoom {
    /// 聊天室大小
    pub size: u16,
    /// 是否公开
    pub is_public: bool,
    /// 密码（如果是私密聊天室）
    pub password: String,
    /// 聊天室标题
    pub title: String,
}

impl CzCreateChatRoom {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let size = u16::from_le_bytes([data[0], data[1]]);
        let is_public = data[2] != 0;
        let password_len = data[3] as usize;
        let mut password = String::new();
        let mut title = String::new();

        if data.len() > 4 + password_len {
            password = String::from_utf8_lossy(&data[4..4 + password_len]).to_string();
            if data.len() > 4 + password_len {
                title = String::from_utf8_lossy(&data[4 + password_len..]).to_string();
            }
        }

        Some(Self { size, is_public, password, title })
    }
}

/// 客户端加入聊天室 (0x00D9)
#[derive(Debug, Clone)]
pub struct CzChatAddMember {
    /// 聊天室ID
    pub chat_id: u32,
    /// 密码
    pub password: String,
}

impl CzChatAddMember {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let chat_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let password = if data.len() > 4 {
            String::from_utf8_lossy(&data[4..]).to_string()
        } else {
            String::new()
        };
        Some(Self { chat_id, password })
    }
}

/// 客户端离开聊天室 (0x00E0)
#[derive(Debug, Clone)]
pub struct CzChatLeave;

impl CzChatLeave {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求好友列表 (0x0201)
#[derive(Debug, Clone)]
pub struct CzFriendsListAdd {
    /// 好友角色ID
    pub char_id: u32,
}

impl CzFriendsListAdd {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { char_id })
    }
}

/// 客户端删除好友 (0x0203)
#[derive(Debug, Clone)]
pub struct CzFriendsListRemove {
    /// 好友角色ID
    pub char_id: u32,
}

impl CzFriendsListRemove {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let char_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { char_id })
    }
}

/// 客户端回复好友请求 (0x0208)
#[derive(Debug, Clone)]
pub struct CzFriendsListReply {
    /// 请求者角色ID
    pub char_id: u32,
    /// 回复：0=拒绝, 1=接受
    pub reply: u8,
}

impl CzFriendsListReply {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let char_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let reply = data[4];
        Some(Self { char_id, reply })
    }
}

/// 客户端请求打开邮箱 (0x0260)
#[derive(Debug, Clone)]
pub struct CzMailOpen;

impl CzMailOpen {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求发送邮件 (0x0261)
#[derive(Debug, Clone)]
pub struct CzMailSend {
    /// 收件人名称
    pub receiver: String,
    /// 邮件标题
    pub title: String,
    /// 邮件内容
    pub body: String,
}

impl CzMailSend {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 24 {
            return None;
        }
        let receiver = String::from_utf8_lossy(&data[0..24]).trim_matches('\0').to_string();
        let title_len = u16::from_le_bytes([data[24], data[25]]) as usize;
        let title = if data.len() > 26 + title_len {
            String::from_utf8_lossy(&data[26..26 + title_len]).to_string()
        } else {
            String::new()
        };
        let body = if data.len() > 26 + title_len {
            String::from_utf8_lossy(&data[26 + title_len..]).to_string()
        } else {
            String::new()
        };
        Some(Self { receiver, title, body })
    }
}

/// 客户端请求打开银行 (0x09B7)
#[derive(Debug, Clone)]
pub struct CzBankOpen;

impl CzBankOpen {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求关闭银行 (0x09B8)
#[derive(Debug, Clone)]
pub struct CzBankClose;

impl CzBankClose {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求存款 (0x09B9)
#[derive(Debug, Clone)]
pub struct CzBankDeposit {
    /// 存款金额
    pub amount: u32,
}

impl CzBankDeposit {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let amount = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { amount })
    }
}

/// 客户端请求取款 (0x09BA)
#[derive(Debug, Clone)]
pub struct CzBankWithdraw {
    /// 取款金额
    pub amount: u32,
}

impl CzBankWithdraw {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let amount = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { amount })
    }
}

/// 客户端请求打开商城 (0x0845)
#[derive(Debug, Clone)]
pub struct CzCashShopOpen;

impl CzCashShopOpen {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求购买商城物品 (0x0848)
#[derive(Debug, Clone)]
pub struct CzCashShopBuy {
    /// 物品ID
    pub item_id: u32,
    /// 数量
    pub amount: u16,
}

impl CzCashShopBuy {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 6 {
            return None;
        }
        let item_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let amount = u16::from_le_bytes([data[4], data[5]]);
        Some(Self { item_id, amount })
    }
}

/// 客户端请求关闭商城 (0x084A)
#[derive(Debug, Clone)]
pub struct CzCashShopClose;

impl CzCashShopClose {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求任务状态 (0x02B5)
#[derive(Debug, Clone)]
pub struct CzQuestStateAck {
    /// 任务ID
    pub quest_id: u32,
    /// 状态：0=进行中, 1=完成
    pub state: u8,
}

impl CzQuestStateAck {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 5 {
            return None;
        }
        let quest_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let state = data[4];
        Some(Self { quest_id, state })
    }
}

/// 客户端请求成就奖励 (0x0224)
#[derive(Debug, Clone)]
pub struct CzAchievementCheckReward {
    /// 成就ID
    pub achievement_id: u32,
}

impl CzAchievementCheckReward {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 4 {
            return None;
        }
        let achievement_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        Some(Self { achievement_id })
    }
}

/// 客户端请求PVP信息 (0x0237)
#[derive(Debug, Clone)]
pub struct CzPVPInfo;

impl CzPVPInfo {
    pub fn from_slice(_data: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端请求坐骑 (0x019C)
#[derive(Debug, Clone)]
pub struct CzChangeCart {
    /// 坐骑类型
    pub cart_type: u8,
}

impl CzChangeCart {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self { cart_type: data[0] })
    }
}

/// 客户端请求技能选择菜单 (0x0A35)
#[derive(Debug, Clone)]
pub struct CzSkillSelectMenu {
    /// 技能ID
    pub skill_id: u16,
    /// 选择的等级
    pub level: u8,
}

impl CzSkillSelectMenu {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 3 {
            return None;
        }
        let skill_id = u16::from_le_bytes([data[0], data[1]]);
        let level = data[2];
        Some(Self { skill_id, level })
    }
}

/// 客户端请求自动念咒 (0x01CF)
#[derive(Debug, Clone)]
pub struct CzAutoSpell {
    /// 技能ID
    pub skill_id: u16,
}

impl CzAutoSpell {
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let skill_id = u16::from_le_bytes([data[0], data[1]]);
        Some(Self { skill_id })
    }
}
