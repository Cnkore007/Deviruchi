use deviruchi::core::config::Config;

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.server.name, "Deviruchi");
    assert_eq!(config.network.login_port, 6900);
}

#[test]
fn test_config_save_load() {
    let config = Config::default();
    let path = "/tmp/test_deviruchi_config.toml";

    config.save(path).unwrap();
    let loaded = Config::load(path).unwrap();

    assert_eq!(config.server.name, loaded.server.name);
    assert_eq!(config.network.login_port, loaded.network.login_port);
}
