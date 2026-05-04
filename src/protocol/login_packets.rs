use super::packet_builder::{Packed, PacketBuilder, parse_fixed_string};

/// 客户端登录请求 (0x0064)
#[derive(Debug, Clone)]
pub struct CALogin {
    pub version: u32,
    pub username: String,
    pub password: String,
}

impl Packed for CALogin {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0064)
            .put_u32(self.version)
            .put_fixed_str(&self.username, 24)
            .put_fixed_str(&self.password, 24)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 + 24 + 24 {
            return None;
        }
        let version = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let mut offset = 4;
        let username = parse_fixed_string(slice, &mut offset, 24)?;
        let password = parse_fixed_string(slice, &mut offset, 24)?;
        Some(Self {
            version,
            username,
            password,
        })
    }
}

/// 服务器接受登录 (0x0069)
#[derive(Debug, Clone)]
pub struct ACAceptLogin {
    pub account_id: u32,
    pub login_id1: u32,
    pub login_id2: u32,
    pub sex: u8,
}

impl Packed for ACAceptLogin {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0069)
            .put_u32(self.account_id)
            .put_u32(self.login_id1)
            .put_u32(self.login_id2)
            .put_u8(self.sex)
            .build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None // 服务器包，不解析
    }
}

/// 服务器拒绝登录 (0x006A)
#[derive(Debug, Clone)]
pub struct ACRefuseLogin {
    pub error_code: u8,
}

impl Packed for ACRefuseLogin {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x006A).put_u8(self.error_code).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 踢出通知 (0x0081)
#[derive(Debug, Clone)]
pub struct SCNotifyBan {
    pub error_code: u32,
}

impl Packed for SCNotifyBan {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0081).put_u32(self.error_code).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        None
    }
}

/// 版本协商请求 (0x200)
#[derive(Debug, Clone)]
pub struct CAConnectInfo {
    pub version: u32,
    pub client_type: u8,
}

impl Packed for CAConnectInfo {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0200)
            .put_u32(self.version)
            .put_u8(self.client_type)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let version = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let client_type = slice[4];
        Some(Self {
            version,
            client_type,
        })
    }
}
