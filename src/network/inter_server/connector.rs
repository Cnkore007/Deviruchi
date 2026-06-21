//! Inter-Server TCP 连接器
//!
//! 实现跨进程服务器间的 TCP 通信，目前作为 stub 提供基础连接、
//! 序列化/反序列化和心跳机制。

use crate::game::inter_server::{InterServerConnector, InterServerPacket};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 基于 TCP 流的 Inter-Server 连接器
pub struct TcpInterServerConnector {
    target_server_id: u32,
    stream: Mutex<TcpStream>,
    connected: AtomicBool,
    last_send: Mutex<Instant>,
    last_recv: Mutex<Instant>,
}

impl TcpInterServerConnector {
    /// 连接到目标 inter-server 地址
    pub fn connect(target_server_id: u32, addr: &str) -> Result<Arc<Self>, String> {
        let stream = TcpStream::connect(addr)
            .and_then(|s| {
                s.set_nonblocking(false)?;
                s.set_read_timeout(Some(Duration::from_secs(5)))?;
                s.set_write_timeout(Some(Duration::from_secs(5)))?;
                Ok(s)
            })
            .map_err(|e| format!("无法连接到 {}: {}", addr, e))?;

        Ok(Arc::new(Self {
            target_server_id,
            stream: Mutex::new(stream),
            connected: AtomicBool::new(true),
            last_send: Mutex::new(Instant::now()),
            last_recv: Mutex::new(Instant::now()),
        }))
    }

    /// 从已建立的 TCP 流创建连接器（服务器端接受连接后使用）
    pub fn from_stream(target_server_id: u32, stream: TcpStream) -> Result<Arc<Self>, String> {
        stream
            .set_nonblocking(false)
            .and_then(|_| stream.set_read_timeout(Some(Duration::from_secs(5))))
            .and_then(|_| stream.set_write_timeout(Some(Duration::from_secs(5))))
            .map_err(|e| format!("配置 TCP 流失败: {}", e))?;

        Ok(Arc::new(Self {
            target_server_id,
            stream: Mutex::new(stream),
            connected: AtomicBool::new(true),
            last_send: Mutex::new(Instant::now()),
            last_recv: Mutex::new(Instant::now()),
        }))
    }

    /// 读取并反序列化一个数据包（阻塞）
    pub fn recv_packet(&self) -> Result<Option<InterServerPacket>, String> {
        let mut stream = self.stream.lock().map_err(|e| format!("锁中毒: {}", e))?;

        // 读取 4 字节长度前缀（大端）
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                self.connected.store(false, Ordering::SeqCst);
                return Ok(None);
            }
            Err(e) => {
                self.connected.store(false, Ordering::SeqCst);
                return Err(format!("读取长度前缀失败: {}", e));
            }
        }

        let len = u32::from_be_bytes(len_buf) as usize;
        if len > 8 * 1024 * 1024 {
            return Err(format!("数据包过大: {} bytes", len));
        }

        let mut payload = vec![0u8; len];
        stream
            .read_exact(&mut payload)
            .map_err(|e| format!("读取 payload 失败: {}", e))?;

        *self.last_recv.lock().unwrap() = Instant::now();

        let packet: InterServerPacket = bincode::deserialize(&payload)
            .map_err(|e| format!("反序列化失败: {}", e))?;
        Ok(Some(packet))
    }

    /// 发送心跳包（如需要）
    pub fn maybe_send_heartbeat(&self,
        server_id: u32,
        server_type: crate::game::inter_server::ServerTypeProto,
    ) -> Result<(), String> {
        let last_send = *self.last_send.lock().map_err(|e| e.to_string())?;
        if last_send.elapsed() < Duration::from_secs(30) {
            return Ok(());
        }

        let packet = InterServerPacket::Heartbeat {
            server_id,
            server_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            online_players: 0,
        };
        self.send_packet(&packet)
    }
}

impl InterServerConnector for TcpInterServerConnector {
    fn send_packet(&self,
        packet: &InterServerPacket,
    ) -> Result<(), String> {
        let payload = bincode::serialize(packet)
            .map_err(|e| format!("序列化失败: {}", e))?;
        let len = payload.len() as u32;

        let mut stream = self.stream.lock().map_err(|e| format!("锁中毒: {}", e))?;
        stream
            .write_all(&len.to_be_bytes())
            .and_then(|_| stream.write_all(&payload))
            .and_then(|_| stream.flush())
            .map_err(|e| format!("写入失败: {}", e))?;

        *self.last_send.lock().unwrap() = Instant::now();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn target_server_id(&self) -> u32 {
        self.target_server_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::inter_server::{CharacterTransfer, ServerTypeProto};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_tcp_connector_send_recv() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let connector = TcpInterServerConnector::from_stream(2, stream).unwrap();
            let packet = connector.recv_packet().unwrap().unwrap();
            match packet {
                InterServerPacket::Heartbeat { server_id, .. } => {
                    assert_eq!(server_id, 1);
                }
                _ => panic!("Expected Heartbeat"),
            }
        });

        // 等待服务器 accept 就绪
        thread::sleep(Duration::from_millis(50));

        let client = TcpInterServerConnector::connect(2, &addr).unwrap();
        let packet = InterServerPacket::Heartbeat {
            server_id: 1,
            server_type: ServerTypeProto::Map,
            timestamp: 1234567890,
            online_players: 42,
        };
        client.send_packet(&packet).unwrap();

        server_thread.join().unwrap();
    }

    #[test]
    fn test_tcp_connector_char_transfer_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        let transfer = CharacterTransfer {
            char_id: 1001,
            account_id: 1,
            name: "TestChar".to_string(),
            level: 99,
            job: 0,
            hp: 5000,
            max_hp: 5000,
            sp: 1000,
            max_sp: 1000,
            map_name: "prontera".to_string(),
            pos_x: 100,
            pos_y: 200,
            save_map: "new_1-1".to_string(),
            save_x: 53,
            save_y: 111,
            str: 99,
            agi: 99,
            vit: 99,
            int: 99,
            dex: 99,
            luk: 99,
            zeny: 1000000,
            sex: 1,
            hair_color: 0,
            hair: 1,
            cloak_id: 0,
            boots_id: 0,
            account_level: 0,
        };

        let expected = transfer.clone();
        let server_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let connector = TcpInterServerConnector::from_stream(2, stream).unwrap();
            let packet = connector.recv_packet().unwrap().unwrap();
            match packet {
                InterServerPacket::CharToMap { char_id, character_data, .. } => {
                    assert_eq!(char_id, expected.char_id);
                    assert_eq!(character_data.name, expected.name);
                }
                _ => panic!("Expected CharToMap"),
            }
        });

        thread::sleep(Duration::from_millis(50));

        let client = TcpInterServerConnector::connect(2, &addr).unwrap();
        let packet = InterServerPacket::CharToMap {
            char_id: transfer.char_id,
            account_id: transfer.account_id,
            token: "token123".to_string(),
            map_server_id: 2,
            character_data: transfer,
        };
        client.send_packet(&packet).unwrap();

        server_thread.join().unwrap();
    }
}
