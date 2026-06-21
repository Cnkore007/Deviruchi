//! Inter-Server TCP 服务器
//!
//! 监听 inter-server 端口并接受其他服务器（login/char/map）的连接。
//! 目前作为 stub，只负责接受连接并将连接器注册到 InterServerComm。

use crate::game::inter_server::{InterServerComm, InterServerConnector};
use crate::network::inter_server::connector::TcpInterServerConnector;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};

/// Inter-Server TCP 监听服务器
#[allow(dead_code)]
pub struct InterServerTcpServer {
    addr: String,
    server_id: u32,
    #[allow(dead_code)]
    comm: Arc<InterServerComm>,
}

impl InterServerTcpServer {
    /// 创建新的 inter-server TCP 服务器
    pub fn new(
        addr: String,
        server_id: u32,
        comm: Arc<InterServerComm>,
    ) -> Self {
        Self {
            addr,
            server_id,
            comm,
        }
    }

    /// 在独立线程中启动监听循环（阻塞当前线程）
    pub fn listen_blocking(&self,
        on_accept: impl Fn(u32, Arc<dyn InterServerConnector>) + Send + 'static,
    ) {
        let listener = match TcpListener::bind(&self.addr) {
            Ok(l) => {
                info!(
                    "Inter-server TCP server {} listening on {}",
                    self.server_id, self.addr
                );
                l
            }
            Err(e) => {
                error!(
                    "无法绑定 inter-server 地址 {}: {}",
                    self.addr, e
                );
                return;
            }
        };

        let server_id = self.server_id;

        thread::spawn(move || {
            for incoming in listener.incoming() {
                match incoming {
                    Ok(stream) => {
                        // 初始连接时无法知道对方 server_id，使用 0 占位
                        match TcpInterServerConnector::from_stream(0, stream) {
                            Ok(connector) => {
                                info!(
                                    "Inter-server 连接接入 (server_id={})",
                                    server_id
                                );
                                on_accept(0, connector);
                            }
                            Err(e) => warn!("配置 inter-server 连接失败: {}", e),
                        }
                    }
                    Err(e) => warn!("接受 inter-server 连接失败: {}", e),
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_inter_server_tcp_server_bind() {
        let comm = Arc::new(InterServerComm::new());
        let server = InterServerTcpServer::new(
            "127.0.0.1:0".to_string(),
            1,
            comm,
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        server.listen_blocking(move |_, _| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // 短暂等待线程启动
        thread::sleep(std::time::Duration::from_millis(50));

        // 目前没有实际连接，只验证绑定不 panic
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
