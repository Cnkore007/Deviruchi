use std::io;
use crate::protocol::Packet;

/// 网络传输层连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportState {
    /// 已断开连接
    #[default]
    Disconnected,
    /// 正在连接中
    Connecting,
    /// 已建立连接
    Connected,
}

/// 网络传输层 trait，定义了连接、断开、发送和接收的基本操作
#[async_trait::async_trait]
pub trait NetworkTransport: Send + Sync {
    /// 连接到指定地址和端口
    async fn connect(&mut self, address: &str, port: u16) -> io::Result<()>;
    /// 断开当前连接
    async fn disconnect(&mut self) -> io::Result<()>;
    /// 发送一个协议包
    async fn send(&mut self, packet: &Packet) -> io::Result<()>;
    /// 接收一个协议包
    async fn recv(&mut self) -> io::Result<Packet>;
    /// 获取当前连接状态
    fn state(&self) -> TransportState;
    /// 判断是否已连接
    fn is_connected(&self) -> bool {
        self.state() == TransportState::Connected
    }
}
