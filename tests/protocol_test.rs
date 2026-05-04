use deviruchi::protocol::Packed;
use deviruchi::protocol::login_packets::{ACAceptLogin, CALogin};

#[test]
fn test_ca_login_pack() {
    let packet = CALogin {
        version: 20,
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };
    let bytes = packet.to_packet();
    // Header (2+2=4 bytes) + version (4) + username (24) + password (24) = 56
    assert_eq!(bytes.len(), 4 + 4 + 24 + 24);
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0x0064);
}

#[test]
fn test_ca_login_parse() {
    let mut raw = vec![
        0x14, 0x00, 0x00, 0x00, // version = 20
    ];
    // username: "testuser" padded to 24
    raw.extend_from_slice(b"testuser");
    raw.extend(vec![0; 24 - 8]); // padding
    // password: "testpass" padded to 24
    raw.extend_from_slice(b"testpass");
    raw.extend(vec![0; 24 - 8]); // padding

    let packet = CALogin::from_slice(&raw).unwrap();
    assert_eq!(packet.version, 20);
    assert_eq!(packet.username, "testuser");
    assert_eq!(packet.password, "testpass");
}

#[test]
fn test_ac_acept_login_pack() {
    let packet = ACAceptLogin {
        account_id: 12345,
        login_id1: 11111,
        login_id2: 22222,
        sex: 0,
    };
    let bytes = packet.to_packet();
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0x0069);
}
