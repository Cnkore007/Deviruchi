use anyhow::Result;
use clap::Parser;
use deviruchi::cli::{Cli, Commands};
use deviruchi::core::Core;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 处理子命令
    match &cli.command {
        Some(Commands::Setup) => {
            // 显式运行配置向导
            deviruchi::core::config::Config::run_setup_wizard()?;
            // 同时生成参考文档
            deviruchi::core::guide::generate_guide()?;
            return Ok(());
        }
        Some(Commands::Guide) => {
            // 仅生成参考文档
            deviruchi::core::guide::generate_guide()?;
            return Ok(());
        }
        None => {
            // 无子命令，检查配置文件是否存在
            let config_path = deviruchi::core::config::Config::get_default_path();
            if !deviruchi::core::config::Config::exists(&config_path) {
                println!("检测到未配置，启动配置向导...\n");
                deviruchi::core::config::Config::run_setup_wizard()?;
                deviruchi::core::guide::generate_guide()?;
            }
        }
    }

    let mut core = Core::new(cli);
    core.run().await
}
