use bytes::{BufMut, BytesMut};

pub trait Packed {
    fn to_packet(&self) -> Vec<u8>;
    fn from_slice(slice: &[u8]) -> Option<Self>
    where
        Self: Sized;
}

pub struct PacketBuilder;

impl PacketBuilder {
    pub fn new(packet_id: u16) -> PacketBuilderCtx {
        PacketBuilderCtx {
            packet_id,
            data: BytesMut::with_capacity(256),
        }
    }
}

pub struct PacketBuilderCtx {
    packet_id: u16,
    data: BytesMut,
}

macro_rules! impl_put {
    ($ty:ty, $method:ident) => {
        impl PacketBuilderCtx {
            pub fn $method(mut self, val: $ty) -> Self {
                self.data.$method(val);
                self
            }
        }
    };
}

impl_put!(u8, put_u8);
impl_put!(u16, put_u16);
impl_put!(u32, put_u32);
impl_put!(i32, put_i32);
impl_put!(i64, put_i64);
impl_put!(&str, put_str);

impl PacketBuilderCtx {
    pub fn put_slice(mut self, slice: &[u8]) -> Self {
        self.data.put_slice(slice);
        self
    }

    pub fn build(self) -> Vec<u8> {
        let len = self.data.len() + 4; // header size
        let mut buf = BytesMut::with_capacity(len);
        buf.put_u16_le(len as u16);
        buf.put_u16_le(self.packet_id);
        buf.put_slice(&self.data);
        buf.to_vec()
    }
}

pub fn parse_string(buf: &[u8], offset: &mut usize) -> Option<String> {
    let end = buf[*offset..].iter().position(|&b| b == 0)? + *offset;
    let s = String::from_utf8(buf[*offset..end].to_vec()).ok()?;
    *offset = end + 1;
    Some(s)
}

pub fn parse_fixed_string(buf: &[u8], offset: &mut usize, len: usize) -> Option<String> {
    let end = (*offset + len).min(buf.len());
    let s = String::from_utf8(buf[*offset..end].to_vec()).ok()?;
    *offset = end;
    Some(s.trim_end_matches('\0').to_string())
}
