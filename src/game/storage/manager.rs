use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::data::Storage;

/// 仓库管理器
/// 管理所有在线角色的仓库
pub struct StorageManager {
    storages: RwLock<HashMap<u32, Arc<RwLock<Storage>>>>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            storages: RwLock::new(HashMap::new()),
        }
    }

    /// 获取或创建角色的仓库
    pub fn get_or_create(&self, char_id: u32, max_size: u16) -> Arc<RwLock<Storage>> {
        let mut storages = self.storages.write();

        if let Some(storage) = storages.get(&char_id) {
            return storage.clone();
        }

        // 创建新仓库
        let storage = Arc::new(RwLock::new(
            Storage::new(max_size).with_char_id(char_id)
        ));
        storages.insert(char_id, storage.clone());
        storage
    }

    /// 获取角色的仓库（如果不存在返回 None）
    pub fn get(&self, char_id: u32) -> Option<Arc<RwLock<Storage>>> {
        let storages = self.storages.read();
        storages.get(&char_id).cloned()
    }

    /// 移除角色的仓库
    pub fn remove(&self, char_id: &u32) {
        let mut storages = self.storages.write();
        storages.remove(char_id);
    }

    /// 获取仓库数量
    pub fn count(&self) -> usize {
        let storages = self.storages.read();
        storages.len()
    }

    /// 检查角色是否有仓库
    pub fn has_storage(&self, char_id: u32) -> bool {
        let storages = self.storages.read();
        storages.contains_key(&char_id)
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}
