//! 扩展测试模块
//!
//! 测试更多功能和边界情况

use deviruchi::game::channel::*;
use deviruchi::game::char_logif::*;
use deviruchi::game::chrif::*;
use deviruchi::game::clif::*;
use deviruchi::game::ipban::*;
use deviruchi::game::loginlog::*;
use deviruchi::game::pc::*;

// ============================================================
// 频道系统扩展测试
// ============================================================

#[test]
fn test_channel_password_protection() {
    let manager = ChannelManager::new();
    let config = ChannelConfig {
        name: "密码频道".to_string(),
        channel_type: ChannelType::Private,
        password: Some("secret123".to_string()),
        ..Default::default()
    };

    let id = manager.create_channel(config, Some(1));
    let channel = manager.get_channel(id).unwrap();

    assert_eq!(channel.config.password, Some("secret123".to_string()));
}

#[test]
fn test_channel_type_filtering() {
    let manager = ChannelManager::new();

    // 创建不同类型的频道
    let public_config = ChannelConfig {
        name: "公共频道".to_string(),
        channel_type: ChannelType::Public,
        ..Default::default()
    };

    let private_config = ChannelConfig {
        name: "私人频道".to_string(),
        channel_type: ChannelType::Private,
        ..Default::default()
    };

    let guild_config = ChannelConfig {
        name: "公会频道".to_string(),
        channel_type: ChannelType::Guild,
        ..Default::default()
    };

    manager.create_channel(public_config, Some(1));
    manager.create_channel(private_config, Some(2));
    manager.create_channel(guild_config, Some(3));

    let public_channels = manager.get_public_channels();
    assert_eq!(public_channels.len(), 1);
    assert_eq!(public_channels[0].config.name, "公共频道");
}

#[test]
fn test_channel_message_history() {
    let manager = ChannelManager::new();
    let config = ChannelConfig::default();
    let id = manager.create_channel(config, Some(1));

    // 发送多条消息
    for i in 0..10 {
        manager.send_message(
            id,
            1,
            "玩家1".to_string(),
            format!("消息 {}", i),
            MessageType::Normal,
        );
    }

    let channel = manager.get_channel(id).unwrap();
    let messages = channel.get_messages(5);
    assert_eq!(messages.len(), 5);
    assert_eq!(messages[0].content, "消息 5");
}

// ============================================================
// 角色服务器接口扩展测试
// ============================================================

#[test]
fn test_char_server_multiple_connections() {
    let manager = CharServerManager::new();

    let id1 = manager.register_connection("127.0.0.1".to_string(), 6000);
    let id2 = manager.register_connection("127.0.0.2".to_string(), 6001);
    let id3 = manager.register_connection("127.0.0.3".to_string(), 6002);

    assert_eq!(manager.get_connections().len(), 3);

    manager.update_connection_status(id1, CharServerStatus::Authenticated);
    manager.update_connection_status(id2, CharServerStatus::Connected);

    assert_eq!(
        manager.get_connection_status(id1),
        Some(CharServerStatus::Authenticated)
    );
    assert_eq!(
        manager.get_connection_status(id2),
        Some(CharServerStatus::Connected)
    );
    assert_eq!(
        manager.get_connection_status(id3),
        Some(CharServerStatus::Connecting)
    );
}

#[test]
fn test_char_server_bulk_operations() {
    let manager = CharServerManager::new();

    // 批量上线角色
    for i in 0..100 {
        manager.char_online(i, i, 1);
    }

    assert_eq!(manager.online_char_count(), 100);

    // 批量下线角色
    for i in 0..50 {
        manager.char_offline(i);
    }

    assert_eq!(manager.online_char_count(), 50);
}

// ============================================================
// 客户端接口扩展测试
// ============================================================

#[test]
fn test_client_manager_bulk_connections() {
    let manager = ClientManager::new();

    let mut connection_ids = Vec::new();
    for i in 0..100 {
        let id = manager.add_connection(format!("192.168.1.{}", i), 5000 + i as u16);
        connection_ids.push(id);
    }

    assert_eq!(manager.get_connections().len(), 100);

    // 设置玩家信息
    for (i, &id) in connection_ids.iter().enumerate() {
        manager.set_player_info(id, i as u32 + 1000, i as u32 + 1);
    }

    assert_eq!(manager.online_player_count(), 0); // InGame 状态

    // 更新状态为 InGame
    for &id in &connection_ids {
        manager.update_status(id, ClientStatus::InGame);
    }

    assert_eq!(manager.online_player_count(), 100);
}

#[test]
fn test_client_broadcast() {
    let manager = ClientManager::new();

    // 添加多个在线玩家
    for i in 0..10 {
        let id = manager.add_connection(format!("192.168.1.{}", i), 5000 + i as u16);
        manager.set_player_info(id, i + 1000, i + 1);
        manager.update_status(id, ClientStatus::InGame);
    }

    // 广播消息
    manager.broadcast_message(vec![1, 2, 3, 4, 5]);

    let messages = manager.process_messages();
    assert_eq!(messages.len(), 10);
}

// ============================================================
// 玩家角色扩展测试
// ============================================================

#[test]
fn test_player_stat_calculations() {
    let mut player = PlayerCharacter::new(1, 1, "测试角色".to_string(), 0);

    // 增加属性点
    player.stat_points = 100;
    player.add_stat_point(StatType::Str, 10);
    player.add_stat_point(StatType::Agi, 10);
    player.add_stat_point(StatType::Vit, 10);
    player.add_stat_point(StatType::Int, 10);
    player.add_stat_point(StatType::Dex, 10);
    player.add_stat_point(StatType::Luk, 10);

    assert_eq!(player.stats.str, 11);
    assert_eq!(player.stats.agi, 11);
    assert_eq!(player.stats.vit, 11);
    assert_eq!(player.stats.int, 11);
    assert_eq!(player.stats.dex, 11);
    assert_eq!(player.stats.luk, 11);
    assert_eq!(player.stat_points, 40);

    // 验证属性计算
    assert!(player.stats.max_hp > 100);
    assert!(player.stats.max_sp > 50);
    assert!(player.stats.attack > 10);
    assert!(player.stats.hit > 100);
}

#[test]
fn test_player_combat_operations() {
    let mut player = PlayerCharacter::new(1, 1, "战斗角色".to_string(), 0);

    // 测试 HP/SP 操作
    player.stats.current_hp = 50;
    player.stats.current_sp = 25;

    player.heal_hp(30);
    assert_eq!(player.stats.current_hp, 80);

    player.heal_sp(15);
    assert_eq!(player.stats.current_sp, 40);

    assert!(player.consume_hp(20));
    assert_eq!(player.stats.current_hp, 60);

    assert!(!player.consume_hp(100));
    assert_eq!(player.stats.current_hp, 60);
}

#[test]
fn test_player_zeny_operations() {
    let mut player = PlayerCharacter::new(1, 1, "富翁".to_string(), 0);

    player.add_zeny(10000);
    assert_eq!(player.zeny, 10000);

    player.add_zeny(5000);
    assert_eq!(player.zeny, 15000);

    assert!(player.consume_zeny(8000));
    assert_eq!(player.zeny, 7000);

    assert!(!player.consume_zeny(10000));
    assert_eq!(player.zeny, 7000);
}

#[test]
fn test_player_status_effects() {
    let mut player = PlayerCharacter::new(1, 1, "状态角色".to_string(), 0);

    let effect1 = StatusEffect {
        id: 1,
        name: "中毒".to_string(),
        duration: 60,
        start_time: 0,
        params: std::collections::HashMap::new(),
    };

    let effect2 = StatusEffect {
        id: 2,
        name: "加速".to_string(),
        duration: 120,
        start_time: 0,
        params: std::collections::HashMap::new(),
    };

    player.add_status_effect(effect1);
    player.add_status_effect(effect2);

    assert!(player.has_status_effect(1));
    assert!(player.has_status_effect(2));
    assert!(!player.has_status_effect(3));

    player.remove_status_effect(1);
    assert!(!player.has_status_effect(1));
    assert!(player.has_status_effect(2));
}

#[test]
fn test_player_teleport() {
    let mut player = PlayerCharacter::new(1, 1, "传送角色".to_string(), 0);

    player.teleport(100, 150, 200);
    assert_eq!(player.map_id, 100);
    assert_eq!(player.x, 150);
    assert_eq!(player.y, 200);

    player.teleport(200, 300, 400);
    assert_eq!(player.map_id, 200);
    assert_eq!(player.x, 300);
    assert_eq!(player.y, 400);
}

// ============================================================
// IP 封禁扩展测试
// ============================================================

#[test]
fn test_ip_ban_expiry() {
    let manager = IpBanManager::new(5, 300);

    // 封禁 1 秒
    manager.ban_ip(
        "192.168.1.1".to_string(),
        BanReason::BruteForce,
        Some(1),
        "admin".to_string(),
    );

    assert!(manager.is_banned("192.168.1.1"));

    // 等待过期
    std::thread::sleep(std::time::Duration::from_millis(1100));

    assert!(!manager.is_banned("192.168.1.1"));
}

#[test]
fn test_ip_ban_permanent() {
    let manager = IpBanManager::new(5, 300);

    // 永久封禁
    manager.ban_ip(
        "192.168.1.1".to_string(),
        BanReason::AdminBan,
        None,
        "admin".to_string(),
    );

    assert!(manager.is_banned("192.168.1.1"));

    // 等待一段时间
    std::thread::sleep(std::time::Duration::from_millis(100));

    assert!(manager.is_banned("192.168.1.1"));
}

#[test]
fn test_ip_ban_multiple_reasons() {
    let manager = IpBanManager::new(5, 300);

    manager.ban_ip(
        "192.168.1.1".to_string(),
        BanReason::BruteForce,
        Some(3600),
        "system".to_string(),
    );

    manager.ban_ip(
        "192.168.1.2".to_string(),
        BanReason::MaliciousBehavior,
        Some(7200),
        "admin".to_string(),
    );

    manager.ban_ip(
        "192.168.1.3".to_string(),
        BanReason::Violation,
        None,
        "admin".to_string(),
    );

    assert!(manager.is_banned("192.168.1.1"));
    assert!(manager.is_banned("192.168.1.2"));
    assert!(manager.is_banned("192.168.1.3"));

    let bans = manager.get_all_bans();
    assert_eq!(bans.len(), 3);
}

// ============================================================
// 登录日志扩展测试
// ============================================================

#[test]
fn test_login_log_event_types() {
    let manager = LoginLogManager::new(100);

    manager.log_event(
        1,
        "user1".to_string(),
        "192.168.1.1".to_string(),
        LoginEvent::LoginSuccess,
        "成功".to_string(),
    );
    manager.log_event(
        2,
        "user2".to_string(),
        "192.168.1.2".to_string(),
        LoginEvent::LoginFailed,
        "密码错误".to_string(),
    );
    manager.log_event(
        3,
        "user3".to_string(),
        "192.168.1.3".to_string(),
        LoginEvent::Banned,
        "账号封禁".to_string(),
    );

    let logs = manager.get_recent_logs(10);
    assert_eq!(logs.len(), 3);
    assert_eq!(logs[0].event, LoginEvent::LoginSuccess);
    assert_eq!(logs[1].event, LoginEvent::LoginFailed);
    assert_eq!(logs[2].event, LoginEvent::Banned);
}

#[test]
fn test_login_log_clear() {
    let manager = LoginLogManager::new(100);

    for i in 0..50 {
        manager.log_event(
            i,
            format!("user_{}", i),
            format!("192.168.1.{}", i),
            LoginEvent::LoginSuccess,
            String::new(),
        );
    }

    assert_eq!(manager.count(), 50);

    manager.clear();
    assert_eq!(manager.count(), 0);
}

// ============================================================
// 角色日志扩展测试
// ============================================================

#[test]
fn test_char_log_all_events() {
    let manager = CharLogManager::new(100);

    manager.log_event(
        1001,
        1,
        "角色1".to_string(),
        CharLogEvent::CharCreate,
        "创建".to_string(),
    );
    manager.log_event(
        1001,
        1,
        "角色1".to_string(),
        CharLogEvent::CharOnline,
        "上线".to_string(),
    );
    manager.log_event(
        1001,
        1,
        "角色1".to_string(),
        CharLogEvent::CharOffline,
        "下线".to_string(),
    );
    manager.log_event(
        1001,
        1,
        "角色1".to_string(),
        CharLogEvent::CharDelete,
        "删除".to_string(),
    );

    let logs = manager.get_char_logs(1001);
    assert_eq!(logs.len(), 4);
    assert_eq!(logs[0].event, CharLogEvent::CharCreate);
    assert_eq!(logs[1].event, CharLogEvent::CharOnline);
    assert_eq!(logs[2].event, CharLogEvent::CharOffline);
    assert_eq!(logs[3].event, CharLogEvent::CharDelete);
}

#[test]
fn test_char_log_rename() {
    let manager = CharLogManager::new(100);

    manager.log_event(
        1001,
        1,
        "旧名字".to_string(),
        CharLogEvent::CharRename,
        "改名为新名字".to_string(),
    );

    let logs = manager.get_char_logs(1001);
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].event, CharLogEvent::CharRename);
    assert_eq!(logs[0].message, "改名为新名字");
}
