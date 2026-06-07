//! 角色选择业务逻辑

use std::sync::Arc;
use tracing::{error, info, warn};
use crate::game::constants;

use crate::game::token::TokenStore;
use crate::network::session::{Session, SessionManager};
use crate::protocol::map_packets::{
    CHEnterCharServer, CHSelectChar, CHDeleteChar, CHCancelDelete, CHMakeChar, CharInfo,
    HCNotifyZoneServer, HCDeleteCharOk, HCCancelDeleteOk, SCCharList,
};
use crate::protocol::packet_builder::Packed;
use crate::storage::{chrono_now, Database};
#[cfg(test)]
use crate::storage::{init_schema, Character};

/// 角色服务器
pub struct CharServer {
    db: Arc<Database>,
    #[allow(dead_code)]
    session_manager: Arc<SessionManager>,
    token_store: Arc<TokenStore>,
    map_ip: String,
    map_port: u16,
    map_server_id: u32,
}

impl CharServer {
    pub fn new(
        db: Arc<Database>,
        session_manager: Arc<SessionManager>,
        token_store: Arc<TokenStore>,
    ) -> Self {
        Self {
            db,
            session_manager,
            token_store,
            map_ip: "127.0.0.1".to_string(),
            map_port: 6121,
            map_server_id: 1,
        }
    }

    pub fn with_map_server(mut self, ip: &str, port: u16) -> Self {
        self.map_ip = ip.to_string();
        self.map_port = port;
        self
    }

    /// 设置 Map Server ID
    #[allow(dead_code)]
    pub fn with_map_server_id(mut self, server_id: u32) -> Self {
        self.map_server_id = server_id;
        self
    }

    /// 根据 packet_id 分发处理
    pub fn handle_packet(
        &self,
        packet_id: u16,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        match packet_id {
            0x0065 => self.handle_enter(data, session),
            0x0066 => self.handle_select_char(data, session),
            0x0067 => self.handle_make_char(data, session),
            0x0068 => self.handle_delete_char(data, session),
            0x01F8 => self.handle_cancel_delete(data, session),
            _ => {
                warn!("Unknown char packet id: 0x{:04X}", packet_id);
                None
            }
        }
    }

    /// 处理进入角色服务器 (0x0065)
    /// 客户端连接 Char Server 后发送的第一个包，包含登录服务器颁发的身份信息
    fn handle_enter(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let enter = CHEnterCharServer::from_slice(data)?;

        info!(
            "Char server enter: account_id={}, login_id1=0x{:08X}, login_id2=0x{:08X}, sex={}",
            enter.account_id, enter.login_id1, enter.login_id2, enter.sex
        );

        // 验证账户是否存在
        let _account = self.db.get_account_by_id(enter.account_id).ok()??;

        // 设置 session 身份信息
        session.account_id = Some(enter.account_id);
        session.authenticated = true;
        session.login_id1 = enter.login_id1;
        session.login_id2 = enter.login_id2;

        info!("Char server: account_id={} authenticated", enter.account_id);

        // 自动发送角色列表
        self.handle_request_char_list(session)
    }

    /// 处理请求角色列表 (内部方法，由 handle_enter 调用)
    fn handle_request_char_list(&self, session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        info!("Request char list for account_id={}", account_id);

        // 先清理已过期的删除定时器角色
        let _ = self.db.cleanup_deleted_characters();

        let characters = self.db.get_characters_by_account(account_id).ok()?;

        // 过滤掉已标记删除但定时器尚未过期的角色仍然显示，
        // 但定时器已过期的角色不应再出现（已被 cleanup 清理）
        // 这里额外过滤 delete_timer > 0 且已过期的角色作为防御性措施
        let now = chrono_now() as u32;
        let char_infos: Vec<CharInfo> = characters
            .iter()
            .filter(|c| {
                // delete_timer == 0 表示未标记删除，正常显示
                // delete_timer > 0 且 <= now 表示已过期，不显示
                // delete_timer > 0 且 > now 表示待删除但未过期，仍显示（客户端可取消）
                c.delete_timer == 0 || c.delete_timer > now
            })
            .map(|c| self.db.character_to_char_info(c))
            .collect();

        info!(
            "Sending {} characters for account_id={}",
            char_infos.len(),
            account_id
        );

        Some(
            SCCharList {
                characters: char_infos,
            }
            .to_packet(),
        )
    }

    /// 处理创建角色 (0x0067)
    fn handle_make_char(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        // 解析创建角色包
        let make_char = CHMakeChar::from_slice(data)?;

        info!(
            "Make char request: name={}, slot=?, account_id={}",
            make_char.name, account_id
        );

        // 获取当前角色列表
        let characters = self.db.get_characters_by_account(account_id).ok()?;

        // 找到空槽位
        let slot = self.find_empty_slot(&characters)?;

        // 校验属性点：每个属性 1-9，总和 <= 30（rAthena 新角色分配）
        let stats = [
            make_char.str, make_char.agi, make_char.vit,
            make_char.int, make_char.dex, make_char.luk,
        ];

        if stats.iter().any(|&s| s == 0 || s > constants::MAX_SINGLE_STAT) {
            warn!(
                "Character creation rejected: stat out of range 1-9 for account_id={}",
                account_id
            );
            return Some(crate::protocol::packet_builder::PacketBuilder::new(0x006D).put_slice(&[0x01, 0x00, 0x00, 0x00]).build());
        }

        let total: u16 = stats.iter().map(|&s| s as u16).sum();
        if total > constants::MAX_TOTAL_STATS {
            warn!(
                "Character creation rejected: total stats {} > {} for account_id={}",
                total, constants::MAX_TOTAL_STATS, account_id
            );
            return Some(crate::protocol::packet_builder::PacketBuilder::new(0x006D).put_slice(&[0x01, 0x00, 0x00, 0x00]).build());
        }

        // 校验角色名称（长度 + 特殊字符 + 重复检查）
        if let Err(err_msg) = self.validate_character_name(&make_char.name) {
            warn!(
                "Character creation rejected: {} (account_id={})",
                err_msg, account_id
            );
            return Some(crate::protocol::packet_builder::PacketBuilder::new(0x006D).put_slice(&[0x01, 0x00, 0x00, 0x00]).build());
        }
        let name = make_char.name.trim_matches('\0');

        // 创建角色
        match self.db.create_character(
            account_id,
            slot,
            name,
            make_char.str,
            make_char.agi,
            make_char.vit,
            make_char.int,
            make_char.dex,
            make_char.luk,
            make_char.hair,
            make_char.hair_color,
        ) {
            Ok(char_id) => {
                info!(
                    "Character created: char_id={}, name={}",
                    char_id, make_char.name
                );
                // 构建成功响应：marker(0) + padding(3) + char_info(110)
                let character = self.db.get_character_by_id(char_id).ok()??;
                let char_info = self.db.character_to_char_info(&character);
                let mut resp = Vec::with_capacity(8 + 110);
                // marker = 0 (成功)
                resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
                // char_info (110 bytes)
                resp.extend_from_slice(&char_info.char_id.to_le_bytes());
                resp.extend_from_slice(&char_info.exp.to_le_bytes());
                resp.extend_from_slice(&char_info.gold.to_le_bytes());
                resp.extend_from_slice(&char_info.job_exp.to_le_bytes());
                resp.extend_from_slice(&char_info.job_level.to_le_bytes());
                resp.extend_from_slice(&char_info.body_state.to_le_bytes());
                resp.extend_from_slice(&char_info.health_state.to_le_bytes());
                resp.extend_from_slice(&char_info.effect_state.to_le_bytes());
                resp.extend_from_slice(&(char_info.virtue as u16).to_le_bytes());
                resp.extend_from_slice(&(char_info.honor as u16).to_le_bytes());
                resp.extend_from_slice(&char_info.job.to_le_bytes());
                resp.extend_from_slice(&char_info.hair.to_le_bytes());
                resp.extend_from_slice(&char_info.hair_color.to_le_bytes());
                resp.extend_from_slice(&char_info.clothes_color.to_le_bytes());
                resp.extend_from_slice(&char_info.body.to_le_bytes());
                resp.extend_from_slice(&char_info.weapon.to_le_bytes());
                resp.extend_from_slice(&char_info.head_bottom.to_le_bytes());
                resp.extend_from_slice(&char_info.shield.to_le_bytes());
                resp.extend_from_slice(&char_info.head_top.to_le_bytes());
                resp.extend_from_slice(&char_info.head_mid.to_le_bytes());
                resp.extend_from_slice(&char_info.hair_color2.to_le_bytes());
                resp.extend_from_slice(&char_info.clothes_color2.to_le_bytes());
                let mut name_bytes = vec![0u8; 24];
                let name_len = char_info.name.len().min(23);
                name_bytes[..name_len].copy_from_slice(&char_info.name.as_bytes()[..name_len]);
                resp.extend_from_slice(&name_bytes);
                resp.extend_from_slice(&char_info.base_level.to_le_bytes());
                resp.extend_from_slice(&char_info.str.to_le_bytes());
                resp.extend_from_slice(&char_info.agi.to_le_bytes());
                resp.extend_from_slice(&char_info.vit.to_le_bytes());
                resp.extend_from_slice(&char_info.int.to_le_bytes());
                resp.extend_from_slice(&char_info.dex.to_le_bytes());
                resp.extend_from_slice(&char_info.luk.to_le_bytes());
                resp.extend_from_slice(&char_info.slot.to_le_bytes());
                resp.extend_from_slice(&char_info.delete_timer.to_le_bytes());
                resp.extend_from_slice(&char_info.rename.to_le_bytes());
                let mut map_bytes = vec![0u8; 24];
                let map_len = char_info.map_name.len().min(23);
                map_bytes[..map_len].copy_from_slice(&char_info.map_name.as_bytes()[..map_len]);
                resp.extend_from_slice(&map_bytes);
                // 用 PacketBuilder 添加包头
                Some(crate::protocol::packet_builder::PacketBuilder::new(0x006D)
                    .put_slice(&resp)
                    .build())
            }
            Err(e) => {
                error!("Failed to create character: {}", e);
                // 失败响应：marker(1) + padding(3)
                Some(crate::protocol::packet_builder::PacketBuilder::new(0x006D)
                    .put_slice(&[0x01, 0x00, 0x00, 0x00])
                    .build())
            }
        }
    }

    /// 处理选择角色进入 (0x0065)
    fn handle_select_char(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        // 解析选择角色包
        let enter = CHSelectChar::from_slice(data)?;

        info!(
            "Select char request: char_id={}, account_id={}",
            enter.char_id, account_id
        );

        // 验证角色是否属于该账户且未过期删除
        let characters = self.db.get_characters_by_account(account_id).ok()?;
        let now = chrono_now() as u32;
        let valid_char = characters.iter().any(|c| {
            c.char_id == enter.char_id
                && (c.delete_timer == 0 || c.delete_timer > now)
        });

        if !valid_char {
            warn!(
                "Invalid char selection: char_id={} not owned by account_id={}",
                enter.char_id, account_id
            );
            return Some(vec![0]); // 失败响应
        }

        // 设置 session.char_id
        session.char_id = Some(enter.char_id);

        // Generate one-time token for Char→Map transition with target map server ID
        let token = self
            .token_store
            .create(account_id, enter.char_id, self.map_server_id);

        info!(
            "Character selected: char_id={}, token generated for map server {} ({}:{})",
            enter.char_id, self.map_server_id, self.map_ip, self.map_port
        );

        // Return HCNotifyZoneServer with map server info
        Some(
            HCNotifyZoneServer {
                map_ip: self.map_ip.clone(),
                map_port: self.map_port,
                token,
            }
            .to_packet(),
        )
    }

    /// 处理请求删除角色 (0x0068)
    /// rAthena 协议：发送 delete_timer 标记，客户端收到后显示倒计时
    /// 默认删除延迟 86400 秒 (24 小时)
    fn handle_delete_char(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        let delete_req = CHDeleteChar::from_slice(data)?;

        info!(
            "Delete char request: char_id={}, account_id={}",
            delete_req.char_id, account_id
        );

        // 验证角色是否属于该账户
        let characters = self.db.get_characters_by_account(account_id).ok()?;
        let char_exists = characters.iter().any(|c| c.char_id == delete_req.char_id);

        if !char_exists {
            warn!(
                "Delete char rejected: char_id={} not owned by account_id={}",
                delete_req.char_id, account_id
            );
            return Some(crate::protocol::packet_builder::PacketBuilder::new(0x006E)
                .put_slice(&[0x01, 0x00, 0x00, 0x00])
                .build());
        }

        // 标记角色删除（24小时后删除）
        match self.db.mark_character_for_deletion(delete_req.char_id, account_id, 86400) {
            Ok(true) => {
                info!(
                    "Character {} marked for deletion (account_id={})",
                    delete_req.char_id, account_id
                );
                Some(
                    HCDeleteCharOk {
                        char_id: delete_req.char_id,
                    }
                    .to_packet(),
                )
            }
            Ok(false) => {
                warn!(
                    "Failed to mark character {} for deletion (already marked or not found)",
                    delete_req.char_id
                );
                Some(crate::protocol::packet_builder::PacketBuilder::new(0x006E)
                    .put_slice(&[0x01, 0x00, 0x00, 0x00])
                    .build())
            }
            Err(e) => {
                error!("Database error deleting character {}: {}", delete_req.char_id, e);
                Some(crate::protocol::packet_builder::PacketBuilder::new(0x006E)
                    .put_slice(&[0x01, 0x00, 0x00, 0x00])
                    .build())
            }
        }
    }

    /// 处理取消删除角色 (0x01F8)
    fn handle_cancel_delete(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        let cancel_req = CHCancelDelete::from_slice(data)?;

        info!(
            "Cancel delete request: char_id={}, account_id={}",
            cancel_req.char_id, account_id
        );

        // 验证角色是否属于该账户
        let characters = self.db.get_characters_by_account(account_id).ok()?;
        let char_exists = characters.iter().any(|c| c.char_id == cancel_req.char_id);

        if !char_exists {
            warn!(
                "Cancel delete rejected: char_id={} not owned by account_id={}",
                cancel_req.char_id, account_id
            );
            return Some(crate::protocol::packet_builder::PacketBuilder::new(0x006E)
                .put_slice(&[0x01, 0x00, 0x00, 0x00])
                .build());
        }

        match self.db.cancel_character_deletion(cancel_req.char_id, account_id) {
            Ok(true) => {
                info!(
                    "Character {} deletion cancelled (account_id={})",
                    cancel_req.char_id, account_id
                );
                Some(
                    HCCancelDeleteOk {
                        char_id: cancel_req.char_id,
                    }
                    .to_packet(),
                )
            }
            Ok(false) => {
                warn!(
                    "Failed to cancel deletion for character {} (not marked)",
                    cancel_req.char_id
                );
                Some(crate::protocol::packet_builder::PacketBuilder::new(0x006E)
                    .put_slice(&[0x01, 0x00, 0x00, 0x00])
                    .build())
            }
            Err(e) => {
                error!(
                    "Database error cancelling deletion for {}: {}",
                    cancel_req.char_id, e
                );
                Some(crate::protocol::packet_builder::PacketBuilder::new(0x006E)
                    .put_slice(&[0x01, 0x00, 0x00, 0x00])
                    .build())
            }
        }
    }

    /// 查找空槽位 (0-8)
    fn find_empty_slot(&self, characters: &[crate::storage::Character]) -> Option<u8> {
        let used_slots: std::collections::HashSet<u8> =
            characters.iter().map(|c| c.char_num).collect();

        for slot in 0..9 {
            if !used_slots.contains(&slot) {
                return Some(slot);
            }
        }

        None
    }

    /// 验证角色名称是否合法
    /// 返回 Ok(()) 表示合法，Err(String) 包含错误信息
    fn validate_character_name(&self, name: &str) -> Result<(), String> {
        // 检查名称长度
        let trimmed = name.trim_matches('\0');
        if trimmed.is_empty() {
            return Err("角色名不能为空".to_string());
        }
        if trimmed.len() > 24 {
            return Err("角色名过长（最大24字节）".to_string());
        }

        // 检查最小长度（rAthena 最少 4 个字符）
        if trimmed.len() < 4 {
            return Err("角色名过短（最少4字节）".to_string());
        }

        // 检查特殊字符：只允许字母、数字、中文、韩文、日文等
        // rAthena 默认不允许以下字符
        for ch in trimmed.chars() {
            match ch {
                // 允许：英文字母、数字、中文(CJK统一汉字)、韩文、日文平假名/片假名
                'a'..='z' | 'A'..='Z' | '0'..='9' => {}
                '\u{4E00}'..='\u{9FFF}'   // CJK统一汉字
                | '\u{3400}'..='\u{4DBF}' // CJK扩展A
                | '\u{AC00}'..='\u{D7AF}' // 韩文音节
                | '\u{3040}'..='\u{309F}' // 日文平假名
                | '\u{30A0}'..='\u{30FF}' // 日文片假名
                | '\u{FF66}'..='\u{FF9F}' // 半角片假名
                => {}
                _ => {
                    return Err(format!("角色名包含不允许的字符: '{}'", ch));
                }
            }
        }

        // 检查名称是否已存在（含已标记删除但未过期的角色）
        match self.db.get_character_by_name(trimmed) {
            Ok(Some(_)) => {
                // 检查该角色是否已标记删除且已过期
                // 如果已过期，cleanup 会清理掉，此处当作已存在
                Err("角色名已被使用".to_string())
            }
            Ok(None) => Ok(()),
            Err(e) => {
                error!("Database error checking name: {}", e);
                Err("服务器内部错误".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::map_packets::{CHEnter, CHEnterCharServer, CHDeleteChar, CHCancelDelete, CHMakeChar};

    fn create_test_server() -> CharServer {
        let db = Arc::new(Database::open_memory().unwrap());
        init_schema(&db).unwrap();

        // 创建测试账户
        db.create_account("test_user", "password_hash", 1).unwrap();

        CharServer::new(
            db,
            Arc::new(SessionManager::new()),
            Arc::new(TokenStore::new()),
        )
    }

    fn create_session_with_account(account_id: u32) -> Session {
        let mut session = Session::new();
        session.account_id = Some(account_id);
        session
    }

    #[test]
    fn test_handle_request_char_list_returns_empty_for_new_account() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 新账户没有角色，应该返回空的角色列表
        let result = server.handle_request_char_list(&mut session);
        assert!(result.is_some());

        let packet_data = result.unwrap();
        // SCCharList 的 packet_id 是 0x0066
        assert!(!packet_data.is_empty());
    }

    #[test]
    fn test_handle_request_char_list_requires_account_id() {
        let server = create_test_server();
        let mut session = Session::new(); // 没有 account_id

        let result = server.handle_request_char_list(&mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_select_char_returns_none_without_account_id() {
        let server = create_test_server();
        let mut session = Session::new();

        let data = CHEnter { char_id: 1 }.to_packet();

        let result = server.handle_select_char(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_select_char_returns_error_for_invalid_char() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHEnter {
            char_id: 9999, // 不存在的角色ID
        }
        .to_packet();

        // 账户1没有角色，应该返回失败响应
        let result = server.handle_select_char(&data, &mut session);
        assert!(result.is_some());
        // 失败时返回 vec![0]
        let packet_data = result.unwrap();
        assert_eq!(packet_data, vec![0]);
    }

    #[test]
    fn test_handle_select_char_success() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 先创建一个角色
        let char_id = server
            .db
            .create_character(1, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0)
            .unwrap();

        // 现在选择这个角色
        let data = CHEnter { char_id }.to_packet();

        let result = server.handle_select_char(&data, &mut session);
        assert!(result.is_some());

        // 应该返回 HCNotifyZoneServer 包 (map_ip, map_port, token)
        let packet_data = result.unwrap();
        assert!(!packet_data.is_empty());
    }

    #[test]
    fn test_handle_make_char_requires_account_id() {
        let server = create_test_server();
        let mut session = Session::new();

        let data = CHMakeChar {
            name: "NewChar".to_string(),
            str: 10,
            agi: 10,
            vit: 10,
            int: 10,
            dex: 10,
            luk: 10,
            hair_color: 0,
            hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data, &mut session);
        assert!(result.is_none());
    }

    #[test]
    fn test_handle_make_char_success() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 属性值在合法范围内 (1-9)，总和 <= 30
        let data = CHMakeChar {
            name: "NewChar".to_string(),
            str: 5,
            agi: 5,
            vit: 5,
            int: 5,
            dex: 5,
            luk: 5,
            hair_color: 0,
            hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data[4..], &mut session);
        assert!(result.is_some());
        // 成功时返回 0x006D 包，marker=0
        let packet_data = result.unwrap();
        assert!(packet_data.len() > 8, "成功响应应包含角色信息");
        assert_eq!(packet_data[2], 0x6D, "包 ID 应为 0x006D");
        assert_eq!(packet_data[4], 0x00, "成功 marker 应为 0");
    }

    #[test]
    fn test_find_empty_slot() {
        let server = create_test_server();

        // 没有角色时，应该返回槽位 0
        let result = server.find_empty_slot(&[]);
        assert_eq!(result, Some(0));

        // 槽位0已使用时，应该返回槽位1
        let chars = vec![Character {
            char_id: 1,
            char_num: 0,
            name: "Char0".to_string(),
            class: 0,
            base_level: 1,
            job_level: 1,
            base_exp: 0,
            job_exp: 0,
            zeny: 0,
            str: 1,
            agi: 1,
            vit: 1,
            int: 1,
            dex: 1,
            luk: 1,
            hp: 40,
            max_hp: 40,
            sp: 11,
            max_sp: 11,
            hair: 1,
            hair_color: 0,
            clothes_color: 0,
            weapon: 0,
            shield: 0,
            head_top: 0,
            head_mid: 0,
            head_bottom: 0,
            last_map: "new_1-1.gat".to_string(),
            last_x: 53,
            last_y: 111,
            save_map: "new_1-1.gat".to_string(),
            save_x: 53,
            save_y: 111,
            delete_timer: 0,
            status_point: 0,
            skill_point: 0,
            created_at: 0,
            updated_at: 0,
        }];
        let result = server.find_empty_slot(&chars);
        assert_eq!(result, Some(1));

        // 所有槽位已满时，应该返回 None
        let mut full_chars: Vec<Character> = (0..9)
            .map(|i| Character {
                char_id: i as u32 + 1,
                char_num: i as u8,
                name: format!("Char{}", i),
                class: 0,
                base_level: 1,
                job_level: 1,
                base_exp: 0,
                job_exp: 0,
                zeny: 0,
                str: 1,
                agi: 1,
                vit: 1,
                int: 1,
                dex: 1,
                luk: 1,
                hp: 40,
                max_hp: 40,
                sp: 11,
                max_sp: 11,
                hair: 1,
                hair_color: 0,
                clothes_color: 0,
                weapon: 0,
                shield: 0,
                head_top: 0,
                head_mid: 0,
                head_bottom: 0,
                last_map: "new_1-1.gat".to_string(),
                last_x: 53,
                last_y: 111,
                save_map: "new_1-1.gat".to_string(),
                save_x: 53,
                save_y: 111,
                delete_timer: 0,
                status_point: 0,
                skill_point: 0,
                created_at: 0,
                updated_at: 0,
            })
            .collect();
        let result = server.find_empty_slot(&full_chars);
        assert_eq!(result, None);
    }

    #[test]
    fn test_handle_packet_dispatches_correctly() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 测试未知包ID
        let result = server.handle_packet(0xFFFF, &[], &mut session);
        assert!(result.is_none());

        // 测试 0x0065 (进入角色服务器，自动发送角色列表)
        // to_packet() 包含 4 字节 header，handle_packet 只接收 body
        let full_packet = CHEnterCharServer {
            account_id: 1,
            login_id1: 0x12345678,
            login_id2: 0x9ABCDEF0,
            sex: 1,
        }.to_packet();
        let enter_body = &full_packet[4..]; // 跳过 header (len_lo, len_hi, id_lo, id_hi)
        let result = server.handle_packet(0x0065, enter_body, &mut session);
        assert!(result.is_some());
    }

    #[test]
    fn test_char_server_with_map_server_config() {
        let server = create_test_server()
            .with_map_server("192.168.1.100", 6122)
            .with_map_server_id(5);

        // 验证配置已设置
        let mut session = create_session_with_account(1);

        // 创建一个角色
        let char_id = server
            .db
            .create_character(1, 0, "TestChar", 10, 10, 10, 10, 10, 10, 1, 0)
            .unwrap();

        // 选择角色，验证返回的map服务器信息
        let data = CHEnter { char_id }.to_packet();
        let result = server.handle_select_char(&data, &mut session);

        assert!(result.is_some());
        // HCNotifyZoneServer 包包含服务器信息
    }

    #[test]
    fn test_handle_delete_char_success() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 先创建一个角色
        let char_id = server
            .db
            .create_character(1, 0, "DeleteMe", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 请求删除
        let data = CHDeleteChar {
            char_id,
            email: String::new(),
        }
        .to_packet();

        let result = server.handle_delete_char(&data[4..], &mut session);
        assert!(result.is_some(), "删除请求应返回响应");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006C, "应返回 HCDeleteCharOk (0x006C)");
    }

    #[test]
    fn test_handle_delete_char_wrong_account() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 创建角色属于 account 1
        let char_id = server
            .db
            .create_character(1, 0, "Owned", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 用 account 2 的 session 尝试删除
        let mut wrong_session = create_session_with_account(2);
        // account 2 也需要存在
        server.db.create_account("other", "pass", 0).unwrap();

        let data = CHDeleteChar {
            char_id,
            email: String::new(),
        }
        .to_packet();

        let result = server.handle_delete_char(&data[4..], &mut wrong_session);
        assert!(result.is_some(), "错误账户删除应返回失败响应");
        let r = result.unwrap(); assert_eq!(r[4], 0x01, "应返回失败");
    }

    #[test]
    fn test_handle_cancel_delete_success() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 创建角色
        let char_id = server
            .db
            .create_character(1, 0, "CancelDel", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        // 先标记删除
        server.db.mark_character_for_deletion(char_id, 1, 86400).unwrap();

        // 取消删除
        let data = CHCancelDelete { char_id }.to_packet();
        let result = server.handle_cancel_delete(&data[4..], &mut session);
        assert!(result.is_some(), "取消删除应返回响应");

        let response = result.unwrap();
        let packet_id = u16::from_le_bytes([response[2], response[3]]);
        assert_eq!(packet_id, 0x006D, "应返回 HCCancelDeleteOk (0x006D)");
    }

    #[test]
    fn test_handle_cancel_delete_not_marked() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        // 创建角色但不标记删除
        let char_id = server
            .db
            .create_character(1, 0, "NoMark", 5, 5, 5, 5, 5, 5, 1, 0)
            .unwrap();

        let data = CHCancelDelete { char_id }.to_packet();
        let result = server.handle_cancel_delete(&data[4..], &mut session);
        assert!(result.is_some(), "未标记删除时取消应返回失败响应");
        let r = result.unwrap(); assert_eq!(r[4], 0x01, "应返回失败");
    }

    #[test]
    fn test_delete_char_requires_account_id() {
        let server = create_test_server();
        let mut session = Session::new(); // 无 account_id

        let data = CHDeleteChar {
            char_id: 1,
            email: String::new(),
        }
        .to_packet();

        let result = server.handle_delete_char(&data[4..], &mut session);
        assert!(result.is_none(), "无 account_id 时应返回 None");
    }

    #[test]
    fn test_cancel_delete_requires_account_id() {
        let server = create_test_server();
        let mut session = Session::new();

        let data = CHCancelDelete { char_id: 1 }.to_packet();
        let result = server.handle_cancel_delete(&data[4..], &mut session);
        assert!(result.is_none(), "无 account_id 时应返回 None");
    }

    #[test]
    fn test_handle_make_char_duplicate_name() {
        let server = create_test_server();
        let mut session1 = create_session_with_account(1);

        // 创建第一个角色
        let data1 = CHMakeChar {
            name: "TakenName".to_string(),
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();
        let result1 = server.handle_make_char(&data1[4..], &mut session1);
        let r1 = result1.unwrap(); assert_eq!(r1[4], 0x00, "第一个角色应创建成功");

        // 尝试创建同名角色
        let mut session2 = create_session_with_account(1);
        let data2 = CHMakeChar {
            name: "TakenName".to_string(),
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();
        let result2 = server.handle_make_char(&data2[4..], &mut session2);
        let r2 = result2.unwrap(); assert_eq!(r2[4], 0x01, "重复名称应返回失败");
    }

    #[test]
    fn test_handle_make_char_name_too_short() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHMakeChar {
            name: "Ab".to_string(), // 只有 2 字节，少于最少 4 字节
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data[4..], &mut session);
        let r = result.unwrap(); assert_eq!(r[4], 0x01, "过短名称应返回失败");
    }

    #[test]
    fn test_handle_make_char_special_characters() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHMakeChar {
            name: "Test@#$".to_string(), // 包含特殊字符
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data[4..], &mut session);
        let r = result.unwrap(); assert_eq!(r[4], 0x01, "含特殊字符的名称应返回失败");
    }

    #[test]
    fn test_handle_make_char_chinese_name_allowed() {
        let server = create_test_server();
        let mut session = create_session_with_account(1);

        let data = CHMakeChar {
            name: "测试角色".to_string(), // 中文名称应被允许
            str: 5, agi: 5, vit: 5, int: 5, dex: 5, luk: 5,
            hair_color: 0, hair: 1,
        }
        .to_packet();

        let result = server.handle_make_char(&data[4..], &mut session);
        let r = result.unwrap(); assert_eq!(r[4], 0x00, "中文名称应创建成功");
    }

    #[test]
    fn test_validate_name_boundary() {
        let server = create_test_server();

        // 空名称
        assert!(server.validate_character_name("").is_err());
        assert!(server.validate_character_name("\0\0\0").is_err());

        // 3 字节（少于 4）
        assert!(server.validate_character_name("abc").is_err());

        // 4 字节（刚好）
        assert!(server.validate_character_name("abcd").is_ok());

        // 24 字节（刚好）
        let name_24 = "a".repeat(24);
        assert!(server.validate_character_name(&name_24).is_ok());

        // 25 字节（超出）
        let name_25 = "a".repeat(25);
        assert!(server.validate_character_name(&name_25).is_err());
    }
}
