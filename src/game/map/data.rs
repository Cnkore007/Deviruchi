use super::cell::{Cell, CellType};
use crate::game::rand::GameRng;
use std::collections::HashMap;

/// CharacterData - 角色数据（用于创建 Player 实例）
#[derive(Debug, Clone)]
pub struct CharacterData {
    pub char_id: u32,
    pub account_id: u32,
    pub name: String,
    pub job: u16,
    pub level: u16,
    pub base_level: u16,
    pub base_exp: u64,
    pub job_level: u16,
    pub job_exp: u64,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub zeny: u32,
    pub last_map: String,
    pub last_x: i32,
    pub last_y: i32,
    pub save_map: String,
    pub save_x: i32,
    pub save_y: i32,
}

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
            .map(|y| {
                (0..width)
                    .map(|x| Cell::new(x, y, CellType::Walkable))
                    .collect()
            })
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
        self.get_cell(x, y)
            .map(|c| c.is_walkable())
            .unwrap_or(false)
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell_type: CellType) {
        if let Some(cell) = self.get_cell_mut(x, y) {
            cell.cell_type = cell_type;
        }
    }

    /// 获取随机可行走位置
    pub fn random_walkable_pos(&self, rng: &dyn GameRng) -> Option<(u16, u16)> {
        for attempt in 0..100 {
            let seed = rng
                .rand_range(0, u32::MAX)
                .wrapping_add(attempt as u32 * 7919);
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

impl MapDatabase {
    /// 从目录加载所有 .gat 文件
    pub fn load_from_directory<P: AsRef<std::path::Path>>(
        &mut self,
        dir: P,
    ) -> Result<usize, super::gat::GatError> {
        use super::gat::GatParser;

        let mut loaded = 0;
        let dir = dir.as_ref();

        if !dir.exists() {
            tracing::warn!("地图目录不存在: {:?}", dir);
            return Ok(0);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("gat") {
                match GatParser::parse_file(&path) {
                    Ok(map_data) => {
                        let name = map_data.name.clone();
                        tracing::info!("加载地图: {} ({}x{})", name, map_data.width, map_data.height);
                        self.maps.insert(name, map_data);
                        loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!("加载地图失败 {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// 从嵌入的字节数据加载地图（用于测试或内嵌资源）
    pub fn load_from_bytes(&mut self, name: &str, data: &[u8]) -> Result<(), super::gat::GatError> {
        use super::gat::GatParser;
        let map_data = GatParser::parse_bytes(data, name)?;
        self.maps.insert(map_data.name.clone(), map_data);
        Ok(())
    }
}

impl Default for MapDatabase {
    fn default() -> Self {
        Self::new()
    }
}
