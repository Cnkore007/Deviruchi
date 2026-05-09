use deviruchi::game::{
    item::{Inventory, ItemDatabase, ItemType},
    map::{CellType, MapData},
    mob::Mob,
    skill::{SkillDatabase, SkillType},
};

#[test]
fn test_skill_database() {
    let db = SkillDatabase::new();

    let bash = db.get(1).unwrap();
    assert_eq!(bash.name, "Bash");
    assert_eq!(bash.type_, SkillType::Attack);
    assert_eq!(bash.sp_cost, 8);
}

#[test]
fn test_item_database() {
    let db = ItemDatabase::new();

    let potion = db.get(501).unwrap();
    assert_eq!(potion.name, "Red Potion");
    assert_eq!(potion.type_, ItemType::Heal);
    assert_eq!(potion.hp_restore, 120);
}

#[test]
fn test_inventory_add_remove() {
    let db = std::sync::Arc::new(ItemDatabase::new());
    let mut inv = Inventory::new(100, db);

    assert!(inv.add_item(501, 10));
    assert_eq!(inv.slots()[0].item_id, 501);
    assert_eq!(inv.slots()[0].amount, 10);

    assert!(inv.remove_item(0, 5));
    assert_eq!(inv.slots()[0].amount, 5);
}

#[test]
fn test_mob_creation() {
    let mob = Mob::from_template(1001, 100, 100, "test.gat");

    assert_eq!(mob.mob_id, 1001);
    // mob_db.yml 中 ID 1001 可能对应不同怪物，只验证 ID 和基本属性
    assert!(!mob.name.is_empty());
    assert!(mob.max_hp > 0);
    assert!(!mob.is_dead());
}

#[test]
fn test_mob_take_damage() {
    // Mob::new(mob_id, x, y, map) defaults hp/max_hp to 100
    let mob = Mob::new(1001, 50, 50, "test.gat");
    assert_eq!(*mob.hp.read(), 100);

    let dead = mob.take_damage(50);
    assert!(!dead);
    assert_eq!(*mob.hp.read(), 50);

    let dead = mob.take_damage(100);
    assert!(dead);
    assert_eq!(*mob.hp.read(), 0);
}

#[test]
fn test_map_data() {
    let mut map = MapData::new("test.gat", 100, 100);

    assert!(map.is_walkable(50, 50));

    map.set_cell(10, 10, CellType::Wall);
    assert!(!map.is_walkable(10, 10));
}

#[test]
fn test_map_out_of_bounds() {
    let map = MapData::new("test.gat", 100, 100);

    assert!(!map.is_walkable(150, 50)); // x 越界
    assert!(!map.is_walkable(50, 150)); // y 越界
}
