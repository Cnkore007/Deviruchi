use devi::protocol::Packet;
use devi::protocol::login::{LoginRequest, LoginResponse};
use devi::protocol::char_mod::{
    CharListResponse, CharInfo, CharEnterRequest, CharEnterResponse,
    CharCreateRequest, CharCreateResponse, CharDeleteRequest, CharDeleteResponse,
};
use devi::protocol::map::{
    MapEnterRequest, MapEnteredResponse, PlayerMoveRequest, EntityMoveNotify,
    ChatMessage, EntityAppearNotify, EntityDisappearNotify,
};
use devi::net::transport::TransportState;
use devi::net::codec::PacketCodec;
use devi::net::handler::PacketHandler;

// ===== Packet ID 测试 =====

#[test]
fn test_login_request_packet_id() {
    let req = LoginRequest { version: 20, username: "test".to_string(), password: "pass".to_string() };
    let packet = Packet::LoginRequest(req);
    assert_eq!(packet.packet_id(), 0x0064);
}

#[test]
fn test_login_response_packet_id() {
    let resp = LoginResponse {
        account_id: 1, login_id1: 2, login_id2: 3, sex: 1,
        char_ip: [127, 0, 0, 1], char_port: 6900,
        server_name: "Test".to_string(), user_count: 100,
        server_type: 0, new_flag: 0,
    };
    let packet = Packet::LoginResponse(resp);
    assert_eq!(packet.packet_id(), 0x0069);
}

#[test]
fn test_char_list_request_packet_id() {
    let packet = Packet::CharListRequest;
    assert_eq!(packet.packet_id(), 0x0066);
}

#[test]
fn test_char_enter_request_packet_id() {
    let req = CharEnterRequest { char_id: 1001 };
    let packet = Packet::CharEnterRequest(req);
    assert_eq!(packet.packet_id(), 0x0065);
}

#[test]
fn test_char_list_response_packet_id() {
    let resp = CharListResponse { chars: vec![] };
    let packet = Packet::CharListResponse(resp);
    assert_eq!(packet.packet_id(), 0x006b);
}

#[test]
fn test_char_create_request_packet_id() {
    let req = CharCreateRequest {
        name: "Test".to_string(), str: 5, agi: 5, vit: 5,
        int: 5, dex: 5, luk: 5, hair_color: 0, hair: 1,
    };
    let packet = Packet::CharCreateRequest(req);
    assert_eq!(packet.packet_id(), 0x0067);
}

#[test]
fn test_char_delete_request_packet_id() {
    let req = CharDeleteRequest { char_id: 1001, email: "test@test.com".to_string() };
    let packet = Packet::CharDeleteRequest(req);
    assert_eq!(packet.packet_id(), 0x0068);
}

#[test]
fn test_char_delete_response_packet_id() {
    let resp = CharDeleteResponse { char_id: 1001 };
    let packet = Packet::CharDeleteResponse(resp);
    assert_eq!(packet.packet_id(), 0x006e);
}

#[test]
fn test_char_enter_response_packet_id() {
    let resp = CharEnterResponse {
        map_ip: "192.168.1.1".to_string(), map_port: 5121,
        token: "abc123".to_string(),
    };
    let packet = Packet::CharEnterResponse(resp);
    assert_eq!(packet.packet_id(), 0x0071);
}

#[test]
fn test_map_enter_packet_id() {
    let req = MapEnterRequest { char_id: 1, login_id: 2, client_tick: 100, gender: 1 };
    let packet = Packet::MapEnter(req);
    assert_eq!(packet.packet_id(), 0x0072);
}

#[test]
fn test_map_entered_packet_id() {
    let resp = MapEnteredResponse { start_time: 0, pos_x: 100, pos_y: 200, direction: 0, font: 0 };
    let packet = Packet::MapEntered(resp);
    assert_eq!(packet.packet_id(), 0x0073);
}

#[test]
fn test_player_move_packet_id() {
    let req = PlayerMoveRequest { dest_x: 150, dest_y: 250 };
    let packet = Packet::PlayerMove(req);
    assert_eq!(packet.packet_id(), 0x0085);
}

#[test]
fn test_entity_move_packet_id() {
    let notify = EntityMoveNotify {
        entity_id: 100, from_x: 10, from_y: 20, dest_x: 30, dest_y: 40, speed: 150,
    };
    let packet = Packet::EntityMove(notify);
    assert_eq!(packet.packet_id(), 0x0086);
}

#[test]
fn test_chat_message_packet_id() {
    let msg = ChatMessage { sender_id: 1, sender_name: "Test".to_string(), message: "Hi".to_string() };
    let packet = Packet::ChatMessage(msg);
    assert_eq!(packet.packet_id(), 0x008c);
}

#[test]
fn test_entity_appear_packet_id() {
    let notify = EntityAppearNotify {
        entity_id: 200, entity_type: 0, pos_x: 100, pos_y: 200, direction: 4, look: 1,
    };
    let packet = Packet::EntityAppear(notify);
    assert_eq!(packet.packet_id(), 0x0078);
}

#[test]
fn test_entity_disappear_packet_id() {
    let notify = EntityDisappearNotify { entity_id: 200, reason: 0 };
    let packet = Packet::EntityDisappear(notify);
    assert_eq!(packet.packet_id(), 0x007a);
}

// ===== Transport 状态测试 =====

#[test]
fn test_transport_state_default() {
    let state = TransportState::default();
    assert_eq!(state, TransportState::Disconnected);
}

#[test]
fn test_transport_state_transitions() {
    let mut state = TransportState::default();
    state = TransportState::Connecting;
    assert_eq!(state, TransportState::Connecting);
    state = TransportState::Connected;
    assert_eq!(state, TransportState::Connected);
}

// ===== LoginRequest 编解码测试 =====

#[test]
fn test_encode_login_request() {
    let req = LoginRequest { version: 20, username: "testuser".to_string(), password: "testpass".to_string() };
    let packet = Packet::LoginRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded[0], 0x64);
    assert_eq!(encoded[1], 0x00);
    let len = u16::from_le_bytes([encoded[2], encoded[3]]);
    assert_eq!(len as usize, encoded.len());
    assert_eq!(encoded.len(), 56); // 4 header + 4 version + 24 username + 24 password
}

#[test]
fn test_decode_login_request() {
    let req = LoginRequest { version: 20, username: "test".to_string(), password: "pass".to_string() };
    let packet = Packet::LoginRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::LoginRequest(decoded_req) => {
            assert_eq!(decoded_req.version, 20);
            assert_eq!(decoded_req.username, "test");
            assert_eq!(decoded_req.password, "pass");
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== LoginResponse 编解码测试 =====

#[test]
fn test_encode_login_response() {
    let resp = LoginResponse {
        account_id: 100, login_id1: 200, login_id2: 300, sex: 1,
        char_ip: [192, 168, 1, 1], char_port: 6900,
        server_name: "TestServer".to_string(), user_count: 50,
        server_type: 0, new_flag: 0,
    };
    let packet = Packet::LoginResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 49); // 4 header + 45 payload
}

#[test]
fn test_decode_login_response() {
    let resp = LoginResponse {
        account_id: 100, login_id1: 200, login_id2: 300, sex: 1,
        char_ip: [192, 168, 1, 1], char_port: 6900,
        server_name: "TestServer".to_string(), user_count: 50,
        server_type: 0, new_flag: 0,
    };
    let packet = Packet::LoginResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::LoginResponse(r) => {
            assert_eq!(r.account_id, 100);
            assert_eq!(r.login_id1, 200);
            assert_eq!(r.login_id2, 300);
            assert_eq!(r.sex, 1);
            assert_eq!(r.char_ip, [192, 168, 1, 1]);
            assert_eq!(r.char_port, 6900);
            assert_eq!(r.server_name, "TestServer");
            assert_eq!(r.user_count, 50);
            assert_eq!(r.server_type, 0);
            assert_eq!(r.new_flag, 0);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharEnterRequest 编解码测试 =====

#[test]
fn test_encode_decode_char_enter_request() {
    let req = CharEnterRequest { char_id: 1001 };
    let packet = Packet::CharEnterRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 8); // 4 header + 4 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharEnterRequest(r) => assert_eq!(r.char_id, 1001),
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharListRequest 编解码测试 =====

#[test]
fn test_encode_decode_char_list_request() {
    let packet = Packet::CharListRequest;
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 4); // 仅 header
    let decoded = PacketCodec::decode(&encoded).unwrap();
    assert!(matches!(decoded, Packet::CharListRequest));
}

// ===== CharListResponse 编解码测试 =====

fn make_test_char_info(id: u32) -> CharInfo {
    CharInfo {
        char_id: id, exp: 1000, gold: 500, job_exp: 200, job_level: 50,
        body_state: 0, health_state: 0, effect_state: 0, virtue: 0, honor: 0,
        job: 0, hair: 1, hair_color: 0, clothes_color: 0, body: 0, weapon: 1,
        head_bottom: 0, shield: 0, head_top: 0, head_mid: 0,
        hair_color2: 0, clothes_color2: 0, name: format!("Char{}", id),
        base_level: 99, str: 1, agi: 1, vit: 1, int: 1, dex: 1, luk: 1,
        slot: 0, delete_timer: 0, rename: 0, map_name: "prontera".to_string(),
    }
}

#[test]
fn test_encode_decode_char_list_response_empty() {
    let resp = CharListResponse { chars: vec![] };
    let packet = Packet::CharListResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 5); // 4 header + 1 count
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharListResponse(r) => assert!(r.chars.is_empty()),
        _ => panic!("解码结果类型不匹配"),
    }
}

#[test]
fn test_encode_decode_char_list_response_single() {
    let ch = make_test_char_info(42);
    let resp = CharListResponse { chars: vec![ch] };
    let packet = Packet::CharListResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 5 + 122); // header + count + 1 char (122 bytes each)
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharListResponse(r) => {
            assert_eq!(r.chars.len(), 1);
            assert_eq!(r.chars[0].char_id, 42);
            assert_eq!(r.chars[0].name, "Char42");
            assert_eq!(r.chars[0].base_level, 99);
            assert_eq!(r.chars[0].map_name, "prontera");
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

#[test]
fn test_encode_decode_char_list_response_multiple() {
    let chars = vec![make_test_char_info(1), make_test_char_info(2), make_test_char_info(3)];
    let resp = CharListResponse { chars };
    let packet = Packet::CharListResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 5 + 122 * 3);
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharListResponse(r) => {
            assert_eq!(r.chars.len(), 3);
            assert_eq!(r.chars[0].char_id, 1);
            assert_eq!(r.chars[1].char_id, 2);
            assert_eq!(r.chars[2].char_id, 3);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharCreateRequest 编解码测试 =====

#[test]
fn test_encode_decode_char_create_request() {
    let req = CharCreateRequest {
        name: "NewHero".to_string(), str: 5, agi: 5, vit: 5,
        int: 5, dex: 5, luk: 5, hair_color: 1, hair: 2,
    };
    let packet = Packet::CharCreateRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 38); // 4 header + 34 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharCreateRequest(r) => {
            assert_eq!(r.name, "NewHero");
            assert_eq!(r.str, 5);
            assert_eq!(r.agi, 5);
            assert_eq!(r.hair_color, 1);
            assert_eq!(r.hair, 2);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharCreateResponse 编解码测试 =====

#[test]
fn test_encode_decode_char_create_response_failure() {
    let resp = CharCreateResponse::Failure { error_code: 3 };
    let packet = Packet::CharCreateResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 6); // 4 header + 1 flag + 1 error
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharCreateResponse(CharCreateResponse::Failure { error_code }) => {
            assert_eq!(error_code, 3);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

#[test]
fn test_encode_decode_char_create_response_success() {
    let ch = make_test_char_info(99);
    let resp = CharCreateResponse::Success(ch);
    let packet = Packet::CharCreateResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 4 + 1 + 122); // header + flag + char_info (122 bytes)
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharCreateResponse(CharCreateResponse::Success(r)) => {
            assert_eq!(r.char_id, 99);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharDeleteRequest 编解码测试 =====

#[test]
fn test_encode_decode_char_delete_request() {
    let req = CharDeleteRequest { char_id: 500, email: "user@example.com".to_string() };
    let packet = Packet::CharDeleteRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 48); // 4 header + 44 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharDeleteRequest(r) => {
            assert_eq!(r.char_id, 500);
            assert_eq!(r.email, "user@example.com");
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharDeleteResponse 编解码测试 =====

#[test]
fn test_encode_decode_char_delete_response() {
    let resp = CharDeleteResponse { char_id: 500 };
    let packet = Packet::CharDeleteResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 8); // 4 header + 4 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharDeleteResponse(r) => assert_eq!(r.char_id, 500),
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== CharEnterResponse 编解码测试 =====

#[test]
fn test_encode_decode_char_enter_response() {
    let resp = CharEnterResponse {
        map_ip: "192.168.1.100".to_string(), map_port: 5121,
        token: "session_token_abc".to_string(),
    };
    let packet = Packet::CharEnterResponse(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 54); // 4 header + 16 ip + 2 port + 32 token
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::CharEnterResponse(r) => {
            assert_eq!(r.map_ip, "192.168.1.100");
            assert_eq!(r.map_port, 5121);
            assert_eq!(r.token, "session_token_abc");
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== MapEnterRequest 编解码测试 =====

#[test]
fn test_encode_decode_map_enter() {
    let req = MapEnterRequest { char_id: 100, login_id: 200, client_tick: 5000, gender: 1 };
    let packet = Packet::MapEnter(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 17); // 4 header + 13 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::MapEnter(r) => {
            assert_eq!(r.char_id, 100);
            assert_eq!(r.login_id, 200);
            assert_eq!(r.client_tick, 5000);
            assert_eq!(r.gender, 1);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== MapEnteredResponse 编解码测试 =====

#[test]
fn test_encode_decode_map_entered() {
    let resp = MapEnteredResponse { start_time: 12345, pos_x: 150, pos_y: 250, direction: 4, font: 0 };
    let packet = Packet::MapEntered(resp);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 16); // 4 header + 12 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::MapEntered(r) => {
            assert_eq!(r.start_time, 12345);
            assert_eq!(r.pos_x, 150);
            assert_eq!(r.pos_y, 250);
            assert_eq!(r.direction, 4);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== PlayerMoveRequest 编解码测试 =====

#[test]
fn test_encode_decode_player_move() {
    let req = PlayerMoveRequest { dest_x: 100, dest_y: 200 };
    let packet = Packet::PlayerMove(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 9); // 4 header + 4 coords + 1 move_data
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::PlayerMove(r) => {
            assert_eq!(r.dest_x, 100);
            assert_eq!(r.dest_y, 200);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== EntityMoveNotify 编解码测试 =====

#[test]
fn test_encode_decode_entity_move() {
    let notify = EntityMoveNotify {
        entity_id: 42, from_x: 10, from_y: 20, dest_x: 30, dest_y: 40, speed: 150,
    };
    let packet = Packet::EntityMove(notify);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 18); // 4 header + 14 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::EntityMove(r) => {
            assert_eq!(r.entity_id, 42);
            assert_eq!(r.from_x, 10);
            assert_eq!(r.from_y, 20);
            assert_eq!(r.dest_x, 30);
            assert_eq!(r.dest_y, 40);
            assert_eq!(r.speed, 150);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== ChatMessage 编解码测试 =====

#[test]
fn test_encode_decode_chat_message() {
    let msg = ChatMessage {
        sender_id: 1, sender_name: "Player".to_string(), message: "Hello!".to_string(),
    };
    let packet = Packet::ChatMessage(msg);
    let encoded = PacketCodec::encode(&packet).unwrap();
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::ChatMessage(r) => {
            assert_eq!(r.sender_id, 1);
            assert_eq!(r.sender_name, "Player");
            assert_eq!(r.message, "Hello!");
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== EntityAppearNotify 编解码测试 =====

#[test]
fn test_encode_decode_entity_appear() {
    let notify = EntityAppearNotify {
        entity_id: 500, entity_type: 5, pos_x: 150, pos_y: 250, direction: 4, look: 10,
    };
    let packet = Packet::EntityAppear(notify);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 16); // 4 header + 12 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::EntityAppear(r) => {
            assert_eq!(r.entity_id, 500);
            assert_eq!(r.entity_type, 5);
            assert_eq!(r.pos_x, 150);
            assert_eq!(r.pos_y, 250);
            assert_eq!(r.direction, 4);
            assert_eq!(r.look, 10);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== EntityDisappearNotify 编解码测试 =====

#[test]
fn test_encode_decode_entity_disappear() {
    let notify = EntityDisappearNotify { entity_id: 500, reason: 1 };
    let packet = Packet::EntityDisappear(notify);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded.len(), 9); // 4 header + 5 payload
    let decoded = PacketCodec::decode(&encoded).unwrap();
    match decoded {
        Packet::EntityDisappear(r) => {
            assert_eq!(r.entity_id, 500);
            assert_eq!(r.reason, 1);
        }
        _ => panic!("解码结果类型不匹配"),
    }
}

// ===== 错误处理测试 =====

#[test]
fn test_decode_invalid_packet() {
    let data = vec![0xFF, 0xFF, 0x00, 0x00];
    let result = PacketCodec::decode(&data);
    assert!(result.is_err());
}

#[test]
fn test_decode_too_short() {
    let data = vec![0x64, 0x00];
    let result = PacketCodec::decode(&data);
    assert!(result.is_err());
}

#[test]
fn test_decode_incomplete_login_request() {
    // 正确的 packet_id 但数据不够
    let mut data = vec![0x64, 0x00, 0x00, 0x00]; // header only
    data.resize(20, 0); // 不够 56 字节
    let result = PacketCodec::decode(&data);
    assert!(result.is_err());
}

// ===== Handler 分发测试 =====

#[test]
fn test_handler_register_and_dispatch() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let mut handler = PacketHandler::new();
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    handler.on_login_response(move |_resp| {
        called_clone.store(true, Ordering::Relaxed);
    });

    let resp = LoginResponse {
        account_id: 1, login_id1: 2, login_id2: 3, sex: 1,
        char_ip: [127, 0, 0, 1], char_port: 6900,
        server_name: "Test".to_string(), user_count: 0,
        server_type: 0, new_flag: 0,
    };
    let packet = Packet::LoginResponse(resp);
    handler.dispatch(&packet);
    assert!(called.load(Ordering::Relaxed));
}

#[test]
fn test_handler_char_list_dispatch() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let mut handler = PacketHandler::new();
    let count = Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();
    handler.on_char_list(move |resp| {
        count_clone.store(resp.chars.len(), Ordering::Relaxed);
    });

    let resp = CharListResponse { chars: vec![make_test_char_info(1), make_test_char_info(2)] };
    let packet = Packet::CharListResponse(resp);
    handler.dispatch(&packet);
    assert_eq!(count.load(Ordering::Relaxed), 2);
}
