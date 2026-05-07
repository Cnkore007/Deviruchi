use super::AssetError;

/// SPR 文件（精灵图片数据）
#[derive(Debug)]
pub struct SpriteFile {
    /// 版本号（如 0x0102 表示 1.2）
    pub version: u16,
    /// 索引色帧数量
    pub indexed_frame_count: u16,
    /// RGBA 帧数量
    pub rgba_frame_count: u16,
    /// 帧数据
    pub frames: Vec<SpriteFrame>,
}

/// 精灵帧
#[derive(Debug)]
pub enum SpriteFrame {
    /// 索引色帧（8bit，需配合调色板）
    Indexed { width: u16, height: u16, data: Vec<u8> },
    /// RGBA 帧（32bit 真彩色）
    Rgba { width: u16, height: u16, data: Vec<u8> },
}

impl SpriteFile {
    /// 从字节数据解析 SPR 文件
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 6 {
            return Err(AssetError::ParseError("SPR 文件太小，无法读取头部".to_string()));
        }

        let version = u16::from_le_bytes([data[0], data[1]]);
        let indexed_count = u16::from_le_bytes([data[2], data[3]]);
        let rgba_count = u16::from_le_bytes([data[4], data[5]]);

        let mut frames = Vec::new();
        let mut offset = 6;

        // 读取索引色帧
        for _ in 0..indexed_count {
            if offset + 4 > data.len() {
                return Err(AssetError::ParseError("SPR 索引色帧数据不完整".to_string()));
            }
            let width = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let height = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
            offset += 4;

            let pixel_count = width as usize * height as usize;
            if offset + pixel_count > data.len() {
                return Err(AssetError::ParseError("SPR 索引色帧像素数据不完整".to_string()));
            }
            let frame_data = data[offset..offset + pixel_count].to_vec();
            offset += pixel_count;
            frames.push(SpriteFrame::Indexed { width, height, data: frame_data });
        }

        // 读取 RGBA 帧
        for _ in 0..rgba_count {
            if offset + 4 > data.len() {
                return Err(AssetError::ParseError("SPR RGBA 帧数据不完整".to_string()));
            }
            let width = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let height = u16::from_le_bytes([data[offset + 2], data[offset + 3]]);
            offset += 4;

            let pixel_count = width as usize * height as usize * 4;
            if offset + pixel_count > data.len() {
                return Err(AssetError::ParseError("SPR RGBA 帧像素数据不完整".to_string()));
            }
            let frame_data = data[offset..offset + pixel_count].to_vec();
            offset += pixel_count;
            frames.push(SpriteFrame::Rgba { width, height, data: frame_data });
        }

        Ok(Self { version, indexed_frame_count: indexed_count, rgba_frame_count: rgba_count, frames })
    }

    /// 总帧数
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}
