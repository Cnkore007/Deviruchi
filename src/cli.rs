use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "deviruchi")]
#[command(about = "Deviruchi - High-performance MMORPG game server")]
pub struct Cli {
    /// 配置文件的路径
    #[arg(short, long, default_value = "config/server.toml")]
    pub config: String,

    /// 服务器名称
    #[arg(short, long, default_value = "Deviruchi")]
    pub name: String,

    /// 日志级别
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// 单机模式运行
    #[arg(long, default_value = "true")]
    pub standalone: bool,

    /// 运行模式: login, char, map
    #[arg(long, default_value = "all")]
    pub mode: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 运行交互式配置向导（首次设置或重新配置）
    Setup,
    /// 生成参考文档（物品/怪物/技能/地图列表）
    Guide,
}
