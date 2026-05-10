//! IPC 客户端模块
//! 通过 TCP 连接与游戏服务器通信（跨平台兼容）

pub mod protocol;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;
use protocol::{RpcRequest, RpcResponse};
use anyhow::{Result, anyhow};

/// IPC 客户端
///
/// 与游戏服务器的 Agent API 建立 TCP 连接，
/// 发送 JSON-RPC 请求并接收响应。
/// 使用 TCP 而非 Unix Socket 以支持 Windows 跨平台。
pub struct IpcClient {
    /// 服务器地址（如 "127.0.0.1:16400"）
    addr: String,
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
    /// - `addr`: 服务器地址（如 "127.0.0.1:16400"）
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            stream: Mutex::new(None),
            next_id: Mutex::new(1),
        }
    }

    /// 连接到游戏服务器
    pub async fn connect(&self) -> Result<()> {
        let stream = TcpStream::connect(&self.addr).await
            .map_err(|e| anyhow!("无法连接到游戏服务器 {}: {}", self.addr, e))?;
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
