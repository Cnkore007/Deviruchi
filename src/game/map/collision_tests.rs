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
        db.insert(map);

        let db_arc = Arc::new(db);
        let state = MapState::with_map_database(db_arc);

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
