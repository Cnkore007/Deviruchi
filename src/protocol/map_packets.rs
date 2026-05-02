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
