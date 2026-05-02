//! 核心游戏逻辑模块

pub mod config;
pub mod logging;
pub mod panic;
pub mod timer;
pub mod version;

pub use config::Config;
pub use version::{VERSION, NAME};
pub use panic::PanicHandler;

use crate::cli::Cli;

/// 游戏服务器核心
pub struct Core {
    cli: Cli,
}

impl Core {
    pub fn new(cli: Cli) -> Self {
        Self { cli }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting Deviruchi server: {}", self.cli.name);
        tracing::info!("Mode: {}", self.cli.mode);
        tracing::info!("Standalone: {}", self.cli.standalone);

        // TODO: 实现服务器启动逻辑
        Ok(())
    }
}
