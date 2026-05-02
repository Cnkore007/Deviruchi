use bytes::{BytesMut, BufMut};
use tokio_util::codec::{Decoder, Encoder};
use super::packet::Packet;

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
                    "Invalid packet format"
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
