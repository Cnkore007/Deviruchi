//! 导航系统
//!
//! 对应 rAthena 的 `src/map/navi.cpp`，提供地图间路径搜索功能。
//!
//! Deviruchi 已有 mob AI 的 A* 寻路（`mob/ai.rs`），此模块提供地图级别的导航。

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use parking_lot::RwLock;

/// 地图节点
#[derive(Debug, Clone)]
pub struct MapNode {
    /// 地图名
    pub name: String,
    /// 相邻地图列表 (相邻地图名, 传送费用)
    pub neighbors: Vec<(String, u32)>,
}

/// 路径结果
#[derive(Debug, Clone)]
pub struct NaviPath {
    /// 路径经过的地图列表
    pub maps: Vec<String>,
    /// 总费用
    pub total_cost: u32,
}

/// A* 搜索节点（内部使用）
#[derive(Debug, Clone, Eq, PartialEq)]
struct SearchNode {
    cost: u32,
    position: String,
}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // 注意：反转顺序用于最小堆
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 地图导航管理器
///
/// 提供地图级别的路径搜索，用于 @warp、@go 等命令和 NPC 传送。
pub struct NaviManager {
    /// 地图节点 (map_name -> MapNode)
    maps: RwLock<HashMap<String, MapNode>>,
}

impl NaviManager {
    /// 创建空的导航管理器
    pub fn new() -> Self {
        Self {
            maps: RwLock::new(HashMap::new()),
        }
    }

    /// 添加地图节点
    pub fn add_map(&self, name: String, neighbors: Vec<(String, u32)>) {
        let node = MapNode {
            name: name.clone(),
            neighbors,
        };
        self.maps.write().insert(name, node);
    }

    /// 添加单向连接
    ///
    /// 如果地图不存在，会自动创建。
    pub fn add_connection(&self, from: &str, to: &str, cost: u32) {
        let mut maps = self.maps.write();

        // 确保源地图存在
        if !maps.contains_key(from) {
            maps.insert(
                from.to_string(),
                MapNode {
                    name: from.to_string(),
                    neighbors: Vec::new(),
                },
            );
        }

        // 确保目标地图存在
        if !maps.contains_key(to) {
            maps.insert(
                to.to_string(),
                MapNode {
                    name: to.to_string(),
                    neighbors: Vec::new(),
                },
            );
        }

        // 添加连接
        if let Some(node) = maps.get_mut(from)
            && !node.neighbors.iter().any(|(n, _)| n == to) {
                node.neighbors.push((to.to_string(), cost));
            }
    }

    /// 添加双向连接
    pub fn add_bidirectional(&self, map1: &str, map2: &str, cost: u32) {
        self.add_connection(map1, map2, cost);
        self.add_connection(map2, map1, cost);
    }

    /// A* 路径搜索
    ///
    /// 从源地图到目标地图的最短路径。
    pub fn find_path(&self, from: &str, to: &str) -> Option<NaviPath> {
        let maps = self.maps.read();

        if !maps.contains_key(from) || !maps.contains_key(to) {
            return None;
        }

        if from == to {
            return Some(NaviPath {
                maps: vec![from.to_string()],
                total_cost: 0,
            });
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<String, String> = HashMap::new();
        let mut g_score: HashMap<String, u32> = HashMap::new();
        let mut visited = HashSet::new();

        g_score.insert(from.to_string(), 0);
        open_set.push(SearchNode {
            cost: 0,
            position: from.to_string(),
        });

        while let Some(current) = open_set.pop() {
            if current.position == to {
                // 重建路径
                let mut path = vec![to.to_string()];
                let mut current_pos = to.to_string();
                while let Some(prev) = came_from.get(&current_pos) {
                    path.push(prev.clone());
                    current_pos = prev.clone();
                }
                path.reverse();
                return Some(NaviPath {
                    maps: path,
                    total_cost: current.cost,
                });
            }

            if visited.contains(&current.position) {
                continue;
            }
            visited.insert(current.position.clone());

            if let Some(node) = maps.get(&current.position) {
                for (neighbor, cost) in &node.neighbors {
                    if visited.contains(neighbor) {
                        continue;
                    }

                    let tentative_g = g_score[&current.position] + cost;

                    if tentative_g < *g_score.get(neighbor).unwrap_or(&u32::MAX) {
                        came_from.insert(neighbor.clone(), current.position.clone());
                        g_score.insert(neighbor.clone(), tentative_g);

                        open_set.push(SearchNode {
                            cost: tentative_g,
                            position: neighbor.clone(),
                        });
                    }
                }
            }
        }

        None // 无法到达
    }

    /// 检查两个地图是否连通
    pub fn is_connected(&self, from: &str, to: &str) -> bool {
        self.find_path(from, to).is_some()
    }

    /// 获取地图的相邻地图
    pub fn get_neighbors(&self, map_name: &str) -> Vec<(String, u32)> {
        self.maps
            .read()
            .get(map_name)
            .map(|n| n.neighbors.clone())
            .unwrap_or_default()
    }

    /// 获取地图总数
    pub fn map_count(&self) -> usize {
        self.maps.read().len()
    }

    /// 清空导航数据
    pub fn clear(&self) {
        self.maps.write().clear();
    }
}

impl Default for NaviManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navi_direct_path() {
        let manager = NaviManager::new();
        manager.add_bidirectional("prontera", "gef_fild01", 10);

        let path = manager.find_path("prontera", "gef_fild01").unwrap();
        assert_eq!(path.maps, vec!["prontera", "gef_fild01"]);
        assert_eq!(path.total_cost, 10);
    }

    #[test]
    fn test_navi_multi_hop() {
        let manager = NaviManager::new();
        manager.add_bidirectional("prontera", "gef_fild01", 10);
        manager.add_bidirectional("gef_fild01", "geffen", 5);
        manager.add_bidirectional("geffen", "gef_fild04", 8);

        let path = manager.find_path("prontera", "gef_fild04").unwrap();
        assert_eq!(path.maps, vec!["prontera", "gef_fild01", "geffen", "gef_fild04"]);
        assert_eq!(path.total_cost, 23);
    }

    #[test]
    fn test_navi_shortest_path() {
        let manager = NaviManager::new();
        manager.add_bidirectional("prontera", "gef_fild01", 10);
        manager.add_bidirectional("gef_fild01", "geffen", 5);
        manager.add_bidirectional("prontera", "geffen", 20);

        let path = manager.find_path("prontera", "geffen").unwrap();
        assert_eq!(path.total_cost, 15); // prontera -> gef_fild01 -> geffen
    }

    #[test]
    fn test_navi_same_map() {
        let manager = NaviManager::new();
        manager.add_map("prontera".to_string(), vec![]);

        let path = manager.find_path("prontera", "prontera").unwrap();
        assert_eq!(path.maps, vec!["prontera"]);
        assert_eq!(path.total_cost, 0);
    }

    #[test]
    fn test_navi_no_path() {
        let manager = NaviManager::new();
        manager.add_map("prontera".to_string(), vec![]);
        manager.add_map("geffen".to_string(), vec![]);

        let path = manager.find_path("prontera", "geffen");
        assert!(path.is_none());
    }

    #[test]
    fn test_navi_unknown_map() {
        let manager = NaviManager::new();
        let path = manager.find_path("unknown", "also_unknown");
        assert!(path.is_none());
    }

    #[test]
    fn test_navi_is_connected() {
        let manager = NaviManager::new();
        manager.add_bidirectional("prontera", "geffen", 10);

        assert!(manager.is_connected("prontera", "geffen"));
        assert!(!manager.is_connected("prontera", "alberta"));
    }

    #[test]
    fn test_navi_get_neighbors() {
        let manager = NaviManager::new();
        manager.add_bidirectional("prontera", "gef_fild01", 10);
        manager.add_bidirectional("prontera", "prt_fild01", 5);

        let neighbors = manager.get_neighbors("prontera");
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_navi_clear() {
        let manager = NaviManager::new();
        manager.add_map("prontera".to_string(), vec![]);

        manager.clear();
        assert_eq!(manager.map_count(), 0);
    }
}
