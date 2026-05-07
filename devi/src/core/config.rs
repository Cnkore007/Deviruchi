// 客户端配置模块
// 支持 YAML 反序列化，提供合理的默认值

use serde::Deserialize;

/// 客户端配置
/// 用于从 YAML 配置文件加载客户端运行参数
#[derive(Debug, Clone, Deserialize)]
pub struct ClientConfig {
    /// 窗口宽度（像素）
    pub window_width: u32,
    /// 窗口高度（像素）
    pub window_height: u32,
    /// 服务器地址（IP 或域名）
    pub server_address: String,
    /// 协议类型："modern" (WebSocket) 或 "legacy" (TCP)
    pub protocol: String,
}

/// 默认配置实现
/// 提供常用的默认值，便于开发和测试
impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            window_width: 1024,
            window_height: 768,
            server_address: "127.0.0.1".to_string(),
            protocol: "modern".to_string(),
        }
    }
}
