// 核心模块集成测试
// 覆盖配置系统、游戏状态机、Tick 配置三大核心模块

use devi::core::config::ClientConfig;
use devi::core::state::GameState;
use devi::core::tick::TickConfig;

// ===== 配置系统测试 =====

/// 测试默认配置值是否正确
#[test]
fn test_default_config() {
    let config = ClientConfig::default();
    assert_eq!(config.window_width, 1024);
    assert_eq!(config.window_height, 768);
    assert_eq!(config.server_address, "127.0.0.1");
    assert_eq!(config.protocol, "modern");
}

/// 测试从 YAML 字符串反序列化配置
#[test]
fn test_config_from_yaml() {
    let yaml = r#"
window_width: 1920
window_height: 1080
server_address: "192.168.1.100"
protocol: "legacy"
"#;
    let config: ClientConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.window_width, 1920);
    assert_eq!(config.window_height, 1080);
    assert_eq!(config.server_address, "192.168.1.100");
    assert_eq!(config.protocol, "legacy");
}

// ===== 状态机测试 =====

/// 测试初始状态默认为 Login
#[test]
fn test_initial_state_is_login() {
    let state = GameState::default();
    assert_eq!(state, GameState::Login);
}

/// 测试状态之间的流转是否正确
#[test]
fn test_state_transitions() {
    let mut state = GameState::default();
    assert_eq!(state, GameState::Login);

    state = GameState::CharSelect;
    assert_eq!(state, GameState::CharSelect);

    state = GameState::InGame;
    assert_eq!(state, GameState::InGame);

    state = GameState::Login;
    assert_eq!(state, GameState::Login);
}

// ===== Tick 配置测试 =====

/// 测试默认 tick 配置：20ms/tick = 50Hz
#[test]
fn test_tick_config_default() {
    let config = TickConfig::default();
    assert_eq!(config.tick_rate_ms, 20);
    assert!((config.tick_rate_hz - 50.0).abs() < f64::EPSILON);
}

/// 测试自定义 tick 配置：16ms/tick ≈ 62.5Hz
#[test]
fn test_tick_config_custom() {
    let config = TickConfig::new(16);
    assert_eq!(config.tick_rate_ms, 16);
    assert!((config.tick_rate_hz - 62.5).abs() < f64::EPSILON);
}
