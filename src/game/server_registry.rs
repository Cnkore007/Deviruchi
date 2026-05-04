//! ServerRegistry - 服务注册表，管理所有服务器的注册和心跳

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 服务器信息
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// 服务器唯一ID
    pub id: u32,
    /// 服务器名称
    pub name: String,
    /// 服务器IP
    pub ip: String,
    /// 服务器端口
    pub port: u16,
    /// 当前在线玩家数
    pub online_players: u32,
    /// 最大玩家数
    pub max_players: u32,
    /// 最后心跳时间
    pub last_heartbeat: Instant,
    /// 服务器类型
    pub server_type: ServerType,
}

/// 服务器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerType {
    Login,
    Char,
    Map,
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            ip: String::new(),
            port: 0,
            online_players: 0,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        }
    }
}

/// 服务器注册表
pub struct ServerRegistry {
    servers: RwLock<HashMap<u32, ServerInfo>>,
}

impl ServerRegistry {
    /// 创建新的服务器注册表
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
        }
    }

    /// 注册服务器
    pub fn register_server(&self, info: ServerInfo) -> Result<(), String> {
        let mut servers = self.servers.write();

        // 使用 entry API 简化逻辑
        servers
            .entry(info.id)
            .and_modify(|existing| {
                existing.ip = info.ip.clone();
                existing.port = info.port;
                existing.max_players = info.max_players;
                existing.server_type = info.server_type;
                existing.last_heartbeat = Instant::now();
            })
            .or_insert(info);

        Ok(())
    }

    /// 注销服务器
    pub fn unregister_server(&self, id: u32) -> Result<(), String> {
        let mut servers = self.servers.write();

        if servers.remove(&id).is_some() {
            Ok(())
        } else {
            Err(format!("Server {} not found", id))
        }
    }

    /// 获取指定ID的服务器信息
    pub fn get_server(&self, id: u32) -> Option<ServerInfo> {
        let servers = self.servers.read();
        servers.get(&id).cloned()
    }

    /// 获取所有Map服务器
    pub fn get_map_servers(&self) -> Vec<ServerInfo> {
        let servers = self.servers.read();
        servers
            .values()
            .filter(|s| s.server_type == ServerType::Map)
            .cloned()
            .collect()
    }

    /// 获取一个可用的Map服务器（负载最低的）
    pub fn get_available_map_server(&self) -> Option<ServerInfo> {
        let servers = self.servers.read();

        servers
            .values()
            .filter(|s| s.server_type == ServerType::Map)
            .filter(|s| s.online_players < s.max_players)
            .min_by_key(|s| s.online_players)
            .cloned()
    }

    /// 获取所有服务器列表
    pub fn get_all_servers(&self) -> Vec<ServerInfo> {
        let servers = self.servers.read();
        servers.values().cloned().collect()
    }

    /// 更新心跳
    pub fn update_heartbeat(&self, id: u32) -> Result<(), String> {
        let mut servers = self.servers.write();

        if let Some(server) = servers.get_mut(&id) {
            server.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(format!("Server {} not found", id))
        }
    }

    /// 更新在线玩家数
    pub fn update_player_count(&self, id: u32, count: u32) -> Result<(), String> {
        let mut servers = self.servers.write();

        if let Some(server) = servers.get_mut(&id) {
            server.online_players = count;
            Ok(())
        } else {
            Err(format!("Server {} not found", id))
        }
    }

    /// 增加玩家数
    pub fn increment_players(&self, id: u32) -> Result<(), String> {
        let mut servers = self.servers.write();

        if let Some(server) = servers.get_mut(&id) {
            server.online_players += 1;
            Ok(())
        } else {
            Err(format!("Server {} not found", id))
        }
    }

    /// 减少玩家数
    pub fn decrement_players(&self, id: u32) -> Result<(), String> {
        let mut servers = self.servers.write();

        if let Some(server) = servers.get_mut(&id) {
            if server.online_players > 0 {
                server.online_players -= 1;
            }
            Ok(())
        } else {
            Err(format!("Server {} not found", id))
        }
    }

    /// 清理超时的服务器
    pub fn cleanup_dead_servers(&self, timeout_secs: u64) -> Vec<u32> {
        let mut servers = self.servers.write();
        let timeout = Duration::from_secs(timeout_secs);
        let now = Instant::now();

        let dead_ids: Vec<u32> = servers
            .iter()
            .filter(|(_, s)| now.duration_since(s.last_heartbeat) > timeout)
            .map(|(&id, _)| id)
            .collect();

        for id in &dead_ids {
            servers.remove(id);
        }

        dead_ids
    }

    /// 获取服务器数量
    pub fn server_count(&self) -> usize {
        let servers = self.servers.read();
        servers.len()
    }

    /// 获取Map服务器数量
    pub fn map_server_count(&self) -> usize {
        let servers = self.servers.read();
        servers
            .values()
            .filter(|s| s.server_type == ServerType::Map)
            .count()
    }

    /// 检查服务器是否存在
    pub fn is_server_online(&self, id: u32) -> bool {
        let servers = self.servers.read();
        servers.contains_key(&id)
    }
}

impl Default for ServerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_unregister_server() {
        let registry = ServerRegistry::new();

        let info = ServerInfo {
            id: 1,
            name: "MapServer1".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6121,
            online_players: 0,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };

        assert!(registry.register_server(info.clone()).is_ok());
        assert!(registry.is_server_online(1));

        assert!(registry.unregister_server(1).is_ok());
        assert!(!registry.is_server_online(1));
    }

    #[test]
    fn get_available_map_server() {
        let registry = ServerRegistry::new();

        // 注册两个Map服务器
        let server1 = ServerInfo {
            id: 1,
            name: "Map1".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6121,
            online_players: 50,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };

        let server2 = ServerInfo {
            id: 2,
            name: "Map2".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6122,
            online_players: 20,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };

        registry.register_server(server1).unwrap();
        registry.register_server(server2).unwrap();

        // 应该返回玩家数最少的
        let available = registry.get_available_map_server().unwrap();
        assert_eq!(available.id, 2);
        assert_eq!(available.online_players, 20);
    }

    #[test]
    fn heartbeat_update() {
        let registry = ServerRegistry::new();

        let info = ServerInfo {
            id: 1,
            name: "TestServer".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6121,
            online_players: 0,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };

        registry.register_server(info).unwrap();

        std::thread::sleep(Duration::from_millis(10));
        registry.update_heartbeat(1).unwrap();

        let server = registry.get_server(1).unwrap();
        assert!(server.last_heartbeat.elapsed().as_millis() < 100);
    }

    #[test]
    fn cleanup_dead_servers() {
        let registry = ServerRegistry::new();

        // 注册一个正常服务器
        let server1 = ServerInfo {
            id: 1,
            name: "Alive".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6121,
            online_players: 0,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };

        // 注册一个"已死"的服务器（心跳超时）
        let mut server2 = ServerInfo {
            id: 2,
            name: "Dead".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6122,
            online_players: 0,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };
        server2.last_heartbeat = Instant::now() - Duration::from_secs(100);

        registry.register_server(server1).unwrap();
        registry.register_server(server2).unwrap();

        // 清理30秒超时的服务器
        let dead = registry.cleanup_dead_servers(30);

        assert_eq!(dead.len(), 1);
        assert!(!registry.is_server_online(2));
        assert!(registry.is_server_online(1));
    }

    #[test]
    fn player_count_management() {
        let registry = ServerRegistry::new();

        let info = ServerInfo {
            id: 1,
            name: "TestServer".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 6121,
            online_players: 0,
            max_players: 100,
            last_heartbeat: Instant::now(),
            server_type: ServerType::Map,
        };

        registry.register_server(info).unwrap();

        registry.increment_players(1).unwrap();
        registry.increment_players(1).unwrap();
        assert_eq!(registry.get_server(1).unwrap().online_players, 2);

        registry.decrement_players(1).unwrap();
        assert_eq!(registry.get_server(1).unwrap().online_players, 1);
    }

    #[test]
    fn server_type_filter() {
        let registry = ServerRegistry::new();

        registry
            .register_server(ServerInfo {
                id: 1,
                name: "Login".to_string(),
                ip: "127.0.0.1".to_string(),
                port: 6900,
                online_players: 0,
                max_players: 100,
                last_heartbeat: Instant::now(),
                server_type: ServerType::Login,
            })
            .unwrap();

        registry
            .register_server(ServerInfo {
                id: 2,
                name: "Char".to_string(),
                ip: "127.0.0.1".to_string(),
                port: 6901,
                online_players: 0,
                max_players: 100,
                last_heartbeat: Instant::now(),
                server_type: ServerType::Char,
            })
            .unwrap();

        registry
            .register_server(ServerInfo {
                id: 3,
                name: "Map".to_string(),
                ip: "127.0.0.1".to_string(),
                port: 6121,
                online_players: 0,
                max_players: 100,
                last_heartbeat: Instant::now(),
                server_type: ServerType::Map,
            })
            .unwrap();

        let map_servers = registry.get_map_servers();
        assert_eq!(map_servers.len(), 1);
        assert_eq!(map_servers[0].id, 3);
    }
}
