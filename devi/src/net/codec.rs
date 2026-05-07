use std::io;
use crate::protocol::Packet;
use crate::protocol::login::{LoginRequest, LoginResponse};

/// 协议包编解码器
pub struct PacketCodec;

impl PacketCodec {
    /// 将协议包编码为字节序列
    pub fn encode(packet: &Packet) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let id = packet.packet_id();
        // 写入包 ID（2 字节，小端序）
        buf.extend_from_slice(&id.to_le_bytes());
        // 写入长度占位（2 字节，小端序）
        buf.extend_from_slice(&[0u8, 0u8]);

        match packet {
            Packet::LoginRequest(req) => {
                // 写入用户名（24 字节，null 填充）
                let mut username = [0u8; 24];
                let bytes = req.username.as_bytes();
                let len = bytes.len().min(23);
                username[..len].copy_from_slice(&bytes[..len]);
                buf.extend_from_slice(&username);

                // 写入密码（24 字节，null 填充）
                let mut password = [0u8; 24];
                let bytes = req.password.as_bytes();
                let len = bytes.len().min(23);
                password[..len].copy_from_slice(&bytes[..len]);
                buf.extend_from_slice(&password);
            }
            _ => {}
        }

        // 回填长度字段
        let len = buf.len() as u16;
        buf[2] = (len & 0xFF) as u8;
        buf[3] = ((len >> 8) & 0xFF) as u8;
        Ok(buf)
    }

    /// 将字节序列解码为协议包
    pub fn decode(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "包数据太短"));
        }
        let packet_id = u16::from_le_bytes([data[0], data[1]]);

        match packet_id {
            // LoginRequest
            0x0064 => {
                if data.len() < 52 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "LoginRequest 数据不完整",
                    ));
                }
                let username = Self::read_null_padded_string(&data[4..28]);
                let password = Self::read_null_padded_string(&data[28..52]);
                Ok(Packet::LoginRequest(LoginRequest {
                    username,
                    password,
                }))
            }
            // LoginResponse
            0x0069 => {
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
                        login_id: u32::from_le_bytes([data[5], data[6], data[7], data[8]]),
                        account_id: u32::from_le_bytes([data[9], data[10], data[11], data[12]]),
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
