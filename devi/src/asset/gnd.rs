// GND 地面网格解析器
// GND (Ground) 是 Ragnarok Online 使用的地面网格数据格式，
// 使用 "GRGN" 魔术字节标识。
// 文件结构：头部 → 纹理表 → 光照贴图 → 瓦片表 → 单元格网格

use super::AssetError;

/// GND 纹理条目
/// 纹理文件名（通常为 BMP 格式，路径相对于 data/texture/）
#[derive(Debug, Clone)]
pub struct GndTexture {
    /// 纹理文件路径（如 "ground/brock01.bmp"）
    pub name: String,
}

/// GND 贴图瓦片
/// 定义纹理到地面表面的映射关系，包含纹理索引和 UV 坐标。
/// UV 坐标使用 0-63 范围的整数，除以 64 转换为 0.0-1.0。
#[derive(Debug, Clone)]
pub struct GndTile {
    /// 纹理索引（u16::MAX 表示无纹理）
    pub tex_id: u16,
    /// 光照贴图索引
    pub lightmap_id: u16,
    /// UV 起始 X 坐标（0-63）
    pub x1: u8,
    /// UV 起始 Y 坐标（0-63）
    pub y1: u8,
    /// UV 结束 X 坐标（0-63）
    pub x2: u8,
    /// UV 结束 Y 坐标（0-63）
    pub y2: u8,
}

/// GND 地面单元格
/// 定义地图上一个 2x2 单位区域的地面信息。
/// 包含四角高度和三个面的贴图引用：
/// - 上面（地面）
/// - 前面（南侧墙壁，与相邻行的高差产生）
/// - 右面（东侧墙壁，与相邻列的高差产生）
#[derive(Debug, Clone)]
pub struct GndCell {
    /// 四个角的高度值（左下、右下、左上、右上）
    pub heights: [f32; 4],
    /// 上表面瓦片索引（u16::MAX 表示无贴图）
    pub tile_up: u16,
    /// 前表面瓦片索引（u16::MAX 表示无贴图）
    pub tile_front: u16,
    /// 右表面瓦片索引（u16::MAX 表示无贴图）
    pub tile_right: u16,
}

/// GND 地面网格文件
/// 包含完整的地面渲染数据：纹理列表、瓦片映射和单元格高度网格
#[derive(Debug)]
pub struct GndFile {
    /// 文件版本（如 170 表示 1.7）
    pub version: i32,
    /// 网格宽度（cell 数量）
    pub width: u32,
    /// 网格高度（cell 数量）
    pub height: u32,
    /// 缩放系数（版本 >= 1.8 时由文件指定，否则默认 1.0）
    pub zoom: f32,
    /// 纹理列表
    pub textures: Vec<GndTexture>,
    /// 瓦片列表（纹理到 UV 的映射）
    pub tiles: Vec<GndTile>,
    /// 地面单元格列表（按行优先排列：y * width + x）
    pub cells: Vec<GndCell>,
}

impl GndFile {
    /// 从字节数据解析 GND 文件
    /// 完整解析所有区段：头部、纹理、光照贴图（跳过数据）、瓦片、单元格
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        // 最小头部：4(magic) + 4(version) + 4(width) + 4(height) = 16 字节
        if data.len() < 16 {
            return Err(AssetError::ParseError(
                "GND 文件太小，无法读取头部".to_string(),
            ));
        }

        // 验证魔术字节 "GRGN"
        if &data[0..4] != b"GRGN" {
            return Err(AssetError::ParseError(
                "无效的 GND 文件：魔术字节不匹配".to_string(),
            ));
        }

        // 解析头部字段（全部小端序）
        let version = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let width = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let height = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

        let mut offset = 16;

        // 版本 >= 108（即 1.8）时读取缩放系数
        let zoom = if version >= 108 {
            if data.len() < offset + 4 {
                return Err(AssetError::ParseError(
                    "GND 数据不完整：无法读取 zoom".to_string(),
                ));
            }
            let z = f32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            z
        } else {
            1.0
        };

        // 依次解析各区段
        let textures = Self::parse_textures(data, &mut offset)?;
        Self::skip_lightmaps(data, &mut offset)?;
        let tiles = Self::parse_tiles(data, &mut offset)?;
        let cells = Self::parse_cells(data, &mut offset, width, height)?;

        Ok(Self {
            version,
            width,
            height,
            zoom,
            textures,
            tiles,
            cells,
        })
    }

    /// 获取指定位置的单元格
    /// 坐标系：x 为水平方向（列），y 为垂直方向（行）
    pub fn get_cell(&self, x: u32, y: u32) -> Option<&GndCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        self.cells.get(index)
    }

    /// 获取瓦片引用的纹理路径
    /// 返回 Some(纹理路径) 或 None（瓦片无效或纹理索引越界）
    pub fn tile_texture_path(&self, tile_idx: u16) -> Option<&str> {
        if tile_idx == u16::MAX {
            return None;
        }
        self.tiles
            .get(tile_idx as usize)
            .and_then(|tile| {
                if tile.tex_id == u16::MAX {
                    None
                } else {
                    self.textures.get(tile.tex_id as usize).map(|t| t.name.as_str())
                }
            })
    }

    // ===== 内部解析方法 =====

    /// 解析纹理名称表
    /// 格式：count(u32) + count * char[64]（null 填充的文件名）
    fn parse_textures(data: &[u8], offset: &mut usize) -> Result<Vec<GndTexture>, AssetError> {
        let count = Self::read_u32(data, offset)? as usize;
        let mut textures = Vec::with_capacity(count);

        for _ in 0..count {
            if data.len() < *offset + 64 {
                return Err(AssetError::ParseError(
                    "GND 数据不完整：无法读取纹理名称".to_string(),
                ));
            }
            let name_bytes = &data[*offset..*offset + 64];
            // 截断到第一个 null 字节
            let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(64);
            let name = String::from_utf8_lossy(&name_bytes[..name_len]).to_string();
            textures.push(GndTexture { name });
            *offset += 64;
        }

        Ok(textures)
    }

    /// 跳过光照贴图数据
    /// 格式：count(u32) + width(u32) + height(u32) + per_cell(u32) + data[...]
    /// 光照贴图数据为 RGBA 格式，当前渲染不使用，直接跳过
    fn skip_lightmaps(data: &[u8], offset: &mut usize) -> Result<(), AssetError> {
        let count = Self::read_u32(data, offset)? as usize;
        let lm_width = Self::read_u32(data, offset)? as usize;
        let lm_height = Self::read_u32(data, offset)? as usize;
        let per_cell = Self::read_u32(data, offset)? as usize;

        // 每个光照贴图占 lm_width * lm_height * per_cell * 4 字节 (RGBA)
        let lm_data_size = count * lm_width * lm_height * per_cell * 4;
        if data.len() < *offset + lm_data_size {
            return Err(AssetError::ParseError(
                "GND 数据不完整：光照贴图数据不足".to_string(),
            ));
        }
        *offset += lm_data_size;
        Ok(())
    }

    /// 解析瓦片表
    /// 格式：count(u32) + count * 8字节（tex_id:u16, lightmap_id:u16, x1:u8, y1:u8, x2:u8, y2:u8）
    fn parse_tiles(data: &[u8], offset: &mut usize) -> Result<Vec<GndTile>, AssetError> {
        let count = Self::read_u32(data, offset)? as usize;
        let mut tiles = Vec::with_capacity(count);

        for _ in 0..count {
            if data.len() < *offset + 8 {
                return Err(AssetError::ParseError(
                    "GND 数据不完整：瓦片数据不足".to_string(),
                ));
            }
            let tex_id = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
            let lightmap_id = u16::from_le_bytes([data[*offset + 2], data[*offset + 3]]);
            let x1 = data[*offset + 4];
            let y1 = data[*offset + 5];
            let x2 = data[*offset + 6];
            let y2 = data[*offset + 7];
            *offset += 8;

            tiles.push(GndTile {
                tex_id,
                lightmap_id,
                x1,
                y1,
                x2,
                y2,
            });
        }

        Ok(tiles)
    }

    /// 解析地面单元格网格
    /// 格式：count(u32) + count * 22字节
    /// 每个单元格：4*f32(高度) + 3*i16(瓦片索引，-1=无)
    fn parse_cells(
        data: &[u8],
        offset: &mut usize,
        width: u32,
        height: u32,
    ) -> Result<Vec<GndCell>, AssetError> {
        let count = Self::read_u32(data, offset)? as usize;
        let expected = (width * height) as usize;

        if count != expected {
            return Err(AssetError::ParseError(format!(
                "GND 单元格数量不匹配：期望 {}，实际 {}",
                expected, count
            )));
        }

        let mut cells = Vec::with_capacity(count);

        for _ in 0..count {
            if data.len() < *offset + 22 {
                return Err(AssetError::ParseError(
                    "GND 数据不完整：单元格数据不足".to_string(),
                ));
            }

            // 4 个角的高度值（小端序 f32）
            let h0 = f32::from_le_bytes([
                data[*offset],
                data[*offset + 1],
                data[*offset + 2],
                data[*offset + 3],
            ]);
            let h1 = f32::from_le_bytes([
                data[*offset + 4],
                data[*offset + 5],
                data[*offset + 6],
                data[*offset + 7],
            ]);
            let h2 = f32::from_le_bytes([
                data[*offset + 8],
                data[*offset + 9],
                data[*offset + 10],
                data[*offset + 11],
            ]);
            let h3 = f32::from_le_bytes([
                data[*offset + 12],
                data[*offset + 13],
                data[*offset + 14],
                data[*offset + 15],
            ]);

            // 瓦片索引为有符号 i16，-1 表示无贴图，转为 u16::MAX
            let tile_up_raw = i16::from_le_bytes([data[*offset + 16], data[*offset + 17]]);
            let tile_front_raw = i16::from_le_bytes([data[*offset + 18], data[*offset + 19]]);
            let tile_right_raw = i16::from_le_bytes([data[*offset + 20], data[*offset + 21]]);
            *offset += 22;

            let tile_up = if tile_up_raw < 0 {
                u16::MAX
            } else {
                tile_up_raw as u16
            };
            let tile_front = if tile_front_raw < 0 {
                u16::MAX
            } else {
                tile_front_raw as u16
            };
            let tile_right = if tile_right_raw < 0 {
                u16::MAX
            } else {
                tile_right_raw as u16
            };

            cells.push(GndCell {
                heights: [h0, h1, h2, h3],
                tile_up,
                tile_front,
                tile_right,
            });
        }

        Ok(cells)
    }

    /// 辅助方法：读取小端序 u32 并推进偏移
    fn read_u32(data: &[u8], offset: &mut usize) -> Result<u32, AssetError> {
        if data.len() < *offset + 4 {
            return Err(AssetError::ParseError(
                "GND 数据不完整：无法读取 u32".to_string(),
            ));
        }
        let value = u32::from_le_bytes([
            data[*offset],
            data[*offset + 1],
            data[*offset + 2],
            data[*offset + 3],
        ]);
        *offset += 4;
        Ok(value)
    }
}
