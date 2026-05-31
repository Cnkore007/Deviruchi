use anyhow::Result;
use clap::Parser;
use deviruchi::cli::{Cli, Commands};
use deviruchi::core::Core;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 处理子命令
    match &cli.command {
        Some(Commands::Setup) => {
            // 显式运行配置向导
            let config = deviruchi::core::SetupWizard::run()?;
            let config_path = "deviruchi.toml";
            config.save(config_path)?;
            println!("\n✓ 配置已保存到 {}", config_path);
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
            let config_path = "deviruchi.toml";
            if !Path::new(config_path).exists() {
                // 首次启动：运行配置向导
                println!("检测到首次启动，运行配置向导...\n");
                let config = deviruchi::core::SetupWizard::run()?;
                config.save(config_path)?;
                println!("\n✓ 配置已保存到 {}", config_path);
                println!("✓ 正在启动服务器...\n");
                deviruchi::core::guide::generate_guide()?;
            }
        }
    }

    let mut core = Core::new(cli);
    core.run().await
}
