use super::AssetError;

/// GAT 文件（地面高度网格）
/// GAT 是 Ragnarok Online 使用的地面高度数据格式，
/// 使用 "GRAT" 魔术字节标识，每个单元格包含四个角的高度和类型信息。
#[derive(Debug)]
pub struct GatFile {
    /// 版本号
    pub version: u16,
    /// 网格宽度（cell 数量）
    pub width: u32,
    /// 网格高度（cell 数量）
    pub height: u32,
    /// 地面单元格列表（按行优先排列）
    pub cells: Vec<GatCell>,
}

/// 地面单元格
/// 每个单元格定义了地图上一个 5x5 单位区域的地面信息
#[derive(Debug, Clone)]
pub struct GatCell {
    /// 四个角的高度值（左下、右下、左上、右上）
    pub heights: [f32; 4],
    /// 单元格类型（0=可行走, 1=不可行走, 2=水, 等）
    pub cell_type: u32,
}

impl GatFile {
    /// 从字节数据解析 GAT 文件
    /// 文件结构：4字节魔术字节 + 2字节版本 + 4字节宽度 + 4字节高度 + N*20字节单元格
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        // 验证最小头部大小（4+2+4+4=14字节）
        if data.len() < 14 {
            return Err(AssetError::ParseError("GAT 文件太小，无法读取头部".to_string()));
        }
        // 验证魔术字节 "GRAT"
        if &data[0..4] != b"GRAT" {
            return Err(AssetError::ParseError("无效的 GAT 文件：魔术字节不匹配".to_string()));
        }

        // 解析头部字段（全部小端序）
        let version = u16::from_le_bytes([data[4], data[5]]);
        let width = u32::from_le_bytes([data[6], data[7], data[8], data[9]]);
        let height = u32::from_le_bytes([data[10], data[11], data[12], data[13]]);

        // 计算预期数据大小并验证完整性
        let cell_count = (width * height) as usize;
        let expected_size = 14 + cell_count * 20;
        if data.len() < expected_size {
            return Err(AssetError::ParseError("GAT 文件数据不完整".to_string()));
        }

        // 逐个解析单元格，每个 20 字节：4个f32高度 + 1个u32类型
        let mut cells = Vec::with_capacity(cell_count);
        let mut offset = 14;
        for _ in 0..cell_count {
            let h1 = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            let h2 = f32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
            let h3 = f32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
            let h4 = f32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]);
            let cell_type = u32::from_le_bytes([data[offset+16], data[offset+17], data[offset+18], data[offset+19]]);
            offset += 20;
            cells.push(GatCell { heights: [h1, h2, h3, h4], cell_type });
        }

        Ok(Self { version, width, height, cells })
    }

    /// 获取指定位置的单元格
    /// 坐标系：x 为水平方向（列），y 为垂直方向（行）
    pub fn get_cell(&self, x: u32, y: u32) -> Option<&GatCell> {
        if x >= self.width || y >= self.height { return None; }
        let index = (y * self.width + x) as usize;
        self.cells.get(index)
    }

    /// 检查指定位置是否可行走
    /// cell_type == 0 表示可行走
    pub fn is_walkable(&self, x: u32, y: u32) -> bool {
        self.get_cell(x, y).map(|c| c.cell_type == 0).unwrap_or(false)
    }
}
