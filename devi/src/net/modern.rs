//! WebSocket 传输层实现
//!
//! 使用 WebSocket 协议进行通信，适用于支持 WebSocket 的现代服务器。
//! 数据以 Binary 消息类型传输，内部使用 PacketCodec 编解码。

use std::io;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::transport::{NetworkTransport, TransportState};
use super::codec::PacketCodec;
use crate::protocol::Packet;

/// WebSocket 流类型别名
type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// WebSocket 传输层
///
/// 实现了基于 WebSocket 的网络传输，适用于现代 Web 端或支持 WS 协议的服务器。
/// 所有协议包以 Binary 消息形式发送和接收。
pub struct ModernTransport {
    /// WebSocket 连接流
    stream: Option<WsStream>,
    /// 当前连接状态
    state: TransportState,
}

impl ModernTransport {
    /// 创建一个新的 WebSocket 传输层实例
    pub fn new() -> Self {
        Self {
            stream: None,
            state: TransportState::Disconnected,
        }
    }
}

#[async_trait::async_trait]
impl NetworkTransport for ModernTransport {
    /// 连接到指定的 WebSocket 地址和端口
    ///
    /// 构造 `ws://{address}:{port}` URL 并发起 WebSocket 握手。
    async fn connect(&mut self, address: &str, port: u16) -> io::Result<()> {
        self.state = TransportState::Connecting;
        let url = format!("ws://{}:{}", address, port);
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
        self.stream = Some(ws_stream);
        self.state = TransportState::Connected;
        tracing::info!("WebSocket 已连接到 {}:{}", address, port);
        Ok(())
    }

    /// 关闭 WebSocket 连接
    async fn disconnect(&mut self) -> io::Result<()> {
        if let Some(mut stream) = self.stream.take() {
            // 发送关闭帧，忽略可能的错误（连接可能已经断开）
            let _ = stream.close(None).await;
        }
        self.state = TransportState::Disconnected;
        Ok(())
    }

    /// 通过 WebSocket 发送一个协议包
    ///
    /// 使用 PacketCodec 将包编码为字节序列，封装为 Binary 消息发送。
    async fn send(&mut self, packet: &Packet) -> io::Result<()> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;
        let data = PacketCodec::encode(packet)?;
        stream
            .send(Message::Binary(data))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))
    }

    /// 从 WebSocket 接收一个协议包
    ///
    /// 循环读取消息，跳过非 Binary 类型的消息（如 Ping/Pong/Text），
    /// 遇到 Binary 消息时通过 PacketCodec 解码为协议包。
    /// 遇到 Close 消息或连接关闭时返回错误。
    async fn recv(&mut self) -> io::Result<Packet> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;
        loop {
            match stream.next().await {
                // 收到 Binary 消息，解码为协议包
                Some(Ok(Message::Binary(data))) => {
                    return PacketCodec::decode(&data);
                }
                // 收到关闭帧，标记为已断开
                Some(Ok(Message::Close(_))) => {
                    self.state = TransportState::Disconnected;
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "服务器关闭连接",
                    ));
                }
                // 收到错误，直接返回
                Some(Err(e)) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
                }
                // 流结束（None），标记为已断开
                None => {
                    self.state = TransportState::Disconnected;
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "连接已关闭",
                    ));
                }
                // 跳过其他消息类型（Ping、Pong、Text 等）
                _ => continue,
            }
        }
    }

    /// 获取当前连接状态
    fn state(&self) -> TransportState {
        self.state
    }
}
