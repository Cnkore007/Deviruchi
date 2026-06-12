//! 配置管理系统
//!
//! 支持:
//! - TOML 配置文件
//! - 热重载 (配置文件变更自动重新加载)
//! - 默认值自动创建
//! - 分层配置 (默认 < 文件 < 命令行)

use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

/// 服务器配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub game: GameConfig,
    pub battle: BattleConfig,
    pub drop: DropConfig,
    pub exp: ExpConfig,
    pub respawn: RespawnConfig,
    pub logging: LoggingConfig,
    pub skill: SkillConfig,
    pub party: PartyConfig,
    pub storage: StorageConfig,
    pub chat: ChatConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub mode: ServerMode,
    pub standalone: bool,
    pub pid_file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ServerMode {
    #[default]
    All,
    Login,
    Char,
    Map,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// 后端类型: "sqlite" 或 "mysql"
    pub backend: String,
    pub path: String,
    pub backup_path: Option<String>,
    pub auto_vacuum: bool,
    pub wal_mode: bool,
    pub busy_timeout_ms: u32,
    pub auto_backup_interval_hours: u32,
    /// MySQL 主机地址
    #[cfg(feature = "mysql-backend")]
    pub mysql_host: String,
    /// MySQL 端口
    #[cfg(feature = "mysql-backend")]
    pub mysql_port: u16,
    /// MySQL 用户名
    #[cfg(feature = "mysql-backend")]
    pub mysql_user: String,
    /// MySQL 密码
    #[cfg(feature = "mysql-backend")]
    pub mysql_password: String,
    /// MySQL 数据库名称
    #[cfg(feature = "mysql-backend")]
    pub mysql_database: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".to_string(),
            path: "deviruchi.db".to_string(),
            backup_path: None,
            auto_vacuum: true,
            wal_mode: true,
            busy_timeout_ms: 5000,
            auto_backup_interval_hours: 24,
            #[cfg(feature = "mysql-backend")]
            mysql_host: "127.0.0.1".to_string(),
            #[cfg(feature = "mysql-backend")]
            mysql_port: 3306,
            #[cfg(feature = "mysql-backend")]
            mysql_user: "deviruchi".to_string(),
            #[cfg(feature = "mysql-backend")]
            mysql_password: String::new(),
            #[cfg(feature = "mysql-backend")]
            mysql_database: "deviruchi".to_string(),
        }
    }
}

impl DatabaseConfig {
    /// 获取 MySQL 密码，优先使用环境变量 DEVIRUCHI_MYSQL_PASSWORD
    /// 环境变量优先级高于配置文件，避免明文密码存储在磁盘上
    #[cfg(feature = "mysql-backend")]
    pub fn resolved_password(&self) -> &str {
        // 环境变量覆盖配置文件中的密码
        // 使用 leak() 避免生命周期问题，仅在服务器启动时调用一次
        if let Ok(env_pw) = std::env::var("DEVIRUCHI_MYSQL_PASSWORD") {
            if !env_pw.is_empty() {
                return Box::leak(env_pw.into_boxed_str());
            }
        }
        &self.mysql_password
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub login_port: u16,
    pub char_port: u16,
    pub map_port: u16,
    pub max_connections: usize,
    pub tcp_nodelay: bool,
    pub keepalive: bool,
    pub read_buffer_size: usize,
    pub write_buffer_size: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            login_port: 6900,
            char_port: 6000,
            map_port: 6121,
            max_connections: 10000,
            tcp_nodelay: true,
            keepalive: true,
            read_buffer_size: 8192,
            write_buffer_size: 8192,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameConfig {
    pub max_players: usize,
    pub timeout_seconds: u64,
    pub death_drop_items: bool,
    pub max_level: u16,
    pub base_level_cap: u16,
    pub job_level_cap: u16,
    pub player_name_length_min: u8,
    pub player_name_length_max: u8,
    pub guild_name_length_min: u8,
    pub guild_name_length_max: u8,
    pub autosave_interval_seconds: u64,
    pub autosave_enabled: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            max_players: 5000,
            timeout_seconds: 300,
            death_drop_items: false,
            max_level: 99,
            base_level_cap: 99,
            job_level_cap: 50,
            player_name_length_min: 4,
            player_name_length_max: 24,
            guild_name_length_min: 4,
            guild_name_length_max: 24,
            autosave_interval_seconds: 60,
            autosave_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BattleConfig {
    pub base_exp_rate: f64,
    pub job_exp_rate: f64,
    pub zeny_rate: f64,
    pub item_drop_rate: f64,
    pub pvp_mode: bool,
    pub pvp_damage_rate: f64,
    pub gvg_mode: bool,
    pub gvg_damage_rate: f64,
    pub atcommand_give_level: u16,
    pub max_hp_base_cap: u32,
    pub max_sp_base_cap: u32,
    pub natural_heal_hp_rate: u32,
    pub natural_heal_sp_rate: u32,
    pub sit_heal_hp_rate: u32,
    pub sit_heal_sp_rate: u32,
    pub natural_heal_interval_ms: u64,
    pub natural_heal_threshold_hp: u32,
    pub natural_heal_threshold_sp: u32,
    pub battle_heal_penalty: bool,     // 战斗惩罚开关
    pub overweight_heal_penalty: bool, // 超重惩罚开关
    pub status_heal_modifier: bool,    // 状态效果修饰开关
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self {
            base_exp_rate: 1.0,
            job_exp_rate: 1.0,
            zeny_rate: 1.0,
            item_drop_rate: 1.0,
            pvp_mode: false,
            pvp_damage_rate: 1.0,
            gvg_mode: false,
            gvg_damage_rate: 1.0,
            atcommand_give_level: 99,
            max_hp_base_cap: 999999,
            max_sp_base_cap: 99999,
            natural_heal_hp_rate: 3,
            natural_heal_sp_rate: 3,
            sit_heal_hp_rate: 10,
            sit_heal_sp_rate: 10,
            natural_heal_interval_ms: 6000,
            natural_heal_threshold_hp: 50,
            natural_heal_threshold_sp: 50,
            battle_heal_penalty: true,
            overweight_heal_penalty: true,
            status_heal_modifier: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DropConfig {
    pub mvp_bonus_multiplier: f64,
    pub zeny_drop_rate: u32,
    pub zeny_drop_percent: u32,
    pub pickup_range: u16,
    pub drop_item_min_amount: u16,
    pub drop_item_max_amount: u16,
    pub drop_zeny_min: u32,
    pub drop_zeny_max: u32,
    pub drop_item_expire_seconds: u64,
    pub drop_zeny_expire_seconds: u64,
}

impl Default for DropConfig {
    fn default() -> Self {
        Self {
            mvp_bonus_multiplier: 1.1,
            zeny_drop_rate: 5000,
            zeny_drop_percent: 50,
            pickup_range: 2,
            drop_item_min_amount: 1,
            drop_item_max_amount: 10000,
            drop_zeny_min: 1,
            drop_zeny_max: 10000,
            drop_item_expire_seconds: 300,
            drop_zeny_expire_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpConfig {
    pub party_share_mode: String,
    pub level_penalty_diff_10: f64,
    pub level_penalty_diff_15: f64,
    pub level_penalty_diff_20: f64,
    pub level_penalty_diff_25: f64,
    pub level_penalty_diff_above: f64,
    pub party_share_near_range: u16,
    pub mvp_exp_bonus: f64,
}

impl Default for ExpConfig {
    fn default() -> Self {
        Self {
            party_share_mode: "equal".to_string(),
            level_penalty_diff_10: 1.0,
            level_penalty_diff_15: 0.75,
            level_penalty_diff_20: 0.5,
            level_penalty_diff_25: 0.25,
            level_penalty_diff_above: 0.1,
            party_share_near_range: 12,
            mvp_exp_bonus: 1.1,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RespawnConfig {
    pub normal_respawn_delay_ms: u64,
    pub instant_call_delay_ms: u64,
    pub default_map: String,
    pub default_x: u16,
    pub default_y: u16,
    pub auto_respawn_enabled: bool,
    pub auto_respawn_delay_ms: u64,
}

impl Default for RespawnConfig {
    fn default() -> Self {
        Self {
            normal_respawn_delay_ms: 5000,
            instant_call_delay_ms: 1000,
            default_map: "prontera".to_string(),
            default_x: 157,
            default_y: 183,
            auto_respawn_enabled: true,
            auto_respawn_delay_ms: 10000,
        }
    }
}

/// 日志配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub console: bool,
    pub level: String,
    pub log_dir: String,
    pub rotation_hourly: bool,
    pub timestamp: bool,
    pub timestamp_format: String,
    pub categories: LogCategoryConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogCategoryConfig {
    pub log_pick: bool,
    pub log_trade: bool,
    pub log_shop: bool,
    pub log_storage: bool,
    pub log_chat: bool,
    pub log_battle: bool,
    pub log_gm_command: bool,
    pub log_npc: bool,
    pub log_zeny: bool,
    pub log_login: bool,
    pub log_char: bool,
    pub log_map: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            console: true,
            level: "info".to_string(),
            log_dir: "logs".to_string(),
            rotation_hourly: true,
            timestamp: true,
            timestamp_format: "%Y-%m-%d %H:%M:%S".to_string(),
            categories: LogCategoryConfig {
                log_pick: true,
                log_trade: true,
                log_shop: true,
                log_storage: true,
                log_chat: true,
                log_battle: true,
                log_gm_command: true,
                log_npc: true,
                log_zeny: true,
                log_login: true,
                log_char: true,
                log_map: true,
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillConfig {
    pub default_cast_time_ms: u64,
    pub default_cooldown_ms: u64,
    pub aspd_base: u16,
    pub aspd_max: u16,
    pub aspd_interval_base_ms: u32,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            default_cast_time_ms: 1000,
            default_cooldown_ms: 1000,
            aspd_base: 2000,
            aspd_max: 190,
            aspd_interval_base_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartyConfig {
    pub max_party_members: u8,
    pub max_party_name_length: u8,
    pub party_exp_share_near_range: u16,
    pub auto_accept_invite: bool,
    pub auto_leave_on_disconnect: bool,
}

impl Default for PartyConfig {
    fn default() -> Self {
        Self {
            max_party_members: 12,
            max_party_name_length: 24,
            party_exp_share_near_range: 12,
            auto_accept_invite: false,
            auto_leave_on_disconnect: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub max_storage_size: u16,
    pub max_guild_storage_size: u16,
    pub premium_storage_enabled: bool,
    pub premium_storage_max_slots: u16,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_storage_size: 300,
            max_guild_storage_size: 1000,
            premium_storage_enabled: false,
            premium_storage_max_slots: 600,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatConfig {
    pub global_chat_enabled: bool,
    pub whisper_enabled: bool,
    pub party_chat_enabled: bool,
    pub guild_chat_enabled: bool,
    pub map_chat_enabled: bool,
    pub auto_channel_join_enabled: bool,
    pub whisper_cooldown_ms: u64,
    pub chat_flood_threshold_per_min: u32,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            global_chat_enabled: true,
            whisper_enabled: true,
            party_chat_enabled: true,
            guild_chat_enabled: true,
            map_chat_enabled: true,
            auto_channel_join_enabled: true,
            whisper_cooldown_ms: 1000,
            chat_flood_threshold_per_min: 10,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                name: "Deviruchi".to_string(),
                version: "0.1.0".to_string(),
                mode: ServerMode::All,
                standalone: true,
                pid_file: None,
            },
            database: DatabaseConfig::default(),
            network: NetworkConfig::default(),
            game: GameConfig::default(),
            battle: BattleConfig::default(),
            drop: DropConfig::default(),
            exp: ExpConfig::default(),
            respawn: RespawnConfig::default(),
            logging: LoggingConfig::default(),
            skill: SkillConfig::default(),
            party: PartyConfig::default(),
            storage: StorageConfig::default(),
            chat: ChatConfig::default(),
        }
    }
}

impl Config {
    /// 加载配置文件
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

        let mut config: Config =
            toml::from_str(&content).with_context(|| format!("解析配置文件失败: {:?}", path))?;

        // 环境变量覆盖敏感配置，避免明文密码存储在配置文件中
        #[cfg(feature = "mysql-backend")]
        {
            if let Ok(host) = std::env::var("DEVIRUCHI_MYSQL_HOST") {
                config.database.mysql_host = host;
            }
            if let Ok(port) = std::env::var("DEVIRUCHI_MYSQL_PORT") {
                if let Ok(p) = port.parse() { config.database.mysql_port = p; }
            }
            if let Ok(user) = std::env::var("DEVIRUCHI_MYSQL_USER") {
                config.database.mysql_user = user;
            }
            if let Ok(pw) = std::env::var("DEVIRUCHI_MYSQL_PASSWORD") {
                config.database.mysql_password = pw;
            }
        }

        Ok(config)
    }

    /// 保存配置文件
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 获取配置路径
    pub fn get_default_path() -> PathBuf {
        PathBuf::from("config/server.toml")
    }

    /// 加载或获取默认配置
    pub fn load_or_default() -> Result<Self> {
        Self::load(Self::get_default_path())
    }

    /// 检查配置文件是否存在
    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    /// 运行交互式配置向导
    ///
    /// 引导用户配置服务器基本参数，生成 config/server.toml。
    /// 所有选项可直接回车使用默认值。
    pub fn run_setup_wizard() -> Result<Self> {
        use std::io::{self, BufRead, Write};

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        println!();
        println!("============================================");
        println!("  Deviruchi 服务器配置向导");
        println!("============================================");
        println!("检测到首次运行，将引导您完成基础配置。");
        println!("所有选项可直接回车使用默认值。");
        println!();

        let mut config = Config::default();

        // 辅助函数：读取用户输入，空输入返回默认值
        let mut read_input = |prompt: &str, default: &str| -> String {
            print!("{} [{}]: ", prompt, default);
            let _ = stdout.flush();
            let mut input = String::new();
            let _ = stdin.lock().read_line(&mut input);
            let input = input.trim().to_string();
            if input.is_empty() { default.to_string() } else { input }
        };

        // 1. 服务器名称
        let name = read_input("服务器名称", "Deviruchi");
        config.server.name = name;

        // 2. 基础经验倍率
        let base_exp = read_input("基础经验倍率 (1.0 = 100%)", "1.0");
        config.battle.base_exp_rate = base_exp.parse().unwrap_or(1.0);

        // 3. 职业经验倍率
        let job_exp = read_input("职业经验倍率 (1.0 = 100%)", "1.0");
        config.battle.job_exp_rate = job_exp.parse().unwrap_or(1.0);

        // 4. 物品掉落倍率
        let drop_rate = read_input("物品掉落倍率 (1.0 = 100%)", "1.0");
        config.battle.item_drop_rate = drop_rate.parse().unwrap_or(1.0);

        // 5. Zeny 掉落倍率
        let zeny_rate = read_input("Zeny 掉落倍率 (1.0 = 100%)", "1.0");
        config.battle.zeny_rate = zeny_rate.parse().unwrap_or(1.0);

        // 6. 最大基础等级
        let base_lv = read_input("最大基础等级", "99");
        config.game.base_level_cap = base_lv.parse().unwrap_or(99);

        // 7. 最大职业等级
        let job_lv = read_input("最大职业等级", "50");
        config.game.job_level_cap = job_lv.parse().unwrap_or(50);

        // 8. 死亡经验惩罚
        let death_penalty = read_input("死亡时损失经验？(y/N)", "n");
        config.game.death_drop_items = death_penalty.to_lowercase() == "y";

        // 9. 默认出生地图
        let default_map = read_input("默认出生地图", "prontera");
        config.respawn.default_map = default_map;

        // 10. 最大玩家数
        let max_players = read_input("最大玩家数", "5000");
        config.game.max_players = max_players.parse().unwrap_or(5000);

        // 确保 config 目录存在
        let config_path = Self::get_default_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 保存配置
        config.save(&config_path)?;

        println!();
        println!("============================================");
        println!("配置已保存到 {:?}", config_path);
        println!("============================================");
        println!();

        Ok(config)
    }
}

/// 热重载配置管理器
pub struct HotReloadConfig {
    config: Arc<RwLock<Config>>,
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    receiver: Option<Receiver<Result<Event, notify::Error>>>,
}

impl HotReloadConfig {
    /// 创建新的热重载配置管理器
    pub fn new(config: Config, path: impl Into<PathBuf>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            path: path.into(),
            watcher: None,
            receiver: None,
        }
    }

    /// 启动热重载监视
    pub fn start_watching(&mut self) -> anyhow::Result<()> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = tx.send(res);
            },
            NotifyConfig::default(),
        )?;

        watcher.watch(&self.path, RecursiveMode::NonRecursive)?;
        self.watcher = Some(watcher);
        self.receiver = Some(rx);

        tracing::info!("热重载配置已启用: {:?}", self.path);
        Ok(())
    }

    /// 检查配置是否需要重载
    pub fn check_reload(&self) -> bool {
        if let Some(rx) = &self.receiver {
            while let Ok(result) = rx.try_recv() {
                if let Ok(event) = result
                    && event.kind.is_modify()
                {
                    return true;
                }
            }
        }
        false
    }

    /// 执行热重载
    pub fn reload(&self) -> anyhow::Result<()> {
        tracing::info!("正在重载配置: {:?}", self.path);

        let new_config = Config::load(&self.path)?;
        *self.config.write() = new_config;

        tracing::info!("配置重载完成");
        Ok(())
    }

    /// 获取当前配置
    pub fn get(&self) -> Config {
        self.config.read().clone()
    }

    /// 获取配置引用
    pub fn get_ref(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.name, "Deviruchi");
        assert_eq!(config.network.login_port, 6900);
        assert_eq!(config.battle.base_exp_rate, 1.0);
        assert_eq!(config.drop.pickup_range, 2);
        assert_eq!(config.exp.level_penalty_diff_10, 1.0);
    }

    #[test]
    fn test_save_load_config() {
        let config = Config::default();
        let path = std::env::temp_dir().join("test_config.toml");

        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.server.name, config.server.name);
        assert_eq!(loaded.network.map_port, config.network.map_port);

        // 清理
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_battle_config_rates() {
        let battle = BattleConfig::default();
        assert_eq!(battle.base_exp_rate, 1.0);
        assert_eq!(battle.item_drop_rate, 1.0);
        assert!(!battle.pvp_mode);
    }

    #[test]
    fn test_drop_config() {
        let drop = DropConfig::default();
        assert_eq!(drop.mvp_bonus_multiplier, 1.1);
        assert_eq!(drop.zeny_drop_rate, 5000);
        assert_eq!(drop.drop_item_expire_seconds, 300);
    }

    #[test]
    fn test_exp_config_penalties() {
        let exp = ExpConfig::default();
        assert_eq!(exp.level_penalty_diff_10, 1.0);
        assert_eq!(exp.level_penalty_diff_15, 0.75);
        assert_eq!(exp.level_penalty_diff_20, 0.5);
        assert_eq!(exp.level_penalty_diff_25, 0.25);
        assert_eq!(exp.level_penalty_diff_above, 0.1);
    }

    #[test]
    fn test_server_mode() {
        assert_eq!(ServerMode::default(), ServerMode::All);
    }
}
