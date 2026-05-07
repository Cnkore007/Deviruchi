use super::AssetError;

/// ACT 文件（动画定义）
#[derive(Debug)]
pub struct ActionFile {
    /// 版本号
    pub version: u16,
    /// 动作数量
    pub action_count: u16,
    /// 动作列表
    pub actions: Vec<Action>,
}

/// 单个动画动作
#[derive(Debug)]
pub struct Action {
    /// 帧列表
    pub frames: Vec<ActionFrame>,
}

/// 动画帧
#[derive(Debug)]
pub struct ActionFrame {
    /// 精灵索引列表（一帧可由多个精灵组合）
    pub sprites: Vec<FrameSprite>,
    /// 事件类型（音效、特效触发等）
    pub event_type: u32,
}

/// 帧中的单个精灵
#[derive(Debug)]
pub struct FrameSprite {
    /// 精灵图集索引
    pub sprite_index: u32,
    /// X 偏移（像素）
    pub x: i32,
    /// Y 偏移（像素）
    pub y: i32,
    /// 缩放 X
    pub scale_x: f32,
    /// 缩放 Y
    pub scale_y: f32,
    /// 旋转角度（度）
    pub rotation: f32,
    /// 颜色 (RGBA)
    pub color: [u8; 4],
    /// 变换类型
    pub mirroring: u8,
}

impl ActionFile {
    /// 从字节数据解析 ACT 文件
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 6 {
            return Err(AssetError::ParseError("ACT 文件太小，无法读取头部".to_string()));
        }
        if data[0] != b'A' || data[1] != b'C' {
            return Err(AssetError::ParseError("无效的 ACT 文件：魔术字节不匹配".to_string()));
        }

        let version = u16::from_le_bytes([data[2], data[3]]);
        let action_count = u16::from_le_bytes([data[4], data[5]]);

        let mut actions = Vec::new();
        let mut offset = 6;

        for _ in 0..action_count {
            if offset + 4 > data.len() {
                return Err(AssetError::ParseError("ACT 动作数据不完整".to_string()));
            }
            let frame_count = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
            offset += 4;

            let mut frames = Vec::new();
            for _ in 0..frame_count {
                let (frame, new_offset) = Self::read_frame(data, offset, version)?;
                frames.push(frame);
                offset = new_offset;
            }
            actions.push(Action { frames });
        }

        Ok(Self { version, action_count, actions })
    }

    /// 读取单个动画帧
    fn read_frame(data: &[u8], mut offset: usize, version: u16) -> Result<(ActionFrame, usize), AssetError> {
        if offset + 32 > data.len() {
            return Err(AssetError::ParseError("ACT 帧数据不完整".to_string()));
        }
        offset += 16; // 跳过帧范围（4 个 i32）

        let event_type = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        offset += 4;

        let sprite_count = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
        offset += 4;
        offset += 4; // 跳过保留字段

        let mut sprites = Vec::new();
        for _ in 0..sprite_count {
            let (sprite, new_offset) = Self::read_sprite(data, offset, version)?;
            sprites.push(sprite);
            offset = new_offset;
        }

        Ok((ActionFrame { sprites, event_type }, offset))
    }

    /// 读取帧中的单个精灵数据
    fn read_sprite(data: &[u8], offset: usize, version: u16) -> Result<(FrameSprite, usize), AssetError> {
        let sprite_index = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        let x = i32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
        let y = i32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
        let scale_x = f32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
        let scale_y = f32::from_le_bytes([data[offset+16], data[offset+17], data[offset+18], data[offset+19]]);
        let rotation = f32::from_le_bytes([data[offset+20], data[offset+21], data[offset+22], data[offset+23]]);
        let color = [data[offset+24], data[offset+25], data[offset+26], data[offset+27]];
        let mirroring = data[offset+28];
        let mut end = offset + 30;
        // ACT 2.0+ 版本精灵数据多 2 字节
        if version >= 0x0200 { end += 2; }

        Ok((FrameSprite { sprite_index, x, y, scale_x, scale_y, rotation, color, mirroring }, end))
    }
}
