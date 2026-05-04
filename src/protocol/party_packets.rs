use super::packet_builder::{Packed, PacketBuilder, parse_fixed_string, parse_string};

/// 客户端创建队伍 (0x0100)
#[derive(Debug, Clone)]
pub struct CZMakeParty {
    pub party_name: String,
}

impl Packed for CZMakeParty {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0100)
            .put_fixed_str(&self.party_name, 24)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let party_name = parse_fixed_string(slice, &mut offset, 24)?;
        Some(Self { party_name })
    }
}

/// 客户端邀请加入队伍 (0x0101)
#[derive(Debug, Clone)]
pub struct CZReqPartyInvite {
    pub target_account_id: u32,
}

impl Packed for CZReqPartyInvite {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0101)
            .put_u32(self.target_account_id)
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 4 {
            return None;
        }
        let target_account_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        Some(Self { target_account_id })
    }
}

/// 客户端回应组队邀请 (0x0102)
#[derive(Debug, Clone)]
pub struct CZReqPartyJoin {
    pub party_id: u32,
    pub accept: bool,
}

impl Packed for CZReqPartyJoin {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0102)
            .put_u32(self.party_id)
            .put_u8(if self.accept { 1 } else { 0 })
            .build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() < 5 {
            return None;
        }
        let party_id = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
        let accept = slice[4] != 0;
        Some(Self { party_id, accept })
    }
}

/// 客户端离开队伍 (0x0103)
#[derive(Debug, Clone)]
pub struct CZLeaveParty;

impl Packed for CZLeaveParty {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0103).build()
    }

    fn from_slice(_slice: &[u8]) -> Option<Self> {
        Some(Self)
    }
}

/// 客户端队伍聊天 (0x0109)
#[derive(Debug, Clone)]
pub struct CZPartyChat {
    pub message: String,
}

impl Packed for CZPartyChat {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x0109).put_str(&self.message).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let message = parse_string(slice, &mut offset)?;
        Some(Self { message })
    }
}

/// 客户端地图聊天 (0x010C)
#[derive(Debug, Clone)]
pub struct CZChatMessage {
    pub message: String,
}

impl Packed for CZChatMessage {
    fn to_packet(&self) -> Vec<u8> {
        PacketBuilder::new(0x010C).put_str(&self.message).build()
    }

    fn from_slice(slice: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let message = parse_string(slice, &mut offset)?;
        Some(Self { message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cz_make_party_parse() {
        let mut data = vec![0u8; 24];
        data[..10].copy_from_slice(b"TestParty\0");
        let pkt = CZMakeParty::from_slice(&data).unwrap();
        assert_eq!(pkt.party_name, "TestParty");
    }

    #[test]
    fn test_cz_req_party_join_parse_accept() {
        let data = vec![1, 0, 0, 0, 1];
        let pkt = CZReqPartyJoin::from_slice(&data).unwrap();
        assert_eq!(pkt.party_id, 1);
        assert!(pkt.accept);
    }

    #[test]
    fn test_cz_req_party_join_parse_decline() {
        let data = vec![1, 0, 0, 0, 0];
        let pkt = CZReqPartyJoin::from_slice(&data).unwrap();
        assert_eq!(pkt.party_id, 1);
        assert!(!pkt.accept);
    }
}
