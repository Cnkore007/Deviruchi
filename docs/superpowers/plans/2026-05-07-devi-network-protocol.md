# Devi 客户端网络层与协议实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现双协议网络层（Legacy TCP + WebSocket），定义协议包结构，实现登录/选角色/地图通信流程。

**Architecture:** 定义统一的 `NetworkTransport` trait，Legacy TCP 和 WebSocket 各自实现。协议包使用枚举定义，codec 层负责编解码，handler 层负责分发到 ECS 系统。

**Tech Stack:** Rust, tokio (异步), tokio-tungstenite (WebSocket), byteorder, bevy_ecs

---

## 文件结构

| 操作 | 文件路径 | 职责 |
|------|----------|------|
| Modify | `devi/src/net/mod.rs` | 网络模块声明和通用类型 |
| Create | `devi/src/net/transport.rs` | NetworkTransport trait 定义 |
| Create | `devi/src/net/legacy.rs` | Legacy TCP 协议实现 |
| Create | `devi/src/net/modern.rs` | WebSocket 协议实现 |
| Create | `devi/src/net/codec.rs` | 包编解码（共享） |
| Create | `devi/src/net/handler.rs` | 包分发处理 |
| Modify | `devi/src/protocol/mod.rs` | 协议包枚举定义 |
| Create | `devi/src/protocol/login.rs` | 登录包定义 |
| Create | `devi/src/protocol/char.rs` | 选角色包定义 |
| Create | `devi/src/protocol/map.rs` | 地图包定义 |
| Create | `devi/tests/net_test.rs` | 网络层测试 |

---

### Task 1: 协议包定义

**Files:**
- Modify: `devi/src/protocol/mod.rs`
- Create: `devi/src/protocol/login.rs`
- Create: `devi/src/protocol/char.rs`
- Create: `devi/src/protocol/map.rs`

- [ ] **Step 1: 编写协议包测试**

创建 `devi/tests/net_test.rs`:
```rust
use devi::protocol::Packet;
use devi::protocol::login::{LoginRequest, LoginResponse};
use devi::protocol::char::{CharSelectRequest, CharListResponse};

#[test]
fn test_login_request_packet_id() {
    let req = LoginRequest {
        username: "test".to_string(),
        password: "pass".to_string(),
    };
    let packet = Packet::LoginRequest(req);
    assert_eq!(packet.packet_id(), 0x0064);
}

#[test]
fn test_login_response_success() {
    let resp = LoginResponse::Success {
        login_id: 12345,
        account_id: 67890,
        session_key: [0u8; 16],
    };
    let packet = Packet::LoginResponse(resp);
    assert_eq!(packet.packet_id(), 0x0069);
}

#[test]
fn test_char_list_request() {
    let req = CharSelectRequest {
        server_index: 0,
    };
    let packet = Packet::CharSelectRequest(req);
    assert_eq!(packet.packet_id(), 0x0065);
}

#[test]
fn test_char_list_response() {
    let resp = CharListResponse {
        chars: vec![],
    };
    let packet = Packet::CharListResponse(resp);
    assert_eq!(packet.packet_id(), 0x006b);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test net_test`
Expected: FAIL，编译错误

- [ ] **Step 3: 实现协议模块**

修改 `devi/src/protocol/mod.rs`:
```rust
// 协议定义模块
pub mod login;
pub mod char;
pub mod map;

/// 所有协议包的统一枚举
#[derive(Debug, Clone)]
pub enum Packet {
    // ===== 登录包 =====
    LoginRequest(login::LoginRequest),
    LoginResponse(login::LoginResponse),

    // ===== 选角色包 =====
    CharSelectRequest(char::CharSelectRequest),
    CharListResponse(char::CharListResponse),
    CharCreateRequest(char::CharCreateRequest),
    CharCreateResponse(char::CharCreateResponse),
    CharDeleteRequest(char::CharDeleteRequest),
    CharDeleteResponse(char::CharDeleteResponse),
    CharEnterRequest(char::CharEnterRequest),
    CharEnterResponse(char::CharEnterResponse),

    // ===== 地图包 =====
    MapEnter(map::MapEnterRequest),
    MapEntered(map::MapEnteredResponse),
    PlayerMove(map::PlayerMoveRequest),
    EntityMove(map::EntityMoveNotify),
    ChatMessage(map::ChatMessage),
    EntityAppear(map::EntityAppearNotify),
    EntityDisappear(map::EntityDisappearNotify),
}

impl Packet {
    /// 获取包 ID
    pub fn packet_id(&self) -> u16 {
        match self {
            Packet::LoginRequest(_) => 0x0064,
            Packet::LoginResponse(_) => 0x0069,
            Packet::CharSelectRequest(_) => 0x0065,
            Packet::CharListResponse(_) => 0x006b,
            Packet::CharCreateRequest(_) => 0x0067,
            Packet::CharCreateResponse(_) => 0x006d,
            Packet::CharDeleteRequest(_) => 0x0068,
            Packet::CharDeleteResponse(_) => 0x006e,
            Packet::CharEnterRequest(_) => 0x0066,
            Packet::CharEnterResponse(_) => 0x0071,
            Packet::MapEnter(_) => 0x0072,
            Packet::MapEntered(_) => 0x0073,
            Packet::PlayerMove(_) => 0x0085,
            Packet::EntityMove(_) => 0x0086,
            Packet::ChatMessage(_) => 0x008c,
            Packet::EntityAppear(_) => 0x0078,
            Packet::EntityDisappear(_) => 0x007a,
        }
    }

    /// 是否是登录阶段包
    pub fn is_login_packet(&self) -> bool {
        matches!(self,
            Packet::LoginRequest(_) |
            Packet::LoginResponse(_)
        )
    }

    /// 是否是选角色阶段包
    pub fn is_char_packet(&self) -> bool {
        matches!(self,
            Packet::CharSelectRequest(_) |
            Packet::CharListResponse(_) |
            Packet::CharCreateRequest(_) |
            Packet::CharCreateResponse(_) |
            Packet::CharDeleteRequest(_) |
            Packet::CharDeleteResponse(_) |
            Packet::CharEnterRequest(_) |
            Packet::CharEnterResponse(_)
        )
    }

    /// 是否是地图阶段包
    pub fn is_map_packet(&self) -> bool {
        matches!(self,
            Packet::MapEnter(_) |
            Packet::MapEntered(_) |
            Packet::PlayerMove(_) |
            Packet::EntityMove(_) |
            Packet::ChatMessage(_) |
            Packet::EntityAppear(_) |
            Packet::EntityDisappear(_)
        )
    }
}
```

- [ ] **Step 4: 实现登录包**

创建 `devi/src/protocol/login.rs`:
```rust
/// 登录请求
#[derive(Debug, Clone)]
pub struct LoginRequest {
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
}

/// 登录响应
#[derive(Debug, Clone)]
pub enum LoginResponse {
    /// 登录成功
    Success {
        login_id: u32,
        account_id: u32,
        session_key: [u8; 16],
    },
    /// 登录失败
    Failure {
        error_code: u8,
    },
}
```

- [ ] **Step 5: 实现选角色包**

创建 `devi/src/protocol/char.rs`:
```rust
/// 选角色服务器请求
#[derive(Debug, Clone)]
pub struct CharSelectRequest {
    /// 服务器索引
    pub server_index: u16,
}

/// 角色列表响应
#[derive(Debug, Clone)]
pub struct CharListResponse {
    /// 角色列表
    pub chars: Vec<CharInfo>,
}

/// 角色信息
#[derive(Debug, Clone)]
pub struct CharInfo {
    /// 角色 ID
    pub char_id: u32,
    /// 基础等级
    pub base_level: u32,
    /// 职业等级
    pub job_level: u32,
    /// 角色名称
    pub name: String,
    /// 职业 ID
    pub job_id: u16,
    /// 地图名
    pub map_name: String,
}

/// 创建角色请求
#[derive(Debug, Clone)]
pub struct CharCreateRequest {
    /// 角色名称
    pub name: String,
    /// 职业 ID（初始为初心者）
    pub job_id: u16,
    /// 头发样式
    pub hair_style: u8,
    /// 头发颜色
    pub hair_color: u8,
}

/// 创建角色响应
#[derive(Debug, Clone)]
pub enum CharCreateResponse {
    /// 创建成功
    Success(CharInfo),
    /// 创建失败
    Failure { error_code: u8 },
}

/// 删除角色请求
#[derive(Debug, Clone)]
pub struct CharDeleteRequest {
    /// 角色 ID
    pub char_id: u32,
    /// 邮箱验证
    pub email: String,
}

/// 删除角色响应
#[derive(Debug, Clone)]
pub enum CharDeleteResponse {
    Success,
    Failure { error_code: u8 },
}

/// 进入游戏请求
#[derive(Debug, Clone)]
pub struct CharEnterRequest {
    /// 角色 ID
    pub char_id: u32,
}

/// 进入游戏响应
#[derive(Debug, Clone)]
pub enum CharEnterResponse {
    /// 成功，返回地图服务器信息
    Success {
        map_server_ip: String,
        map_server_port: u16,
        char_id: u32,
        session_key: [u8; 16],
    },
    Failure { error_code: u8 },
}
```

- [ ] **Step 6: 实现地图包**

创建 `devi/src/protocol/map.rs`:
```rust
/// 进入地图请求
#[derive(Debug, Clone)]
pub struct MapEnterRequest {
    /// 角色 ID
    pub char_id: u32,
    /// 登录 ID
    pub login_id: u32,
    /// 客户端时间戳
    pub client_tick: u32,
    /// 性别
    pub gender: u8,
}

/// 进入地图响应
#[derive(Debug, Clone)]
pub struct MapEnteredResponse {
    /// 角色 ID
    pub char_id: u32,
    /// 当前地图名
    pub map_name: String,
    /// 位置 X
    pub pos_x: u16,
    /// 位置 Y
    pub pos_y: u16,
}

/// 玩家移动请求
#[derive(Debug, Clone)]
pub struct PlayerMoveRequest {
    /// 目标 X
    pub dest_x: u16,
    /// 目标 Y
    pub dest_y: u16,
}

/// 实体移动通知
#[derive(Debug, Clone)]
pub struct EntityMoveNotify {
    /// 实体 ID
    pub entity_id: u32,
    /// 起始 X
    pub from_x: u16,
    /// 起始 Y
    pub from_y: u16,
    /// 目标 X
    pub dest_x: u16,
    /// 目标 Y
    pub dest_y: u16,
    /// 移动速度
    pub speed: u16,
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// 发送者 ID
    pub sender_id: u32,
    /// 发送者名称
    pub sender_name: String,
    /// 消息内容
    pub message: String,
}

/// 实体出现通知
#[derive(Debug, Clone)]
pub struct EntityAppearNotify {
    /// 实体 ID
    pub entity_id: u32,
    /// 实体类型（0=玩家, 5=怪物, 6=NPC）
    pub entity_type: u8,
    /// 位置 X
    pub pos_x: u16,
    /// 位置 Y
    pub pos_y: u16,
    /// 方向
    pub direction: u8,
    /// 外观 ID
    pub look: u16,
}

/// 实体消失通知
#[derive(Debug, Clone)]
pub struct EntityDisappearNotify {
    /// 实体 ID
    pub entity_id: u32,
    /// 消失原因（0=走出视野, 1=死亡）
    pub reason: u8,
}
```

- [ ] **Step 7: 运行测试确认通过**

Run: `cd devi && cargo test --test net_test`
Expected: 4 tests PASS

- [ ] **Step 8: Commit**

```bash
git add devi/src/protocol/
git commit -m "feat(devi/protocol): 定义协议包枚举和登录/选角色/地图包结构"
```

---

### Task 2: NetworkTransport trait

**Files:**
- Modify: `devi/src/net/mod.rs`
- Create: `devi/src/net/transport.rs`
- Modify: `devi/tests/net_test.rs`

- [ ] **Step 1: 编写 transport 测试**

在 `devi/tests/net_test.rs` 中追加:
```rust
use devi::net::transport::TransportState;

#[test]
fn test_transport_state_default() {
    let state = TransportState::default();
    assert_eq!(state, TransportState::Disconnected);
}

#[test]
fn test_transport_state_transitions() {
    let mut state = TransportState::default();
    assert_eq!(state, TransportState::Disconnected);

    state = TransportState::Connecting;
    assert_eq!(state, TransportState::Connecting);

    state = TransportState::Connected;
    assert_eq!(state, TransportState::Connected);

    state = TransportState::Disconnected;
    assert_eq!(state, TransportState::Disconnected);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test net_test`
Expected: FAIL，`unresolved import devi::net::transport`

- [ ] **Step 3: 实现 transport trait**

修改 `devi/src/net/mod.rs`:
```rust
// 网络模块
pub mod transport;
pub mod legacy;
pub mod modern;
pub mod codec;
pub mod handler;
```

创建 `devi/src/net/transport.rs`:
```rust
use std::io;
use devi::protocol::Packet;

/// 传输层连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportState {
    /// 未连接
    #[default]
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
}

/// 网络传输层 trait
/// Legacy TCP 和 WebSocket 各自实现此 trait
#[async_trait::async_trait]
pub trait NetworkTransport: Send + Sync {
    /// 连接到服务器
    async fn connect(&mut self, address: &str, port: u16) -> io::Result<()>;

    /// 断开连接
    async fn disconnect(&mut self) -> io::Result<()>;

    /// 发送包
    async fn send(&mut self, packet: &Packet) -> io::Result<()>;

    /// 接收包
    async fn recv(&mut self) -> io::Result<Packet>;

    /// 获取当前连接状态
    fn state(&self) -> TransportState;

    /// 是否已连接
    fn is_connected(&self) -> bool {
        self.state() == TransportState::Connected
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test net_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/net/mod.rs devi/src/net/transport.rs devi/tests/net_test.rs
git commit -m "feat(devi/net): 定义 NetworkTransport trait 和连接状态枚举"
```

---

### Task 3: 包编解码器

**Files:**
- Create: `devi/src/net/codec.rs`
- Modify: `devi/tests/net_test.rs`

- [ ] **Step 1: 编写 codec 测试**

在 `devi/tests/net_test.rs` 中追加:
```rust
use devi::net::codec::PacketCodec;
use devi::protocol::Packet;
use devi::protocol::login::LoginRequest;

#[test]
fn test_encode_login_request() {
    let req = LoginRequest {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };
    let packet = Packet::LoginRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();

    // 前 2 字节是包 ID (0x0064, little-endian)
    assert_eq!(encoded[0], 0x64);
    assert_eq!(encoded[1], 0x00);

    // 接下来 2 字节是包长度
    let len = u16::from_le_bytes([encoded[2], encoded[3]]);
    assert_eq!(len as usize, encoded.len());
}

#[test]
fn test_decode_login_request() {
    let req = LoginRequest {
        username: "test".to_string(),
        password: "pass".to_string(),
    };
    let packet = Packet::LoginRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    let decoded = PacketCodec::decode(&encoded).unwrap();

    match decoded {
        Packet::LoginRequest(decoded_req) => {
            assert_eq!(decoded_req.username, "test");
            assert_eq!(decoded_req.password, "pass");
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

#[test]
fn test_decode_invalid_packet() {
    let data = vec![0xFF, 0xFF, 0x00, 0x00]; // 无效包 ID
    let result = PacketCodec::decode(&data);
    assert!(result.is_err());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test net_test`
Expected: FAIL，`unresolved import devi::net::codec`

- [ ] **Step 3: 实现 codec**

创建 `devi/src/net/codec.rs`:
```rust
use std::io;
use devi::protocol::Packet;
use devi::protocol::login::{LoginRequest, LoginResponse};

/// 包编解码器
pub struct PacketCodec;

impl PacketCodec {
    /// 编码包为字节
    pub fn encode(packet: &Packet) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();

        // 写入包 ID (2 字节, little-endian)
        let id = packet.packet_id();
        buf.extend_from_slice(&id.to_le_bytes());

        match packet {
            Packet::LoginRequest(req) => {
                // 用户名 (24 字节, null-padded)
                let mut username = [0u8; 24];
                let bytes = req.username.as_bytes();
                let len = bytes.len().min(23); // 留 1 字节给 null
                username[..len].copy_from_slice(&bytes[..len]);
                buf.extend_from_slice(&username);

                // 密码 (24 字节, null-padded, 可加密)
                let mut password = [0u8; 24];
                let bytes = req.password.as_bytes();
                let len = bytes.len().min(23);
                password[..len].copy_from_slice(&bytes[..len]);
                buf.extend_from_slice(&password);
            }
            _ => {
                // 其他包类型暂时只写包 ID
                // 后续逐步实现
            }
        }

        // 回填包长度 (第 2-3 字节)
        let len = buf.len() as u16;
        buf[2] = (len & 0xFF) as u8;
        buf[3] = ((len >> 8) & 0xFF) as u8;

        Ok(buf)
    }

    /// 解码字节为包
    pub fn decode(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "包数据太短",
            ));
        }

        let packet_id = u16::from_le_bytes([data[0], data[1]]);
        let _packet_len = u16::from_le_bytes([data[2], data[3]]);

        match packet_id {
            0x0064 => {
                // LoginRequest
                if data.len() < 52 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "LoginRequest 数据不完整",
                    ));
                }
                let username = Self::read_null_padded_string(&data[4..28]);
                let password = Self::read_null_padded_string(&data[28..52]);
                Ok(Packet::LoginRequest(LoginRequest { username, password }))
            }
            0x0069 => {
                // LoginResponse
                if data.len() < 5 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "LoginResponse 数据不完整",
                    ));
                }
                let status = data[4];
                if status == 0 {
                    Ok(Packet::LoginResponse(LoginResponse::Failure {
                        error_code: data.get(5).copied().unwrap_or(0),
                    }))
                } else {
                    Ok(Packet::LoginResponse(LoginResponse::Success {
                        login_id: u32::from_le_bytes([
                            data[5], data[6], data[7], data[8],
                        ]),
                        account_id: u32::from_le_bytes([
                            data[9], data[10], data[11], data[12],
                        ]),
                        session_key: data[13..29].try_into().unwrap_or([0u8; 16]),
                    }))
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("未知包 ID: 0x{:04X}", packet_id),
            )),
        }
    }

    /// 读取 null 填充的字符串
    fn read_null_padded_string(data: &[u8]) -> String {
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        String::from_utf8_lossy(&data[..end]).to_string()
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test net_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/net/codec.rs devi/tests/net_test.rs
git commit -m "feat(devi/net): 实现包编解码器，支持 LoginRequest/LoginResponse"
```

---

### Task 4: Legacy TCP 传输实现

**Files:**
- Create: `devi/src/net/legacy.rs`

- [ ] **Step 1: 实现 Legacy TCP 传输**

创建 `devi/src/net/legacy.rs`:
```rust
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::transport::{NetworkTransport, TransportState};
use super::codec::PacketCodec;
use devi::protocol::Packet;

/// Legacy TCP 传输层（rAthena 协议）
pub struct LegacyTransport {
    stream: Option<TcpStream>,
    state: TransportState,
    recv_buf: Vec<u8>,
}

impl LegacyTransport {
    /// 创建新的 Legacy 传输实例
    pub fn new() -> Self {
        Self {
            stream: None,
            state: TransportState::Disconnected,
            recv_buf: Vec::with_capacity(8192),
        }
    }
}

#[async_trait::async_trait]
impl NetworkTransport for LegacyTransport {
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

    async fn disconnect(&mut self) -> io::Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.shutdown().await?;
        }
        self.state = TransportState::Disconnected;
        self.recv_buf.clear();
        tracing::info!("Legacy TCP 已断开连接");
        Ok(())
    }

    async fn send(&mut self, packet: &Packet) -> io::Result<()> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;

        let data = PacketCodec::encode(packet)?;
        stream.write_all(&data).await?;
        stream.flush().await?;

        tracing::debug!("发送包: ID=0x{:04X}, 大小={} 字节", packet.packet_id(), data.len());
        Ok(())
    }

    async fn recv(&mut self) -> io::Result<Packet> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;

        // 读取包头（4 字节：ID + 长度）
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).await?;

        let packet_id = u16::from_le_bytes([header[0], header[1]]);
        let packet_len = u16::from_le_bytes([header[2], header[3]]) as usize;

        if packet_len < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("包长度太小: {}", packet_len),
            ));
        }

        // 读取包体
        let mut body = vec![0u8; packet_len - 4];
        stream.read_exact(&mut body).await?;

        // 组合完整包数据
        let mut full_packet = Vec::with_capacity(packet_len);
        full_packet.extend_from_slice(&header);
        full_packet.extend_from_slice(&body);

        let packet = PacketCodec::decode(&full_packet)?;
        tracing::debug!("接收包: ID=0x{:04X}, 大小={} 字节", packet_id, packet_len);
        Ok(packet)
    }

    fn state(&self) -> TransportState {
        self.state
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd devi && cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add devi/src/net/legacy.rs
git commit -m "feat(devi/net): 实现 Legacy TCP 传输层（rAthena 协议）"
```

---

### Task 5: WebSocket 传输实现

**Files:**
- Create: `devi/src/net/modern.rs`

- [ ] **Step 1: 实现 WebSocket 传输**

创建 `devi/src/net/modern.rs`:
```rust
use std::io;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::transport::{NetworkTransport, TransportState};
use super::codec::PacketCodec;
use devi::protocol::Packet;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// WebSocket 传输层（Deviruchi 协议）
pub struct ModernTransport {
    stream: Option<WsStream>,
    state: TransportState,
}

impl ModernTransport {
    /// 创建新的 WebSocket 传输实例
    pub fn new() -> Self {
        Self {
            stream: None,
            state: TransportState::Disconnected,
        }
    }
}

#[async_trait::async_trait]
impl NetworkTransport for ModernTransport {
    async fn connect(&mut self, address: &str, port: u16) -> io::Result<()> {
        self.state = TransportState::Connecting;

        let url = format!("ws://{}:{}", address, port);
        let (ws_stream, _) = connect_async(&url).await.map_err(|e| {
            io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string())
        })?;

        self.stream = Some(ws_stream);
        self.state = TransportState::Connected;

        tracing::info!("WebSocket 已连接到 {}:{}", address, port);
        Ok(())
    }

    async fn disconnect(&mut self) -> io::Result<()> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.close().await;
        }
        self.state = TransportState::Disconnected;
        tracing::info!("WebSocket 已断开连接");
        Ok(())
    }

    async fn send(&mut self, packet: &Packet) -> io::Result<()> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;

        let data = PacketCodec::encode(packet)?;
        stream.send(Message::Binary(data)).await.map_err(|e| {
            io::Error::new(io::ErrorKind::BrokenPipe, e.to_string())
        })?;

        tracing::debug!("WebSocket 发送包: ID=0x{:04X}", packet.packet_id());
        Ok(())
    }

    async fn recv(&mut self) -> io::Result<Packet> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "未连接")
        })?;

        loop {
            match stream.next().await {
                Some(Ok(Message::Binary(data))) => {
                    let packet = PacketCodec::decode(&data)?;
                    tracing::debug!("WebSocket 接收包: ID=0x{:04X}", packet.packet_id());
                    return Ok(packet);
                }
                Some(Ok(Message::Close(_))) => {
                    self.state = TransportState::Disconnected;
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "服务器关闭连接",
                    ));
                }
                Some(Err(e)) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
                }
                None => {
                    self.state = TransportState::Disconnected;
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "连接已关闭",
                    ));
                }
                _ => continue, // 忽略 Ping/Pong/Text 等消息
            }
        }
    }

    fn state(&self) -> TransportState {
        self.state
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd devi && cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add devi/src/net/modern.rs
git commit -m "feat(devi/net): 实现 WebSocket 传输层（Deviruchi 协议）"
```

---

### Task 6: 包处理器

**Files:**
- Create: `devi/src/net/handler.rs`
- Modify: `devi/tests/net_test.rs`

- [ ] **Step 1: 编写 handler 测试**

在 `devi/tests/net_test.rs` 中追加:
```rust
use devi::net::handler::PacketHandler;
use devi::protocol::Packet;
use devi::protocol::login::LoginResponse;

#[test]
fn test_handler_register_and_dispatch() {
    let mut handler = PacketHandler::new();
    let mut called = false;

    handler.on_login_response(|resp| {
        match resp {
            LoginResponse::Success { .. } => {}
            LoginResponse::Failure { .. } => {}
        }
        called = true;
    });

    let packet = Packet::LoginResponse(LoginResponse::Failure { error_code: 1 });
    handler.dispatch(&packet);
    assert!(called);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test net_test`
Expected: FAIL，`unresolved import devi::net::handler`

- [ ] **Step 3: 实现 handler**

创建 `devi/src/net/handler.rs`:
```rust
use devi::protocol::Packet;
use devi::protocol::login::LoginResponse;
use devi::protocol::char::{CharListResponse, CharCreateResponse, CharDeleteResponse, CharEnterResponse};

/// 包处理器回调类型
type LoginResponseCallback = Box<dyn Fn(&LoginResponse) + Send + Sync>;
type CharListCallback = Box<dyn Fn(&CharListResponse) + Send + Sync>;
type CharCreateCallback = Box<dyn Fn(&CharCreateResponse) + Send + Sync>;
type CharDeleteCallback = Box<dyn Fn(&CharDeleteResponse) + Send + Sync>;
type CharEnterCallback = Box<dyn Fn(&CharEnterResponse) + Send + Sync>;

/// 包处理器
/// 注册回调并分发接收到的包
pub struct PacketHandler {
    login_response_cb: Option<LoginResponseCallback>,
    char_list_cb: Option<CharListCallback>,
    char_create_cb: Option<CharCreateCallback>,
    char_delete_cb: Option<CharDeleteCallback>,
    char_enter_cb: Option<CharEnterCallback>,
}

impl PacketHandler {
    /// 创建新的包处理器
    pub fn new() -> Self {
        Self {
            login_response_cb: None,
            char_list_cb: None,
            char_create_cb: None,
            char_delete_cb: None,
            char_enter_cb: None,
        }
    }

    /// 注册登录响应回调
    pub fn on_login_response<F>(&mut self, callback: F)
    where
        F: Fn(&LoginResponse) + Send + Sync + 'static,
    {
        self.login_response_cb = Some(Box::new(callback));
    }

    /// 注册角色列表回调
    pub fn on_char_list<F>(&mut self, callback: F)
    where
        F: Fn(&CharListResponse) + Send + Sync + 'static,
    {
        self.char_list_cb = Some(Box::new(callback));
    }

    /// 注册创建角色回调
    pub fn on_char_create<F>(&mut self, callback: F)
    where
        F: Fn(&CharCreateResponse) + Send + Sync + 'static,
    {
        self.char_create_cb = Some(Box::new(callback));
    }

    /// 注册删除角色回调
    pub fn on_char_delete<F>(&mut self, callback: F)
    where
        F: Fn(&CharDeleteResponse) + Send + Sync + 'static,
    {
        self.char_delete_cb = Some(Box::new(callback));
    }

    /// 注册进入游戏回调
    pub fn on_char_enter<F>(&mut self, callback: F)
    where
        F: Fn(&CharEnterResponse) + Send + Sync + 'static,
    {
        self.char_enter_cb = Some(Box::new(callback));
    }

    /// 分发包到对应的回调
    pub fn dispatch(&self, packet: &Packet) {
        match packet {
            Packet::LoginResponse(resp) => {
                if let Some(cb) = &self.login_response_cb {
                    cb(resp);
                }
            }
            Packet::CharListResponse(resp) => {
                if let Some(cb) = &self.char_list_cb {
                    cb(resp);
                }
            }
            Packet::CharCreateResponse(resp) => {
                if let Some(cb) = &self.char_create_cb {
                    cb(resp);
                }
            }
            Packet::CharDeleteResponse(resp) => {
                if let Some(cb) = &self.char_delete_cb {
                    cb(resp);
                }
            }
            Packet::CharEnterResponse(resp) => {
                if let Some(cb) = &self.char_enter_cb {
                    cb(resp);
                }
            }
            _ => {
                tracing::warn!("未处理的包类型: ID=0x{:04X}", packet.packet_id());
            }
        }
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test net_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/net/handler.rs devi/tests/net_test.rs
git commit -m "feat(devi/net): 实现包处理器，支持回调注册和包分发"
```
