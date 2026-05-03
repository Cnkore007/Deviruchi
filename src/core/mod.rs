//! 核心游戏逻辑模块

pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;

pub use config::Config;
pub use version::VERSION;

use std::sync::Arc;
use crate::cli::Cli;
use crate::storage::{Database, init_schema};
use crate::network::{SessionManager, GameServer, PacketHandler};
use crate::game::map::{MapState, ChannelBus, DropManager};
use crate::game::party::PartyManager;
use crate::game::token::TokenStore;

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
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        let config = Config::load(&cli.config).unwrap_or_default();
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
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // 初始化日志
        crate::core::logging::init_logging("logs", "info")?;

        // 设置 panic hook
        crate::core::panic::PanicHandler::init();

        tracing::info!("{} v{} 启动中...", crate::core::version::NAME, crate::core::VERSION);

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

        // 创建 MapDatabase
        let map_database = Arc::new(crate::game::map::data::MapDatabase::new());

        // 创建 MobSpawnManager（init_default_spawns 在 new() 中自动调用）
        let spawn_manager = Arc::new(crate::game::mob::MobSpawnManager::new());

        // 创建 MobAI
        let mob_ai = Arc::new(crate::game::mob::MobAI::new(
            spawn_manager.clone(),
            channel_bus.clone(),
            drop_manager.clone(),
            party_manager.clone(),
            map_database.clone(),
        ));

        // 创建并启动 GameLoop
        let game_loop = Arc::new(crate::game::GameLoop::new(
            map_state.clone(),
            drop_manager.clone(),
            token_store.clone(),
            mob_ai.clone(),
            spawn_manager.clone(),
        ));
        let _game_loop_handle = game_loop.clone().start();

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

        // 根据模式启动服务器
        let mode = self.cli.mode.as_str();
        let run_login = mode == "login" || mode == "all";
        let run_char = mode == "char" || mode == "all";
        let run_map = mode == "map" || mode == "all";

        if run_login {
            let addr = format!("0.0.0.0:{}", self.config.network.login_port);
            tracing::info!("启动 Login Server: {}", addr);
            let server = GameServer::new(addr, session_manager.clone(), packet_handler.clone());
            server.listen().await?;
        }
        if run_char {
            let addr = format!("0.0.0.0:{}", self.config.network.char_port);
            tracing::info!("启动 Char Server: {}", addr);
            let server = GameServer::new(addr, session_manager.clone(), packet_handler.clone());
            server.listen().await?;
        }
        if run_map {
            let addr = format!("0.0.0.0:{}", self.config.network.map_port);
            tracing::info!("启动 Map Server: {}", addr);
            let server = GameServer::new(addr, session_manager.clone(), packet_handler.clone());
            server.listen().await?;
        }

        if !run_login && !run_char && !run_map {
            tracing::error!("未知运行模式: {}", mode);
        }

        Ok(())
    }
}
