use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::path::Path;

pub fn init_logging<P: AsRef<Path>>(log_dir: P, log_level: &str) -> anyhow::Result<()> {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "deviruchi.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 保存 guard 防止被 drop
    std::mem::forget(_guard);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .init();

    tracing::info!("日志系统初始化完成");
    Ok(())
}
