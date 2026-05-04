use super::packet_builder::{Packed, PacketBuilder};

/// 客户端进入地图请求 (0x007C)
#[derive(Debug, Clone)]
pub struct CZEnter {
    pub gc_id: u32,
}

impl Packed for CZEnter {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x007C).put_u32(self.gc_id).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let gc_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { gc_id })
    }
}

/// 服务器接受进入 (0x02D3)
#[derive(Debug, Clone)]
pub struct ZCAcceptEnter {
    pub start_time: u32,
    pub pos_x: u16,
    pub pos_y: u16,
}

impl Packed for ZCAcceptEnter {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x02D3)
            .put_u32(self.start_time)
            .put_u16(self.pos_x)
            .put_u16(self.pos_y)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端移动请求 (0x0085)
#[derive(Debug, Clone)]
pub struct CZRequestMove {
    pub pos_x: u16,
    pub pos_y: u16,
    pub move_data: Vec<u8>,
}

impl Packed for CZRequestMove {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x0085);
        ctx = ctx.put_u16(self.pos_x);
        ctx = ctx.put_u16(self.pos_y);
        ctx = ctx.put_slice(&self.move_data);
        ctx.build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let pos_x = u16::from_le_bytes([slice[0], slice[1]]);
        let pos_y = u16::from_le_bytes([slice[2], slice[3]]);
        let move_data = slice[4..].to_vec();
        Some(Self {
            pos_x,
            pos_y,
            move_data,
        })
    }
}

/// 服务器广播移动 (0x0086)
#[derive(Debug, Clone)]
pub struct ZCMove {
    pub entity_id: u32,
    pub move_data: Vec<u8>,
}

impl Packed for ZCMove {
    fn to_packet(&self) -> Vec<u8> {
        let mut ctx = PacketBuilder::new(0x0086);
        ctx = ctx.put_u32(self.entity_id);
        ctx = ctx.put_slice(&self.move_data);
        ctx.build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 客户端使用技能 (0x0112)
#[derive(Debug, Clone)]
pub struct CZUseSkill {
    pub skill_id: u16,
    pub target_id: u32,
    pub target_x: u16,
    pub target_y: u16,
}

impl Packed for CZUseSkill {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0112)
            .put_u16(self.skill_id)
            .put_u32(self.target_id)
            .put_u16(self.target_x)
            .put_u16(self.target_y)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 12 {
            return None;
        }
        let skill_id = u16::from_le_bytes([slice[0], slice[1]]);
        let target_id = u32::from_le_bytes([slice[2], slice[3], slice[4], slice[5]]);
        let target_x = u16::from_le_bytes([slice[6], slice[7]]);
        let target_y = u16::from_le_bytes([slice[8], slice[9]]);
        Some(Self {
            skill_id,
            target_id,
            target_x,
            target_y,
        })
    }
}
