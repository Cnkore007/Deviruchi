//! 网络会话管理器
//!
//! 使用 tokio 通道桥接异步网络操作和 Bevy ECS 系统。
//! Bevy 系统通过 `NetworkManager` 资源发送命令，后台 tokio 任务
//! 负责实际的网络 I/O，并将结果通过事件通道返回给 ECS。
//!
//! 设计要点：
//! - `poll_events()` 是同步方法，可直接在 Bevy 系统中调用
//! - 内部使用 `std::sync::Mutex` 保护事件接收通道
//! - 网络 I/O 在独立的 tokio 任务中运行

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use bevy::prelude::*;

use crate::net::transport::NetworkTransport;
use crate::net::legacy::LegacyTransport;
use crate::net::modern::ModernTransport;
use crate::protocol::Packet;

// ============================================================================
// 命令和事件定义
// ============================================================================

/// 发送到网络后台任务的命令
#[derive(Debug)]
pub enum NetworkCommand {
    /// 连接到服务器
    Connect { address: String, port: u16 },
    /// 发送协议包
    Send(Packet),
    /// 断开连接
    Disconnect,
}

/// 从网络后台任务返回的事件
#[derive(Debug, Event)]
pub enum NetworkEvent {
    /// 连接成功
    Connected,
    /// 连接失败
    ConnectFailed(String),
    /// 收到协议包
    PacketReceived(Packet),
    /// 接收错误
    RecvError(String),
    /// 连接已断开
    Disconnected,
}

// ============================================================================
// NetworkManager 资源
// ============================================================================

/// 网络管理器，作为 Bevy 资源插入到 ECS 中
///
/// 持有命令发送通道和事件接收通道。
/// Bevy 系统通过 `send_command()` 发送网络命令，
/// 通过 `poll_events()` 接收网络事件（同步、非阻塞）。
#[derive(Resource)]
pub struct NetworkManager {
    /// 发送命令到后台任务的通道
    cmd_tx: mpsc::UnboundedSender<NetworkCommand>,
    /// 从后台任务接收事件的通道（用 std::sync::Mutex 保护，支持同步访问）
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<NetworkEvent>>>,
}

impl NetworkManager {
    /// 创建新的网络管理器，同时启动后台网络任务
    ///
    /// 根据 `protocol` 参数选择传输层实现：
    /// - `"legacy"` → TCP
    /// - 其他 → WebSocket
    pub fn new(protocol: &str) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // 根据协议类型选择传输层，启动后台任务
        match protocol {
            "legacy" => {
                let transport = LegacyTransport::new();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(Self::run_network_loop(transport, cmd_rx, event_tx));
                });
            }
            _ => {
                let transport = ModernTransport::new();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(Self::run_network_loop(transport, cmd_rx, event_tx));
                });
            }
        }

        Self {
            cmd_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        }
    }

    /// 发送网络命令到后台任务
    pub fn send_command(&self, cmd: NetworkCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    /// 轮询所有待处理的网络事件（同步、非阻塞）
    ///
    /// 返回所有已到达的事件列表。无事件时返回空列表。
    /// 可安全地在 Bevy 系统中调用。
    pub fn poll_events(&self) -> Vec<NetworkEvent> {
        let mut rx = self.event_rx.lock().unwrap();
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// 后台网络任务主循环
    ///
    /// 在独立的 tokio 运行时中运行，处理所有异步网络 I/O。
    async fn run_network_loop(
        transport: impl NetworkTransport + 'static,
        mut cmd_rx: mpsc::UnboundedReceiver<NetworkCommand>,
        event_tx: mpsc::UnboundedSender<NetworkEvent>,
    ) {
        let transport = Arc::new(tokio::sync::Mutex::new(transport));
        let transport_recv = Arc::clone(&transport);
        let event_tx_recv = event_tx.clone();

        // 启动接收循环
        let recv_handle = tokio::spawn(async move {
            loop {
                // 等待连接建立
                {
                    let t = transport_recv.lock().await;
                    if !t.is_connected() {
                        drop(t);
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        continue;
                    }
                }

                // 接收一个包
                let result = {
                    let mut t = transport_recv.lock().await;
                    t.recv().await
                };

                match result {
                    Ok(packet) => {
                        if event_tx_recv.send(NetworkEvent::PacketReceived(packet)).is_err() {
                            break; // 接收端已关闭
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::ConnectionAborted
                            || e.kind() == std::io::ErrorKind::NotConnected
                        {
                            let _ = event_tx_recv.send(NetworkEvent::Disconnected);
                            break;
                        }
                        let _ = event_tx_recv.send(NetworkEvent::RecvError(e.to_string()));
                    }
                }
            }
        });

        // 命令处理循环
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                NetworkCommand::Connect { address, port } => {
                    let mut t = transport.lock().await;
                    match t.connect(&address, port).await {
                        Ok(()) => {
                            let _ = event_tx.send(NetworkEvent::Connected);
                        }
                        Err(e) => {
                            let _ = event_tx.send(NetworkEvent::ConnectFailed(e.to_string()));
                        }
                    }
                }
                NetworkCommand::Send(packet) => {
                    let mut t = transport.lock().await;
                    if let Err(e) = t.send(&packet).await {
                        tracing::error!("发送包失败: {}", e);
                    }
                }
                NetworkCommand::Disconnect => {
                    let mut t = transport.lock().await;
                    let _ = t.disconnect().await;
                    let _ = event_tx.send(NetworkEvent::Disconnected);
                }
            }
        }

        recv_handle.abort();
    }
}
