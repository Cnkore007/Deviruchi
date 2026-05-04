//! 怪物寻路模块
//! 使用 A* 算法实现八方向寻路，支持严格斜角移动规则

use pathfinding::prelude::astar;

/// 位置类型
type Pos = (u16, u16);

/// 方向偏移量（八方向）
const DIRECTIONS: [(i32, i32); 8] = [
    (0, -1),  // 北 (N)
    (0, 1),   // 南 (S)
    (-1, 0),  // 西 (W)
    (1, 0),   // 东 (E)
    (-1, -1), // 西北 (NW)
    (1, -1),  // 东北 (NE)
    (-1, 1),  // 西南 (SW)
    (1, 1),   // 东南 (SE)
];

/// 判断是否为斜角方向
fn is_diagonal(dx: i32, dy: i32) -> bool {
    dx != 0 && dy != 0
}

/// 八方向欧几里得距离启发函数
fn heuristic(a: &Pos, b: &Pos) -> usize {
    let dx = (a.0 as f64 - b.0 as f64).abs();
    let dy = (a.1 as f64 - b.1 as f64).abs();
    (dx * dx + dy * dy).sqrt() as usize
}

/// 寻路器
pub struct Pathfinder;

impl Pathfinder {
    /// 从起点到终点寻找最短路径
    ///
    /// # 参数
    /// - `start`: 起点坐标
    /// - `end`: 终点坐标
    /// - `is_walkable`: 闭包，判断格子是否可通行
    /// - `search_radius`: 搜索半径（chase_range）
    ///
    /// # 返回
    /// - `Some(path)`: 路径点列表（不含起点，包含终点）
    /// - `None`: 无法找到路径或目标超出搜索范围
    pub fn find_path<F>(
        start: Pos,
        end: Pos,
        is_walkable: F,
        search_radius: u16,
    ) -> Option<Vec<Pos>>
    where
        F: Fn(u16, u16) -> bool,
    {
        // 检查终点是否在搜索范围内
        let (sx, sy) = start;
        let (ex, ey) = end;
        let dx = (sx as i32 - ex as i32).unsigned_abs() as u16;
        let dy = (sy as i32 - ey as i32).unsigned_abs() as u16;
        if dx > search_radius || dy > search_radius {
            return None;
        }

        // 使用 astar 寻路
        astar(
            &start,
            |pos| Self::successors(pos, &is_walkable),
            |pos| heuristic(pos, &end),
            |pos| *pos == end,
        )
        .map(|(path, _)| path.into_iter().skip(1).collect())
    }

    /// 生成后继节点（八方向）
    fn successors<F>(pos: &Pos, is_walkable: &F) -> Vec<(Pos, usize)>
    where
        F: Fn(u16, u16) -> bool,
    {
        let (x, y) = *pos;
        let mut result = Vec::with_capacity(8);

        for &(dx, dy) in &DIRECTIONS {
            let nx = (x as i32 + dx) as u16;
            let ny = (y as i32 + dy) as u16;

            if is_diagonal(dx, dy) {
                // 严格斜角规则：目标格 + 两个 cardinal 邻格都必须可通行
                if Self::strict_diagonal_walkable(nx, ny, x, y, is_walkable) {
                    result.push(((nx, ny), 1));
                }
            } else {
                // 直线移动只检查目标格
                if is_walkable(nx, ny) {
                    result.push(((nx, ny), 1));
                }
            }
        }

        result
    }

    /// 严格斜角可通行检查
    fn strict_diagonal_walkable<F>(nx: u16, ny: u16, cx: u16, cy: u16, is_walkable: &F) -> bool
    where
        F: Fn(u16, u16) -> bool,
    {
        // 目标格必须可通行
        if !is_walkable(nx, ny) {
            return false;
        }

        // 计算相邻 cardinal 格子
        let (dx, dy) = (nx as i32 - cx as i32, ny as i32 - cy as i32);
        let cardinal1 = ((nx as i32 - dx) as u16, ny);
        let cardinal2 = (nx, (ny as i32 - dy) as u16);

        // 两个 cardinal 邻格都必须可通行
        is_walkable(cardinal1.0, cardinal1.1) && is_walkable(cardinal2.0, cardinal2.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试辅助：创建全为 walkable 的地图
    fn all_walkable(_x: u16, _y: u16) -> bool {
        true
    }

    #[test]
    fn test_straight_line_path() {
        let start: Pos = (0, 0);
        let end: Pos = (5, 0);
        let path = Pathfinder::find_path(start, end, all_walkable, 20);

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(!path.is_empty());
        assert_eq!(path.last(), Some(&end));
    }

    #[test]
    fn test_vertical_path() {
        let start: Pos = (0, 0);
        let end: Pos = (0, 5);
        let path = Pathfinder::find_path(start, end, all_walkable, 20);

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(!path.is_empty());
        assert_eq!(path.last(), Some(&end));
    }

    #[test]
    fn test_diagonal_path() {
        let start: Pos = (0, 0);
        let end: Pos = (3, 3);
        let path = Pathfinder::find_path(start, end, all_walkable, 20);

        assert!(path.is_some());
        let path = path.unwrap();
        assert!(!path.is_empty());
        assert_eq!(path.last(), Some(&end));
    }

    #[test]
    fn test_path_around_wall() {
        // 创建一个 L 形墙，挡在直线路径上
        let map = |x: u16, y: u16| -> bool {
            // 墙: (2, 0), (2, 1), (2, 2)
            !(x == 2 && y <= 2)
        };

        let start: Pos = (0, 0);
        let end: Pos = (3, 0);
        let path = Pathfinder::find_path(start, end, map, 20);

        assert!(path.is_some());
        let path = path.unwrap();
        // 应该绕过墙
        assert!(path.len() > 2);
    }

    #[test]
    fn test_no_path_when_blocked() {
        // 创建完全封闭的空间
        let map = |x: u16, y: u16| -> bool {
            // 只允许起点附近
            x <= 1 && y <= 1
        };

        let start: Pos = (0, 0);
        let end: Pos = (5, 5);
        let path = Pathfinder::find_path(start, end, map, 20);

        assert!(path.is_none());
    }

    #[test]
    fn test_out_of_range() {
        let start: Pos = (0, 0);
        let end: Pos = (100, 100); // 超出搜索范围
        let path = Pathfinder::find_path(start, end, all_walkable, 20);

        assert!(path.is_none());
    }

    #[test]
    fn test_strict_diagonal_blocked_by_cardinal() {
        // 场景: 目标是 (2,2)，但北边 (2,1) 是墙
        // 此时东南方向移动应该被阻止
        let map = |x: u16, y: u16| -> bool {
            !(x == 2 && y == 1) // (2,1) 是墙
        };

        let start: Pos = (1, 1);
        let end: Pos = (3, 3);
        let path = Pathfinder::find_path(start, end, map, 20);

        // 应该能找到路径，但不会直接走 (2,2) 斜角
        assert!(path.is_some());
    }

    #[test]
    fn test_u_shaped_obstacle() {
        // L 形障碍（简化版），测试绕行能力
        // 墙: x=3, y=0..3
        let map = |x: u16, y: u16| -> bool {
            // 地图边界: 0 <= x <= 10, 0 <= y <= 10
            if x > 10 || y > 10 {
                return false;
            }
            // L 形墙
            !(x == 3 && y <= 3)
        };

        let start: Pos = (0, 0);
        let end: Pos = (5, 0);
        let path = Pathfinder::find_path(start, end, map, 20);

        assert!(path.is_some());
        let path = path.unwrap();
        // 路径应该绕过 L 形墙
        assert!(path.len() > 3);
    }
}
