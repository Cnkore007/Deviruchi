//! 路径搜索系统
//!
//! 对应 rAthena 的 `src/map/path.cpp`，提供 A* 寻路和距离计算功能。
//! 用于怪物 AI、NPC 移动、玩家移动验证等。

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// 方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    NorthWest = 1,
    West = 2,
    SouthWest = 3,
    South = 4,
    SouthEast = 5,
    East = 6,
    NorthEast = 7,
}

impl Direction {
    /// 从 u8 转换
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::North),
            1 => Some(Self::NorthWest),
            2 => Some(Self::West),
            3 => Some(Self::SouthWest),
            4 => Some(Self::South),
            5 => Some(Self::SouthEast),
            6 => Some(Self::East),
            7 => Some(Self::NorthEast),
            _ => None,
        }
    }

    /// 获取相反方向
    pub fn opposite(&self) -> Self {
        match self {
            Self::North => Self::South,
            Self::NorthWest => Self::SouthEast,
            Self::West => Self::East,
            Self::SouthWest => Self::NorthEast,
            Self::South => Self::North,
            Self::SouthEast => Self::NorthWest,
            Self::East => Self::West,
            Self::NorthEast => Self::SouthWest,
        }
    }

    /// 检查是否为对角线方向
    pub fn is_diagonal(&self) -> bool {
        matches!(
            self,
            Self::NorthWest | Self::SouthWest | Self::SouthEast | Self::NorthEast
        )
    }

    /// 获取方向的 dx, dy 偏移
    pub fn offset(&self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::NorthWest => (-1, -1),
            Self::West => (-1, 0),
            Self::SouthWest => (-1, 1),
            Self::South => (0, 1),
            Self::SouthEast => (1, 1),
            Self::East => (1, 0),
            Self::NorthEast => (1, -1),
        }
    }
}

/// 坐标点
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 移动到指定方向
    pub fn step(&self, dir: Direction) -> Self {
        let (dx, dy) = dir.offset();
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// A* 搜索节点（内部使用）
#[derive(Debug, Clone, Eq, PartialEq)]
struct SearchNode {
    f_score: i32,
    g_score: i32,
    position: Point,
    parent: Option<Point>,
}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // 注意：反转顺序用于最小堆
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 路径搜索结果
#[derive(Debug, Clone)]
pub struct PathResult {
    /// 路径点列表
    pub path: Vec<Point>,
    /// 路径长度
    pub length: usize,
    /// 是否到达目标
    pub reached: bool,
}

/// 路径搜索配置
#[derive(Debug, Clone)]
pub struct PathConfig {
    /// 最大搜索步数
    pub max_steps: usize,
    /// 是否允许对角线移动
    pub allow_diagonal: bool,
    /// 对角线移动代价（通常为 14，水平/垂直为 10）
    pub diagonal_cost: i32,
    /// 水平/垂直移动代价
    pub straight_cost: i32,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            allow_diagonal: true,
            diagonal_cost: 14,
            straight_cost: 10,
        }
    }
}

/// 地图碰撞检测接口
pub trait CollisionMap {
    /// 检查指定位置是否可通行
    fn is_walkable(&self, x: i32, y: i32) -> bool;

    /// 获取地图宽度
    fn width(&self) -> i32;

    /// 获取地图高度
    fn height(&self) -> i32;
}

/// 路径搜索器
///
/// 提供 A* 寻路算法和距离计算功能。
pub struct PathSearcher {
    config: PathConfig,
}

impl PathSearcher {
    /// 创建路径搜索器
    pub fn new() -> Self {
        Self {
            config: PathConfig::default(),
        }
    }

    /// 使用指定配置创建
    pub fn with_config(config: PathConfig) -> Self {
        Self { config }
    }

    /// A* 路径搜索
    ///
    /// 从起点到终点的最短路径，考虑地图碰撞。
    pub fn search<C: CollisionMap>(&self, map: &C, from: Point, to: Point) -> PathResult {
        if !map.is_walkable(from.x, from.y) || !map.is_walkable(to.x, to.y) {
            return PathResult {
                path: vec![],
                length: 0,
                reached: false,
            };
        }

        if from == to {
            return PathResult {
                path: vec![from],
                length: 0,
                reached: true,
            };
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: std::collections::HashMap<Point, Point> =
            std::collections::HashMap::new();
        let mut g_score: std::collections::HashMap<Point, i32> = std::collections::HashMap::new();
        let mut visited = std::collections::HashSet::new();
        let mut steps = 0;

        g_score.insert(from, 0);
        open_set.push(SearchNode {
            f_score: self.heuristic(from, to),
            g_score: 0,
            position: from,
            parent: None,
        });

        while let Some(current) = open_set.pop() {
            steps += 1;
            if steps > self.config.max_steps {
                break;
            }

            if current.position == to {
                // 重建路径
                let path = self.rebuild_path(&came_from, to);
                let length = path.len();
                return PathResult {
                    path,
                    length,
                    reached: true,
                };
            }

            if visited.contains(&current.position) {
                continue;
            }
            visited.insert(current.position);

            // 探索相邻节点
            for dir in self.get_directions() {
                let next = current.position.step(dir);

                if !map.is_walkable(next.x, next.y) {
                    continue;
                }

                if visited.contains(&next) {
                    continue;
                }

                let move_cost = if dir.is_diagonal() {
                    self.config.diagonal_cost
                } else {
                    self.config.straight_cost
                };

                let tentative_g = current.g_score + move_cost;

                if tentative_g < *g_score.get(&next).unwrap_or(&i32::MAX) {
                    came_from.insert(next, current.position);
                    g_score.insert(next, tentative_g);

                    open_set.push(SearchNode {
                        f_score: tentative_g + self.heuristic(next, to),
                        g_score: tentative_g,
                        position: next,
                        parent: Some(current.position),
                    });
                }
            }
        }

        // 无法到达目标，返回最近的路径
        let path = self.rebuild_path(&came_from, from);
        PathResult {
            path,
            length: 0,
            reached: false,
        }
    }

    /// 计算两点间的曼哈顿距离
    pub fn distance(from: Point, to: Point) -> i32 {
        (from.x - to.x).abs() + (from.y - to.y).abs()
    }

    /// 计算两点间的欧几里得距离（整数近似）
    pub fn distance_sqrt(from: Point, to: Point) -> i32 {
        let dx = (from.x - to.x).abs();
        let dy = (from.y - to.y).abs();
        ((dx * dx + dy * dy) as f64).sqrt() as i32
    }

    /// 检查两点是否在指定距离内
    pub fn check_distance(from: Point, to: Point, range: i32) -> bool {
        Self::distance(from, to) <= range
    }

    /// 获取从一个方向到另一个方向的中间方向
    pub fn direction_diagonal(from: Direction, to: Direction) -> Direction {
        let from_val = from as u8;
        let to_val = to as u8;
        let diff = (to_val as i32 - from_val as i32 + 8) % 8;

        match diff {
            0 => from,
            1 | 2 => Direction::from_u8((from_val + 1) % 8).unwrap_or(from),
            3 | 4 => Direction::from_u8((from_val + 2) % 8).unwrap_or(from),
            5 | 6 => Direction::from_u8((from_val + 3) % 8).unwrap_or(from),
            _ => Direction::from_u8((from_val + 4) % 8).unwrap_or(from),
        }
    }

    /// 计算启发式距离（A* 用）
    fn heuristic(&self, from: Point, to: Point) -> i32 {
        // 使用对角线距离（Chebyshev distance）
        let dx = (from.x - to.x).abs();
        let dy = (from.y - to.y).abs();
        let straight = dx.max(dy) - dx.min(dy);
        let diagonal = dx.min(dy);
        straight * self.config.straight_cost + diagonal * self.config.diagonal_cost
    }

    /// 重建路径
    fn rebuild_path(
        &self,
        came_from: &std::collections::HashMap<Point, Point>,
        end: Point,
    ) -> Vec<Point> {
        let mut path = vec![end];
        let mut current = end;

        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }

        path.reverse();
        path
    }

    /// 获取要探索的方向列表
    fn get_directions(&self) -> Vec<Direction> {
        if self.config.allow_diagonal {
            vec![
                Direction::North,
                Direction::NorthWest,
                Direction::West,
                Direction::SouthWest,
                Direction::South,
                Direction::SouthEast,
                Direction::East,
                Direction::NorthEast,
            ]
        } else {
            vec![
                Direction::North,
                Direction::West,
                Direction::South,
                Direction::East,
            ]
        }
    }
}

impl Default for PathSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMap {
        width: i32,
        height: i32,
        blocked: std::collections::HashSet<(i32, i32)>,
    }

    impl TestMap {
        fn new(width: i32, height: i32) -> Self {
            Self {
                width,
                height,
                blocked: std::collections::HashSet::new(),
            }
        }

        fn block(&mut self, x: i32, y: i32) {
            self.blocked.insert((x, y));
        }
    }

    impl CollisionMap for TestMap {
        fn is_walkable(&self, x: i32, y: i32) -> bool {
            x >= 0 && x < self.width && y >= 0 && y < self.height && !self.blocked.contains(&(x, y))
        }

        fn width(&self) -> i32 {
            self.width
        }

        fn height(&self) -> i32 {
            self.height
        }
    }

    #[test]
    fn test_direction_opposite() {
        assert_eq!(Direction::North.opposite(), Direction::South);
        assert_eq!(Direction::East.opposite(), Direction::West);
        assert_eq!(Direction::NorthEast.opposite(), Direction::SouthWest);
    }

    #[test]
    fn test_direction_offset() {
        assert_eq!(Direction::North.offset(), (0, -1));
        assert_eq!(Direction::South.offset(), (0, 1));
        assert_eq!(Direction::East.offset(), (1, 0));
        assert_eq!(Direction::West.offset(), (-1, 0));
    }

    #[test]
    fn test_direction_diagonal() {
        assert!(Direction::NorthWest.is_diagonal());
        assert!(!Direction::North.is_diagonal());
    }

    #[test]
    fn test_path_search_direct() {
        let map = TestMap::new(10, 10);
        let searcher = PathSearcher::new();

        let from = Point::new(0, 0);
        let to = Point::new(5, 5);

        let result = searcher.search(&map, from, to);
        assert!(result.reached);
        assert!(result.length > 0);
        assert_eq!(*result.path.first().unwrap(), from);
        assert_eq!(*result.path.last().unwrap(), to);
    }

    #[test]
    fn test_path_search_same_point() {
        let map = TestMap::new(10, 10);
        let searcher = PathSearcher::new();

        let point = Point::new(5, 5);
        let result = searcher.search(&map, point, point);

        assert!(result.reached);
        assert_eq!(result.path.len(), 1);
        assert_eq!(result.path[0], point);
    }

    #[test]
    fn test_path_search_blocked() {
        let mut map = TestMap::new(10, 10);
        map.block(5, 5);
        let searcher = PathSearcher::new();

        let from = Point::new(0, 0);
        let to = Point::new(5, 5);

        let result = searcher.search(&map, from, to);
        assert!(!result.reached);
    }

    #[test]
    fn test_path_search_with_obstacle() {
        let mut map = TestMap::new(10, 10);
        // 创建障碍墙
        for y in 0..8 {
            map.block(5, y);
        }
        let searcher = PathSearcher::new();

        let from = Point::new(0, 0);
        let to = Point::new(9, 0);

        let result = searcher.search(&map, from, to);
        assert!(result.reached);
        // 路径应该绕过障碍
        assert!(result.path.iter().all(|p| map.is_walkable(p.x, p.y)));
    }

    #[test]
    fn test_distance() {
        let from = Point::new(0, 0);
        let to = Point::new(3, 4);

        assert_eq!(PathSearcher::distance(from, to), 7); // 3 + 4
        assert_eq!(PathSearcher::distance_sqrt(from, to), 5); // sqrt(9 + 16)
    }

    #[test]
    fn test_check_distance() {
        let from = Point::new(0, 0);
        let to = Point::new(3, 4);

        assert!(PathSearcher::check_distance(from, to, 7));
        assert!(!PathSearcher::check_distance(from, to, 6));
    }

    #[test]
    fn test_path_search_no_diagonal() {
        let config = PathConfig {
            allow_diagonal: false,
            ..Default::default()
        };
        let map = TestMap::new(10, 10);
        let searcher = PathSearcher::with_config(config);

        let from = Point::new(0, 0);
        let to = Point::new(3, 3);

        let result = searcher.search(&map, from, to);
        assert!(result.reached);
        // 不允许对角线时，路径应该是直线+转弯
        assert!(result.length >= 6); // 至少需要 6 步（3 + 3）
    }
}
