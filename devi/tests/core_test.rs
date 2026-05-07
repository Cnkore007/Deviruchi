// 客户端配置系统集成测试
// 验证 ClientConfig 的默认值和 YAML 反序列化功能

use devi::core::config::ClientConfig;

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
