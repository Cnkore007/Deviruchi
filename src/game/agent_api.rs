//! Agent API 实现
//!
//! 桥接游戏服务器状态与 Agent 进程的请求。
//! 提供 JSON-RPC 方法调用，用于查询和管理游戏服务器。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::core::config::Config;
use crate::game::map::MapState;

/// Agent API 请求处理器
///
/// 持有游戏服务器的核心状态引用，
/// 将 JSON-RPC 方法调用分发到对应的子系统。
pub struct AgentApi {
    /// 配置文件路径（用于读写 TOML 配置）
    config_path: String,
    /// 玩家状态管理（查询在线玩家信息）
    map_state: Arc<MapState>,
    /// 服务器启动时间（用于计算 uptime）
    start_time: std::time::Instant,
}

impl AgentApi {
    /// 创建 AgentApi 实例
    ///
    /// - `config_path`: 配置文件路径
    /// - `map_state`: 游戏地图状态引用
    pub fn new(config_path: String, map_state: Arc<MapState>) -> Self {
        Self {
            config_path,
            map_state,
            start_time: std::time::Instant::now(),
        }
    }

    /// 处理 JSON-RPC 请求，返回结果或错误信息
    ///
    /// 根据 `method` 名称分发到对应的处理函数，
    /// 未知方法返回错误。
    pub fn handle(&self, method: &str, params: &Value) -> Result<Value, String> {
        match method {
            "server.status" => self.server_status(),
            "config.get" => self.config_get(params),
            "config.set" => self.config_set(params),
            "config.reload" => self.config_reload(),
            "player.list" => self.player_list(params),
            "player.info" => self.player_info(params),
            _ => Err(format!("未知方法: {}", method)),
        }
    }

    /// 获取服务器运行状态
    ///
    /// 返回运行时长（秒）和在线玩家数量。
    fn server_status(&self) -> Result<Value, String> {
        let uptime_secs = self.start_time.elapsed().as_secs();
        let player_count = self.map_state.player_count();
        Ok(json!({
            "uptime_seconds": uptime_secs,
            "online_players": player_count,
        }))
    }

    /// 读取指定配置节
    ///
    /// params 需要包含 `section` 字段，值为配置节名称。
    /// 支持的节: server, database, network, game, battle, drop,
    ///           exp, respawn, log/logging, skill, party, storage, chat
    fn config_get(&self, params: &Value) -> Result<Value, String> {
        let section = params
            .get("section")
            .and_then(|v| v.as_str())
            .ok_or("缺少 section 参数")?;

        let config =
            Config::load(&self.config_path).map_err(|e| format!("加载配置失败: {}", e))?;

        let value = match section {
            "server" => serde_json::to_value(&config.server),
            "database" => serde_json::to_value(&config.database),
            "network" => serde_json::to_value(&config.network),
            "game" => serde_json::to_value(&config.game),
            "battle" => serde_json::to_value(&config.battle),
            "drop" => serde_json::to_value(&config.drop),
            "exp" => serde_json::to_value(&config.exp),
            "respawn" => serde_json::to_value(&config.respawn),
            "log" | "logging" => serde_json::to_value(&config.logging),
            "skill" => serde_json::to_value(&config.skill),
            "party" => serde_json::to_value(&config.party),
            "storage" => serde_json::to_value(&config.storage),
            "chat" => serde_json::to_value(&config.chat),
            _ => return Err(format!("未知配置节: {}", section)),
        };

        value.map_err(|e| format!("序列化失败: {}", e))
    }

    /// 修改配置字段
    ///
    /// params 需要包含:
    /// - `section`: 配置节名称
    /// - `key`: 字段名
    /// - `value`: 新值
    ///
    /// 修改前会自动备份原文件为 `.bak`。
    fn config_set(&self, params: &Value) -> Result<Value, String> {
        let section = params
            .get("section")
            .and_then(|v| v.as_str())
            .ok_or("缺少 section")?;
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("缺少 key")?;
        let value = params.get("value").ok_or("缺少 value")?;

        let mut config =
            Config::load(&self.config_path).map_err(|e| format!("加载配置失败: {}", e))?;

        // 备份原文件，防止修改失败导致数据丢失
        let backup_path = format!("{}.bak", self.config_path);
        if let Err(e) = std::fs::copy(&self.config_path, &backup_path) {
            tracing::warn!("Failed to backup config: {}", e);
        }

        self.set_config_field(&mut config, section, key, value)?;

        config
            .save(&self.config_path)
            .map_err(|e| format!("保存配置失败: {}", e))?;

        Ok(json!({"success": true, "message": format!("{}.{} 已更新", section, key)}))
    }

    /// 设置配置结构体中的字段
    ///
    /// 通过 JSON 中转实现通用字段修改：
    /// 1. 将目标配置节序列化为 JSON
    /// 2. 在 JSON 对象中插入/替换指定字段
    /// 3. 反序列化回强类型结构体
    fn set_config_field(
        &self,
        config: &mut Config,
        section: &str,
        key: &str,
        value: &Value,
    ) -> Result<(), String> {
        macro_rules! update_section {
            ($config_section:expr) => {{
                let mut section_json = serde_json::to_value(&$config_section)
                    .map_err(|e| format!("序列化失败: {}", e))?;
                if let Some(obj) = section_json.as_object_mut() {
                    obj.insert(key.to_string(), value.clone());
                }
                $config_section = serde_json::from_value(section_json)
                    .map_err(|e| format!("反序列化失败: {}", e))?;
                Ok(())
            }};
        }

        match section {
            "server" => update_section!(config.server),
            "database" => update_section!(config.database),
            "network" => update_section!(config.network),
            "game" => update_section!(config.game),
            "battle" => update_section!(config.battle),
            "drop" => update_section!(config.drop),
            "exp" => update_section!(config.exp),
            "respawn" => update_section!(config.respawn),
            "log" | "logging" => update_section!(config.logging),
            "skill" => update_section!(config.skill),
            "party" => update_section!(config.party),
            "storage" => update_section!(config.storage),
            "chat" => update_section!(config.chat),
            _ => Err(format!("未知配置节: {}", section)),
        }
    }

    /// 触发配置热重载
    ///
    /// 当前实现仅保存配置文件，实际热重载需要重启服务器。
    fn config_reload(&self) -> Result<Value, String> {
        Ok(json!({"success": true, "message": "配置已保存，重启服务器后生效"}))
    }

    /// 查询在线玩家列表
    ///
    /// 可选 params:
    /// - `map`: 按地图名过滤
    fn player_list(&self, params: &Value) -> Result<Value, String> {
        let map_filter = params.get("map").and_then(|v| v.as_str());

        let players = if let Some(map_name) = map_filter {
            self.map_state.get_players_on_map(map_name)
        } else {
            self.map_state.get_all_players()
        };

        let player_list: Vec<Value> = players
            .iter()
            .map(|p| {
                let pos = p.pos.read();
                let combat = p.combat.read();
                let level = p.level.read();
                json!({
                    "name": p.name,
                    "map": p.map_name,
                    "x": pos.x,
                    "y": pos.y,
                    "hp": combat.hp,
                    "max_hp": combat.max_hp,
                    "sp": combat.sp,
                    "max_sp": combat.max_sp,
                    "base_level": level.base_level,
                    "job_level": level.job_level,
                })
            })
            .collect();

        Ok(json!({
            "count": player_list.len(),
            "players": player_list,
        }))
    }

    /// 查询单个玩家详细信息
    ///
    /// params 需要包含:
    /// - `name`: 玩家角色名
    fn player_info(&self, params: &Value) -> Result<Value, String> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("缺少 name 参数")?;

        let player = self
            .map_state
            .find_player_by_name(name)
            .ok_or(format!("未找到玩家: {}", name))?;

        let pos = player.pos.read();
        let combat = player.combat.read();
        let level = player.level.read();
        let attrs = player.attrs.read();
        let economy = player.economy.read();

        Ok(json!({
            "name": player.name,
            "char_id": player.char_id,
            "account_id": player.account_id,
            "map": player.map_name,
            "x": pos.x,
            "y": pos.y,
            "hp": combat.hp,
            "max_hp": combat.max_hp,
            "sp": combat.sp,
            "max_sp": combat.max_sp,
            "base_level": level.base_level,
            "job_level": level.job_level,
            "base_exp": level.base_exp,
            "job_exp": level.job_exp,
            "status_point": level.status_point,
            "skill_point": level.skill_point,
            "str": attrs.str,
            "agi": attrs.agi,
            "vit": attrs.vit,
            "int": attrs.int,
            "dex": attrs.dex,
            "luk": attrs.luk,
            "zeny": economy.zeny,
            "job": economy.job,
        }))
    }
}
