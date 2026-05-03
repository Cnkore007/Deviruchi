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
impl_put!(u64, put_u64);
impl_put!(i16, put_i16);
impl_put!(i32, put_i32);
impl_put!(i64, put_i64);

impl PacketBuilderCtx {
    pub fn put_str(mut self, s: &str) -> Self {
        self.data.put_slice(s.as_bytes());
        self
    }

    pub fn put_fixed_str(mut self, s: &str, len: usize) -> Self {
        let bytes = s.as_bytes();
        let write_len = bytes.len().min(len - 1);
        self.data.put_slice(&bytes[..write_len]);
        // Pad with null bytes
        for _ in write_len..len {
            self.data.put_u8(0);
        }
        self
    }

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
    if *offset >= buf.len() {
        return None;
    }
    let end = (*offset + len).min(buf.len());
    let s = String::from_utf8(buf[*offset..end].to_vec()).ok()?;
    *offset = end;
    Some(s.trim_end_matches('\0').to_string())
}
