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
