use std::sync::Arc;
use deviruchi::game::storage::{StorageManager, Storage};

#[test]
fn test_storage_full_workflow() {
    let manager = Arc::new(StorageManager::new());

    // 1. Create storage
    let storage = manager.get_or_create(1, 100);

    // 2. Add items
    {
        let mut s = storage.write();
        assert!(s.add_item(501, 10));
        assert!(s.add_item(502, 5));
        assert_eq!(s.used_count(), 2);
    }

    // 3. Move items
    {
        let mut s = storage.write();
        assert!(s.move_item(0, 10));
    }

    // 4. Remove items
    {
        let mut s = storage.write();
        assert!(s.remove_item(10, 5));
        let slot = s.get_slot(10).unwrap();
        assert_eq!(slot.amount, 5);
    }

    // 5. Remove storage and recreate (simulating persistence)
    manager.remove(&1);
    let storage = manager.get_or_create(1, 100);
    {
        let s = storage.read();
        // New storage should be empty
        assert_eq!(s.used_count(), 0);
    }
}

#[test]
fn test_storage_slot_stack_limit() {
    let mut storage = Storage::new(100);

    // Add items up to stack limit
    assert!(storage.add_item(501, 30000));

    // Adding more should create new stack
    assert!(storage.add_item(501, 1));

    // Should use 2 slots
    assert_eq!(storage.used_count(), 2);
}

#[test]
fn test_storage_merge_on_move() {
    let mut storage = Storage::new(100);

    // Manually set up two slots with the same item in different positions
    {
        let slot0 = storage.get_slot_mut(0).unwrap();
        slot0.item_id = 501;
        slot0.amount = 100;
        slot0.identified = true;
    }
    {
        let slot5 = storage.get_slot_mut(5).unwrap();
        slot5.item_id = 501;
        slot5.amount = 100;
        slot5.identified = true;
    }

    assert_eq!(storage.used_count(), 2);

    // Move slot 5 to slot 0 (should merge)
    let slot5_item = storage.get_slot(5).unwrap().item_id;
    assert_eq!(slot5_item, 501);

    assert!(storage.move_item(5, 0));

    // Should now be one stack with 200
    let slot0 = storage.get_slot(0).unwrap();
    assert_eq!(slot0.amount, 200);
    assert_eq!(storage.used_count(), 1);

    // Slot 5 should now be empty
    let slot5 = storage.get_slot(5).unwrap();
    assert!(slot5.is_empty());
}

#[test]
fn test_storage_concurrent_access() {
    use std::thread;
    use parking_lot::RwLock;

    let manager = Arc::new(RwLock::new(StorageManager::new()));

    // Create storage with initial items
    {
        let mgr = manager.write();
        let storage = mgr.get_or_create(1, 100);
        storage.write().add_item(501, 50);
    }

    // Spawn multiple threads to access storage
    let mut handles = vec![];
    for _ in 0..10 {
        let mgr = manager.clone();
        let handle = thread::spawn(move || {
            let mgr = mgr.read();
            let storage = mgr.get(1).unwrap();
            let s = storage.read();
            assert!(s.get_slot(0).is_some());
            // Just reading, no modification
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_storage_db_persistence() {
    use deviruchi::storage::Database;

    // Create in-memory database
    let db = Arc::new(Database::open_memory().unwrap());

    // Initialize schema (storage table)
    db.execute(
        "CREATE TABLE IF NOT EXISTS storage (
            char_id INTEGER NOT NULL,
            slot_index INTEGER NOT NULL,
            item_id INTEGER NOT NULL,
            amount INTEGER NOT NULL,
            identified INTEGER NOT NULL,
            refine INTEGER NOT NULL,
            card0 INTEGER NOT NULL,
            card1 INTEGER NOT NULL,
            card2 INTEGER NOT NULL,
            card3 INTEGER NOT NULL,
            PRIMARY KEY (char_id, slot_index)
        )"
    ).expect("Failed to create storage table");

    // Create and populate storage
    let mut storage = Storage::new(100);
    storage.add_item(501, 10);
    storage.add_item(502, 20);
    storage = storage.with_char_id(1);

    // Save to database
    db.save_storage(&storage).expect("Failed to save storage");

    // Load from database
    let loaded = db.load_storage(1, 100).expect("Failed to load storage");

    assert_eq!(loaded.char_id(), 1);
    assert_eq!(loaded.used_count(), 2);

    // Verify items
    let slot0 = loaded.get_slot(0).unwrap();
    assert_eq!(slot0.item_id, 501);
    assert_eq!(slot0.amount, 10);

    let slot1 = loaded.get_slot(1).unwrap();
    assert_eq!(slot1.item_id, 502);
    assert_eq!(slot1.amount, 20);
}

#[test]
fn test_storage_packet_serialization() {
    use deviruchi::protocol::storage_packets::*;

    // Test ZCStorageItems serialization
    let items = vec![
        StorageItem { index: 0, item_id: 501, amount: 10, identified: true },
        StorageItem { index: 1, item_id: 502, amount: 5, identified: false },
    ];
    let packet = ZCStorageItems { count: 2, items };
    let data = packet.to_packet();

    // Should have: header (4 bytes) + count (2 bytes) + 2 items (each: index 2 + item_id 2 + amount 2 + identified 1 = 7 bytes)
    assert!(!data.is_empty());

    // Test CZReqStorageMoveItem parsing
    let move_data = vec![0x01, 0x00, 0x02, 0x00, 0x0A, 0x00, 0x01, 0x00]; // from=1, to=2, amount=10, is_to_storage=true
    let parsed = CZReqStorageMoveItem::from_packet(&move_data).unwrap();
    assert_eq!(parsed.from_index, 1);
    assert_eq!(parsed.to_index, 2);
    assert_eq!(parsed.amount, 10);
    assert!(parsed.is_to_storage);
}
