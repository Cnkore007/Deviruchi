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
        let storage = Arc::new(RwLock::new(Storage::new(max_size).with_char_id(char_id)));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// 新建管理器仓库数量为 0
    #[test]
    fn new_manager_has_zero_count() {
        let manager = StorageManager::new();
        assert_eq!(manager.count(), 0);
    }

    /// get_or_create 创建新仓库
    #[test]
    fn get_or_create_creates_new_storage() {
        let manager = StorageManager::new();
        let storage = manager.get_or_create(1, 100);
        assert_eq!(manager.count(), 1);
        assert_eq!(storage.read().max_size(), 100);
        assert_eq!(storage.read().char_id(), 1);
    }

    /// get_or_create 返回已有仓库（不重复创建）
    #[test]
    fn get_or_create_returns_existing() {
        let manager = StorageManager::new();
        let s1 = manager.get_or_create(1, 100);
        s1.write().add_item(501, 10);
        let s2 = manager.get_or_create(1, 200); // max_size 参数应被忽略
        assert_eq!(s2.read().get_slot(0).unwrap().item_id, 501);
        assert_eq!(manager.count(), 1);
    }

    /// 获取不存在的角色返回 None
    #[test]
    fn get_returns_none_for_missing() {
        let manager = StorageManager::new();
        assert!(manager.get(999).is_none());
    }

    /// 获取已存在的角色返回 Some
    #[test]
    fn get_returns_some_for_existing() {
        let manager = StorageManager::new();
        manager.get_or_create(1, 100);
        assert!(manager.get(1).is_some());
    }

    /// remove 移除仓库
    #[test]
    fn remove_removes_storage() {
        let manager = StorageManager::new();
        manager.get_or_create(1, 100);
        assert_eq!(manager.count(), 1);
        manager.remove(&1);
        assert_eq!(manager.count(), 0);
        assert!(manager.get(1).is_none());
    }

    /// remove 不存在的角色不会 panic
    #[test]
    fn remove_nonexistent_is_noop() {
        let manager = StorageManager::new();
        manager.remove(&999);
        assert_eq!(manager.count(), 0);
    }

    /// has_storage 正确反映状态变化
    #[test]
    fn has_storage_returns_correct_state() {
        let manager = StorageManager::new();
        assert!(!manager.has_storage(1));
        manager.get_or_create(1, 100);
        assert!(manager.has_storage(1));
        manager.remove(&1);
        assert!(!manager.has_storage(1));
    }

    /// 多角色仓库互相独立
    #[test]
    fn multiple_char_storages_are_independent() {
        let manager = StorageManager::new();
        let s1 = manager.get_or_create(1, 100);
        let s2 = manager.get_or_create(2, 200);
        s1.write().add_item(501, 10);
        s2.write().add_item(601, 20);
        assert_eq!(s1.read().get_slot(0).unwrap().item_id, 501);
        assert_eq!(s2.read().get_slot(0).unwrap().item_id, 601);
        assert_eq!(manager.count(), 2);
    }

    /// 并发访问不会 panic
    #[test]
    fn concurrent_access_does_not_panic() {
        let manager = Arc::new(StorageManager::new());
        let mut handles = vec![];

        for i in 0..10 {
            let mgr = manager.clone();
            handles.push(thread::spawn(move || {
                let storage = mgr.get_or_create(i, 100);
                storage.write().add_item(501, 1);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(manager.count(), 10);
    }

    /// Default trait 正常工作
    #[test]
    fn default_trait_works() {
        let manager = StorageManager::default();
        assert_eq!(manager.count(), 0);
    }
}
