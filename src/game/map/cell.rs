use serde::{Deserialize, Serialize};

/// 地图格子类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellType {
    Walkable,      // 可行走
    Wall,          // 墙壁
    Water,         // 水域
    Cliff,         // 悬崖
    Npc,           // NPC位置
    Warp,          // 传送点
    Snipable,      // 可射击(可穿过但不可行走)
    Icetrap,       // 冰陷阱
    Basilica,      // 圣域
    Landmine,      // 地雷
    NoChat,        // 禁止聊天
    Novice,        // 新手区
}

/// 地图格子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub x: u16,
    pub y: u16,
    pub cell_type: CellType,
    pub flags: u32,
}

impl Cell {
    pub fn new(x: u16, y: u16, cell_type: CellType) -> Self {
        Self {
            x,
            y,
            cell_type,
            flags: 0,
        }
    }

    pub fn is_walkable(&self) -> bool {
        matches!(self.cell_type, CellType::Walkable | CellType::Npc | CellType::Warp | CellType::Novice)
    }

    pub fn is_sight_blocking(&self) -> bool {
        matches!(self.cell_type, CellType::Wall | CellType::Cliff)
    }

    pub fn is_water(&self) -> bool {
        self.cell_type == CellType::Water
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::new(0, 0, CellType::Walkable)
    }
}
