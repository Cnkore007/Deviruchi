//! 核心游戏逻辑模块

pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;

pub use config::Config;
pub use version::VERSION;

use crate::cli::Cli;
use crate::storage::{Database, init_schema};
use crate::network::SessionManager;

pub struct Core {
    cli: Cli,
    config: Config,
    db: Option<Database>,
    session_manager: SessionManager,
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        let config = Config::load(&cli.config).unwrap_or_default();
        Self {
            cli,
            config,
            db: None,
            session_manager: SessionManager::new(),
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // 初始化日志
        crate::core::logging::init_logging("logs", "info")?;

        // 设置 panic hook
        crate::core::panic::PanicHandler::init();

        tracing::info!("{} v{} 启动中...", crate::core::version::NAME, crate::core::VERSION);

        // 初始化数据库
        if let Ok(db) = Database::open(&self.config.database.path) {
            init_schema(&db)?;
            self.db = Some(db);
        }

        tracing::info!("服务器初始化完成");
        tracing::info!("运行模式: {}", self.cli.mode);

        // 保持运行
        tokio::signal::ctrl_c().await?;

        tracing::info!("服务器关闭中...");
        Ok(())
    }
}
