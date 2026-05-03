use std::collections::HashMap;
use super::cell::{Cell, CellType};
use crate::game::rand::GameRng;

/// 地图数据
#[derive(Debug, Clone)]
pub struct MapData {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Vec<Cell>>,
}

impl MapData {
    pub fn new(name: &str, width: u16, height: u16) -> Self {
        let cells = (0..height)
            .map(|y| (0..width)
                .map(|x| Cell::new(x, y, CellType::Walkable))
                .collect())
            .collect();

        Self {
            name: name.to_string(),
            width,
            height,
            cells,
        }
    }

    pub fn get_cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.cells.get(y as usize)?.get(x as usize)
    }

    pub fn get_cell_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.cells.get_mut(y as usize)?.get_mut(x as usize)
    }

    pub fn is_walkable(&self, x: u16, y: u16) -> bool {
        self.get_cell(x, y).map(|c| c.is_walkable()).unwrap_or(false)
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell_type: CellType) {
        if let Some(cell) = self.get_cell_mut(x, y) {
            cell.cell_type = cell_type;
        }
    }

    /// 获取随机可行走位置
    pub fn random_walkable_pos(&self, rng: &dyn GameRng) -> Option<(u16, u16)> {
        for attempt in 0..100 {
            let seed = rng.rand_range(0, u32::MAX).wrapping_add(attempt as u32 * 7919);
            let x = (seed % self.width as u32) as u16;
            let y = ((seed / self.width as u32) % self.height as u32) as u16;

            if self.is_walkable(x, y) {
                return Some((x, y));
            }
        }

        None
    }
}

/// 地图数据库
pub struct MapDatabase {
    maps: HashMap<String, MapData>,
}

impl MapDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            maps: HashMap::new(),
        };
        db.init_default_maps();
        db
    }

    fn init_default_maps(&mut self) {
        // 新手村
        let mut new_1 = MapData::new("new_1-1.gat", 200, 200);
        // 边界设为墙
        for x in 0..200 {
            new_1.set_cell(x, 0, CellType::Wall);
            new_1.set_cell(x, 199, CellType::Wall);
        }
        for y in 0..200 {
            new_1.set_cell(0, y, CellType::Wall);
            new_1.set_cell(199, y, CellType::Wall);
        }
        // 中间水域
        for y in 80..120 {
            for x in 80..120 {
                new_1.set_cell(x, y, CellType::Water);
            }
        }
        self.maps.insert("new_1-1.gat".to_string(), new_1);

        // 普隆德拉
        let mut prontera = MapData::new("prontera.gat", 300, 300);
        for x in 0..300 {
            prontera.set_cell(x, 0, CellType::Wall);
            prontera.set_cell(x, 299, CellType::Wall);
        }
        for y in 0..300 {
            prontera.set_cell(0, y, CellType::Wall);
            prontera.set_cell(299, y, CellType::Wall);
        }
        self.maps.insert("prontera.gat".to_string(), prontera);
    }

    pub fn get(&self, map_name: &str) -> Option<&MapData> {
        self.maps.get(map_name)
    }

    pub fn get_mut(&mut self, map_name: &str) -> Option<&mut MapData> {
        self.maps.get_mut(map_name)
    }

    pub fn all(&self) -> impl Iterator<Item = &MapData> {
        self.maps.values()
    }
}

impl Default for MapDatabase {
    fn default() -> Self {
        Self::new()
    }
}
