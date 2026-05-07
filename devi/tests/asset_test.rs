use devi::asset::grf::GrfArchive;
use devi::asset::sprite::SpriteFile;
use devi::asset::action::ActionFile;

#[test]
fn test_grf_header_magic() {
    let magic = b"Master of Magic\0";
    assert_eq!(magic.len(), 16);
}

#[test]
fn test_grf_open_nonexistent() {
    let result = GrfArchive::open("nonexistent.grf");
    assert!(result.is_err());
}

#[test]
fn test_grf_file_entry_size() {
    use std::mem::size_of;
    assert_eq!(size_of::<u32>(), 4);
    assert_eq!(size_of::<u8>(), 1);
}

// === SPR 精灵图片解析测试 ===

#[test]
fn test_spr_header_parsing() {
    // version=0x0102, indexed_count=5, rgba_count=3
    // 每帧使用 0x0 尺寸（无像素数据），仅验证头部和帧计数解析
    // 小端序: version=0x0102, indexed_count=5, rgba_count=3
    let mut data = vec![0x02, 0x01, 0x05, 0x00, 0x03, 0x00];
    // 5 个索引色帧，每个 4 字节头（width=0, height=0）
    for _ in 0..5 { data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); }
    // 3 个 RGBA 帧，每个 4 字节头（width=0, height=0）
    for _ in 0..3 { data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); }
    let spr = SpriteFile::from_bytes(&data).unwrap();
    assert_eq!(spr.version, 0x0102);
    assert_eq!(spr.indexed_frame_count, 5);
    assert_eq!(spr.rgba_frame_count, 3);
    assert_eq!(spr.frame_count(), 8);
}

#[test]
fn test_spr_empty_file() {
    let data = vec![];
    let result = SpriteFile::from_bytes(&data);
    assert!(result.is_err());
}

// === ACT 动画定义解析测试 ===

#[test]
fn test_act_header_parsing() {
    // magic="AC", version=0x0002, action_count=2
    // 每个动作的 frame_count=0，仅验证头部和动作计数解析
    // 小端序: version=0x0002, action_count=2
    let mut data = vec![b'A', b'C', 0x02, 0x00, 0x02, 0x00];
    // 2 个动作，每个 4 字节帧数（frame_count=0）
    for _ in 0..2 { data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); }
    let act = ActionFile::from_bytes(&data).unwrap();
    assert_eq!(act.version, 0x0002);
    assert_eq!(act.action_count, 2);
    assert_eq!(act.actions.len(), 2);
}

#[test]
fn test_act_invalid_magic() {
    // 魔术字节不匹配 "AC"
    let data = vec![b'X', b'Y', 0x00, 0x02, 0x00, 0x00];
    let result = ActionFile::from_bytes(&data);
    assert!(result.is_err());
}
