//! 角色选择业务逻辑

use std::sync::Arc;
use tracing::{info, warn, error};

use crate::protocol::map_packets::{SCCharList, CHEnter, CHMakeChar, CharInfo, HCNotifyZoneServer};
use crate::protocol::packet_builder::Packed;
use crate::storage::Database;
use crate::network::session::{SessionManager, Session};
use crate::game::token::TokenStore;

/// 角色服务器
pub struct CharServer {
    db: Arc<Database>,
    #[allow(dead_code)]
    session_manager: Arc<SessionManager>,
    token_store: Arc<TokenStore>,
    map_ip: String,
    map_port: u16,
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
        }
    }

    pub fn with_map_server(mut self, ip: &str, port: u16) -> Self {
        self.map_ip = ip.to_string();
        self.map_port = port;
        self
    }

    /// 根据 packet_id 分发处理
    pub fn handle_packet(&self, packet_id: u16, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        match packet_id {
            0x0066 => self.handle_request_char_list(session),
            0x0067 => self.handle_make_char(data, session),
            0x0065 => self.handle_select_char(data, session),
            _ => {
                warn!("Unknown char packet id: 0x{:04X}", packet_id);
                None
            }
        }
    }

    /// 处理请求角色列表 (0x0066)
    fn handle_request_char_list(&self, session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        info!("Request char list for account_id={}", account_id);

        let characters = self.db.get_characters_by_account(account_id).ok()?;

        let char_infos: Vec<CharInfo> = characters
            .iter()
            .map(|c| self.db.character_to_char_info(c))
            .collect();

        info!("Sending {} characters for account_id={}", char_infos.len(), account_id);

        Some(SCCharList { characters: char_infos }.to_packet())
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

        // 创建角色
        match self.db.create_character(
            account_id,
            slot,
            &make_char.name,
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
                info!("Character created: char_id={}, name={}", char_id, make_char.name);
                Some(vec![0]) // 成功响应
            }
            Err(e) => {
                error!("Failed to create character: {}", e);
                Some(vec![0x00]) // 失败响应
            }
        }
    }

    /// 处理选择角色进入 (0x0065)
    fn handle_select_char(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let account_id = session.account_id?;

        // 解析选择角色包
        let enter = CHEnter::from_slice(data)?;

        info!(
            "Select char request: char_id={}, account_id={}",
            enter.char_id, account_id
        );

        // 验证角色是否属于该账户
        let characters = self.db.get_characters_by_account(account_id).ok()?;
        let valid_char = characters.iter().any(|c| c.char_id == enter.char_id);

        if !valid_char {
            warn!("Invalid char selection: char_id={} not owned by account_id={}", enter.char_id, account_id);
            return Some(vec![0]); // 失败响应
        }

        // 设置 session.char_id
        session.char_id = Some(enter.char_id);

        // Generate one-time token for Char→Map transition
        let token = self.token_store.create(account_id, enter.char_id);

        info!(
            "Character selected: char_id={}, token generated for map {}:{}",
            enter.char_id, self.map_ip, self.map_port
        );

        // Return HCNotifyZoneServer with map server info
        Some(HCNotifyZoneServer {
            map_ip: self.map_ip.clone(),
            map_port: self.map_port,
            token,
        }.to_packet())
    }

    /// 查找空槽位 (0-8)
    fn find_empty_slot(&self, characters: &[crate::storage::Character]) -> Option<u8> {
        let used_slots: std::collections::HashSet<u8> = characters
            .iter()
            .map(|c| c.char_num)
            .collect();

        for slot in 0..9 {
            if !used_slots.contains(&slot) {
                return Some(slot);
            }
        }

        None
    }
}
