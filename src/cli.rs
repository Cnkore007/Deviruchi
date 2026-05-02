use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "deviruchi")]
#[command(about = "Deviruchi - High-performance MMORPG game server")]
pub struct Cli {
    /// 配置文件的路径
    #[arg(short, long, default_value = "deviruchi.toml")]
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
}
