//! 核心游戏逻辑模块

pub mod config;
pub mod guide;
pub mod logging;
pub mod panic;
pub mod setup_wizard;
pub mod timer;
pub mod version;

pub use crate::game::AtCommandHandler;
pub use config::{Config, HotReloadConfig};
pub use logging::{LogCategory, LogConfig, LogLevel, LogManager};
pub use setup_wizard::SetupWizard;
pub use version::VERSION;

use crate::cli::Cli;
use crate::game::AgentApi;
use crate::game::GameLoop;
use crate::game::battle::{BattleHandler, ExpDistributor};
use crate::game::heal;
use crate::game::map::data::MapDatabase;
use crate::game::map::{ChannelBus, DropManager, MapState};
use crate::game::mob::{MobAI, MobSpawnManager};
use crate::game::party::PartyManager;
use crate::game::status::{StatusTickConfig, StatusTickService};
use crate::game::token::TokenStore;
use crate::network::{AgentServer, GameServer, PacketHandler, SessionManager};
use crate::storage::{Database, init_schema};
use std::sync::Arc;

pub struct Core {
    cli: Cli,
    config: Arc<Config>,
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
        let config = Arc::new(config);
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
            heal_service: Arc::new(heal::HealService::new(config_for_heal)),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // 初始化日志系统
        let log_config = logging::LogConfig {
            enabled: self.config.logging.enabled,
            console: self.config.logging.console,
            default_level: logging::LogLevel::parse(&self.config.logging.level),
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

        // 初始化数据库
        let db = Arc::new(Database::open(&self.config.database.path)?);
        init_schema(&db)?;
        self.db = Some(db.clone());

        tracing::info!("服务器初始化完成");

        tracing::info!("运行模式: {}", self.cli.mode);

        // 根据模式启动服务器 (并发运行)
        let mode = self.cli.mode.as_str();
        let run_login = mode == "login" || mode == "all";
        let run_char = mode == "char" || mode == "all";
        let run_map = mode == "map" || mode == "all";

        // 使用 Arc 包装以便跨任务共享
        let session_manager = self.session_manager.clone();

        // 收集所有服务器任务
        let mut handles = Vec::new();

        if run_login {
            let addr = format!("0.0.0.0:{}", self.config.network.login_port);
            let sm = session_manager.clone();
            let db = db.clone();
            handles.push(tokio::spawn(async move {
                tracing::info!("启动 Login Server: {}", addr);
                let packet_handler = Arc::new(PacketHandler::new_login(
                    db,
                    sm.clone(),
                ));
                let server = GameServer::new(addr, sm, packet_handler)
                    .with_initial_stage(crate::network::session::SessionStage::Login);
                server.listen().await
            }));
        }

        if run_char {
            let addr = format!("0.0.0.0:{}", self.config.network.char_port);
            let sm = session_manager.clone();
            let db = db.clone();
            let token_store = self.token_store.clone();
            handles.push(tokio::spawn(async move {
                tracing::info!("启动 Char Server: {}", addr);
                let packet_handler = Arc::new(PacketHandler::new_char(
                    db,
                    sm.clone(),
                    token_store,
                ));
                let server = GameServer::new(addr, sm, packet_handler)
                    .with_initial_stage(crate::network::session::SessionStage::Char);
                server.listen().await
            }));
        }

        if run_map {
            // 启动 HP/SP 回复服务
            self.heal_service.start(self.map_state.clone());

            // 启动状态效果周期处理服务
            let tick_service = StatusTickService::new(StatusTickConfig::default());
            tick_service.start(self.map_state.clone());

            // 创建共享的游戏系统组件
            let battle_handler = Arc::new(BattleHandler::default());
            let spawn_manager = Arc::new(MobSpawnManager::new());
            let guild_manager = Arc::new(crate::game::guild::GuildManager::with_db(db.clone()));

            let packet_handler = Arc::new(PacketHandler::new_map(
                db.clone(),
                session_manager.clone(),
                self.token_store.clone(),
                self.map_state.clone(),
                self.channel_bus.clone(),
                self.drop_manager.clone(),
                self.party_manager.clone(),
                battle_handler.clone(),
                spawn_manager.clone(),
                self.config.game.death_drop_items,
                guild_manager.clone(),
            ));

            // 启动 Timer 驱动循环（处理 HealService 等定时回调）
            tokio::spawn({
                async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
                    loop {
                        interval.tick().await;
                        crate::core::timer::Timer::process();
                    }
                }
            });

            // 创建并启动 GameLoop
            let rng = crate::game::rand::thread_rng();
            let mob_ai = Arc::new(MobAI::new(
                spawn_manager.clone(),
                self.channel_bus.clone(),
                self.drop_manager.clone(),
                self.party_manager.clone(),
                Arc::new(MapDatabase::new()),
                rng,
                battle_handler.clone(),
                Arc::new(crate::game::skill::SkillDatabase::new()),
            ));
            let game_loop = Arc::new(
                GameLoop::new(
                    self.map_state.clone(),
                    self.drop_manager.clone(),
                    self.token_store.clone(),
                    mob_ai,
                    spawn_manager.clone(),
                    Arc::new(crate::game::mob::droptable::DropResolver),
                    self.channel_bus.clone(),
                    Arc::new(ExpDistributor),
                    self.heal_service.clone(),
                    Arc::new(crate::game::heal::FoodManager::new()),
                    guild_manager.clone(),
                )
                .with_db(db.clone()),
            );
            game_loop.clone().start();
            tracing::info!("GameLoop 已启动");

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

        // Agent API 服务器
        if let Some(agent_port) = self.config.network.agent_port {
            let agent_api = Arc::new(AgentApi::new(self.cli.config.clone(), self.map_state.clone()));
            let agent_addr = format!("127.0.0.1:{}", agent_port);
            let agent_server = AgentServer::new(agent_addr, agent_api);
            handles.push(tokio::spawn(async move {
                if let Err(e) = agent_server.listen().await {
                    tracing::error!("Agent API 错误: {}", e);
                }
                Ok::<(), anyhow::Error>(())
            }));
        }

        // Web HTTP API 服务器
        if let Some(web_port) = self.config.network.web_port {
            let web_addr = format!("0.0.0.0:{}", web_port);
            let web_server = crate::web::WebServer::new(web_addr, self.map_state.clone());
            handles.push(tokio::spawn(async move {
                if let Err(e) = web_server.listen().await {
                    tracing::error!("Web API 错误: {}", e);
                }
                Ok::<(), anyhow::Error>(())
            }));
        }

        // 优雅关机：监听 SIGINT/SIGTERM，收到信号后通知所有任务退出
        let shutdown = async {
            let ctrl_c = tokio::signal::ctrl_c();
            #[cfg(unix)]
            {
                let mut sigterm =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("无法注册 SIGTERM 处理器");
                tokio::select! {
                    _ = ctrl_c => tracing::info!("收到 SIGINT 信号，开始优雅关机..."),
                    _ = sigterm.recv() => tracing::info!("收到 SIGTERM 信号，开始优雅关机..."),
                }
            }
            #[cfg(not(unix))]
            {
                ctrl_c.await.ok();
                tracing::info!("收到中断信号，开始优雅关机...");
            }
        };

        // 等待所有服务器或关机信号
        let mut abort_handles = Vec::new();
        for handle in &handles {
            abort_handles.push(handle);
        }

        tokio::select! {
            // 正常退出：所有服务器任务完成
            _ = async {
                for handle in handles {
                    if let Err(e) = handle.await {
                        tracing::error!("Server task failed: {}", e);
                    }
                }
            } => {
                tracing::info!("所有服务器任务已完成");
            }
            // 信号退出：取消所有任务
            _ = shutdown => {
                tracing::info!("正在关闭所有服务器连接...");
                // 任务在 drop 时自动取消
            }
        }

        // 执行数据库最终备份（如有需要）
        if let Some(ref db) = self.db {
            tracing::info!("正在执行关机前数据库清理...");
            if let Err(e) = db.execute("PRAGMA wal_checkpoint(TRUNCATE)") {
                tracing::warn!("WAL checkpoint 失败: {}", e);
            }
        }

        tracing::info!("服务器已安全关闭");
        Ok(())
    }
}
