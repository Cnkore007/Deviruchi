//! 核心游戏逻辑模块

pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;

use crate::game::battle::{BattleHandler, ExpDistributor};
use crate::game::heal;
use crate::game::map::data::MapDatabase;
use crate::game::mob::{MobAI, MobSpawnManager};
use crate::game::status::{StatusTickConfig, StatusTickService};
use crate::game::GameLoop;

pub use crate::game::AtCommandHandler;
pub use config::{Config, HotReloadConfig};
pub use logging::{LogCategory, LogConfig, LogLevel, LogManager};
pub use version::VERSION;

use crate::cli::Cli;
use crate::game::map::{ChannelBus, DropManager, MapState};
use crate::game::party::PartyManager;
use crate::game::token::TokenStore;
use crate::game::AgentApi;
use crate::network::{AgentServer, GameServer, ModernServer, PacketHandler, SessionManager};
use crate::storage::{Database, init_schema};
use std::sync::Arc;

#[allow(dead_code)]
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
    at_command_handler: Arc<AtCommandHandler>,
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        let config = Config::load(&cli.config).unwrap_or_default();
        let config_for_heal = config.clone();
        let at_command_handler = Arc::new({
            let handler = AtCommandHandler::new();
            handler.register_default_commands();
            handler
        });
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
            at_command_handler,
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

        tracing::info!(
            "{} v{} 启动中...",
            crate::core::version::NAME,
            crate::core::VERSION
        );

        // 启动 HP/SP 回复服务
        self.heal_service.start(self.map_state.clone());

        // 启动状态效果周期处理服务
        let tick_service = StatusTickService::new(StatusTickConfig::default());
        tick_service.start(self.map_state.clone());

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

        // 创建共享的游戏系统组件
        let battle_handler = Arc::new(BattleHandler::default());
        let spawn_manager = Arc::new(MobSpawnManager::new());
        let guild_manager = Arc::new(crate::game::guild::GuildManager::with_db(db.clone()));

        // 创建 PacketHandler
        let packet_handler = Arc::new(PacketHandler::new(
            db.clone(),
            session_manager.clone(),
            token_store.clone(),
            map_state.clone(),
            channel_bus.clone(),
            drop_manager.clone(),
            party_manager.clone(),
            battle_handler.clone(),
            spawn_manager.clone(),
            self.config.game.death_drop_items,
            guild_manager.clone(),
        ));

        tracing::info!("服务器初始化完成");

        // 启动 Timer 驱动循环（处理 HealService 等定时回调）
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
            loop {
                interval.tick().await;
                crate::core::timer::Timer::process();
            }
        });

        // 创建并启动 GameLoop
        let rng = crate::game::rand::thread_rng();
        let mob_ai = Arc::new(MobAI::new(
            spawn_manager.clone(),
            channel_bus.clone(),
            drop_manager.clone(),
            party_manager.clone(),
            Arc::new(MapDatabase::new()),
            rng,
            battle_handler.clone(),
            Arc::new(crate::game::skill::SkillDatabase::new()),
        ));
        let game_loop = Arc::new(GameLoop::new(
            map_state.clone(),
            drop_manager.clone(),
            token_store.clone(),
            mob_ai,
            spawn_manager.clone(),
            Arc::new(crate::game::mob::droptable::DropResolver),
            channel_bus.clone(),
            Arc::new(ExpDistributor),
            self.heal_service.clone(),
            Arc::new(crate::game::heal::FoodManager::new()),
            guild_manager.clone(),
        ).with_db(db.clone()));
        game_loop.clone().start();
        tracing::info!("GameLoop 已启动");

        tracing::info!("运行模式: {}", self.cli.mode);

        // 根据模式启动服务器 (并发运行)
        let mode = self.cli.mode.as_str();
        let run_login = mode == "login" || mode == "all";
        let run_char = mode == "char" || mode == "all";
        let run_map = mode == "map" || mode == "all";

        // 使用 Arc 包装以便跨任务共享
        let session_manager = self.session_manager.clone();
        let packet_handler = packet_handler.clone();

        // 启动 Agent API 服务器
        let agent_api = Arc::new(AgentApi::new(
            self.cli.config.clone(),
            map_state.clone(),
        ));
        let agent_addr = "127.0.0.1:16400".to_string();
        let agent_server = AgentServer::new(agent_addr, agent_api);

        // 收集所有服务器任务
        let mut handles = Vec::new();

        if run_login || run_char || run_map {
            // Login Server
            if run_login {
                let addr = format!("0.0.0.0:{}", self.config.network.login_port);
                let sm = session_manager.clone();
                let ph = packet_handler.clone();
                handles.push(tokio::spawn(async move {
                    tracing::info!("启动 Login Server: {}", addr);
                    let server = GameServer::new(addr, sm, ph)
                        .with_initial_stage(crate::network::session::SessionStage::Login);
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
                    let server = GameServer::new(addr, sm, ph)
                        .with_initial_stage(crate::network::session::SessionStage::Char);
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
                    let server = GameServer::new(addr, sm, ph)
                        .with_initial_stage(crate::network::session::SessionStage::Map);
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

        // Agent API 服务器（Unix Socket）
        handles.push(tokio::spawn(async move {
            if let Err(e) = agent_server.listen().await {
                tracing::error!("Agent API 错误: {}", e);
            }
            Ok::<(), anyhow::Error>(())
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
