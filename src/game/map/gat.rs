//! .gat 文件解析器
//!
//! rAthena 地图文件格式 (版本 5):
//! - 文件头: "GRAT" (4 bytes magic) + version (u16) + width (u16) + height (u16)
//! - Cell 数据: width * height 个 cell，每个 cell 4 bytes
//!   - byte 0: cell type (0=walkable, 1=wall, 2=water, 3=cliff, ...)
//!   - byte 1-3: padding (unused)
//!
//! 注意: rAthena 使用小端序 (little-endian)

use super::cell::{Cell, CellType};
use super::data::MapData;
use std::fs;
use std::io;
use std::path::Path;

/// .gat 文件解析错误
#[derive(Debug, thiserror::Error)]
pub enum GatError {
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),

    #[error("无效的 magic number: 期望 'GRAT'，实际: {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("不支持的版本: {0}")]
    UnsupportedVersion(u16),

    #[error("地图尺寸无效: {width}x{height}")]
    InvalidDimensions { width: u16, height: u16 },

    #[error("文件数据不完整: 期望 {expected} bytes，实际 {actual} bytes")]
    IncompleteData { expected: usize, actual: usize },
}

/// rAthena .gat cell 类型映射
///
/// rAthena 的 cell type 值:
/// 0 = 可行走 (Walkable)
/// 1 = 墙壁 (Wall)
/// 2 = 水域 (Water)
/// 3 = 悬崖 (Cliff)
/// 4 = 可射击 (Snipable)
/// 5 = NPC 位置
/// 6 = 传送点 (Warp)
/// 7 = 冰陷阱 (Icetrap)
/// 8 = 圣域 (Basilica)
/// 9 = 地雷 (Landmine)
/// 10 = 禁止聊天 (NoChat)
/// 11 = 新手区 (Novice)
fn cell_type_from_u8(value: u8) -> CellType {
    match value {
        0 => CellType::Walkable,
        1 => CellType::Wall,
        2 => CellType::Water,
        3 => CellType::Cliff,
        4 => CellType::Snipable,
        5 => CellType::Npc,
        6 => CellType::Warp,
        7 => CellType::Icetrap,
        8 => CellType::Basilica,
        9 => CellType::Landmine,
        10 => CellType::NoChat,
        11 => CellType::Novice,
        _ => CellType::Wall, // 未知类型视为墙壁
    }
}

fn cell_type_to_u8(cell_type: CellType) -> u8 {
    match cell_type {
        CellType::Walkable => 0,
        CellType::Wall => 1,
        CellType::Water => 2,
        CellType::Cliff => 3,
        CellType::Snipable => 4,
        CellType::Npc => 5,
        CellType::Warp => 6,
        CellType::Icetrap => 7,
        CellType::Basilica => 8,
        CellType::Landmine => 9,
        CellType::NoChat => 10,
        CellType::Novice => 11,
    }
}

/// .gat 文件解析器
pub struct GatParser;

impl GatParser {
    /// 从文件路径解析 .gat 文件
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<MapData, GatError> {
        let data = fs::read(path.as_ref())?;
        let map_name = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        Self::parse_bytes(&data, &map_name)
    }

    /// 将 MapData 序列化为 .gat 二进制格式
    pub fn to_gat_bytes(map: &MapData) -> Vec<u8> {
        let mut data = Vec::with_capacity(10 + map.width as usize * map.height as usize * 4);
        // Magic: "GRAT"
        data.extend_from_slice(b"GRAT");
        // Version: 5
        data.extend_from_slice(&5u16.to_le_bytes());
        // Width, Height
        data.extend_from_slice(&map.width.to_le_bytes());
        data.extend_from_slice(&map.height.to_le_bytes());
        // Cell data: 每个 cell 4 bytes
        for y in 0..map.height {
            for x in 0..map.width {
                let cell_type = map
                    .get_cell(x, y)
                    .map(|c| c.cell_type)
                    .unwrap_or(CellType::Wall);
                data.push(cell_type_to_u8(cell_type));
                data.push(0); // padding
                data.push(0); // padding
                data.push(0); // padding
            }
        }
        data
    }

    /// 将 MapData 写入 .gat 文件
    pub fn write_file<P: AsRef<Path>>(path: P, map: &MapData) -> Result<(), GatError> {
        let data = Self::to_gat_bytes(map);
        fs::write(path, data)?;
        Ok(())
    }

    /// 从字节数组解析 .gat 数据
    pub fn parse_bytes(data: &[u8], map_name: &str) -> Result<MapData, GatError> {
        // 验证最小大小: 4 (magic) + 2 (version) + 2 (width) + 2 (height) = 10 bytes
        if data.len() < 10 {
            return Err(GatError::IncompleteData {
                expected: 10,
                actual: data.len(),
            });
        }

        // 解析 magic number ("GRAT")
        let magic = [data[0], data[1], data[2], data[3]];
        if &magic != b"GRAT" {
            return Err(GatError::InvalidMagic(magic));
        }

        // 解析版本号 (小端序)
        let version = u16::from_le_bytes([data[4], data[5]]);

        // 解析地图尺寸 (小端序)
        let width = u16::from_le_bytes([data[6], data[7]]);
        let height = u16::from_le_bytes([data[8], data[9]]);

        // 验证版本
        if version > 5 {
            return Err(GatError::UnsupportedVersion(version));
        }

        // 验证尺寸
        if width == 0 || height == 0 {
            return Err(GatError::InvalidDimensions { width, height });
        }

        // 验证数据完整性: header(10) + width*height*4 (每个 cell 4 bytes)
        let expected_size = 10 + (width as usize * height as usize * 4);
        if data.len() < expected_size {
            return Err(GatError::IncompleteData {
                expected: expected_size,
                actual: data.len(),
            });
        }

        // 解析 cell 数据
        let mut cells = Vec::with_capacity(height as usize);
        let mut offset = 10; // 跳过文件头

        for y in 0..height {
            let mut row = Vec::with_capacity(width as usize);
            for x in 0..width {
                let cell_type_byte = data[offset];
                // 每个 cell 4 bytes，但我们只需要第一个字节（类型）
                offset += 4;

                let cell_type = cell_type_from_u8(cell_type_byte);
                row.push(Cell::new(x, y, cell_type));
            }
            cells.push(row);
        }

        Ok(MapData {
            name: format!("{}.gat", map_name),
            width,
            height,
            cells,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用的 .gat 二进制数据
    fn build_gat_bytes(width: u16, height: u16, cell_types: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        // Magic: "GRAT"
        data.extend_from_slice(b"GRAT");
        // Version: 5
        data.extend_from_slice(&5u16.to_le_bytes());
        // Width, Height
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        // Cell data: 每个 cell 4 bytes
        for &ct in cell_types {
            data.push(ct); // cell type
            data.push(0); // padding
            data.push(0); // padding
            data.push(0); // padding
        }
        data
    }

    #[test]
    fn test_parse_simple_map() {
        // 2x2 地图: [walkable, wall, water, cliff]
        let data = build_gat_bytes(2, 2, &[0, 1, 2, 3]);
        let map = GatParser::parse_bytes(&data, "test_map").unwrap();

        assert_eq!(map.name, "test_map.gat");
        assert_eq!(map.width, 2);
        assert_eq!(map.height, 2);

        assert_eq!(map.get_cell(0, 0).unwrap().cell_type, CellType::Walkable);
        assert_eq!(map.get_cell(1, 0).unwrap().cell_type, CellType::Wall);
        assert_eq!(map.get_cell(0, 1).unwrap().cell_type, CellType::Water);
        assert_eq!(map.get_cell(1, 1).unwrap().cell_type, CellType::Cliff);
    }

    #[test]
    fn test_parse_walkable_check() {
        let data = build_gat_bytes(3, 1, &[0, 1, 0]);
        let map = GatParser::parse_bytes(&data, "walk_test").unwrap();

        assert!(map.is_walkable(0, 0));
        assert!(!map.is_walkable(1, 0));
        assert!(map.is_walkable(2, 0));
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = build_gat_bytes(1, 1, &[0]);
        data[0] = b'X'; // 破坏 magic
        let result = GatParser::parse_bytes(&data, "bad");
        assert!(matches!(result, Err(GatError::InvalidMagic(_))));
    }

    #[test]
    fn test_incomplete_data() {
        let data = b"GRAT\x05\x00\x02\x00\x02\x00"; // header only, no cell data
        let result = GatParser::parse_bytes(data, "incomplete");
        assert!(matches!(result, Err(GatError::IncompleteData { .. })));
    }

    #[test]
    fn test_zero_dimensions() {
        let data = build_gat_bytes(0, 0, &[]);
        let result = GatParser::parse_bytes(&data, "zero");
        assert!(matches!(result, Err(GatError::InvalidDimensions { .. })));
    }

    #[test]
    fn test_unknown_cell_type() {
        // 类型 255 应该映射为 Wall
        let data = build_gat_bytes(1, 1, &[255]);
        let map = GatParser::parse_bytes(&data, "unknown").unwrap();
        assert_eq!(map.get_cell(0, 0).unwrap().cell_type, CellType::Wall);
    }

    #[test]
    fn test_write_and_read_roundtrip() {
        // 创建一个有各种 cell 类型的地图
        let mut map = MapData::new("roundtrip_test", 4, 4);
        map.set_cell(0, 0, CellType::Walkable);
        map.set_cell(1, 0, CellType::Wall);
        map.set_cell(2, 0, CellType::Water);
        map.set_cell(3, 0, CellType::Npc);
        map.set_cell(0, 1, CellType::Warp);
        map.set_cell(1, 1, CellType::Snipable);

        // 写入字节
        let bytes = GatParser::to_gat_bytes(&map);

        // 读回
        let parsed = GatParser::parse_bytes(&bytes, "roundtrip_test").unwrap();

        assert_eq!(parsed.name, "roundtrip_test.gat");
        assert_eq!(parsed.width, 4);
        assert_eq!(parsed.height, 4);
        assert_eq!(parsed.get_cell(0, 0).unwrap().cell_type, CellType::Walkable);
        assert_eq!(parsed.get_cell(1, 0).unwrap().cell_type, CellType::Wall);
        assert_eq!(parsed.get_cell(2, 0).unwrap().cell_type, CellType::Water);
        assert_eq!(parsed.get_cell(3, 0).unwrap().cell_type, CellType::Npc);
        assert_eq!(parsed.get_cell(0, 1).unwrap().cell_type, CellType::Warp);
        assert_eq!(parsed.get_cell(1, 1).unwrap().cell_type, CellType::Snipable);
    }

    #[test]
    fn test_all_cell_types() {
        // 测试所有 rAthena cell 类型映射
        let types = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let data = build_gat_bytes(12, 1, &types);
        let map = GatParser::parse_bytes(&data, "all_types").unwrap();

        assert_eq!(map.get_cell(0, 0).unwrap().cell_type, CellType::Walkable);
        assert_eq!(map.get_cell(1, 0).unwrap().cell_type, CellType::Wall);
        assert_eq!(map.get_cell(2, 0).unwrap().cell_type, CellType::Water);
        assert_eq!(map.get_cell(3, 0).unwrap().cell_type, CellType::Cliff);
        assert_eq!(map.get_cell(4, 0).unwrap().cell_type, CellType::Snipable);
        assert_eq!(map.get_cell(5, 0).unwrap().cell_type, CellType::Npc);
        assert_eq!(map.get_cell(6, 0).unwrap().cell_type, CellType::Warp);
        assert_eq!(map.get_cell(7, 0).unwrap().cell_type, CellType::Icetrap);
        assert_eq!(map.get_cell(8, 0).unwrap().cell_type, CellType::Basilica);
        assert_eq!(map.get_cell(9, 0).unwrap().cell_type, CellType::Landmine);
        assert_eq!(map.get_cell(10, 0).unwrap().cell_type, CellType::NoChat);
        assert_eq!(map.get_cell(11, 0).unwrap().cell_type, CellType::Novice);
    }
}
