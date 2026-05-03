use super::packet_builder::{PacketBuilder, Packed, parse_fixed_string};

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
        PacketBuilder::new(0x0065)
            .put_u32(self.char_id)
            .build()
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
        if slice.len() < offset + 6 {
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
        Some(Self { account_id, target_id, action_type })
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
        // BytesMut uses big-endian, so read as BE
        assert_eq!(u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 12345);
        assert_eq!(u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 67890);
        assert_eq!(u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]), 999);
    }
}
