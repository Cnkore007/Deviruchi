//! 协议包编解码器
//!
//! 负责将 `Packet` 枚举与字节序列之间相互转换。
//! 所有数值字段使用小端序（Little-Endian），IP 地址字段例外（大端序）。
//! 包格式：[packet_id: u16 LE] [length: u16 LE] [payload...]

use std::io;
use crate::protocol::Packet;
use crate::protocol::login::{LoginRequest, LoginResponse};
use crate::protocol::char_mod::{
    CharListResponse, CharInfo, CharEnterRequest, CharEnterResponse,
    CharCreateRequest, CharCreateResponse, CharDeleteRequest, CharDeleteResponse,
};
use crate::protocol::map::{
    MapEnterRequest, MapEnteredResponse, PlayerMoveRequest, EntityMoveNotify,
    ChatMessage, EntityAppearNotify, EntityDisappearNotify,
};

/// 协议包编解码器
pub struct PacketCodec;

impl PacketCodec {
    // ========================================================================
    // 编码：Packet → 字节序列
    // ========================================================================

    /// 将协议包编码为字节序列
    pub fn encode(packet: &Packet) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let id = packet.packet_id();
        // 写入包 ID（2 字节，小端序）
        buf.extend_from_slice(&id.to_le_bytes());
        // 写入长度占位（2 字节，小端序），稍后回填
        buf.extend_from_slice(&[0u8, 0u8]);

        match packet {
            // ===== 登录 =====
            Packet::LoginRequest(req) => Self::encode_login_request(&mut buf, req),
            Packet::LoginResponse(resp) => Self::encode_login_response(&mut buf, resp),

            // ===== 角色 =====
            Packet::CharListRequest => { /* 无包体 */ }
            Packet::CharListResponse(resp) => Self::encode_char_list_response(&mut buf, resp),
            Packet::CharEnterRequest(req) => Self::encode_char_enter_request(&mut buf, req),
            Packet::CharEnterResponse(resp) => Self::encode_char_enter_response(&mut buf, resp),
            Packet::CharCreateRequest(req) => Self::encode_char_create_request(&mut buf, req),
            Packet::CharCreateResponse(resp) => Self::encode_char_create_response(&mut buf, resp),
            Packet::CharDeleteRequest(req) => Self::encode_char_delete_request(&mut buf, req),
            Packet::CharDeleteResponse(resp) => Self::encode_char_delete_response(&mut buf, resp),

            // ===== 地图 =====
            Packet::MapEnter(req) => Self::encode_map_enter(&mut buf, req),
            Packet::MapEntered(resp) => Self::encode_map_entered(&mut buf, resp),
            Packet::PlayerMove(req) => Self::encode_player_move(&mut buf, req),
            Packet::EntityMove(notify) => Self::encode_entity_move(&mut buf, notify),
            Packet::ChatMessage(msg) => Self::encode_chat_message(&mut buf, msg),
            Packet::EntityAppear(notify) => Self::encode_entity_appear(&mut buf, notify),
            Packet::EntityDisappear(notify) => Self::encode_entity_disappear(&mut buf, notify),
        }

        // 回填长度字段（偏移 2-3）
        let len = buf.len() as u16;
        buf[2] = (len & 0xFF) as u8;
        buf[3] = ((len >> 8) & 0xFF) as u8;
        Ok(buf)
    }

    // ----- 登录编码 -----

    /// 编码 LoginRequest (0x0064)
    /// 格式：version(u32) + username(24B null-pad) + password(24B null-pad)
    fn encode_login_request(buf: &mut Vec<u8>, req: &LoginRequest) {
        buf.extend_from_slice(&req.version.to_le_bytes());
        buf.extend_from_slice(&Self::make_fixed_str(&req.username, 24));
        buf.extend_from_slice(&Self::make_fixed_str(&req.password, 24));
    }

    /// 编码 LoginResponse (0x0069)
    /// 格式：account_id(u32) + login_id1(u32) + login_id2(u32) + sex(u8) + pad(u8)
    ///        + char_ip(4B BE) + char_port(u16) + server_name(20B) + user_count(u16)
    ///        + server_type(u8) + new_flag(u16)
    fn encode_login_response(buf: &mut Vec<u8>, resp: &LoginResponse) {
        buf.extend_from_slice(&resp.account_id.to_le_bytes());
        buf.extend_from_slice(&resp.login_id1.to_le_bytes());
        buf.extend_from_slice(&resp.login_id2.to_le_bytes());
        buf.push(resp.sex);
        buf.push(0); // padding
        // IP 地址使用大端序（网络字节序）
        buf.extend_from_slice(&resp.char_ip);
        buf.extend_from_slice(&resp.char_port.to_le_bytes());
        buf.extend_from_slice(&Self::make_fixed_str(&resp.server_name, 20));
        buf.extend_from_slice(&resp.user_count.to_le_bytes());
        buf.push(resp.server_type);
        buf.extend_from_slice(&resp.new_flag.to_le_bytes());
    }

    // ----- 角色编码 -----

    /// 编码 CharEnterRequest (0x0065)
    /// 格式：char_id(u32)
    fn encode_char_enter_request(buf: &mut Vec<u8>, req: &CharEnterRequest) {
        buf.extend_from_slice(&req.char_id.to_le_bytes());
    }

    /// 编码 CharEnterResponse (0x0071)
    /// 格式：map_ip(16B) + map_port(u16) + token(32B)
    fn encode_char_enter_response(buf: &mut Vec<u8>, resp: &CharEnterResponse) {
        buf.extend_from_slice(&Self::make_fixed_str(&resp.map_ip, 16));
        buf.extend_from_slice(&resp.map_port.to_le_bytes());
        buf.extend_from_slice(&Self::make_fixed_str(&resp.token, 32));
    }

    /// 编码 CharListResponse (0x006b)
    /// 格式：count(u8) + [char_info(122B) * count]
    fn encode_char_list_response(buf: &mut Vec<u8>, resp: &CharListResponse) {
        buf.push(resp.chars.len() as u8);
        for ch in &resp.chars {
            Self::encode_char_info(buf, ch);
        }
    }

    /// 编码单条角色信息（122 字节定长）
    fn encode_char_info(buf: &mut Vec<u8>, ch: &CharInfo) {
        buf.extend_from_slice(&ch.char_id.to_le_bytes());
        buf.extend_from_slice(&ch.exp.to_le_bytes());
        buf.extend_from_slice(&ch.gold.to_le_bytes());
        buf.extend_from_slice(&ch.job_exp.to_le_bytes());
        buf.extend_from_slice(&ch.job_level.to_le_bytes());
        buf.extend_from_slice(&ch.body_state.to_le_bytes());
        buf.extend_from_slice(&ch.health_state.to_le_bytes());
        buf.extend_from_slice(&ch.effect_state.to_le_bytes());
        buf.extend_from_slice(&ch.virtue.to_le_bytes());
        buf.extend_from_slice(&ch.honor.to_le_bytes());
        buf.extend_from_slice(&ch.job.to_le_bytes());
        buf.extend_from_slice(&ch.hair.to_le_bytes());
        buf.extend_from_slice(&ch.hair_color.to_le_bytes());
        buf.extend_from_slice(&ch.clothes_color.to_le_bytes());
        buf.extend_from_slice(&ch.body.to_le_bytes());
        buf.extend_from_slice(&ch.weapon.to_le_bytes());
        buf.extend_from_slice(&ch.head_bottom.to_le_bytes());
        buf.extend_from_slice(&ch.shield.to_le_bytes());
        buf.extend_from_slice(&ch.head_top.to_le_bytes());
        buf.extend_from_slice(&ch.head_mid.to_le_bytes());
        buf.extend_from_slice(&ch.hair_color2.to_le_bytes());
        buf.extend_from_slice(&ch.clothes_color2.to_le_bytes());
        buf.extend_from_slice(&Self::make_fixed_str(&ch.name, 24));
        buf.extend_from_slice(&ch.base_level.to_le_bytes());
        buf.extend_from_slice(&ch.str.to_le_bytes());
        buf.extend_from_slice(&ch.agi.to_le_bytes());
        buf.extend_from_slice(&ch.vit.to_le_bytes());
        buf.extend_from_slice(&ch.int.to_le_bytes());
        buf.extend_from_slice(&ch.dex.to_le_bytes());
        buf.extend_from_slice(&ch.luk.to_le_bytes());
        buf.push(ch.slot);
        buf.extend_from_slice(&ch.delete_timer.to_le_bytes());
        buf.push(ch.rename);
        buf.extend_from_slice(&Self::make_fixed_str(&ch.map_name, 24));
    }

    /// 编码 CharCreateRequest (0x0067)
    /// 格式：name(24B) + str(u8) + agi(u8) + vit(u8) + int(u8) + dex(u8) + luk(u8)
    ///        + hair_color(u16) + hair(u16)
    fn encode_char_create_request(buf: &mut Vec<u8>, req: &CharCreateRequest) {
        buf.extend_from_slice(&Self::make_fixed_str(&req.name, 24));
        buf.push(req.str);
        buf.push(req.agi);
        buf.push(req.vit);
        buf.push(req.int);
        buf.push(req.dex);
        buf.push(req.luk);
        buf.extend_from_slice(&req.hair_color.to_le_bytes());
        buf.extend_from_slice(&req.hair.to_le_bytes());
    }

    /// 编码 CharCreateResponse (0x006d)
    /// 成功时返回角色信息，失败时返回错误码
    fn encode_char_create_response(buf: &mut Vec<u8>, resp: &CharCreateResponse) {
        match resp {
            CharCreateResponse::Success(ch) => {
                buf.push(1); // 成功标志
                Self::encode_char_info(buf, ch);
            }
            CharCreateResponse::Failure { error_code } => {
                buf.push(0); // 失败标志
                buf.push(*error_code);
            }
        }
    }

    /// 编码 CharDeleteRequest (0x0068)
    /// 格式：char_id(u32) + email(40B null-pad)
    fn encode_char_delete_request(buf: &mut Vec<u8>, req: &CharDeleteRequest) {
        buf.extend_from_slice(&req.char_id.to_le_bytes());
        buf.extend_from_slice(&Self::make_fixed_str(&req.email, 40));
    }

    /// 编码 CharDeleteResponse (0x006e)
    /// 格式：char_id(u32)
    fn encode_char_delete_response(buf: &mut Vec<u8>, resp: &CharDeleteResponse) {
        buf.extend_from_slice(&resp.char_id.to_le_bytes());
    }

    // ----- 地图编码 -----

    /// 编码 MapEnterRequest (0x0072)
    /// 格式：char_id(u32) + login_id(u32) + client_tick(u32) + gender(u8)
    fn encode_map_enter(buf: &mut Vec<u8>, req: &MapEnterRequest) {
        buf.extend_from_slice(&req.char_id.to_le_bytes());
        buf.extend_from_slice(&req.login_id.to_le_bytes());
        buf.extend_from_slice(&req.client_tick.to_le_bytes());
        buf.push(req.gender);
    }

    /// 编码 MapEnteredResponse (0x0073)
    /// 格式：start_time(u32) + pos_x(u16) + pos_y(u16) + direction(u16) + font(u16)
    fn encode_map_entered(buf: &mut Vec<u8>, resp: &MapEnteredResponse) {
        buf.extend_from_slice(&resp.start_time.to_le_bytes());
        buf.extend_from_slice(&resp.pos_x.to_le_bytes());
        buf.extend_from_slice(&resp.pos_y.to_le_bytes());
        buf.extend_from_slice(&resp.direction.to_le_bytes());
        buf.extend_from_slice(&resp.font.to_le_bytes());
    }

    /// 编码 PlayerMoveRequest (0x0085)
    /// 格式：dest_x(u16) + dest_y(u16) + move_data(1B+)
    fn encode_player_move(buf: &mut Vec<u8>, req: &PlayerMoveRequest) {
        buf.extend_from_slice(&req.dest_x.to_le_bytes());
        buf.extend_from_slice(&req.dest_y.to_le_bytes());
        // 移动数据占位字节（服务端要求至少 1 字节 move_data）
        buf.push(0);
    }

    /// 编码 EntityMoveNotify (0x0086)
    /// 格式：entity_id(u32) + from_x(u16) + from_y(u16) + dest_x(u16) + dest_y(u16) + speed(u16)
    fn encode_entity_move(buf: &mut Vec<u8>, notify: &EntityMoveNotify) {
        buf.extend_from_slice(&notify.entity_id.to_le_bytes());
        buf.extend_from_slice(&notify.from_x.to_le_bytes());
        buf.extend_from_slice(&notify.from_y.to_le_bytes());
        buf.extend_from_slice(&notify.dest_x.to_le_bytes());
        buf.extend_from_slice(&notify.dest_y.to_le_bytes());
        buf.extend_from_slice(&notify.speed.to_le_bytes());
    }

    /// 编码 ChatMessage (0x008c)
    /// 格式：sender_id(u32) + msg_len(u16) + "sender_name:message"
    fn encode_chat_message(buf: &mut Vec<u8>, msg: &ChatMessage) {
        buf.extend_from_slice(&msg.sender_id.to_le_bytes());
        // 消息格式："sender_name:message"
        let full_msg = format!("{} : {}", msg.sender_name, msg.message);
        let msg_len = full_msg.len() as u16 + 4; // +4 for sender_id + msg_len 本身
        buf.extend_from_slice(&msg_len.to_le_bytes());
        buf.extend_from_slice(full_msg.as_bytes());
    }

    /// 编码 EntityAppearNotify (0x0078)
    /// 格式：entity_id(u32) + entity_type(u8) + pos_x(u16) + pos_y(u16) + direction(u8) + look(u16)
    fn encode_entity_appear(buf: &mut Vec<u8>, notify: &EntityAppearNotify) {
        buf.extend_from_slice(&notify.entity_id.to_le_bytes());
        buf.push(notify.entity_type);
        buf.extend_from_slice(&notify.pos_x.to_le_bytes());
        buf.extend_from_slice(&notify.pos_y.to_le_bytes());
        buf.push(notify.direction);
        buf.extend_from_slice(&notify.look.to_le_bytes());
    }

    /// 编码 EntityDisappearNotify (0x007a)
    /// 格式：entity_id(u32) + reason(u8)
    fn encode_entity_disappear(buf: &mut Vec<u8>, notify: &EntityDisappearNotify) {
        buf.extend_from_slice(&notify.entity_id.to_le_bytes());
        buf.push(notify.reason);
    }

    // ========================================================================
    // 解码：字节序列 → Packet
    // ========================================================================

    /// 将字节序列解码为协议包
    pub fn decode(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "包数据太短，至少需要 4 字节"));
        }
        let packet_id = u16::from_le_bytes([data[0], data[1]]);

        match packet_id {
            0x0064 => Self::decode_login_request(data),
            0x0069 => Self::decode_login_response(data),
            0x0065 => Self::decode_char_enter_request(data),
            0x0066 => Ok(Packet::CharListRequest),
            0x006b => Self::decode_char_list_response(data),
            0x0067 => Self::decode_char_create_request(data),
            0x006d => Self::decode_char_create_response(data),
            0x0068 => Self::decode_char_delete_request(data),
            0x006e => Self::decode_char_delete_response(data),
            0x0071 => Self::decode_char_enter_response(data),
            0x0072 => Self::decode_map_enter(data),
            0x0073 => Self::decode_map_entered(data),
            0x0085 => Self::decode_player_move(data),
            0x0086 => Self::decode_entity_move(data),
            0x008c => Self::decode_chat_message(data),
            0x0078 => Self::decode_entity_appear(data),
            0x007a => Self::decode_entity_disappear(data),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("未知包 ID: 0x{:04X}", packet_id),
            )),
        }
    }

    // ----- 登录解码 -----

    /// 解码 LoginRequest (0x0064)
    /// 需要至少 52 字节 payload（4+24+24），总包 56 字节
    fn decode_login_request(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 56 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "LoginRequest 数据不完整，需要 56 字节"));
        }
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let username = Self::read_fixed_str(&data[8..32]);
        let password = Self::read_fixed_str(&data[32..56]);
        Ok(Packet::LoginRequest(LoginRequest { version, username, password }))
    }

    /// 解码 LoginResponse (0x0069)
    /// 需要至少 45 字节 payload，总包 49 字节
    fn decode_login_response(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 49 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "LoginResponse 数据不完整，需要 49 字节"));
        }
        let account_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let login_id1 = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let login_id2 = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let sex = data[16];
        // data[17] 是 padding
        let char_ip = [data[18], data[19], data[20], data[21]]; // 大端序，原样存储
        let char_port = u16::from_le_bytes([data[22], data[23]]);
        let server_name = Self::read_fixed_str(&data[24..44]);
        let user_count = u16::from_le_bytes([data[44], data[45]]);
        let server_type = data[46];
        let new_flag = u16::from_le_bytes([data[47], data[48]]);

        Ok(Packet::LoginResponse(LoginResponse {
            account_id,
            login_id1,
            login_id2,
            sex,
            char_ip,
            char_port,
            server_name,
            user_count,
            server_type,
            new_flag,
        }))
    }

    // ----- 角色解码 -----

    /// 解码 CharEnterRequest (0x0065)
    /// 需要至少 8 字节（4 header + 4 payload）
    fn decode_char_enter_request(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 8 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharEnterRequest 数据不完整"));
        }
        let char_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        Ok(Packet::CharEnterRequest(CharEnterRequest { char_id }))
    }

    /// 解码 CharListResponse (0x006b)
    /// 格式：header(4) + count(1) + [char_info(122) * count]
    fn decode_char_list_response(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 5 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharListResponse 数据不完整"));
        }
        let count = data[4] as usize;
        let expected = 5 + count * 122;
        if data.len() < expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("CharListResponse 数据不足：期望 {} 字节，实际 {} 字节", expected, data.len()),
            ));
        }
        let mut chars = Vec::with_capacity(count);
        for i in 0..count {
            let offset = 5 + i * 122;
            let ch = Self::decode_char_info(&data[offset..offset + 122])?;
            chars.push(ch);
        }
        Ok(Packet::CharListResponse(CharListResponse { chars }))
    }

    /// 解码单条角色信息（122 字节定长）
    fn decode_char_info(data: &[u8]) -> io::Result<CharInfo> {
        if data.len() < 122 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "角色信息数据不足 122 字节"));
        }
        Ok(CharInfo {
            char_id: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            exp: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            gold: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            job_exp: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            job_level: u16::from_le_bytes([data[16], data[17]]),
            body_state: u16::from_le_bytes([data[18], data[19]]),
            health_state: u16::from_le_bytes([data[20], data[21]]),
            effect_state: u32::from_le_bytes([data[22], data[23], data[24], data[25]]),
            virtue: i16::from_le_bytes([data[26], data[27]]),
            honor: i16::from_le_bytes([data[28], data[29]]),
            job: u16::from_le_bytes([data[30], data[31]]),
            hair: u16::from_le_bytes([data[32], data[33]]),
            hair_color: u16::from_le_bytes([data[34], data[35]]),
            clothes_color: u16::from_le_bytes([data[36], data[37]]),
            body: u16::from_le_bytes([data[38], data[39]]),
            weapon: u16::from_le_bytes([data[40], data[41]]),
            head_bottom: u16::from_le_bytes([data[42], data[43]]),
            shield: u16::from_le_bytes([data[44], data[45]]),
            head_top: u16::from_le_bytes([data[46], data[47]]),
            head_mid: u16::from_le_bytes([data[48], data[49]]),
            hair_color2: u16::from_le_bytes([data[50], data[51]]),
            clothes_color2: u16::from_le_bytes([data[52], data[53]]),
            name: Self::read_fixed_str(&data[54..78]),
            base_level: u16::from_le_bytes([data[78], data[79]]),
            str: u16::from_le_bytes([data[80], data[81]]),
            agi: u16::from_le_bytes([data[82], data[83]]),
            vit: u16::from_le_bytes([data[84], data[85]]),
            int: u16::from_le_bytes([data[86], data[87]]),
            dex: u16::from_le_bytes([data[88], data[89]]),
            luk: u16::from_le_bytes([data[90], data[91]]),
            slot: data[92],
            delete_timer: u32::from_le_bytes([data[93], data[94], data[95], data[96]]),
            rename: data[97],
            map_name: Self::read_fixed_str(&data[98..122]),
        })
    }

    /// 解码 CharCreateRequest (0x0067)
    /// 需要至少 34 字节 payload，总包 38 字节
    fn decode_char_create_request(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 38 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharCreateRequest 数据不完整"));
        }
        let name = Self::read_fixed_str(&data[4..28]);
        let str = data[28];
        let agi = data[29];
        let vit = data[30];
        let int = data[31];
        let dex = data[32];
        let luk = data[33];
        let hair_color = u16::from_le_bytes([data[34], data[35]]);
        let hair = u16::from_le_bytes([data[36], data[37]]);
        Ok(Packet::CharCreateRequest(CharCreateRequest {
            name, str, agi, vit, int, dex, luk, hair_color, hair,
        }))
    }

    /// 解码 CharCreateResponse (0x006d)
    /// 成功：header(4) + flag(1) + char_info(122) = 127 字节
    /// 失败：header(4) + flag(1) + error_code(1) = 6 字节
    fn decode_char_create_response(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 6 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharCreateResponse 数据不完整"));
        }
        let flag = data[4];
        if flag == 0 {
            let error_code = data[5];
            Ok(Packet::CharCreateResponse(CharCreateResponse::Failure { error_code }))
        } else {
            if data.len() < 127 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "CharCreateResponse 成功数据不完整"));
            }
            let ch = Self::decode_char_info(&data[5..127])?;
            Ok(Packet::CharCreateResponse(CharCreateResponse::Success(ch)))
        }
    }

    /// 解码 CharDeleteRequest (0x0068)
    /// 需要至少 44 字节 payload，总包 48 字节
    fn decode_char_delete_request(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 48 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharDeleteRequest 数据不完整"));
        }
        let char_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let email = Self::read_fixed_str(&data[8..48]);
        Ok(Packet::CharDeleteRequest(CharDeleteRequest { char_id, email }))
    }

    /// 解码 CharDeleteResponse (0x006e)
    /// 格式：header(4) + char_id(u32) = 8 字节
    fn decode_char_delete_response(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 8 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharDeleteResponse 数据不完整"));
        }
        let char_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        Ok(Packet::CharDeleteResponse(CharDeleteResponse { char_id }))
    }

    /// 解码 CharEnterResponse (0x0071)
    /// 格式：header(4) + map_ip(16B) + map_port(u16) + token(32B) = 54 字节
    fn decode_char_enter_response(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 54 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "CharEnterResponse 数据不完整"));
        }
        let map_ip = Self::read_fixed_str(&data[4..20]);
        let map_port = u16::from_le_bytes([data[20], data[21]]);
        let token = Self::read_fixed_str(&data[22..54]);
        Ok(Packet::CharEnterResponse(CharEnterResponse { map_ip, map_port, token }))
    }

    // ----- 地图解码 -----

    /// 解码 MapEnterRequest (0x0072)
    /// 需要至少 17 字节（4 header + 13 payload）
    fn decode_map_enter(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 17 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "MapEnterRequest 数据不完整"));
        }
        let char_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let login_id = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let client_tick = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let gender = data[16];
        Ok(Packet::MapEnter(MapEnterRequest { char_id, login_id, client_tick, gender }))
    }

    /// 解码 MapEnteredResponse (0x0073)
    /// 需要至少 16 字节（4 header + 12 payload）
    fn decode_map_entered(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "MapEnteredResponse 数据不完整"));
        }
        let start_time = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let pos_x = u16::from_le_bytes([data[8], data[9]]);
        let pos_y = u16::from_le_bytes([data[10], data[11]]);
        let direction = u16::from_le_bytes([data[12], data[13]]);
        let font = u16::from_le_bytes([data[14], data[15]]);
        Ok(Packet::MapEntered(MapEnteredResponse { start_time, pos_x, pos_y, direction, font }))
    }

    /// 解码 PlayerMoveRequest (0x0085)
    /// 需要至少 9 字节（4 header + 4 payload + 1 move_data）
    fn decode_player_move(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 9 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "PlayerMoveRequest 数据不完整"));
        }
        let dest_x = u16::from_le_bytes([data[4], data[5]]);
        let dest_y = u16::from_le_bytes([data[6], data[7]]);
        Ok(Packet::PlayerMove(PlayerMoveRequest { dest_x, dest_y }))
    }

    /// 解码 EntityMoveNotify (0x0086)
    /// 需要至少 16 字节（4 header + 12 payload）
    fn decode_entity_move(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "EntityMoveNotify 数据不完整"));
        }
        let entity_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let from_x = u16::from_le_bytes([data[8], data[9]]);
        let from_y = u16::from_le_bytes([data[10], data[11]]);
        let dest_x = u16::from_le_bytes([data[12], data[13]]);
        let dest_y = u16::from_le_bytes([data[14], data[15]]);
        let speed = if data.len() >= 18 {
            u16::from_le_bytes([data[16], data[17]])
        } else {
            0
        };
        Ok(Packet::EntityMove(EntityMoveNotify { entity_id, from_x, from_y, dest_x, dest_y, speed }))
    }

    /// 解码 ChatMessage (0x008c)
    /// 格式：header(4) + sender_id(u32) + msg_len(u16) + message(variable)
    fn decode_chat_message(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 10 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ChatMessage 数据不完整"));
        }
        let sender_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let msg_len = u16::from_le_bytes([data[8], data[9]]) as usize;
        let msg_start = 10;
        let msg_end = msg_start + msg_len.saturating_sub(4); // msg_len 包含自身和 sender_id
        if data.len() < msg_end {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "ChatMessage 消息数据不足"));
        }
        let full_msg = String::from_utf8_lossy(&data[msg_start..msg_end]).to_string();
        // 解析 "sender_name : message" 格式
        let (sender_name, message) = if let Some(pos) = full_msg.find(" : ") {
            (full_msg[..pos].to_string(), full_msg[pos + 3..].to_string())
        } else {
            (String::new(), full_msg)
        };
        Ok(Packet::ChatMessage(ChatMessage { sender_id, sender_name, message }))
    }

    /// 解码 EntityAppearNotify (0x0078)
    /// 需要至少 16 字节（4 header + 12 payload）
    fn decode_entity_appear(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "EntityAppearNotify 数据不完整"));
        }
        let entity_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let entity_type = data[8];
        let pos_x = u16::from_le_bytes([data[9], data[10]]);
        let pos_y = u16::from_le_bytes([data[11], data[12]]);
        let direction = data[13];
        let look = u16::from_le_bytes([data[14], data[15]]);
        Ok(Packet::EntityAppear(EntityAppearNotify { entity_id, entity_type, pos_x, pos_y, direction, look }))
    }

    /// 解码 EntityDisappearNotify (0x007a)
    /// 需要至少 9 字节（4 header + 5 payload）
    fn decode_entity_disappear(data: &[u8]) -> io::Result<Packet> {
        if data.len() < 9 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "EntityDisappearNotify 数据不完整"));
        }
        let entity_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let reason = data[8];
        Ok(Packet::EntityDisappear(EntityDisappearNotify { entity_id, reason }))
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 生成固定长度的 null 填充字节数组
    fn make_fixed_str(s: &str, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(len - 1); // 保留至少一个 null 终止符
        buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
        buf
    }

    /// 读取 null 填充的固定长度字符串
    fn read_fixed_str(data: &[u8]) -> String {
        let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        String::from_utf8_lossy(&data[..end]).to_string()
    }
}
