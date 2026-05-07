// 资源系统模块
pub mod grf;
pub mod sprite;
pub mod action;
pub mod gat;
pub mod gnd;
pub mod loader;

/// 资源加载错误
#[derive(Debug)]
pub enum AssetError {
    /// 文件未找到
    NotFound(String),
    /// IO 错误
    Io(std::io::Error),
    /// 解析错误
    ParseError(String),
    /// 不支持的格式
    UnsupportedFormat(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::NotFound(path) => write!(f, "资源未找到: {}", path),
            AssetError::Io(err) => write!(f, "IO 错误: {}", err),
            AssetError::ParseError(msg) => write!(f, "解析错误: {}", msg),
            AssetError::UnsupportedFormat(fmt) => write!(f, "不支持的格式: {}", fmt),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(err: std::io::Error) -> Self {
        AssetError::Io(err)
    }
}
