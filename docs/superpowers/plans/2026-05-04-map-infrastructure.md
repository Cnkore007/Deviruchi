# Map Infrastructure + Advanced Systems Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development

**Goal:** 实现地图基础设施（.gat 解析、碰撞检测）、数据库迁移框架、数据 YAML 加载器、Homunculus 和 Mercenary 系统补全

**Architecture:** 地图加载 -> 碰撞检测 -> 数据迁移 -> YAML 数据加载 -> 伴侣/雇佣兵系统

**Tech Stack:** Rust, tokio, rusqlite, serde_yaml, parking_lot, thiserror, chrono

---

## 现状分析

### 关键问题

1. **`MapState::is_walkable()` 是 STUB** — 始终返回 `true`，玩家可以穿墙
2. **无 .gat 文件解析** — 只有硬编码的 2 个地图 (new_1-1.gat, prontera.gat)
3. **`MobDatabase::get()` 硬编码 4 个怪物** — 无法加载 rAthena 数据
4. **无数据库迁移框架** — `CREATE TABLE IF NOT EXISTS` 无法处理 schema 变更
5. **Homunculus 缺失字段** — 无 sp/六属性/技能/进化/持久化
6. **HomunculusManager::summon() 有 BUG** — `insert(char_id, char_id)` 应该是 `insert(char_id, homun_id)`
7. **MercenaryData 未使用** — 模板数据已定义但未接入

### rAthena 数据文件位置

- `rathena/db/re/mob_db.yml` — Renewal 怪物数据库（完整的 AegisName/属性/掉落格式）
- `rathena/db/re/homunculus_db.yml` — 8 种生命体（含进化形态），每种有 Status + SkillTree
- `rathena/db/mercenary_db.yml` — 30+ 种雇佣兵（弓手/枪兵/剑士各 10 级）
- `rathena/db/re/exp_homun.yml` — 生命体经验表

---

## Phase 1: 地图基础设施

### Task 1.1: .gat 文件解析器

**目标:** 解析 rAthena 格式的 .gat 二进制文件，提取地图 cell 数据

**Files:**
- Create: `src/game/map/gat.rs`
- Modify: `src/game/map/mod.rs`
- Modify: `src/game/map/data.rs`

- [ ] **Step 1: 创建 gat.rs 解析器模块**

```rust
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
use std::io::{self, Read};
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
            data.push(0);  // padding
            data.push(0);  // padding
            data.push(0);  // padding
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
    fn test_niplet_cell_types() {
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
```

- [ ] **Step 2: 修改 mod.rs 导出 gat 模块**

```rust
// 在 src/game/map/mod.rs 中添加:
pub mod gat;

pub use gat::{GatError, GatParser};
```

- [ ] **Step 3: 扩展 MapDatabase 支持从 .gat 加载**

```rust
// 在 src/game/map/data.rs 的 MapDatabase impl 中添加:

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
```

- [ ] **Step 4: 运行测试**

```bash
cargo test --lib game::map::gat::tests -- --nocapture
```

---

### Task 1.2: MapState::is_walkable 实现

**目标:** 将 STUB 替换为真实的 cell 碰撞检测

**Files:**
- Modify: `src/game/map/map_state.rs`

- [ ] **Step 1: 修改 MapState 持有 MapDatabase 引用**

```rust
// src/game/map/map_state.rs

use super::data::MapDatabase;
use super::player::Player;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct MapState {
    players: RwLock<HashMap<Uuid, Player>>,
    players_by_map: RwLock<HashMap<String, Vec<Uuid>>>,
    /// 地图数据库引用（用于碰撞检测）
    map_database: Arc<MapDatabase>,
}

impl MapState {
    pub fn new(map_database: Arc<MapDatabase>) -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
            players_by_map: RwLock::new(HashMap::new()),
            map_database,
        }
    }

    // ... 其他方法保持不变 ...

    /// 检查位置是否可通行（真实实现）
    pub fn is_walkable(&self, map_name: &str, x: u16, y: u16) -> bool {
        self.map_database
            .get(map_name)
            .map(|map| map.is_walkable(x, y))
            .unwrap_or(false)
    }

    /// 获取地图数据引用
    pub fn map_database(&self) -> &Arc<MapDatabase> {
        &self.map_database
    }
}

impl Default for MapState {
    fn default() -> Self {
        Self::new(Arc::new(MapDatabase::new()))
    }
}
```

- [ ] **Step 2: 更新测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::constants;
    use crate::game::item::Equipment;
    use crate::game::map::player::PlayerState;
    use parking_lot::RwLock;

    fn create_test_map_database() -> Arc<MapDatabase> {
        Arc::new(MapDatabase::new())
    }

    fn create_test_player(x: u16, y: u16, map: &str) -> Player {
        // ... 保持不变 ...
    }

    #[test]
    fn test_is_walkable_with_real_data() {
        let db = create_test_map_database();
        let state = MapState::new(db.clone());

        // new_1-1.gat 的边界是墙
        assert!(!state.is_walkable("new_1-1.gat", 0, 0)); // 角落是墙
        assert!(state.is_walkable("new_1-1.gat", 50, 50)); // 中心可行走
        assert!(!state.is_walkable("new_1-1.gat", 90, 90)); // 中间水域

        // 不存在的地图返回 false
        assert!(!state.is_walkable("nonexistent.gat", 0, 0));
    }

    #[test]
    fn test_is_walkable_boundary() {
        let db = create_test_map_database();
        let state = MapState::new(db);

        // 超出地图范围
        assert!(!state.is_walkable("new_1-1.gat", 200, 200));
        assert!(!state.is_walkable("new_1-1.gat", 999, 999));
    }
}
```

- [ ] **Step 3: 更新所有 MapState::new() 调用点**

需要搜索所有 `MapState::new()` 的调用位置，传入 `Arc<MapDatabase>`。主要在:
- `src/game/map/map_server.rs`
- `src/game/game_loop.rs`
- 其他使用 MapState 的地方

- [ ] **Step 4: 运行测试**

```bash
cargo test --lib game::map::map_state::tests -- --nocapture
```

---

### Task 1.3: 地图数据扩充

**目标:** 从 .gat 文件加载更多地图，替换硬编码的 init_default_maps

**Files:**
- Modify: `src/game/map/data.rs`
- Create: `assets/maps/` 目录（存放 .gat 文件）

- [ ] **Step 1: 创建地图资源目录结构**

```
assets/
  maps/
    new_1-1.gat   (从 rAthena 复制或生成)
    prontera.gat
    geffen.gat
    payon.gat
    morocc.gat
    alberta.gat
```

- [ ] **Step 2: 修改 MapDatabase 初始化逻辑**

```rust
// src/game/map/data.rs

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
        }

        db
    }
}
```

- [ ] **Step 3: 运行测试确认地图加载正常**

```bash
cargo test --lib game::map::data::tests -- --nocapture
```

---

### Task 1.4: 碰撞检测测试

**目标:** 全面测试 is_walkable、边界条件、各种 cell 类型

**Files:**
- Create: `src/game/map/collision_tests.rs`（或在现有测试模块中扩展）

- [ ] **Step 1: 创建碰撞检测集成测试**

```rust
//! 地图碰撞检测集成测试

#[cfg(test)]
mod collision_tests {
    use crate::game::map::cell::{Cell, CellType};
    use crate::game::map::data::{MapData, MapDatabase};
    use crate::game::map::gat::GatParser;
    use crate::game::map::map_state::MapState;
    use std::sync::Arc;

    /// 创建带特定 cell 分布的测试地图
    fn create_test_map_with_pattern() -> MapData {
        // 10x10 地图:
        // 边界 = 墙
        // (3,3) = 水域
        // (5,5) = 悬崖
        // (7,7) = NPC
        // (8,8) = 传送点
        let mut map = MapData::new("test_collision.gat", 10, 10);

        // 边界墙
        for x in 0..10 {
            map.set_cell(x, 0, CellType::Wall);
            map.set_cell(x, 9, CellType::Wall);
        }
        for y in 0..10 {
            map.set_cell(0, y, CellType::Wall);
            map.set_cell(9, y, CellType::Wall);
        }

        map.set_cell(3, 3, CellType::Water);
        map.set_cell(5, 5, CellType::Cliff);
        map.set_cell(7, 7, CellType::Npc);
        map.set_cell(8, 8, CellType::Warp);

        map
    }

    #[test]
    fn test_walkable_cells() {
        let map = create_test_map_with_pattern();
        // 普通地面可行走
        assert!(map.is_walkable(1, 1));
        assert!(map.is_walkable(5, 1));
        // NPC 和 Warp 可行走
        assert!(map.is_walkable(7, 7));
        assert!(map.is_walkable(8, 8));
    }

    #[test]
    fn test_non_walkable_cells() {
        let map = create_test_map_with_pattern();
        // 墙壁不可行走
        assert!(!map.is_walkable(0, 0));
        assert!(!map.is_walkable(0, 5));
        assert!(!map.is_walkable(9, 5));
        // 水域不可行走
        assert!(!map.is_walkable(3, 3));
        // 悬崖不可行走
        assert!(!map.is_walkable(5, 5));
    }

    #[test]
    fn test_out_of_bounds() {
        let map = create_test_map_with_pattern();
        // 超出地图范围
        assert!(!map.is_walkable(10, 10));
        assert!(!map.is_walkable(100, 100));
        assert!(!map.is_walkable(u16::MAX, u16::MAX));
    }

    #[test]
    fn test_gat_parsed_walkability() {
        // 构造 .gat 数据并验证 walkability
        let mut data = Vec::new();
        data.extend_from_slice(b"GRAT");
        data.extend_from_slice(&5u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes()); // width=3
        data.extend_from_slice(&3u16.to_le_bytes()); // height=3

        // 9 个 cell: 交替 walkable/wall
        for i in 0..9u8 {
            let cell_type = if i % 2 == 0 { 0 } else { 1 }; // walkable / wall
            data.push(cell_type);
            data.push(0);
            data.push(0);
            data.push(0);
        }

        let map = GatParser::parse_bytes(&data, "test_gat").unwrap();

        // 偶数索引可行走，奇数索引是墙
        assert!(map.is_walkable(0, 0));   // (0,0) = index 0
        assert!(!map.is_walkable(1, 0));  // (1,0) = index 1
        assert!(map.is_walkable(2, 0));   // (2,0) = index 2
        assert!(!map.is_walkable(0, 1));  // (0,1) = index 3
        assert!(map.is_walkable(1, 1));   // (1,1) = index 4
    }

    #[test]
    fn test_map_state_walkability_integration() {
        let mut db = MapDatabase::new();
        let map = create_test_map_with_pattern();
        db.get_mut("test_collision.gat")
            .map(|m| *m = map.clone())
            .unwrap_or_else(|| {
                // 如果不存在则插入
            });

        let db_arc = Arc::new(db);
        let state = MapState::new(db_arc);

        // 真实碰撞检测
        assert!(!state.is_walkable("test_collision.gat", 0, 0)); // 墙
        assert!(state.is_walkable("test_collision.gat", 1, 1));   // 地面
        assert!(!state.is_walkable("test_collision.gat", 3, 3));  // 水
        assert!(state.is_walkable("test_collision.gat", 7, 7));   // NPC

        // 不存在的地图
        assert!(!state.is_walkable("no_such_map.gat", 0, 0));
    }

    #[test]
    fn test_snipable_not_walkable() {
        let mut map = MapData::new("snipable_test.gat", 5, 5);
        map.set_cell(2, 2, CellType::Snipable);
        map.set_cell(3, 3, CellType::Icetrap);
        map.set_cell(4, 4, CellType::Basilica);

        // Snipable 不可行走（可射击穿过但不能走）
        assert!(!map.is_walkable(2, 2));
        // Icetrap 不可行走
        assert!(!map.is_walkable(3, 3));
        // Basilica 不可行走
        assert!(!map.is_walkable(4, 4));
    }
}
```

- [ ] **Step 2: 运行所有碰撞测试**

```bash
cargo test collision_tests -- --nocapture
```

---

## Phase 2: 数据库迁移框架

### Task 2.1: 迁移框架实现

**目标:** 实现 schema 版本管理，支持 up/down 迁移

**Files:**
- Create: `src/storage/migration.rs`
- Modify: `src/storage/mod.rs`

- [ ] **Step 1: 创建迁移框架**

```rust
//! 数据库迁移框架
//!
//! 使用 schema_version 表追踪当前数据库版本
//! 每个迁移有 up (升级) 和 down (降级) 操作

use crate::storage::Database;
use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// 迁移定义
pub struct Migration {
    /// 版本号（单调递增）
    pub version: u32,
    /// 迁移描述
    pub description: &'static str,
    /// 升级 SQL
    pub up: &'static str,
    /// 降级 SQL（可选）
    pub down: Option<&static str>,
}

/// 迁移管理器
pub struct MigrationManager {
    migrations: BTreeMap<u32, Migration>,
}

impl MigrationManager {
    pub fn new() -> Self {
        Self {
            migrations: BTreeMap::new(),
        }
    }

    /// 注册迁移
    pub fn register(&mut self, migration: Migration) {
        self.migrations.insert(migration.version, migration);
    }

    /// 初始化 schema_version 表
    fn ensure_version_table(&self, db: &Database) -> Result<()> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            )",
        )?;
        Ok(())
    }

    /// 获取当前数据库版本
    pub fn current_version(&self, db: &Database) -> Result<u32> {
        self.ensure_version_table(db)?;

        let version = db.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get::<_, u32>(0),
        )?;

        Ok(version)
    }

    /// 执行所有待执行的升级迁移
    pub fn migrate_up(&self, db: &Database) -> Result<u32> {
        self.ensure_version_table(db)?;
        let current = self.current_version(db)?;
        let mut applied = 0;

        for (version, migration) in &self.migrations {
            if *version > current {
                tracing::info!(
                    "执行迁移 v{}: {}",
                    version,
                    migration.description
                );

                db.execute(migration.up)
                    .with_context(|| format!("迁移 v{} 失败", version))?;

                db.execute(
                    "INSERT INTO schema_version (version, description, applied_at)
                     VALUES (?1, ?2, ?3)",
                )
                .with_context(|| format!("记录迁移版本 v{} 失败", version))?;

                applied += 1;
            }
        }

        if applied > 0 {
            tracing::info!("应用了 {} 个迁移", applied);
        }

        Ok(applied)
    }

    /// 降级到指定版本
    pub fn migrate_down(&self, db: &Database, target_version: u32) -> Result<u32> {
        self.ensure_version_table(db)?;
        let current = self.current_version(db)?;
        let mut reverted = 0;

        // 按版本降序执行降级
        for (version, migration) in self.migrations.iter().rev() {
            if *version > target_version && *version <= current {
                if let Some(down_sql) = migration.down {
                    tracing::info!(
                        "回滚迁移 v{}: {}",
                        version,
                        migration.description
                    );

                    db.execute(down_sql)
                        .with_context(|| format!("回滚迁移 v{} 失败", version))?;

                    db.execute(
                        "DELETE FROM schema_version WHERE version = ?1",
                    )
                    .with_context(|| format!("删除迁移版本 v{} 记录失败", version))?;

                    reverted += 1;
                } else {
                    tracing::warn!("迁移 v{} 不支持降级", version);
                }
            }
        }

        Ok(reverted)
    }

    /// 检查是否有待执行的迁移
    pub fn has_pending(&self, db: &Database) -> Result<bool> {
        let current = self.current_version(db)?;
        Ok(self.migrations.keys().any(|v| *v > current))
    }

    /// 获取所有已注册的迁移版本
    pub fn registered_versions(&self) -> Vec<u32> {
        self.migrations.keys().copied().collect()
    }
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建默认迁移管理器（包含所有内置迁移）
pub fn create_default_migrations() -> MigrationManager {
    let mut manager = MigrationManager::new();

    // 迁移 v1: 添加 homunculus 表
    manager.register(Migration {
        version: 1,
        description: "创建 homunculus 表",
        up: "CREATE TABLE IF NOT EXISTS homunculus (
            homun_id INTEGER PRIMARY KEY,
            owner_id INTEGER NOT NULL,
            homunculus_type TEXT NOT NULL,
            name TEXT NOT NULL,
            level INTEGER DEFAULT 1,
            exp INTEGER DEFAULT 0,
            hunger INTEGER DEFAULT 100,
            intimacy INTEGER DEFAULT 100,
            hp INTEGER DEFAULT 500,
            max_hp INTEGER DEFAULT 500,
            sp INTEGER DEFAULT 100,
            max_sp INTEGER DEFAULT 100,
            str INTEGER DEFAULT 1,
            agi INTEGER DEFAULT 1,
            vit INTEGER DEFAULT 1,
            int INTEGER DEFAULT 1,
            dex INTEGER DEFAULT 1,
            luk INTEGER DEFAULT 1,
            evolved INTEGER DEFAULT 0,
            alive INTEGER DEFAULT 1,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (owner_id) REFERENCES characters(char_id) ON DELETE CASCADE
        )",
        down: Some("DROP TABLE IF EXISTS homunculus"),
    });

    // 迁移 v2: 添加 mercenary 表
    manager.register(Migration {
        version: 2,
        description: "创建 mercenary 表",
        up: "CREATE TABLE IF NOT EXISTS mercenaries (
            mercenary_id INTEGER PRIMARY KEY,
            owner_id INTEGER NOT NULL,
            mercenary_class INTEGER NOT NULL,
            name TEXT NOT NULL,
            level INTEGER DEFAULT 1,
            hp INTEGER DEFAULT 1000,
            max_hp INTEGER DEFAULT 1000,
            sp INTEGER DEFAULT 100,
            max_sp INTEGER DEFAULT 100,
            atk INTEGER DEFAULT 50,
            loyalty INTEGER DEFAULT 100,
            contract_end INTEGER,
            alive INTEGER DEFAULT 1,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (owner_id) REFERENCES characters(char_id) ON DELETE CASCADE
        )",
        down: Some("DROP TABLE IF EXISTS mercenaries"),
    });

    // 迁移 v3: 添加 pet 持久化表
    manager.register(Migration {
        version: 3,
        description: "创建 pets 表",
        up: "CREATE TABLE IF NOT EXISTS pets (
            pet_id INTEGER PRIMARY KEY,
            owner_id INTEGER NOT NULL,
            monster_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            renamed INTEGER DEFAULT 0,
            intimacy INTEGER DEFAULT 10000,
            hunger INTEGER DEFAULT 500,
            level INTEGER DEFAULT 1,
            egg_id INTEGER DEFAULT 0,
            equip_id INTEGER DEFAULT 0,
            born_at INTEGER NOT NULL,
            FOREIGN KEY (owner_id) REFERENCES characters(char_id) ON DELETE CASCADE
        )",
        down: Some("DROP TABLE IF EXISTS pets"),
    });

    manager
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    fn create_test_db() -> Database {
        Database::open(":memory:").expect("创建测试数据库失败")
    }

    #[test]
    fn test_schema_version_table_created() {
        let db = create_test_db();
        let manager = MigrationManager::new();

        manager.ensure_version_table(&db).unwrap();

        // 验证表存在
        let version = manager.current_version(&db).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_migrate_up_single() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试迁移",
            up: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
            down: Some("DROP TABLE test_table"),
        });

        let applied = manager.migrate_up(&db).unwrap();
        assert_eq!(applied, 1);

        let version = manager.current_version(&db).unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_migrate_up_idempotent() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试迁移",
            up: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
            down: Some("DROP TABLE test_table"),
        });

        // 第一次执行
        let applied1 = manager.migrate_up(&db).unwrap();
        assert_eq!(applied1, 1);

        // 第二次执行应该不应用任何迁移
        let applied2 = manager.migrate_up(&db).unwrap();
        assert_eq!(applied2, 0);
    }

    #[test]
    fn test_migrate_down() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "创建表",
            up: "CREATE TABLE test_table (id INTEGER PRIMARY KEY)",
            down: Some("DROP TABLE test_table"),
        });

        manager.register(Migration {
            version: 2,
            description: "添加列",
            up: "ALTER TABLE test_table ADD COLUMN name TEXT",
            down: Some("ALTER TABLE test_table DROP COLUMN name"),
        });

        // 升级到 v2
        manager.migrate_up(&db).unwrap();
        assert_eq!(manager.current_version(&db).unwrap(), 2);

        // 降级到 v1
        let reverted = manager.migrate_down(&db, 1).unwrap();
        assert_eq!(reverted, 1);
        assert_eq!(manager.current_version(&db).unwrap(), 1);
    }

    #[test]
    fn test_migrate_down_to_zero() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: Some("DROP TABLE t1"),
        });

        manager.migrate_up(&db).unwrap();
        let reverted = manager.migrate_down(&db, 0).unwrap();
        assert_eq!(reverted, 1);
        assert_eq!(manager.current_version(&db).unwrap(), 0);
    }

    #[test]
    fn test_has_pending() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "测试",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: None,
        });

        assert!(manager.has_pending(&db).unwrap());

        manager.migrate_up(&db).unwrap();

        assert!(!manager.has_pending(&db).unwrap());
    }

    #[test]
    fn test_multiple_migrations_order() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        // 故意乱序注册
        manager.register(Migration {
            version: 3,
            description: "第三个",
            up: "CREATE TABLE t3 (id INTEGER)",
            down: Some("DROP TABLE t3"),
        });
        manager.register(Migration {
            version: 1,
            description: "第一个",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: Some("DROP TABLE t1"),
        });
        manager.register(Migration {
            version: 2,
            description: "第二个",
            up: "CREATE TABLE t2 (id INTEGER)",
            down: Some("DROP TABLE t2"),
        });

        let applied = manager.migrate_up(&db).unwrap();
        assert_eq!(applied, 3);
        assert_eq!(manager.current_version(&db).unwrap(), 3);
    }

    #[test]
    fn test_no_down_migration_warning() {
        let db = create_test_db();
        let mut manager = MigrationManager::new();

        manager.register(Migration {
            version: 1,
            description: "不可降级",
            up: "CREATE TABLE t1 (id INTEGER)",
            down: None,
        });

        manager.migrate_up(&db).unwrap();

        // 尝试降级，应该返回 0（没有执行任何降级）
        let reverted = manager.migrate_down(&db, 0).unwrap();
        assert_eq!(reverted, 0);
        // 版本不变
        assert_eq!(manager.current_version(&db).unwrap(), 1);
    }
}
```

- [ ] **Step 2: 修改 storage/mod.rs 导出迁移模块**

```rust
// src/storage/mod.rs 中添加:
pub mod migration;

pub use migration::{Migration, MigrationManager};
```

- [ ] **Step 3: 在服务器启动时执行迁移**

```rust
// src/main.rs 或 src/lib.rs 的初始化流程中:

// 创建迁移管理器
let migrations = migration::create_default_migrations();

// 执行迁移
match migrations.migrate_up(&db) {
    Ok(applied) => {
        if applied > 0 {
            tracing::info!("数据库迁移完成，应用了 {} 个迁移", applied);
        }
    }
    Err(e) => {
        tracing::error!("数据库迁移失败: {}", e);
        return Err(e.into());
    }
}
```

- [ ] **Step 4: 运行迁移测试**

```bash
cargo test --lib storage::migration::tests -- --nocapture
```

---

## Phase 3: 数据扩充

### Task 3.1: Mob 数据 YAML 加载器

**目标:** 从 rAthena mob_db.yml 格式加载怪物模板，替换硬编码的 MobDatabase

**Files:**
- Create: `src/game/mob/yaml_loader.rs`
- Modify: `src/game/mob/data.rs`
- Modify: `src/game/mob/mod.rs`

- [ ] **Step 1: 创建 YAML 加载器**

```rust
//! Mob YAML 数据加载器
//!
//! 从 rAthena mob_db.yml 格式加载怪物模板数据

use super::data::{MobBehavior, MobDrop, MobSkill, MobTemplate};
use crate::game::battle::element::{Element, ElementLevel, MobSize};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// rAthena mob_db.yml 文件结构
#[derive(Deserialize, Debug)]
struct MobYamlFile {
    Header: MobYamlHeader,
    Body: Option<Vec<MobYamlEntry>>,
    Footer: Option<MobYamlFooter>,
}

#[derive(Deserialize, Debug)]
struct MobYamlHeader {
    #[serde(rename = "Type")]
    _type: String,
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct MobYamlFooter {
    Imports: Option<Vec<MobYamlImport>>,
}

#[derive(Deserialize, Debug)]
struct MobYamlImport {
    Path: String,
    Mode: Option<String>,
}

/// rAthena mob_db.yml 中的怪物条目
#[derive(Deserialize, Debug)]
struct MobYamlEntry {
    Id: u16,
    #[serde(rename = "AegisName")]
    _aegis_name: String,
    Name: String,
    #[serde(default)]
    Level: u16,
    #[serde(default = "default_hp")]
    Hp: u32,
    #[serde(default)]
    Sp: u32,
    #[serde(default)]
    BaseExp: u64,
    #[serde(default)]
    JobExp: u64,
    #[serde(default)]
    Attack: u16,
    #[serde(default)]
    Attack2: u16,
    #[serde(default)]
    Defense: u16,
    #[serde(default)]
    MagicDefense: u16,
    #[serde(default = "default_stat")]
    Str: u16,
    #[serde(default = "default_stat")]
    Agi: u16,
    #[serde(default = "default_stat")]
    Vit: u16,
    #[serde(default = "default_stat")]
    Int: u16,
    #[serde(default = "default_stat")]
    Dex: u16,
    #[serde(default = "default_stat")]
    Luk: u16,
    #[serde(default)]
    AttackRange: u16,
    #[serde(default)]
    SkillRange: u16,
    #[serde(default)]
    ChaseRange: u16,
    #[serde(default = "default_size")]
    Size: String,
    #[serde(default)]
    Race: String,
    #[serde(default)]
    Element: String,
    #[serde(default = "default_element_level")]
    ElementLevel: u8,
    #[serde(default = "default_walk_speed")]
    WalkSpeed: u16,
    #[serde(default)]
    AttackDelay: u32,
    #[serde(default)]
    AttackMotion: u32,
    #[serde(default)]
    DamageMotion: u32,
    #[serde(default)]
    Ai: String,
    #[serde(default)]
    Class: String,
    #[serde(default)]
    Modes: Option<HashMap<String, bool>>,
    #[serde(default)]
    Drops: Option<Vec<MobYamlDrop>>,
    #[serde(default)]
    MvpDrops: Option<Vec<MobYamlDrop>>,
}

#[derive(Deserialize, Debug)]
struct MobYamlDrop {
    Item: String,
    #[serde(default)]
    Rate: u32,
}

fn default_hp() -> u32 { 1 }
fn default_stat() -> u16 { 1 }
fn default_size() -> String { "Small".to_string() }
fn default_element_level() -> u8 { 1 }
fn default_walk_speed() -> u16 { 200 }

/// 解析元素类型
fn parse_element(s: &str) -> Element {
    match s.to_lowercase().as_str() {
        "neutral" => Element::Neutral,
        "water" => Element::Water,
        "earth" => Element::Earth,
        "fire" => Element::Fire,
        "wind" => Element::Wind,
        "poison" => Element::Poison,
        "holy" => Element::Holy,
        "dark" => Element::Dark,
        "ghost" => Element::Ghost,
        "undead" => Element::Undead,
        _ => Element::Neutral,
    }
}

/// 解析元素等级
fn parse_element_level(level: u8) -> ElementLevel {
    match level {
        1 => ElementLevel::Level1,
        2 => ElementLevel::Level2,
        3 => ElementLevel::Level3,
        4 => ElementLevel::Level4,
        _ => ElementLevel::Level1,
    }
}

/// 解析体型
fn parse_size(s: &str) -> MobSize {
    match s.to_lowercase().as_str() {
        "small" => MobSize::Small,
        "medium" => MobSize::Medium,
        "large" => MobSize::Large,
        _ => MobSize::Medium,
    }
}

/// 解析 AI 行为
fn parse_behavior(ai: &str, modes: &Option<HashMap<String, bool>>) -> MobBehavior {
    // rAthena AI 类型:
    // 01 = 攻击性 (Aggressive)
    // 02 = 攻击性 + 逃跑
    // 03 = 被动 (Passive)
    // 04 = 协助 (Assist)
    // 05 = 被动 + 协助
    // 06 = 默认（被动）
    match ai.as_str() {
        "01" => MobBehavior::Aggressive,
        "03" => MobBehavior::Passive,
        "04" => MobBehavior::Assist,
        "05" => MobBehavior::PassiveAssist,
        _ => {
            // 检查 Modes
            if let Some(modes) = modes {
                if modes.get("CanMove").copied().unwrap_or(true) == false {
                    return MobBehavior::Immobile;
                }
            }
            MobBehavior::Passive
        }
    }
}

/// 从 rAthena mob_db.yml 加载怪物模板
pub fn load_mob_db(path: &str) -> Result<HashMap<u16, MobTemplate>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: MobYamlFile = serde_yaml::from_str(&content)?;

    let mut mobs = HashMap::new();

    if let Some(body) = yaml.Body {
        for entry in body {
            let template = MobTemplate {
                name: entry.Name.clone(),
                level: if entry.Level == 0 { 1 } else { entry.Level },
                hp: entry.Hp,
                sp: entry.Sp,
                atk: entry.Attack,
                matk: entry.Attack2,
                defense: entry.Defense,
                magic_defense: entry.MagicDefense,
                hit: entry.Dex as i16, // rAthena 用 Dex 作为 hit
                flee: entry.Agi as i16, // rAthena 用 Agi 作为 flee
                crit: entry.Luk as i16 / 3, // 大约的 crit 值
                walk_speed: entry.WalkSpeed,
                atk_range: entry.AttackRange,
                sight_range: 12, // 默认视野
                chase_range: if entry.ChaseRange > 0 { entry.ChaseRange } else { 12 },
                aggro_rate: 0,
                spawn_delay: entry.AttackDelay,
                respawn_time: 60000, // 默认重生时间
                behavior: parse_behavior(&entry.Ai, &entry.Modes),
                skills: Vec::new(), // TODO: 从 Skills 字段加载
                drops: entry.Drops.as_ref().map(|drops| {
                    drops.iter().map(|d| {
                        // Rate 是万分比 (100 = 1%)
                        MobDrop::new(0, d.Rate * 100) // 暂时 item_id=0，需要名称映射
                    }).collect()
                }).unwrap_or_default(),
                base_exp: entry.BaseExp,
                job_exp: entry.JobExp,
                zeny: None,
                element: parse_element(&entry.Element),
                element_level: parse_element_level(entry.ElementLevel),
                size: parse_size(&entry.Size),
            };

            mobs.insert(entry.Id, template);
        }
    }

    Ok(mobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_element() {
        assert_eq!(parse_element("Neutral"), Element::Neutral);
        assert_eq!(parse_element("Fire"), Element::Fire);
        assert_eq!(parse_element("Water"), Element::Water);
        assert_eq!(parse_element("unknown"), Element::Neutral);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("Small"), MobSize::Small);
        assert_eq!(parse_size("Medium"), MobSize::Medium);
        assert_eq!(parse_size("Large"), MobSize::Large);
    }

    #[test]
    fn test_parse_behavior() {
        assert_eq!(parse_behavior("01", &None), MobBehavior::Aggressive);
        assert_eq!(parse_behavior("03", &None), MobBehavior::Passive);
        assert_eq!(parse_behavior("04", &None), MobBehavior::Assist);
    }

    #[test]
    fn test_load_mob_db_from_string() {
        let yaml_str = r#"
Header:
  Type: MOB_DB
  Version: 5
Body:
  - Id: 1001
    AegisName: SCORPION
    Name: Scorpion
    Level: 16
    Hp: 136
    BaseExp: 169
    JobExp: 115
    Attack: 7
    Attack2: 7
    Defense: 16
    MagicDefense: 5
    Str: 12
    Agi: 15
    Vit: 10
    Int: 5
    Dex: 19
    Luk: 5
    AttackRange: 1
    Size: Small
    Race: Insect
    Element: Fire
    ElementLevel: 1
    WalkSpeed: 200
    Ai: "01"
    Drops:
      - Item: Boody_Red
        Rate: 35
"#;

        let yaml: MobYamlFile = serde_yaml::from_str(yaml_str).unwrap();
        let body = yaml.Body.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].Id, 1001);
        assert_eq!(body[0].Name, "Scorpion");
        assert_eq!(body[0].Hp, 136);
    }
}
```

- [ ] **Step 2: 替换 MobDatabase 为可配置的结构**

```rust
// src/game/mob/data.rs 中修改 MobDatabase:

/// 怪物数据库（支持 YAML 加载 + 硬编码回退）
pub struct MobDatabase {
    templates: HashMap<u16, MobTemplate>,
}

impl MobDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            templates: HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = [
            "rathena/db/re/mob_db.yml",
            "db/mob_db.yml",
        ];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match crate::game::mob::yaml_loader::load_mob_db(path) {
                    Ok(mobs) => {
                        db.templates.extend(mobs);
                        tracing::info!("从 {} 加载了 {} 个怪物模板", path, db.templates.len());
                        return db;
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码数据
        tracing::info!("使用硬编码怪物数据");
        db.init_hardcoded();
        db
    }

    fn init_hardcoded(&mut self) {
        // 保留原有硬编码数据作为回退
        self.templates.insert(1001, MobTemplate {
            name: "Poring".to_string(),
            level: 1,
            hp: 50,
            // ... 原有数据 ...
            ..MobTemplate::default(1001)
        });
        // ... 其他硬编码怪物 ...
    }

    pub fn get(&self, mob_id: u16) -> &MobTemplate {
        self.templates
            .get(&mob_id)
            .unwrap_or_else(|| {
                // 返回默认模板的静态引用
                static DEFAULT: std::sync::OnceLock<MobTemplate> = std::sync::OnceLock::new();
                DEFAULT.get_or_init(|| MobTemplate::default(0))
            })
    }

    pub fn get_opt(&self, mob_id: u16) -> Option<&MobTemplate> {
        self.templates.get(&mob_id)
    }

    pub fn all(&self) -> impl Iterator<Item = (&u16, &MobTemplate)> {
        self.templates.iter()
    }

    pub fn count(&self) -> usize {
        self.templates.len()
    }
}

impl Default for MobDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: 更新 mod.rs 导出**

```rust
// src/game/mob/mod.rs 中添加:
pub mod yaml_loader;
```

- [ ] **Step 4: 运行测试**

```bash
cargo test --lib game::mob::yaml_loader::tests -- --nocapture
cargo test --lib game::mob::data::tests -- --nocapture
```

---

### Task 3.2: NPC 数据 YAML 加载器

**目标:** 从 YAML 加载 NPC 模板，替换硬编码的 NpcDatabase

**Files:**
- Create: `src/game/npc/yaml_loader.rs`
- Modify: `src/game/npc/data.rs`
- Modify: `src/game/npc/mod.rs`

- [ ] **Step 1: 定义 NPC YAML 格式**

```rust
//! NPC YAML 数据加载器

use super::data::{Npc, NpcType, ShopItem};
use serde::Deserialize;
use std::collections::HashMap;

/// NPC YAML 文件结构
#[derive(Deserialize, Debug)]
struct NpcYamlFile {
    npcs: Vec<NpcYamlEntry>,
}

/// NPC YAML 条目
#[derive(Deserialize, Debug)]
struct NpcYamlEntry {
    id: u32,
    name: String,
    display_name: Option<String>,
    #[serde(rename = "type")]
    npc_type: String,
    map: String,
    x: u16,
    y: u16,
    sprite_id: Option<u16>,
    level: Option<u16>,
    script: Option<String>,
    #[serde(default)]
    shop_items: Vec<ShopYamlItem>,
}

#[derive(Deserialize, Debug)]
struct ShopYamlItem {
    item_id: u16,
    buy_price: u32,
    sell_price: u32,
}

fn parse_npc_type(s: &str) -> NpcType {
    match s.to_lowercase().as_str() {
        "shop" => NpcType::Shop,
        "skill_trainer" | "skilltrainer" => NpcType::SkillTrainer,
        "quest" => NpcType::Quest,
        "warp" => NpcType::Warp,
        "cashshop" | "cash_shop" => NpcType::CashShop,
        _ => NpcType::Shop,
    }
}

/// NPC 数据库（支持 YAML 加载 + 硬编码回退）
pub struct NpcDatabase {
    npcs: HashMap<u32, Npc>,
}

impl NpcDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            npcs: HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = [
            "db/npc_db.yml",
            "rathena/db/npc_db.yml",
        ];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match Self::load_from_yaml(path) {
                    Ok(npcs) => {
                        db.npcs = npcs;
                        tracing::info!("从 {} 加载了 {} 个 NPC", path, db.npcs.len());
                        return db;
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码
        tracing::info!("使用硬编码 NPC 数据");
        db.init_hardcoded();
        db
    }

    fn load_from_yaml(path: &str) -> Result<HashMap<u32, Npc>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let yaml: NpcYamlFile = serde_yaml::from_str(&content)?;

        let mut npcs = HashMap::new();

        for entry in yaml.npcs {
            let mut npc = Npc {
                id: entry.id,
                name: entry.name.clone(),
                display_name: entry.display_name.unwrap_or(entry.name),
                type_: parse_npc_type(&entry.npc_type),
                pos_x: entry.x,
                pos_y: entry.y,
                map_name: entry.map,
                sprite_id: entry.sprite_id.unwrap_or(100),
                level: entry.level.unwrap_or(1),
                flags: 0,
                shop_items: parking_lot::RwLock::new(Vec::new()),
                skills: parking_lot::RwLock::new(Vec::new()),
                script: entry.script,
            };

            // 加载商店物品
            for item in entry.shop_items {
                npc.add_shop_item(item.item_id, item.buy_price, item.sell_price);
            }

            npcs.insert(npc.id, npc);
        }

        Ok(npcs)
    }

    fn init_hardcoded(&mut self) {
        // 保留原有硬编码 NPC
        // 注意: 原有代码用静态方法 get_npc(id)，需要重构为实例方法
    }

    pub fn get(&self, id: u32) -> Option<&Npc> {
        self.npcs.get(&id)
    }

    pub fn all(&self) -> impl Iterator<Item = (&u32, &Npc)> {
        self.npcs.iter()
    }

    pub fn get_npcs_on_map(&self, map_name: &str) -> Vec<&Npc> {
        self.npcs.values().filter(|n| n.map_name == map_name).collect()
    }
}

impl Default for NpcDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test --lib game::npc::yaml_loader::tests -- --nocapture
```

---

## Phase 4: Homunculus 系统

### Task 4.1: 数据结构补全

**目标:** 添加缺失字段（sp/六属性/技能/进化/食物/种族/元素）

**Files:**
- Modify: `src/game/homunculus/data.rs`

- [ ] **Step 1: 补全 Homunculus 数据结构**

```rust
//! Homunculus 数据模块
//!
//! 参考 rAthena homunculus_db.yml 格式

use serde::{Deserialize, Serialize};

/// 生命体类型（基础形态）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomunculusType {
    Lif,
    Amistr,
    Filir,
    Vanilmirth,
    // Renewal 生命体
    Eira,
    Bayeri,
    Sera,
    Dieter,
    Eleanor,
}

/// 生命体种族
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomunculusRace {
    Demihuman,
    Brute,
    Formless,
    Angel,
    Insect,
}

/// 生命体进化阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionStage {
    /// 基础形态
    Base,
    /// 进化形态 (H)
    Evolved,
    /// S 级进化形态 (H2)
    S级进化,
}

/// 生命体技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomunculusSkill {
    pub skill_id: u16,
    pub skill_name: String,
    pub level: u8,
    pub max_level: u8,
    /// 需要的基础等级
    pub required_level: u16,
    /// 需要的亲密度
    pub required_intimacy: u32,
    /// 是否需要进化
    pub require_evolution: bool,
    /// 前置技能
    pub prerequisites: Vec<(String, u8)>,
}

/// 生命体属性成长数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatGrowth {
    pub base: u32,
    pub growth_min: u32,
    pub growth_max: u32,
    pub evolution_min: u32,
    pub evolution_max: u32,
}

/// 生命体模板数据（从 YAML 加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomunculusTemplate {
    pub class_name: String,
    pub name: String,
    pub evolution_class: Option<String>,
    pub food_item: Option<String>,
    pub hungry_delay: u32,
    pub race: HomunculusRace,
    pub element: String,
    pub size: String,
    pub evolution_size: Option<String>,
    pub attack_delay: u32,
    /// 属性成长数据
    pub hp_growth: StatGrowth,
    pub sp_growth: StatGrowth,
    pub str_growth: StatGrowth,
    pub agi_growth: StatGrowth,
    pub vit_growth: StatGrowth,
    pub int_growth: StatGrowth,
    pub dex_growth: StatGrowth,
    pub luk_growth: StatGrowth,
    /// 技能树
    pub skill_tree: Vec<HomunculusSkill>,
}

/// 生命体实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Homunculus {
    pub homun_id: u32,
    pub owner_id: u32,
    pub homunculus_type: HomunculusType,
    pub name: String,
    pub level: u16,
    pub exp: u64,
    pub hunger: u32,
    pub intimacy: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub alive: bool,
    // 六属性
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    // 战斗属性
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub hit: i16,
    pub flee: i16,
    pub walk_speed: u16,
    pub attack_delay: u32,
    // 进化
    pub evolution_stage: EvolutionStage,
    pub evolved: bool,
    // 技能
    pub skills: Vec<HomunculusSkill>,
    pub skill_points: u16,
    // 种族/元素
    pub race: HomunculusRace,
    pub element: String,
}

impl Homunculus {
    /// 从模板创建新生命体
    pub fn from_template(
        homun_id: u32,
        owner_id: u32,
        template: &HomunculusTemplate,
        name: String,
    ) -> Self {
        Self {
            homun_id,
            owner_id,
            homunculus_type: Self::type_from_class(&template.class_name),
            name,
            level: 1,
            exp: 0,
            hunger: 100,
            intimacy: 100,
            hp: template.hp_growth.base,
            max_hp: template.hp_growth.base,
            sp: template.sp_growth.base,
            max_sp: template.sp_growth.base,
            alive: true,
            str: template.str_growth.base as u16,
            agi: template.agi_growth.base as u16,
            vit: template.vit_growth.base as u16,
            int: template.int_growth.base as u16,
            dex: template.dex_growth.base as u16,
            luk: template.luk_growth.base as u16,
            atk: 0,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 0,
            flee: 0,
            walk_speed: 200,
            attack_delay: template.attack_delay,
            evolution_stage: EvolutionStage::Base,
            evolved: false,
            skills: Vec::new(),
            skill_points: 0,
            race: template.race,
            element: template.element.clone(),
        }
    }

    fn type_from_class(class_name: &str) -> HomunculusType {
        match class_name {
            "Lif" | "Lif2" => HomunculusType::Lif,
            "Amistr" | "Amistr2" => HomunculusType::Amistr,
            "Filir" | "Filir2" => HomunculusType::Filir,
            "Vanilmirth" | "Vanilmirth2" => HomunculusType::Vanilmirth,
            "Eira" => HomunculusType::Eira,
            "Bayeri" => HomunculusType::Bayeri,
            "Sera" => HomunculusType::Sera,
            "Dieter" => HomunculusType::Dieter,
            "Eleanor" => HomunculusType::Eleanor,
            _ => HomunculusType::Lif,
        }
    }

    /// 喂食
    pub fn feed(&mut self, hunger_restore: u32) {
        self.hunger = (self.hunger + hunger_restore).min(100);
    }

    /// 增加亲密度
    pub fn increase_intimacy(&mut self, amount: u32) {
        self.intimacy = (self.intimacy + amount).min(100000);
    }

    /// 降低亲密度
    pub fn decrease_intimacy(&mut self, amount: u32) {
        self.intimacy = self.intimacy.saturating_sub(amount);
    }

    /// 检查是否饥饿
    pub fn is_hungry(&self) -> bool {
        self.hunger < 20
    }

    /// 检查是否死亡
    pub fn is_dead(&self) -> bool {
        !self.alive || self.hp == 0
    }

    /// 受到伤害
    pub fn take_damage(&mut self, damage: u32) {
        if damage >= self.hp {
            self.hp = 0;
            self.alive = false;
        } else {
            self.hp -= damage;
        }
    }

    /// 复活
    pub fn revive(&mut self, hp_percent: u32) {
        self.alive = true;
        self.hp = (self.max_hp * hp_percent / 100).max(1);
    }
}
```

- [ ] **Step 2: 创建 HomunculusDatabase**

```rust
// 在 src/game/homunculus/data.rs 中添加:

use std::collections::HashMap;

/// 生命体数据库（从 rAthena homunculus_db.yml 加载）
pub struct HomunculusDatabase {
    templates: HashMap<String, HomunculusTemplate>,
}

impl HomunculusDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            templates: HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = [
            "rathena/db/re/homunculus_db.yml",
            "db/homunculus_db.yml",
        ];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match Self::load_from_yaml(path) {
                    Ok(templates) => {
                        db.templates = templates;
                        tracing::info!("从 {} 加载了 {} 个生命体模板", path, db.templates.len());
                        return db;
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码
        db.init_hardcoded();
        db
    }

    fn load_from_yaml(path: &str) -> Result<HashMap<String, HomunculusTemplate>, Box<dyn std::error::Error>> {
        // 解析 rAthena homunculus_db.yml 格式
        // TODO: 实现完整解析
        Ok(HashMap::new())
    }

    fn init_hardcoded(&mut self) {
        // 硬编码基础生命体数据
        self.templates.insert("Lif".to_string(), HomunculusTemplate {
            class_name: "Lif".to_string(),
            name: "Lif".to_string(),
            evolution_class: Some("Lif_H".to_string()),
            food_item: None,
            hungry_delay: 60000,
            race: HomunculusRace::Demihuman,
            element: "Neutral".to_string(),
            size: "Small".to_string(),
            evolution_size: Some("Medium".to_string()),
            attack_delay: 700,
            hp_growth: StatGrowth { base: 150, growth_min: 60, growth_max: 100, evolution_min: 800, evolution_max: 2400 },
            sp_growth: StatGrowth { base: 40, growth_min: 4, growth_max: 9, evolution_min: 220, evolution_max: 480 },
            str_growth: StatGrowth { base: 17, growth_min: 5, growth_max: 19, evolution_min: 10, evolution_max: 30 },
            agi_growth: StatGrowth { base: 20, growth_min: 5, growth_max: 19, evolution_min: 10, evolution_max: 30 },
            vit_growth: StatGrowth { base: 15, growth_min: 5, growth_max: 19, evolution_min: 20, evolution_max: 40 },
            int_growth: StatGrowth { base: 35, growth_min: 4, growth_max: 20, evolution_min: 30, evolution_max: 50 },
            dex_growth: StatGrowth { base: 24, growth_min: 6, growth_max: 20, evolution_min: 20, evolution_max: 50 },
            luk_growth: StatGrowth { base: 12, growth_min: 6, growth_max: 20, evolution_min: 10, evolution_max: 30 },
            skill_tree: vec![
                HomunculusSkill {
                    skill_id: 1,
                    skill_name: "HLIF_HEAL".to_string(),
                    level: 0,
                    max_level: 5,
                    required_level: 0,
                    required_intimacy: 0,
                    require_evolution: false,
                    prerequisites: Vec::new(),
                },
            ],
        });
        // ... 其他生命体 ...
    }

    pub fn get(&self, class_name: &str) -> Option<&HomunculusTemplate> {
        self.templates.get(class_name)
    }

    pub fn get_by_type(&self, htype: HomunculusType) -> Option<&HomunculusTemplate> {
        let class_name = match htype {
            HomunculusType::Lif => "Lif",
            HomunculusType::Amistr => "Amistr",
            HomunculusType::Filir => "Filir",
            HomunculusType::Vanilmirth => "Vanilmirth",
            HomunculusType::Eira => "Eira",
            HomunculusType::Bayeri => "Bayeri",
            HomunculusType::Sera => "Sera",
            HomunculusType::Dieter => "Dieter",
            HomunculusType::Eleanor => "Eleanor",
        };
        self.templates.get(class_name)
    }
}

impl Default for HomunculusDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: 更新 mod.rs**

```rust
// src/game/homunculus/mod.rs:
pub mod data;
pub mod manager;
pub mod yaml_loader;

pub use data::{Homunculus, HomunculusDatabase, HomunculusTemplate, HomunculusType};
pub use manager::HomunculusManager;
```

---

### Task 4.2: HomunculusManager 重构

**目标:** 修复 BUG，添加经验/升级/进化/喂食/持久化

**Files:**
- Modify: `src/game/homunculus/manager.rs`

- [ ] **Step 1: 重构 HomunculusManager**

```rust
//! 生命体管理器
//!
//! 参考 PetManager 的实现模式

use super::data::{Homunculus, HomunculusDatabase, HomunculusTemplate, HomunculusType};
use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;

/// 生命体错误类型
#[derive(Debug, Error)]
pub enum HomunculusError {
    #[error("生命体未找到: {0}")]
    NotFound(u32),

    #[error("玩家未找到: {0}")]
    PlayerNotFound(u32),

    #[error("玩家已有召唤的生命体")]
    AlreadySummoned,

    #[error("生命体已死亡")]
    Dead,

    #[error("生命体未召唤")]
    NotSummoned,

    #[error("不是你的生命体")]
    NotYours,

    #[error("食物类型不匹配")]
    WrongFood,

    #[error("进化条件不满足")]
    EvolutionFailed,

    #[error("技能前置条件不满足")]
    SkillPrereqNotMet,

    #[error("数据库错误: {0}")]
    Database(String),
}

/// 生命体管理器
pub struct HomunculusManager {
    /// 所有生命体实例 (homun_id -> Homunculus)
    homunculi: RwLock<HashMap<u32, Homunculus>>,
    /// 当前召唤的生命体 (char_id -> homun_id)
    summoned: RwLock<HashMap<u32, u32>>,
    /// 生命体模板数据库
    database: HomunculusDatabase,
    /// 下一个可用 ID
    next_id: RwLock<u32>,
}

impl HomunculusManager {
    pub fn new() -> Self {
        Self {
            homunculi: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: HomunculusDatabase::new(),
            next_id: RwLock::new(1),
        }
    }

    /// 创建新生命体
    pub fn create(
        &self,
        owner_id: u32,
        htype: HomunculusType,
        name: &str,
    ) -> Result<Homunculus, HomunculusError> {
        let template = self.database.get_by_type(htype)
            .ok_or(HomunculusError::NotFound(0))?;

        let mut next_id = self.next_id.write();
        let homun_id = *next_id;
        *next_id += 1;

        let homun = Homunculus::from_template(homun_id, owner_id, template, name.to_string());

        self.homunculi.write().insert(homun_id, homun.clone());
        Ok(homun)
    }

    /// 召唤生命体（修复原有 BUG）
    pub fn summon(&self, char_id: u32, homun_id: u32) -> Result<(), HomunculusError> {
        // 检查是否已有召唤
        if self.summoned.read().contains_key(&char_id) {
            return Err(HomunculusError::AlreadySummoned);
        }

        let homunculi = self.homunculi.read();
        let homun = homunculi.get(&homun_id)
            .ok_or(HomunculusError::NotFound(homun_id))?;

        // 验证归属
        if homun.owner_id != char_id {
            return Err(HomunculusError::NotYours);
        }

        // 检查是否存活
        if homun.is_dead() {
            return Err(HomunculusError::Dead);
        }

        drop(homunculi);

        // BUG FIX: 原来是 insert(char_id, char_id)，现在正确插入 homun_id
        self.summoned.write().insert(char_id, homun_id);
        Ok(())
    }

    /// 解散生命体
    pub fn dismiss(&self, char_id: u32) {
        self.summoned.write().remove(&char_id);
    }

    /// 获取玩家召唤的生命体
    pub fn get_summoned(&self, char_id: u32) -> Option<Homunculus> {
        let summoned = self.summoned.read();
        let homun_id = summoned.get(&char_id)?;
        self.homunculi.read().get(homun_id).cloned()
    }

    /// 喂食生命体
    pub fn feed(&self, char_id: u32, _item_id: u16) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned.get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi.get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        // TODO: 检查食物类型是否匹配
        // let template = self.database.get_by_type(homun.homunculus_type);
        // if template.food_item != Some(item_name) { return Err(WrongFood); }

        homun.feed(20);
        homun.increase_intimacy(10);
        Ok(())
    }

    /// 增加经验
    pub fn add_exp(&self, char_id: u32, exp: u64) -> Result<bool, HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned.get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi.get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        homun.exp += exp;

        // 检查升级
        let exp_needed = Self::exp_for_level(homun.level + 1);
        if homun.exp >= exp_needed {
            homun.level += 1;
            homun.exp -= exp_needed;

            // 属性成长（简化版）
            homun.max_hp += 20;
            homun.hp = homun.max_hp;
            homun.max_sp += 5;
            homun.sp = homun.max_sp;
            homun.str += 1;
            homun.agi += 1;
            homun.vit += 1;
            homun.int += 1;
            homun.dex += 1;
            homun.luk += 1;

            // 每 5 级获得技能点
            if homun.level % 5 == 0 {
                homun.skill_points += 1;
            }

            return Ok(true); // 升级了
        }

        Ok(false) // 未升级
    }

    /// 经验表查询
    fn exp_for_level(level: u16) -> u64 {
        // 简化的经验公式
        match level {
            1..=10 => (level as u64) * 100,
            11..=50 => (level as u64) * 500,
            51..=99 => (level as u64) * 2000,
            _ => 999999,
        }
    }

    /// 进化生命体
    pub fn evolve(&self, char_id: u32) -> Result<(), HomunculusError> {
        let summoned = self.summoned.read();
        let homun_id = summoned.get(&char_id)
            .ok_or(HomunculusError::NotSummoned)?;

        let mut homunculi = self.homunculi.write();
        let homun = homunculi.get_mut(homun_id)
            .ok_or(HomunculusError::NotFound(*homun_id))?;

        // 进化条件: 等级 >= 99, 亲密度 >= 910
        if homun.level < 99 || homun.intimacy < 910 {
            return Err(HomunculusError::EvolutionFailed);
        }

        if homun.evolved {
            return Err(HomunculusError::EvolutionFailed);
        }

        homun.evolved = true;
        homun.evolution_stage = super::data::EvolutionStage::Evolved;

        // 进化属性加成
        homun.max_hp += 500;
        homun.hp = homun.max_hp;
        homun.max_sp += 100;
        homun.sp = homun.max_sp;
        homun.str += 10;
        homun.agi += 10;
        homun.vit += 10;
        homun.int += 10;
        homun.dex += 10;
        homun.luk += 10;

        Ok(())
    }

    /// 保存到数据库
    pub fn save_to_db(&self, db: &crate::storage::Database, char_id: u32) -> Result<(), HomunculusError> {
        if let Some(homun) = self.get_summoned(char_id) {
            db.execute(
                "INSERT OR REPLACE INTO homunculus (
                    homun_id, owner_id, homunculus_type, name, level, exp,
                    hunger, intimacy, hp, max_hp, sp, max_sp,
                    str, agi, vit, int, dex, luk,
                    evolved, alive, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            ).map_err(|e| HomunculusError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// 从数据库加载
    pub fn load_from_db(&self, db: &crate::storage::Database, char_id: u32) -> Result<(), HomunculusError> {
        // TODO: 实现从 SQLite 加载
        Ok(())
    }
}

impl Default for HomunculusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_homunculus() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "MyLif").unwrap();

        assert_eq!(homun.owner_id, 100);
        assert_eq!(homun.name, "MyLif");
        assert_eq!(homun.level, 1);
        assert!(homun.alive);
    }

    #[test]
    fn test_summon_dismiss() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();

        // 召唤
        assert!(manager.summon(100, homun.homun_id).is_ok());
        assert!(manager.get_summoned(100).is_some());

        // 解散
        manager.dismiss(100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = HomunculusManager::new();
        let homun1 = manager.create(100, HomunculusType::Lif, "Lif1").unwrap();
        let homun2 = manager.create(100, HomunculusType::Amistr, "Ami1").unwrap();

        manager.summon(100, homun1.homun_id).unwrap();
        assert!(matches!(
            manager.summon(100, homun2.homun_id),
            Err(HomunculusError::AlreadySummoned)
        ));
    }

    #[test]
    fn test_summon_wrong_owner() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();

        assert!(matches!(
            manager.summon(999, homun.homun_id),
            Err(HomunculusError::NotYours)
        ));
    }

    #[test]
    fn test_feed() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 降低饥饿度
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.hunger = 50;
        }

        manager.feed(100, 0).unwrap();

        let h = manager.get_summoned(100).unwrap();
        assert_eq!(h.hunger, 70); // 50 + 20
    }

    #[test]
    fn test_add_exp_and_level_up() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 添加足够升级的经验
        let leveled = manager.add_exp(100, 200).unwrap();
        assert!(leveled);

        let h = manager.get_summoned(100).unwrap();
        assert_eq!(h.level, 2);
    }

    #[test]
    fn test_evolve() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        // 设置进化条件
        {
            let mut homunculi = manager.homunculi.write();
            let h = homunculi.get_mut(&homun.homun_id).unwrap();
            h.level = 99;
            h.intimacy = 910;
        }

        assert!(manager.evolve(100).is_ok());

        let h = manager.get_summoned(100).unwrap();
        assert!(h.evolved);
    }

    #[test]
    fn test_evolve_insufficient_level() {
        let manager = HomunculusManager::new();
        let homun = manager.create(100, HomunculusType::Lif, "Test").unwrap();
        manager.summon(100, homun.homun_id).unwrap();

        assert!(matches!(
            manager.evolve(100),
            Err(HomunculusError::EvolutionFailed)
        ));
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cargo test --lib game::homunculus::manager::tests -- --nocapture
```

---

## Phase 5: Mercenary 系统

### Task 5.1: 数据结构补全 + MercenaryDatabase

**目标:** 补全 Mercenary 数据结构，接入 MercenaryData 模板

**Files:**
- Modify: `src/game/mercenary/data.rs`
- Modify: `src/game/mercenary/manager.rs`
- Modify: `src/game/mercenary/mod.rs`

- [ ] **Step 1: 补全 Mercenary 数据结构**

```rust
//! 雇佣兵数据模块
//!
//! 参考 rAthena mercenary_db.yml 格式

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 雇佣兵类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MercenaryClass {
    /// 弓手系
    Archer,
    /// 枪兵系
    Lancer,
    /// 剑士系
    Swordman,
}

/// 雇佣兵技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MercenarySkill {
    pub skill_name: String,
    pub max_level: u8,
    pub current_level: u8,
}

/// 雇佣兵实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mercenary {
    pub mercenary_id: u32,
    pub owner_id: u32,
    pub mercenary_class: u16,
    pub name: String,
    pub level: u16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub atk: u32,
    pub defense: u32,
    pub magic_defense: u32,
    // 六属性
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    // 战斗属性
    pub hit: i16,
    pub flee: i16,
    pub walk_speed: u16,
    pub attack_range: u16,
    // 忠诚度
    pub loyalty: u32,
    // 合同
    pub contract_end: Option<DateTime<Utc>>,
    pub contract_cost: u32,
    // 状态
    pub alive: bool,
    // 技能
    pub skills: Vec<MercenarySkill>,
}

impl Mercenary {
    /// 检查合同是否到期
    pub fn is_contract_expired(&self) -> bool {
        if let Some(end) = self.contract_end {
            Utc::now() >= end
        } else {
            false
        }
    }

    /// 剩余合同时间（秒）
    pub fn contract_remaining_secs(&self) -> i64 {
        if let Some(end) = self.contract_end {
            (end - Utc::now()).num_seconds().max(0)
        } else {
            0
        }
    }

    /// 受到伤害
    pub fn take_damage(&mut self, damage: u32) {
        if damage >= self.hp {
            self.hp = 0;
            self.alive = false;
        } else {
            self.hp -= damage;
        }
    }

    /// 增加忠诚度
    pub fn increase_loyalty(&mut self, amount: u32) {
        self.loyalty = (self.loyalty + amount).min(1000);
    }

    /// 降低忠诚度
    pub fn decrease_loyalty(&mut self, amount: u32) {
        self.loyalty = self.loyalty.saturating_sub(amount);
    }
}

/// 雇佣兵模板数据（运行时使用）
#[derive(Debug, Clone)]
pub struct MercenaryData {
    pub class_id: u16,
    pub name: String,
    pub class_type: MercenaryClass,
    pub level: u16,
    pub hp: u32,
    pub sp: u32,
    pub atk: u32,
    pub atk2: u32,
    pub defense: u32,
    pub magic_defense: u32,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub attack_range: u16,
    pub walk_speed: u16,
    pub contract_cost: u32,
    pub skills: Vec<(String, u8)>,
}

/// 雇佣兵数据库
pub struct MercenaryDatabase {
    templates: HashMap<u16, MercenaryData>,
}

impl MercenaryDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            templates: HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = [
            "rathena/db/mercenary_db.yml",
            "db/mercenary_db.yml",
        ];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match Self::load_from_yaml(path) {
                    Ok(templates) => {
                        db.templates = templates;
                        tracing::info!("从 {} 加载了 {} 个雇佣兵模板", path, db.templates.len());
                        return db;
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码
        db.init_hardcoded();
        db
    }

    fn load_from_yaml(path: &str) -> Result<HashMap<u16, MercenaryData>, Box<dyn std::error::Error>> {
        // TODO: 实现完整解析
        Ok(HashMap::new())
    }

    fn init_hardcoded(&mut self) {
        // 基础弓手
        self.templates.insert(6017, MercenaryData {
            class_id: 6017,
            name: "Mina".to_string(),
            class_type: MercenaryClass::Archer,
            level: 20,
            hp: 256,
            sp: 200,
            atk: 170,
            atk2: 85,
            defense: 7,
            magic_defense: 5,
            str: 1,
            agi: 16,
            vit: 5,
            int: 1,
            dex: 28,
            luk: 8,
            attack_range: 10,
            walk_speed: 150,
            contract_cost: 5000,
            skills: vec![("MA_DOUBLE".to_string(), 2), ("MER_AUTOBERSERK".to_string(), 1)],
        });
        // ... 其他雇佣兵 ...
    }

    pub fn get(&self, class_id: u16) -> Option<&MercenaryData> {
        self.templates.get(&class_id)
    }

    pub fn get_all(&self) -> &HashMap<u16, MercenaryData> {
        &self.templates
    }
}

impl Default for MercenaryDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 重构 MercenaryManager**

```rust
//! 雇佣兵管理器

use super::data::{Mercenary, MercenaryDatabase, MercenaryData};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use thiserror::Error;

/// 雇佣兵错误类型
#[derive(Debug, Error)]
pub enum MercenaryError {
    #[error("雇佣兵未找到: {0}")]
    NotFound(u32),

    #[error("玩家已有召唤的雇佣兵")]
    AlreadySummoned,

    #[error("雇佣兵未召唤")]
    NotSummoned,

    #[error("不是你的雇佣兵")]
    NotYours,

    #[error("合同已到期")]
    ContractExpired,

    #[error("雇佣兵已死亡")]
    Dead,

    #[error("数据库错误: {0}")]
    Database(String),
}

/// 雇佣兵管理器
pub struct MercenaryManager {
    /// 所有雇佣兵实例
    mercenaries: RwLock<HashMap<u32, Mercenary>>,
    /// 当前召唤的雇佣兵 (char_id -> mercenary_id)
    summoned: RwLock<HashMap<u32, u32>>,
    /// 雇佣兵模板数据库
    database: MercenaryDatabase,
    /// 下一个可用 ID
    next_id: RwLock<u32>,
}

impl MercenaryManager {
    pub fn new() -> Self {
        Self {
            mercenaries: RwLock::new(HashMap::new()),
            summoned: RwLock::new(HashMap::new()),
            database: MercenaryDatabase::new(),
            next_id: RwLock::new(1),
        }
    }

    /// 创建雇佣兵（使用模板数据）
    pub fn create(
        &self,
        owner_id: u32,
        class_id: u16,
    ) -> Result<Mercenary, MercenaryError> {
        let template = self.database.get(class_id)
            .ok_or(MercenaryError::NotFound(class_id))?;

        let mut next_id = self.next_id.write();
        let mercenary_id = *next_id;
        *next_id += 1;

        let mercenary = Mercenary {
            mercenary_id,
            owner_id,
            mercenary_class: class_id,
            name: template.name.clone(),
            level: template.level,
            hp: template.hp,
            max_hp: template.hp,
            sp: template.sp,
            max_sp: template.sp,
            atk: template.atk,
            defense: template.defense,
            magic_defense: template.magic_defense,
            str: template.str,
            agi: template.agi,
            vit: template.vit,
            int: template.int,
            dex: template.dex,
            luk: template.luk,
            hit: template.dex as i16,
            flee: template.agi as i16,
            walk_speed: template.walk_speed,
            attack_range: template.attack_range,
            loyalty: 100,
            contract_end: Some(Utc::now() + Duration::hours(48)), // 48 小时合同
            contract_cost: template.contract_cost,
            alive: true,
            skills: template.skills.iter().map(|(name, max)| {
                super::data::MercenarySkill {
                    skill_name: name.clone(),
                    max_level: *max,
                    current_level: 0,
                }
            }).collect(),
        };

        self.mercenaries.write().insert(mercenary_id, mercenary.clone());
        Ok(mercenary)
    }

    /// 召唤雇佣兵
    pub fn summon(&self, char_id: u32, mercenary_id: u32) -> Result<(), MercenaryError> {
        if self.summoned.read().contains_key(&char_id) {
            return Err(MercenaryError::AlreadySummoned);
        }

        let mercenaries = self.mercenaries.read();
        let mercenary = mercenaries.get(&mercenary_id)
            .ok_or(MercenaryError::NotFound(mercenary_id))?;

        // 验证归属
        if mercenary.owner_id != char_id {
            return Err(MercenaryError::NotYours);
        }

        // 检查合同
        if mercenary.is_contract_expired() {
            return Err(MercenaryError::ContractExpired);
        }

        // 检查存活
        if !mercenary.alive {
            return Err(MercenaryError::Dead);
        }

        drop(mercenaries);
        self.summoned.write().insert(char_id, mercenary_id);
        Ok(())
    }

    /// 解散雇佣兵
    pub fn dismiss(&self, char_id: u32) -> Option<u32> {
        self.summoned.write().remove(&char_id)
    }

    /// 获取玩家召唤的雇佣兵
    pub fn get_summoned(&self, char_id: u32) -> Option<Mercenary> {
        let summoned = self.summoned.read();
        let mercenary_id = summoned.get(&char_id)?;
        self.mercenaries.read().get(mercenary_id).cloned()
    }

    /// 更新合同（检查到期，自动解散）
    pub fn update_contracts(&self) -> Vec<u32> {
        let mut dismissed = Vec::new();
        let summoned = self.summoned.read();
        let mercenaries = self.mercenaries.read();

        for (char_id, mercenary_id) in summoned.iter() {
            if let Some(mercenary) = mercenaries.get(mercenary_id) {
                if mercenary.is_contract_expired() {
                    dismissed.push(*char_id);
                }
            }
        }

        drop(summoned);
        drop(mercenaries);

        // 解散到期的雇佣兵
        for char_id in &dismissed {
            self.summoned.write().remove(char_id);
        }

        dismissed
    }

    /// 增加忠诚度
    pub fn increase_loyalty(&self, mercenary_id: u32, amount: u32) -> Result<(), MercenaryError> {
        let mut mercenaries = self.mercenaries.write();
        let mercenary = mercenaries.get_mut(&mercenary_id)
            .ok_or(MercenaryError::NotFound(mercenary_id))?;
        mercenary.increase_loyalty(amount);
        Ok(())
    }

    /// 获取雇佣兵模板数据库
    pub fn database(&self) -> &MercenaryDatabase {
        &self.database
    }
}

impl Default for MercenaryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mercenary() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert_eq!(merc.owner_id, 100);
        assert_eq!(merc.name, "Mina");
        assert_eq!(merc.level, 20);
        assert!(merc.alive);
        assert!(merc.contract_end.is_some());
    }

    #[test]
    fn test_summon_dismiss() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert!(manager.summon(100, merc.mercenary_id).is_ok());
        assert!(manager.get_summoned(100).is_some());

        manager.dismiss(100);
        assert!(manager.get_summoned(100).is_none());
    }

    #[test]
    fn test_cannot_summon_twice() {
        let manager = MercenaryManager::new();
        let merc1 = manager.create(100, 6017).unwrap();
        let merc2 = manager.create(100, 6018).unwrap();

        manager.summon(100, merc1.mercenary_id).unwrap();
        assert!(matches!(
            manager.summon(100, merc2.mercenary_id),
            Err(MercenaryError::AlreadySummoned)
        ));
    }

    #[test]
    fn test_wrong_owner() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        assert!(matches!(
            manager.summon(999, merc.mercenary_id),
            Err(MercenaryError::NotYours)
        ));
    }

    #[test]
    fn test_contract_expired() {
        let manager = MercenaryManager::new();
        let mut merc = manager.create(100, 6017).unwrap();

        // 手动设置合同已过期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::hours(1));
        }

        assert!(matches!(
            manager.summon(100, merc.mercenary_id),
            Err(MercenaryError::ContractExpired)
        ));
    }

    #[test]
    fn test_loyalty() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();

        manager.increase_loyalty(merc.mercenary_id, 50).unwrap();

        let m = manager.mercenaries.read().get(&merc.mercenary_id).unwrap().clone();
        assert_eq!(m.loyalty, 150); // 100 + 50
    }

    #[test]
    fn test_update_contracts() {
        let manager = MercenaryManager::new();
        let merc = manager.create(100, 6017).unwrap();
        manager.summon(100, merc.mercenary_id).unwrap();

        // 设置合同即将到期
        {
            let mut mercenaries = manager.mercenaries.write();
            let m = mercenaries.get_mut(&merc.mercenary_id).unwrap();
            m.contract_end = Some(Utc::now() - Duration::seconds(1));
        }

        let dismissed = manager.update_contracts();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0], 100);
        assert!(manager.get_summoned(100).is_none());
    }
}
```

- [ ] **Step 3: 更新 mod.rs**

```rust
// src/game/mercenary/mod.rs:
pub mod data;
pub mod manager;

pub use data::{Mercenary, MercenaryClass, MercenaryData, MercenaryDatabase, MercenarySkill};
pub use manager::{MercenaryError, MercenaryManager};
```

- [ ] **Step 4: 运行测试**

```bash
cargo test --lib game::mercenary::manager::tests -- --nocapture
cargo test --lib game::mercenary::data::tests -- --nocapture
```

---

## 任务依赖图

```
Phase 1: 地图基础设施
  Task 1.1 (.gat 解析器) ──> Task 1.2 (is_walkable) ──> Task 1.3 (地图扩充) ──> Task 1.4 (碰撞测试)

Phase 2: 数据库迁移
  Task 2.1 (迁移框架) ──> 可与 Phase 1 并行

Phase 3: 数据扩充
  Task 3.1 (Mob YAML) ──> Task 3.2 (NPC YAML)
  (可与 Phase 1/2 并行)

Phase 4: Homunculus 系统
  Task 4.1 (数据结构) ──> Task 4.2 (Manager 重构)
  (依赖 Phase 2 的迁移框架创建 homunculus 表)

Phase 5: Mercenary 系统
  Task 5.1 (数据+Manager)
  (依赖 Phase 2 的迁移框架创建 mercenary 表)
```

## 验证检查清单

- [ ] `cargo test --lib` 全部通过
- [ ] `cargo build` 无警告
- [ ] `MapState::is_walkable()` 返回真实碰撞结果
- [ ] .gat 文件可以正确解析
- [ ] 数据库迁移幂等（多次运行结果一致）
- [ ] HomunculusManager::summon() BUG 已修复
- [ ] MercenaryManager 使用真实模板数据
- [ ] 所有新模块有 thiserror 错误类型
- [ ] 所有新功能有单元测试覆盖
