//! 核心游戏逻辑模块

pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;

use crate::game::heal;

pub use config::{Config, HotReloadConfig};
pub use version::VERSION;
pub use logging::{LogManager, LogConfig, LogCategory, LogLevel};

use std::sync::Arc;
use crate::cli::Cli;
use crate::storage::{Database, init_schema};
use crate::network::{SessionManager, GameServer, PacketHandler, ModernServer};
use crate::game::token::TokenStore;
use crate::game::map::{MapState, ChannelBus, DropManager};
use crate::game::party::PartyManager;

pub struct Core {
    cli: Cli,
    config: Config,
    db: Option<Arc<Database>>,
    session_manager: Arc<SessionManager>,
    token_store: Arc<TokenStore>,
    map_state: Arc<MapState>,
    channel_bus: Arc<ChannelBus>,
    drop_manager: Arc<DropManager>,
    party_manager: Arc<PartyManager>,
    heal_service: Arc<heal::HealService>,
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        let config = Config::load(&cli.config).unwrap_or_default();
        let config_for_heal = config.clone();
        Self {
            cli,
            config,
            db: None,
            session_manager: Arc::new(SessionManager::new()),
            token_store: Arc::new(TokenStore::new()),
            map_state: Arc::new(MapState::new()),
            channel_bus: Arc::new(ChannelBus::new()),
            drop_manager: Arc::new(DropManager::new()),
            party_manager: Arc::new(PartyManager::new()),
            heal_service: Arc::new(heal::HealService::new(Arc::new(config_for_heal))),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // 初始化日志系统
        let log_config = logging::LogConfig {
            enabled: self.config.logging.enabled,
            console: self.config.logging.console,
            default_level: logging::LogLevel::from_str(&self.config.logging.level),
            category_levels: std::collections::HashMap::new(),
            log_dir: self.config.logging.log_dir.clone(),
            rotation_hourly: self.config.logging.rotation_hourly,
            timestamp: self.config.logging.timestamp,
            timestamp_format: self.config.logging.timestamp_format.clone(),
        };
        logging::init_logging(log_config)?;

        // 设置 panic hook
        crate::core::panic::PanicHandler::init();

        tracing::info!("{} v{} 启动中...", crate::core::version::NAME, crate::core::VERSION);

        // 启动 HP/SP 回复服务
        self.heal_service.start(self.map_state.clone());

        // 初始化数据库
        let db = Arc::new(Database::open(&self.config.database.path)?);
        init_schema(&db)?;
        self.db = Some(db.clone());

        // 初始化会话管理
        let session_manager = self.session_manager.clone();
        let token_store = self.token_store.clone();
        let map_state = self.map_state.clone();
        let channel_bus = self.channel_bus.clone();
        let drop_manager = self.drop_manager.clone();
        let party_manager = self.party_manager.clone();

        // 创建 PacketHandler
        let packet_handler = Arc::new(PacketHandler::new(
            db,
            session_manager.clone(),
            token_store,
            map_state,
            channel_bus,
            drop_manager,
            party_manager,
        ));

        tracing::info!("服务器初始化完成");
        tracing::info!("运行模式: {}", self.cli.mode);

        // 根据模式启动服务器 (并发运行)
        let mode = self.cli.mode.as_str();
        let run_login = mode == "login" || mode == "all";
        let run_char = mode == "char" || mode == "all";
        let run_map = mode == "map" || mode == "all";

        // 使用 Arc 包装以便跨任务共享
        let session_manager = self.session_manager.clone();
        let packet_handler = packet_handler.clone();

        // 收集所有服务器任务
        let mut handles = Vec::new();

        if run_login || run_char || run_map {
            if !run_login && !run_char && !run_map {
                tracing::error!("未知运行模式: {}", mode);
                return Ok(());
            }

            // Login Server
            if run_login {
                let addr = format!("0.0.0.0:{}", self.config.network.login_port);
                let sm = session_manager.clone();
                let ph = packet_handler.clone();
                handles.push(tokio::spawn(async move {
                    tracing::info!("启动 Login Server: {}", addr);
                    let server = GameServer::new(addr, sm, ph);
                    server.listen().await
                }));
            }

            // Char Server
            if run_char {
                let addr = format!("0.0.0.0:{}", self.config.network.char_port);
                let sm = session_manager.clone();
                let ph = packet_handler.clone();
                handles.push(tokio::spawn(async move {
                    tracing::info!("启动 Char Server: {}", addr);
                    let server = GameServer::new(addr, sm, ph);
                    server.listen().await
                }));
            }

            // Map Server
            if run_map {
                let addr = format!("0.0.0.0:{}", self.config.network.map_port);
                let sm = session_manager.clone();
                let ph = packet_handler.clone();
                handles.push(tokio::spawn(async move {
                    tracing::info!("启动 Map Server: {}", addr);
                    let server = GameServer::new(addr, sm, ph);
                    server.listen().await
                }));
            }
        }

        // Modern Server (WebSocket for Devi client)
        let modern_addr = format!("0.0.0.0:{}", self.config.network.modern_port);
        let sm = session_manager.clone();
        handles.push(tokio::spawn(async move {
            tracing::info!("启动 Modern Server (WebSocket): {}", modern_addr);
            let server = ModernServer::new(modern_addr, sm);
            server.listen().await
        }));

        // 等待所有服务器
        for handle in handles {
            if let Err(e) = handle.await {
                tracing::error!("Server task failed: {}", e);
            }
        }

        Ok(())
    }
}
