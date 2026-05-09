//! 日志系统 - 分类日志、文件输出、日志级别
//!
//! 支持多种日志分类:
//! - 交易日志 (pick, trade, shop)
//! - 聊天日志 (global, whisper, party, guild)
//! - 战斗日志 (damage, mob_death)
//! - 系统日志 (login, char, map, sql)
//! - GM 命令日志

use parking_lot::RwLock;
use std::sync::OnceLock;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// 日志分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogCategory {
    /// 物品拾取/掉落日志
    Pick,
    /// 交易日志
    Trade,
    /// 商店交易日志
    Shop,
    /// 仓库日志
    Storage,
    /// 聊天日志
    Chat,
    /// 战斗日志
    Battle,
    /// 登录服务器日志
    Login,
    /// 角色服务器日志
    Char,
    /// 地图服务器日志
    Map,
    /// SQL 数据库日志
    Sql,
    /// GM 命令日志
    GmCommand,
    /// NPC 脚本日志
    Npc,
    /// Zeny 交易日志
    Zeny,
    /// 现金交易日志
    Cash,
    /// 邮件日志
    Mail,
    /// MVP 掉落日志
    MvpDrop,
    /// 宠物/佣兵日志
    Pet,
    /// 错误日志
    Error,
    /// 警告日志
    Warning,
    /// 调试日志
    Debug,
    /// 普通信息
    Info,
}

impl LogCategory {
    /// 获取日志分类对应的目录名
    pub fn dir_name(&self) -> &'static str {
        match self {
            LogCategory::Pick => "pick",
            LogCategory::Trade => "trade",
            LogCategory::Shop => "shop",
            LogCategory::Storage => "storage",
            LogCategory::Chat => "chat",
            LogCategory::Battle => "battle",
            LogCategory::Login => "login",
            LogCategory::Char => "char",
            LogCategory::Map => "map",
            LogCategory::Sql => "sql",
            LogCategory::GmCommand => "gm",
            LogCategory::Npc => "npc",
            LogCategory::Zeny => "zeny",
            LogCategory::Cash => "cash",
            LogCategory::Mail => "mail",
            LogCategory::MvpDrop => "mvp",
            LogCategory::Pet => "pet",
            LogCategory::Error => "error",
            LogCategory::Warning => "warning",
            LogCategory::Debug => "debug",
            LogCategory::Info => "info",
        }
    }

    /// 获取日志分类对应的文件名
    pub fn file_name(&self) -> &'static str {
        match self {
            LogCategory::Chat | LogCategory::GmCommand | LogCategory::Npc => "messages.log",
            _ => "log.txt",
        }
    }
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warning = 1,
    Info = 2,
    Debug = 3,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" | "warning" => LogLevel::Warning,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            _ => LogLevel::Info,
        }
    }
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 日志根目录
    pub log_dir: String,
    /// 是否启用控制台输出
    pub console: bool,
    /// 默认日志级别
    pub default_level: LogLevel,
    /// 各类别日志级别 (None 表示使用默认级别)
    pub category_levels: std::collections::HashMap<LogCategory, LogLevel>,
    /// 是否启用所有日志
    pub enabled: bool,
    /// 每小时轮转文件
    pub rotation_hourly: bool,
    /// 是否添加时间戳
    pub timestamp: bool,
    /// 时间戳格式
    pub timestamp_format: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_dir: "logs".to_string(),
            console: true,
            default_level: LogLevel::Info,
            category_levels: std::collections::HashMap::new(),
            enabled: true,
            rotation_hourly: true,
            timestamp: true,
            timestamp_format: "%Y-%m-%d %H:%M:%S".to_string(),
        }
    }
}

/// 全局日志管理器
#[allow(dead_code)]
pub struct LogManager {
    /// 日志配置
    config: RwLock<LogConfig>,
    /// 日志目录
    log_dir: String,
    /// 各分类的 appender
    appenders:
        RwLock<std::collections::HashMap<LogCategory, tracing_appender::non_blocking::WorkerGuard>>,
}

impl LogManager {
    /// 创建新的日志管理器
    pub fn new(config: LogConfig) -> Self {
        Self {
            config: RwLock::new(config.clone()),
            log_dir: config.log_dir.clone(),
            appenders: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 初始化日志系统
    pub fn init(&self) -> anyhow::Result<()> {
        let config = self.config.read().clone();

        if !config.enabled {
            // 如果禁用日志，只输出到 /dev/null
            let filter = EnvFilter::new("off");
            tracing_subscriber::registry().with(filter).init();
            return Ok(());
        }

        // 创建日志目录
        std::fs::create_dir_all(&self.log_dir)?;

        // 创建各分类子目录
        for category in [
            LogCategory::Pick,
            LogCategory::Trade,
            LogCategory::Shop,
            LogCategory::Storage,
            LogCategory::Chat,
            LogCategory::Battle,
            LogCategory::Login,
            LogCategory::Char,
            LogCategory::Map,
            LogCategory::Sql,
            LogCategory::GmCommand,
            LogCategory::Npc,
            LogCategory::Zeny,
            LogCategory::Cash,
            LogCategory::Mail,
            LogCategory::MvpDrop,
            LogCategory::Pet,
            LogCategory::Error,
            LogCategory::Warning,
            LogCategory::Debug,
            LogCategory::Info,
        ] {
            let category_dir = format!("{}/{}", self.log_dir, category.dir_name());
            std::fs::create_dir_all(&category_dir)?;
        }

        // 构建 EnvFilter
        let env_filter = match config.default_level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warning => "warn",
            LogLevel::Error => "error",
        };

        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

        // 构建日志层
        let subscriber = tracing_subscriber::registry().with(filter);

        // 控制台层
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .compact();
        let subscriber = subscriber.with(fmt_layer);

        // 文件层 - 使用 daily rotation
        let file_appender = RollingFileAppender::new(
            if config.rotation_hourly {
                Rotation::HOURLY
            } else {
                Rotation::DAILY
            },
            &self.log_dir,
            "server.log",
        );

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        // 保持 guard 存活
        std::mem::forget(guard);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .compact();

        subscriber.with(file_layer).init();

        tracing::info!("日志系统初始化完成, 目录: {}", self.log_dir);
        Ok(())
    }

    /// 写入分类日志
    pub fn write_category(&self, category: LogCategory, message: &str) {
        if !self.config.read().enabled {
            return;
        }

        let timestamp = if self.config.read().timestamp {
            chrono_lite_timestamp()
        } else {
            String::new()
        };

        let file_path = format!(
            "{}/{}/{}",
            self.log_dir,
            category.dir_name(),
            category.file_name()
        );

        let line = if timestamp.is_empty() {
            format!("{}\n", message)
        } else {
            format!("[{}] {}\n", timestamp, message)
        };

        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
        {
            eprintln!("Failed to write category log: {}", e);
        }

        // 截断过长消息以避免日志过长
        let truncated_message = if message.len() > 200 {
            format!(
                "{}...[truncated {} chars]",
                &message[..100],
                message.len() - 100
            )
        } else {
            message.to_string()
        };

        // 通过 tracing 输出 (使用分类名作为 target)
        // 注意: target 必须是字面字符串，所以使用 match
        match category {
            LogCategory::Pick => tracing::info!(target: "pick", "{}", truncated_message),
            LogCategory::Trade => tracing::info!(target: "trade", "{}", truncated_message),
            LogCategory::Shop => tracing::info!(target: "shop", "{}", truncated_message),
            LogCategory::Storage => tracing::info!(target: "storage", "{}", truncated_message),
            LogCategory::Chat => tracing::info!(target: "chat", "{}", truncated_message),
            LogCategory::Battle => tracing::info!(target: "battle", "{}", truncated_message),
            LogCategory::Login => tracing::info!(target: "login", "{}", truncated_message),
            LogCategory::Char => tracing::info!(target: "char", "{}", truncated_message),
            LogCategory::Map => tracing::info!(target: "map", "{}", truncated_message),
            LogCategory::Sql => tracing::info!(target: "sql", "{}", truncated_message),
            LogCategory::GmCommand => tracing::info!(target: "gm", "{}", truncated_message),
            LogCategory::Npc => tracing::info!(target: "npc", "{}", truncated_message),
            LogCategory::Zeny => tracing::info!(target: "zeny", "{}", truncated_message),
            LogCategory::Cash => tracing::info!(target: "cash", "{}", truncated_message),
            LogCategory::Mail => tracing::info!(target: "mail", "{}", truncated_message),
            LogCategory::MvpDrop => tracing::info!(target: "mvp", "{}", truncated_message),
            LogCategory::Pet => tracing::info!(target: "pet", "{}", truncated_message),
            LogCategory::Error => tracing::info!(target: "error", "{}", truncated_message),
            LogCategory::Warning => tracing::info!(target: "warning", "{}", truncated_message),
            LogCategory::Debug => tracing::info!(target: "debug", "{}", truncated_message),
            LogCategory::Info => tracing::info!(target: "info", "{}", truncated_message),
        }
    }

    /// 记录物品拾取日志
    pub fn log_pick(
        &self,
        player_id: u32,
        player_name: &str,
        _item_id: u32,
        item_name: &str,
        amount: i32,
        reason: &str,
    ) {
        self.write_category(
            LogCategory::Pick,
            &format!(
                "{} ({}) {} x{} - {}",
                player_name, player_id, item_name, amount, reason
            ),
        );
    }

    /// 记录 Zeny 交易日志
    pub fn log_zeny(
        &self,
        player_id: u32,
        player_name: &str,
        amount: i32,
        reason: &str,
        target: Option<(&str, u32)>,
    ) {
        let target_str = target
            .map(|(t, id)| format!(" -> {} ({})", t, id))
            .unwrap_or_default();
        self.write_category(
            LogCategory::Zeny,
            &format!(
                "{} ({}) {} zeny - {}{}",
                player_name, player_id, amount, reason, target_str
            ),
        );
    }

    /// 记录聊天日志
    pub fn log_chat(
        &self,
        chat_type: &str,
        sender_id: u32,
        sender_name: &str,
        message: &str,
        channel: Option<&str>,
    ) {
        let channel_str = channel.map(|c| format!(" [{}]", c)).unwrap_or_default();
        self.write_category(
            LogCategory::Chat,
            &format!(
                "[{}] {} ({}): {}{}",
                chat_type, sender_name, sender_id, message, channel_str
            ),
        );
    }

    /// 记录 GM 命令
    pub fn log_gm_command(
        &self,
        gm_id: u32,
        gm_name: &str,
        command: &str,
        target: Option<(&str, u32)>,
    ) {
        let target_str = target
            .map(|(t, id)| format!(" on {} ({})", t, id))
            .unwrap_or_default();
        self.write_category(
            LogCategory::GmCommand,
            &format!("{} ({}) used: {}{}", gm_name, gm_id, command, target_str),
        );
    }

    /// 记录战斗日志
    pub fn log_battle(
        &self,
        attacker_name: &str,
        attacker_id: u32,
        target_name: &str,
        target_id: u32,
        damage: i32,
        skill_name: Option<&str>,
    ) {
        let skill_str = skill_name.map(|s| format!(" [{}]", s)).unwrap_or_default();
        self.write_category(
            LogCategory::Battle,
            &format!(
                "{} ({}) -> {} ({}) damage: {}{}",
                attacker_name, attacker_id, target_name, target_id, damage, skill_str
            ),
        );
    }

    /// 记录怪物死亡
    pub fn log_mob_death(&self, mob_id: u32, mob_name: &str, killer_name: &str, killer_id: u32) {
        self.write_category(
            LogCategory::Battle,
            &format!(
                "Monster {} ({}) killed by {} ({})",
                mob_name, mob_id, killer_name, killer_id
            ),
        );
    }

    /// 记录 MVP 掉落
    pub fn log_mvp_drop(
        &self,
        mob_id: u32,
        mob_name: &str,
        item_id: u32,
        item_name: &str,
        winner_name: &str,
        winner_id: u32,
    ) {
        self.write_category(
            LogCategory::MvpDrop,
            &format!(
                "MVP {} ({}) dropped {} ({}) to {} ({})",
                mob_name, mob_id, item_name, item_id, winner_name, winner_id
            ),
        );
    }

    /// 记录登录
    pub fn log_login(&self, account_id: u32, username: &str, ip: &str, success: bool) {
        let status = if success { "SUCCESS" } else { "FAILED" };
        self.write_category(
            LogCategory::Login,
            &format!("{} {} from {} - {}", username, account_id, ip, status),
        );
    }

    /// 记录角色选择
    pub fn log_char_select(&self, account_id: u32, char_name: &str, char_id: u32, ip: &str) {
        self.write_category(
            LogCategory::Char,
            &format!(
                "Account {} selected {} ({}) from {}",
                account_id, char_name, char_id, ip
            ),
        );
    }

    /// 记录地图登录
    pub fn log_map_login(&self, char_name: &str, char_id: u32, map: &str) {
        self.write_category(
            LogCategory::Map,
            &format!("{} ({}) entered map {}", char_name, char_id, map),
        );
    }

    /// 记录 NPC 对话
    pub fn log_npc(&self, npc_name: &str, player_name: &str, player_id: u32, action: &str) {
        self.write_category(
            LogCategory::Npc,
            &format!("{}: {} ({}) - {}", npc_name, player_name, player_id, action),
        );
    }

    /// 记录交易
    pub fn log_trade(
        &self,
        player1_name: &str,
        player1_id: u32,
        player2_name: &str,
        player2_id: u32,
        item_count: u32,
        zeny: u32,
    ) {
        self.write_category(
            LogCategory::Trade,
            &format!(
                "{} ({}) <-> {} ({}): items={}, zeny={}",
                player1_name, player1_id, player2_name, player2_id, item_count, zeny
            ),
        );
    }

    /// 更新配置
    pub fn update_config(&self, config: LogConfig) {
        *self.config.write() = config;
    }

    /// 获取当前配置
    pub fn get_config(&self) -> LogConfig {
        self.config.read().clone()
    }
}

/// 简单的 chrono 替代 - 生成时间戳字符串
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = now.as_secs();
    let millis = now.subsec_millis();

    // 简化时间计算
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

/// 全局日志管理器实例
static LOG_MANAGER: OnceLock<LogManager> = OnceLock::new();

/// 初始化日志系统
pub fn init_logging(config: LogConfig) -> anyhow::Result<()> {
    let manager = LogManager::new(config);
    manager.init()?;
    let _ = LOG_MANAGER.set(manager);
    Ok(())
}

/// 获取全局日志管理器
pub fn get_log_manager() -> Option<&'static LogManager> {
    LOG_MANAGER.get()
}

/// 全局便捷方法 - 记录物品拾取
pub fn log_pick(
    player_id: u32,
    player_name: &str,
    item_id: u32,
    item_name: &str,
    amount: i32,
    reason: &str,
) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_pick(player_id, player_name, item_id, item_name, amount, reason);
    }
}

/// 全局便捷方法 - 记录 Zeny
pub fn log_zeny(
    player_id: u32,
    player_name: &str,
    amount: i32,
    reason: &str,
    target: Option<(&str, u32)>,
) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_zeny(player_id, player_name, amount, reason, target);
    }
}

/// 全局便捷方法 - 记录聊天
pub fn log_chat(
    chat_type: &str,
    sender_id: u32,
    sender_name: &str,
    message: &str,
    channel: Option<&str>,
) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_chat(chat_type, sender_id, sender_name, message, channel);
    }
}

/// 全局便捷方法 - 记录 GM 命令
pub fn log_gm_command(gm_id: u32, gm_name: &str, command: &str, target: Option<(&str, u32)>) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_gm_command(gm_id, gm_name, command, target);
    }
}

/// 全局便捷方法 - 记录战斗
pub fn log_battle(
    attacker_name: &str,
    attacker_id: u32,
    target_name: &str,
    target_id: u32,
    damage: i32,
    skill_name: Option<&str>,
) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_battle(
            attacker_name,
            attacker_id,
            target_name,
            target_id,
            damage,
            skill_name,
        );
    }
}

/// 全局便捷方法 - 记录怪物死亡
pub fn log_mob_death(mob_id: u32, mob_name: &str, killer_name: &str, killer_id: u32) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_mob_death(mob_id, mob_name, killer_name, killer_id);
    }
}

/// 全局便捷方法 - 记录 MVP 掉落
pub fn log_mvp_drop(
    mob_id: u32,
    mob_name: &str,
    item_id: u32,
    item_name: &str,
    winner_name: &str,
    winner_id: u32,
) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_mvp_drop(mob_id, mob_name, item_id, item_name, winner_name, winner_id);
    }
}

/// 全局便捷方法 - 记录登录
pub fn log_login(account_id: u32, username: &str, ip: &str, success: bool) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_login(account_id, username, ip, success);
    }
}

/// 全局便捷方法 - 记录角色选择
pub fn log_char_select(account_id: u32, char_name: &str, char_id: u32, ip: &str) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_char_select(account_id, char_name, char_id, ip);
    }
}

/// 全局便捷方法 - 记录地图登录
pub fn log_map_login(char_name: &str, char_id: u32, map: &str) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_map_login(char_name, char_id, map);
    }
}

/// 全局便捷方法 - 记录 NPC
pub fn log_npc(npc_name: &str, player_name: &str, player_id: u32, action: &str) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_npc(npc_name, player_name, player_id, action);
    }
}

/// 全局便捷方法 - 记录交易
pub fn log_trade(
    player1_name: &str,
    player1_id: u32,
    player2_name: &str,
    player2_id: u32,
    item_count: u32,
    zeny: u32,
) {
    if let Some(manager) = LOG_MANAGER.get() {
        manager.log_trade(
            player1_name,
            player1_id,
            player2_name,
            player2_id,
            item_count,
            zeny,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_category_names() {
        assert_eq!(LogCategory::Pick.dir_name(), "pick");
        assert_eq!(LogCategory::Chat.dir_name(), "chat");
        assert_eq!(LogCategory::GmCommand.file_name(), "messages.log");
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("info"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn"), LogLevel::Warning);
        assert_eq!(LogLevel::from_str("error"), LogLevel::Error);
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert!(config.enabled);
        assert!(config.console);
        assert_eq!(config.default_level, LogLevel::Info);
    }

    #[test]
    fn test_chrono_lite_timestamp() {
        let ts = chrono_lite_timestamp();
        // 格式: HH:MM:SS.mmm
        assert!(ts.len() >= 12);
        assert!(ts.contains(':'));
    }
}
