use devi::protocol::Packet;
use devi::protocol::login::{LoginRequest, LoginResponse};
use devi::protocol::char_mod::{CharSelectRequest, CharListResponse};
use devi::net::transport::TransportState;
use devi::net::codec::PacketCodec;
use devi::net::handler::PacketHandler;

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
    let req = CharSelectRequest { server_index: 0 };
    let packet = Packet::CharSelectRequest(req);
    assert_eq!(packet.packet_id(), 0x0065);
}

#[test]
fn test_char_list_response() {
    let resp = CharListResponse { chars: vec![] };
    let packet = Packet::CharListResponse(resp);
    assert_eq!(packet.packet_id(), 0x006b);
}

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

#[test]
fn test_encode_login_request() {
    let req = LoginRequest {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };
    let packet = Packet::LoginRequest(req);
    let encoded = PacketCodec::encode(&packet).unwrap();
    assert_eq!(encoded[0], 0x64);
    assert_eq!(encoded[1], 0x00);
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
    let data = vec![0xFF, 0xFF, 0x00, 0x00];
    let result = PacketCodec::decode(&data);
    assert!(result.is_err());
}

#[test]
fn test_handler_register_and_dispatch() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let mut handler = PacketHandler::new();
    // 使用 Arc<AtomicBool> 以便在 Fn 回调中修改共享状态
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    handler.on_login_response(move |_resp| {
        called_clone.store(true, Ordering::Relaxed);
    });
    let packet = Packet::LoginResponse(LoginResponse::Failure { error_code: 1 });
    handler.dispatch(&packet);
    assert!(called.load(Ordering::Relaxed));
}
