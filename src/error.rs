use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("协议错误: {0}")]
    Protocol(String),

    #[error("游戏逻辑错误: {0}")]
    Game(String),
}

pub type Result<T> = std::result::Result<T, Error>;
