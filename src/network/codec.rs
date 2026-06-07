use super::packet::Packet;
use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// 单包最大有效载荷（排除 4 字节 header），防止恶意大包消耗内存
const MAX_PACKET_PAYLOAD: usize = 32 * 1024; // 32 KB，rAthena 常规包不超过 8 KB

pub struct PacketCodec;

impl Decoder for PacketCodec {
    type Item = Packet;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 需要至少 4 字节来读取 header
        if src.len() < 4 {
            return Ok(None);
        }

        // 读取长度
        let length = u16::from_le_bytes([src[0], src[1]]) as usize;

        // 包大小上限校验：拒绝超大包，防止内存耗尽攻击
        if length > MAX_PACKET_PAYLOAD + 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Packet too large: {} bytes (max {})", length, MAX_PACKET_PAYLOAD + 4),
            ));
        }

        // 检查是否收到完整数据包
        if src.len() < length {
            return Ok(None);
        }

        // 提取数据包
        let packet_bytes = src.split_to(length);
        let packet = match Packet::from_bytes(&packet_bytes) {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid packet format",
                ));
            }
        };

        Ok(Some(packet))
    }
}

impl Encoder<Vec<u8>> for PacketCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len());
        dst.put_slice(&item);
        Ok(())
    }
}
