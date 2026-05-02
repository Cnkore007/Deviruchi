use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub game: GameConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub name: String,
    pub mode: String,
    pub standalone: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub login_port: u16,
    pub char_port: u16,
    pub map_port: u16,
    pub max_connections: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameConfig {
    pub max_players: usize,
    pub timeout_seconds: u64,
    pub death_drop_items: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                name: "Deviruchi".to_string(),
                mode: "all".to_string(),
                standalone: true,
            },
            database: DatabaseConfig {
                path: "deviruchi.db".to_string(),
            },
            network: NetworkConfig {
                login_port: 6900,
                char_port: 6000,
                map_port: 6121,
                max_connections: 10000,
            },
            game: GameConfig {
                max_players: 5000,
                timeout_seconds: 300,
                death_drop_items: false,
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            let config = Self::default();
            config.save(path)?;
            tracing::info!("配置文件不存在，已创建默认配置: {:?}", path);
            return Ok(config);
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取配置文件失败: {:?}", path))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {:?}", path))?;

        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_death_drop_items_is_false() {
        let config = Config::default();
        assert!(!config.game.death_drop_items);
    }

    #[test]
    fn test_game_config_with_death_drop_items() {
        let mut config = Config::default();
        config.game.death_drop_items = true;
        assert!(config.game.death_drop_items);
    }
}
