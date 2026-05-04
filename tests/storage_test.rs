use deviruchi::game::storage::{Storage, StorageSlot};

#[test]
fn test_storage_slot_empty() {
    let slot = StorageSlot::empty(0);
    assert!(slot.is_empty());
    assert_eq!(slot.index, 0);
}

#[test]
fn test_storage_slot_with_item() {
    let slot = StorageSlot {
        index: 0,
        item_id: 501,
        amount: 10,
        identified: true,
        refine: 0,
        cards: [0; 4],
    };
    assert!(!slot.is_empty());
    assert_eq!(slot.item_id, 501);
    assert_eq!(slot.amount, 10);
}

#[test]
fn test_storage_new() {
    let storage = Storage::new(100);
    assert_eq!(storage.len(), 100);
    assert!(storage.get_slot(0).is_some());
    assert!(storage.get_slot(0).unwrap().is_empty());
}

#[test]
fn test_storage_add_item() {
    let mut storage = Storage::new(100);
    assert!(storage.add_item(501, 10));

    let slot = storage.find_item_slot(501).unwrap();
    assert_eq!(slot.item_id, 501);
    assert_eq!(slot.amount, 10);
}

#[test]
fn test_storage_remove_item() {
    let mut storage = Storage::new(100);
    assert!(storage.add_item(501, 10));
    assert!(storage.remove_item(0, 5));

    let slot = storage.get_slot(0).unwrap();
    assert_eq!(slot.amount, 5);
}

#[test]
fn test_storage_move_item() {
    let mut storage = Storage::new(100);
    storage.add_item(501, 10);

    // Move from slot 0 to slot 10
    assert!(storage.move_item(0, 10));

    let slot0 = storage.get_slot(0).unwrap();
    assert!(slot0.is_empty());

    let slot10 = storage.get_slot(10).unwrap();
    assert_eq!(slot10.item_id, 501);
    assert_eq!(slot10.amount, 10);
}

#[test]
fn test_storage_move_item_merge() {
    let mut storage = Storage::new(100);

    // Manually set up two slots with the same item for testing merge
    {
        let slot0 = storage.get_slot_mut(0).unwrap();
        slot0.item_id = 501;
        slot0.amount = 10;
        slot0.identified = true;
    }
    {
        let slot5 = storage.get_slot_mut(5).unwrap();
        slot5.item_id = 501;
        slot5.amount = 20;
        slot5.identified = true;
    }

    // Move from slot 5 to slot 0 - should merge
    assert!(storage.move_item(5, 0));

    let slot0 = storage.get_slot(0).unwrap();
    let slot5 = storage.get_slot(5).unwrap();
    assert_eq!(slot0.amount, 30);
    assert!(slot5.is_empty());
}

#[test]
fn test_storage_add_to_full() {
    let mut storage = Storage::new(2);
    assert!(storage.add_item(501, 10));
    assert!(storage.add_item(502, 10));
    assert!(!storage.add_item(503, 10)); // Should fail - full
}

#[test]
fn test_storage_stack_limit() {
    let mut storage = Storage::new(100);
    assert!(storage.add_item(501, 30000));
    assert!(storage.add_item(501, 1)); // Should create new stack
    assert_eq!(storage.used_count(), 2);
}

#[test]
fn test_storage_remove_more_than_available() {
    let mut storage = Storage::new(100);
    assert!(storage.add_item(501, 10));
    assert!(!storage.remove_item(0, 20)); // Cannot remove more than available

    let slot = storage.get_slot(0).unwrap();
    assert_eq!(slot.amount, 10); // Amount unchanged
}

#[test]
fn test_storage_remove_all() {
    let mut storage = Storage::new(100);
    assert!(storage.add_item(501, 10));
    assert!(storage.remove_item(0, 10)); // Remove all

    let slot = storage.get_slot(0).unwrap();
    assert!(slot.is_empty());
    assert_eq!(slot.item_id, 0);
}

use deviruchi::game::storage::manager::StorageManager;
use std::sync::Arc;

#[test]
fn test_storage_manager_get_or_create() {
    let manager = StorageManager::new();

    // 获取角色1的仓库
    let storage1 = manager.get_or_create(1, 100);
    assert_eq!(storage1.read().char_id(), 1);

    // 再次获取应该是同一个
    let storage2 = manager.get_or_create(1, 100);
    assert_eq!(storage2.read().char_id(), 1);
}

#[test]
fn test_storage_manager_remove() {
    let manager = StorageManager::new();

    // 创建仓库
    let storage = manager.get_or_create(1, 100);
    assert_eq!(storage.read().char_id(), 1);

    // 移除仓库
    manager.remove(&1);

    // 再次获取应该是新的（会重新创建）
    let storage = manager.get_or_create(1, 100);
    assert_eq!(storage.read().char_id(), 1);
}

#[test]
fn test_storage_manager_save_and_load() {
    use parking_lot::RwLock;

    let manager = Arc::new(RwLock::new(StorageManager::new()));

    // 创建并修改仓库
    {
        let mut mgr = manager.write();
        let storage = mgr.get_or_create(1, 100);
        storage.write().add_item(501, 10);
        storage.write().add_item(502, 5);
    }

    // 验证物品存在
    {
        let mgr = manager.read();
        let storage = mgr.get(1).unwrap();
        assert_eq!(storage.read().used_count(), 2);
    }
}
