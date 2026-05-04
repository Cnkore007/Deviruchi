//! 协议层 - 数据包定义与构造

pub mod char_packets;
pub mod guild_packets;
pub mod login_packets;
pub mod map_packets;
pub mod packet_builder;
pub mod party_packets;
pub mod storage_packets;
pub mod teleport_packets;
pub mod trade_packets;

pub use packet_builder::{Packed, PacketBuilder, parse_fixed_string, parse_string};
