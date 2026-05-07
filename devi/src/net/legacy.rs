//! Legacy TCP 传输层实现
//!
//! 使用原生 TCP 连接进行通信，适用于传统的 rAthena 服务器。
//! 每个包的格式为: [包ID: 2字节][长度: 2字节][负载: N字节]

use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::transport::{NetworkTransport, TransportState};
use super::codec::PacketCodec;
use crate::protocol::Packet;

/// Legacy TCP 传输层
///
/// 实现了基于原生 TCP 的网络传输，直接读写字节流。
/// 适用于标准的 rAthena/Hercules 服务器协议。
pub struct LegacyTransport {
    /// TCP 连接流
    stream: Option<TcpStream>,
    /// 当前连接状态
    state: TransportState,
}

impl LegacyTransport {
    /// 创建一个新的 Legacy TCP 传输层实例
    pub fn new() -> Self {
        Self {
            stream: None,
            state: TransportState::Disconnected,
        }
    }
}

#[async_trait::async_trait]
impl NetworkTransport for LegacyTransport {
    /// 连接到指定的 TCP 地址和端口
    async fn connect(&mut self, address: &str, port: u16) -> io::Result<()> {
        self.state = TransportState::Connecting;
        let addr: SocketAddr = format!("{}:{}", address, port)
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let stream = TcpStream::connect(addr).await?;
        self.stream = Some(stream);
        self.state = TransportState::Connected;
        tracing::info!("Legacy TCP 已连接到 {}:{}", address, port);
        Ok(())
    }

    /// 断开当前 TCP 连接
    async fn disconnect(&mut self) -> io::Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.shutdown().await?;
        }
        self.state = TransportState::Disconnected;
        Ok(())
    }

    /// 通过 TCP 发送一个协议包
    ///
    /// 使用 PacketCodec 将包编码为字节序列，然后写入 TCP 流。
    async fn send(&mut self, packet: &Packet) -> io::Result<()> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;
        let data = PacketCodec::encode(packet)?;
        stream.write_all(&data).await?;
        stream.flush().await?;
        Ok(())
    }

    /// 从 TCP 流接收一个协议包
    ///
    /// 先读取 4 字节头部（包ID + 长度），再根据长度读取剩余数据，
    /// 最后通过 PacketCodec 解码为协议包。
    async fn recv(&mut self) -> io::Result<Packet> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;
        // 读取 4 字节头部: [包ID: 2字节][长度: 2字节]
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).await?;
        // 解析包总长度（包含头部）
        let packet_len = u16::from_le_bytes([header[2], header[3]]) as usize;
        if packet_len < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("包长度太小: {}", packet_len),
            ));
        }
        // 读取包体（总长度减去头部 4 字节）
        let mut body = vec![0u8; packet_len - 4];
        stream.read_exact(&mut body).await?;
        // 拼接完整包数据并解码
        let mut full_packet = Vec::with_capacity(packet_len);
        full_packet.extend_from_slice(&header);
        full_packet.extend_from_slice(&body);
        PacketCodec::decode(&full_packet)
    }

    /// 获取当前连接状态
    fn state(&self) -> TransportState {
        self.state
    }
}
