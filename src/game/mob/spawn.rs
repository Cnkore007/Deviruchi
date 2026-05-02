use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::game::mob::Mob;

/// 怪物刷新点
#[derive(Debug, Clone)]
pub struct SpawnPoint {
    pub mob_id: u16,
    pub x: u16,
    pub y: u16,
    pub count: u8,
    pub interval: u32,
}

/// 地图怪物刷新管理
pub struct MobSpawnManager {
    spawns: RwLock<HashMap<String, Vec<SpawnPoint>>>,
    active_mobs: RwLock<HashMap<String, Vec<Arc<Mob>>>>,
}

impl MobSpawnManager {
    pub fn new() -> Self {
        Self {
            spawns: RwLock::new(HashMap::new()),
            active_mobs: RwLock::new(HashMap::new()),
        }
    }

    /// 添加刷新点
    pub fn add_spawn(&self, map_name: &str, spawn: SpawnPoint) {
        let mut spawns = self.spawns.write();
        spawns.entry(map_name.to_string()).or_default().push(spawn);
    }

    /// 获取地图的刷新点
    pub fn get_spawns(&self, map_name: &str) -> Vec<SpawnPoint> {
        self.spawns.read().get(map_name).cloned().unwrap_or_default()
    }

    /// 注册活跃怪物
    pub fn register_mob(&self, map_name: &str, mob: Arc<Mob>) {
        let mut active = self.active_mobs.write();
        active.entry(map_name.to_string()).or_default().push(mob);
    }

    /// 移除死亡怪物
    pub fn unregister_mob(&self, map_name: &str, mob_id: &uuid::Uuid) {
        let mut active = self.active_mobs.write();
        if let Some(mobs) = active.get_mut(map_name) {
            mobs.retain(|m| &m.id != mob_id);
        }
    }

    /// 获取地图上所有活跃怪物
    pub fn get_active_mobs(&self, map_name: &str) -> Vec<Arc<Mob>> {
        self.active_mobs.read().get(map_name).cloned().unwrap_or_default()
    }

    /// 初始化默认刷新点
    pub fn init_default_spawns(&self) {
        self.add_spawn("prontera.gat", SpawnPoint {
            mob_id: 1001,
            x: 100,
            y: 100,
            count: 10,
            interval: 10000,
        });
        self.add_spawn("prontera.gat", SpawnPoint {
            mob_id: 1002,
            x: 150,
            y: 120,
            count: 5,
            interval: 15000,
        });
        self.add_spawn("new_1-1.gat", SpawnPoint {
            mob_id: 1001,
            x: 50,
            y: 50,
            count: 15,
            interval: 5000,
        });
        self.add_spawn("new_1-1.gat", SpawnPoint {
            mob_id: 1312,
            x: 100,
            y: 100,
            count: 10,
            interval: 10000,
        });
    }
}

impl Default for MobSpawnManager {
    fn default() -> Self {
        Self::new()
    }
}
