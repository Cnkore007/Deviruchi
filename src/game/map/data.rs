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

        // 尝试从 assets/maps/ 加载 .gat 文件
        let maps_dir = std::path::Path::new("assets/maps");
        if maps_dir.exists() {
            match db.load_from_directory(maps_dir) {
                Ok(count) => tracing::info!("从文件加载了 {} 张地图", count),
                Err(e) => tracing::warn!("加载地图文件失败: {}", e),
            }
        }

        // 如果没有加载到任何地图，使用硬编码默认值
        if db.maps.is_empty() {
            tracing::info!("使用硬编码默认地图");
            db.init_default_maps();
            // 将默认地图保存为 .gat 文件，下次启动可直接加载
            db.save_default_maps_to_files(maps_dir);
        }

        db
    }

    /// 将当前内存中的地图保存为 .gat 文件
    fn save_default_maps_to_files(&self, maps_dir: &std::path::Path) {
        use super::gat::GatParser;

        // 确保目录存在
        if let Err(e) = std::fs::create_dir_all(maps_dir) {
            tracing::warn!("创建地图目录失败: {}", e);
            return;
        }

        for (name, map_data) in &self.maps {
            let path = maps_dir.join(name);
            match GatParser::write_file(&path, map_data) {
                Ok(()) => tracing::info!("保存地图文件: {:?}", path),
                Err(e) => tracing::warn!("保存地图文件失败 {:?}: {}", path, e),
            }
        }
    }

    fn init_default_maps(&mut self) {
        // 新手村 (new_1-1) - 200x200
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
        // 中央水域
        for y in 80..120 {
            for x in 80..120 {
                new_1.set_cell(x, y, CellType::Water);
            }
        }
        // 北部森林区域（部分墙壁模拟树木）
        for y in 150..170 {
            for x in 30..60 {
                if (x + y) % 3 == 0 {
                    new_1.set_cell(x, y, CellType::Wall);
                }
            }
        }
        // NPC 区域
        for x in 95..105 {
            new_1.set_cell(x, 150, CellType::Npc);
        }
        // 传送到普隆德拉
        new_1.set_cell(100, 10, CellType::Warp);
        self.maps.insert("new_1-1.gat".to_string(), new_1);

        // 普隆德拉 (prontera) - 300x300
        let mut prontera = MapData::new("prontera.gat", 300, 300);
        for x in 0..300 {
            prontera.set_cell(x, 0, CellType::Wall);
            prontera.set_cell(x, 299, CellType::Wall);
        }
        for y in 0..300 {
            prontera.set_cell(0, y, CellType::Wall);
            prontera.set_cell(299, y, CellType::Wall);
        }
        // 城堡区域（中央大型建筑）
        for y in 130..170 {
            for x in 130..170 {
                if x == 130 || x == 169 || y == 130 || y == 169 {
                    prontera.set_cell(x, y, CellType::Wall);
                }
            }
        }
        // 城堡入口
        prontera.set_cell(150, 169, CellType::Walkable);
        prontera.set_cell(151, 169, CellType::Walkable);
        // 护城河
        for y in 125..175 {
            for x in 125..175 {
                if (x == 125 || x == 174 || y == 125 || y == 174)
                    && !((130..=169).contains(&x) && (130..=169).contains(&y))
                {
                    prontera.set_cell(x, y, CellType::Water);
                }
            }
        }
        // 市场区域（NPC 密集）
        for y in 50..70 {
            for x in 100..130 {
                if x % 5 == 0 {
                    prontera.set_cell(x, y, CellType::Npc);
                }
            }
        }
        // 传送到新手村
        prontera.set_cell(150, 280, CellType::Warp);
        // 各方向出口
        for x in 145..155 {
            prontera.set_cell(x, 1, CellType::Warp); // 北出口
            prontera.set_cell(x, 298, CellType::Warp); // 南出口
        }
        for y in 145..155 {
            prontera.set_cell(1, y, CellType::Warp); // 西出口
            prontera.set_cell(298, y, CellType::Warp); // 东出口
        }
        self.maps.insert("prontera.gat".to_string(), prontera);

        // 普隆德拉下水道 (prt_sewb1) - 100x100 地下城
        let mut sewer = MapData::new("prt_sewb1.gat", 100, 100);
        for x in 0..100 {
            sewer.set_cell(x, 0, CellType::Wall);
            sewer.set_cell(x, 99, CellType::Wall);
        }
        for y in 0..100 {
            sewer.set_cell(0, y, CellType::Wall);
            sewer.set_cell(99, y, CellType::Wall);
        }
        // 走廊和房间（墙壁形成迷宫）
        for y in 10..90 {
            sewer.set_cell(20, y, CellType::Wall);
            sewer.set_cell(40, y, CellType::Wall);
            sewer.set_cell(60, y, CellType::Wall);
            sewer.set_cell(80, y, CellType::Wall);
        }
        // 走廊开口
        for gap_start in [20, 45, 70] {
            for dy in 0..5 {
                sewer.set_cell(20, gap_start + dy, CellType::Walkable);
                sewer.set_cell(40, gap_start + dy, CellType::Walkable);
                sewer.set_cell(60, gap_start + dy, CellType::Walkable);
                sewer.set_cell(80, gap_start + dy, CellType::Walkable);
            }
        }
        // 水道
        for x in 5..95 {
            sewer.set_cell(x, 50, CellType::Water);
        }
        // 入口传送
        sewer.set_cell(50, 5, CellType::Warp);
        self.maps.insert("prt_sewb1.gat".to_string(), sewer);
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

    /// 插入地图数据
    pub fn insert(&mut self, map_data: MapData) {
        self.maps.insert(map_data.name.clone(), map_data);
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
                        tracing::info!(
                            "加载地图: {} ({}x{})",
                            name,
                            map_data.width,
                            map_data.height
                        );
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
