// 游戏模块单元测试
// 测试玩家、怪物实体组件和移动系统

use devi::game::mob::Mob;
use devi::game::movement::Movement;
use devi::game::player::Player;

/// 测试玩家创建
/// 验证默认属性值是否正确
#[test]
fn test_player_creation() {
    let player = Player::new(1, "TestPlayer".to_string());
    assert_eq!(player.entity_id, 1);
    assert_eq!(player.name, "TestPlayer");
    assert_eq!(player.base_level, 1);
}

/// 测试怪物创建
/// 验证怪物 ID、名称和默认属性
#[test]
fn test_mob_creation() {
    let mob = Mob::new(100, 1001, "Poring".to_string());
    assert_eq!(mob.entity_id, 100);
    assert_eq!(mob.mob_id, 1001);
    assert_eq!(mob.name, "Poring");
}

/// 测试移动路径设置
/// 验证移动状态和目标位置的设置
#[test]
fn test_movement_pathfinding() {
    let mut movement = Movement::new(10.0);
    assert!(!movement.is_moving());
    movement.set_destination(5.0, 5.0);
    assert!(movement.is_moving());
    assert_eq!(movement.destination, Some((5.0, 5.0)));
}
