//! IPC 客户端模块
//! 通过 Unix Socket 与游戏服务器通信

pub mod protocol;

use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;
use protocol::{RpcRequest, RpcResponse};
use anyhow::{Result, anyhow};

/// Unix Socket IPC 客户端
///
/// 与游戏服务器的 Agent API 建立连接，
/// 发送 JSON-RPC 请求并接收响应。
pub struct IpcClient {
    /// Unix Socket 文件路径
    socket_path: PathBuf,
    /// 连接流（读半部 + 写半部）
    stream: Mutex<Option<(
        BufReader<OwnedReadHalf>,
        OwnedWriteHalf,
    )>>,
    /// 请求 ID 计数器
    next_id: Mutex<u64>,
}

impl IpcClient {
    /// 创建新的 IPC 客户端
    ///
    /// # 参数
    /// - `socket_path`: Unix Socket 文件路径
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            stream: Mutex::new(None),
            next_id: Mutex::new(1),
        }
    }

    /// 连接到游戏服务器
    ///
    /// 建立 Unix Socket 连接并分离读写半部
    pub async fn connect(&self) -> Result<()> {
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| anyhow!("无法连接到游戏服务器 {}: {}", self.socket_path.display(), e))?;
        let (reader, writer) = stream.into_split();
        let mut guard = self.stream.lock().await;
        *guard = Some((BufReader::new(reader), writer));
        Ok(())
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        self.stream.lock().await.is_some()
    }

    /// 发送 RPC 请求并等待响应
    ///
    /// # 参数
    /// - `method`: 要调用的方法名
    /// - `params`: 方法参数（JSON 值）
    ///
    /// # 返回
    /// 服务器返回的 RPC 响应
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<RpcResponse> {
        let mut guard = self.stream.lock().await;
        let (reader, writer) = guard.as_mut()
            .ok_or(anyhow!("未连接到游戏服务器，请先执行 /connect"))?;

        // 分配唯一请求 ID
        let mut id_guard = self.next_id.lock().await;
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        // 构造并发送请求
        let request = RpcRequest { id, method: method.to_string(), params };
        let mut request_str = serde_json::to_string(&request)?;
        request_str.push('\n');
        writer.write_all(request_str.as_bytes()).await?;

        // 读取响应
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: RpcResponse = serde_json::from_str(response_line.trim())?;
        Ok(response)
    }

    /// 断开连接
    pub async fn disconnect(&self) {
        let mut guard = self.stream.lock().await;
        *guard = None;
    }
}
