//! NPC 对话 handler：交互、对话推进、选项、关闭

use super::MapServer;
use crate::game::script::dialogue::{DialogueResponse, NpcDialogueState};
use crate::game::script::parser::parse_script;
use crate::network::session::Session;
use crate::protocol::map_packets::{
    CZContactNpc, CzAckCloseDialog, CzAckNextDialog, CzAckSelectMenu, ZcCloseDialog, ZcMenuList,
    ZcSayDialog, ZcWaitDialog,
};
use crate::protocol::packet_builder::Packed;
use uuid::Uuid;

impl MapServer {
    /// Handle NPC interact (0x0190)
    pub(super) fn handle_npc_interact(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let npc_pkt = CZContactNpc::from_slice(data)?;
        let npc = self.npc_handler.get_npc(npc_pkt.npc_id)?;

        tracing::info!(
            "Player {} interacting with NPC {} ({})",
            player_id,
            npc.id,
            npc.display_name
        );

        // If NPC has a script, start dialogue
        if let Some(script_text) = &npc.script {
            let script_node = parse_script(script_text);
            // 获取玩家数据并构建脚本上下文
            let context = if let Some(player) = self.map_state.get_player(&player_id) {
                player.to_script_context()
            } else {
                tracing::warn!("Player {} not found for NPC dialogue", player_id);
                crate::game::script::commands::ScriptContext::default()
            };
            let dialogue = NpcDialogueState::with_context(player_id, npc.id, script_node, context);
            self.active_dialogues.write().insert(player_id, dialogue);
            return self.advance_dialogue(player_id, npc.id);
        }

        // Otherwise, handle by NPC type
        // Quest 和 CashShop 类型的 NPC 如果没有脚本，显示名称
        // Warp 类型 NPC 直接处理（不需要玩家引用）
        match npc.type_ {
            crate::game::npc::data::NpcType::Shop => {
                let msg = ZcSayDialog {
                    npc_id: npc.id,
                    message: format!("Welcome to {}!", npc.display_name),
                };
                Some(msg.to_packet())
            }
            crate::game::npc::data::NpcType::Warp => {
                // Warp NPC 的目标坐标已存储在 dest_map/dest_x/dest_y
                let dest_map = npc.dest_map.clone().unwrap_or_else(|| npc.map_name.clone());
                tracing::info!(
                    "NPC warp: {} -> {} ({}, {})",
                    npc.id,
                    dest_map,
                    npc.dest_x,
                    npc.dest_y
                );
                Some(ZcCloseDialog { npc_id: npc.id }.to_packet())
            }
            _ => {
                let msg = ZcSayDialog {
                    npc_id: npc.id,
                    message: format!("{}: Hello!", npc.display_name),
                };
                Some(msg.to_packet())
            }
        }
    }

    /// Advance NPC dialogue for a player, returning the appropriate packet.
    /// Processes consecutive script commands (e.g. mes followed by next)
    /// until reaching a command that requires user input or ends the dialogue.
    fn advance_dialogue(&self, player_id: Uuid, npc_id: u32) -> Option<Vec<u8>> {
        let mut dialogues = self.active_dialogues.write();
        let dialogue = dialogues.get_mut(&player_id)?;

        // Process commands in a loop to handle sequences like mes -> next -> mes -> close
        // where `next` produces a Pending that should be skipped for the next mes.
        let mut last_response = dialogue.process();

        loop {
            match &last_response {
                DialogueResponse::Continue => {
                    // Script wants to continue immediately; process next command
                    last_response = dialogue.process();
                }
                DialogueResponse::Pending => {
                    // "next" command - skip and continue processing
                    last_response = dialogue.process();
                }
                _ => break,
            }
        }

        match last_response {
            DialogueResponse::Message(text) => {
                let pkt = ZcSayDialog {
                    npc_id,
                    message: text,
                };
                Some(pkt.to_packet())
            }
            DialogueResponse::Select(options) => {
                let menu_text = options.join(":");
                let pkt = ZcMenuList { npc_id, menu_text };
                Some(pkt.to_packet())
            }
            DialogueResponse::Closed => {
                dialogues.remove(&player_id);
                let pkt = ZcCloseDialog { npc_id };
                Some(pkt.to_packet())
            }
            DialogueResponse::Warp { map, x, y } => {
                dialogues.remove(&player_id);
                tracing::info!("NPC dialogue warp to {} ({}, {})", map, x, y);
                let pkt = ZcCloseDialog { npc_id };
                Some(pkt.to_packet())
            }
            // These are unreachable due to the loop above, but handle defensively
            DialogueResponse::Continue | DialogueResponse::Pending => {
                let pkt = ZcWaitDialog { npc_id };
                Some(pkt.to_packet())
            }
            // 输入请求：使用 WaitDialog 等待玩家输入
            DialogueResponse::Input(_) => {
                // rAthena 使用 ZcEditDlg (0x0142) 请求文本输入
                // 当前使用 WaitDialog 作为简化实现
                let pkt = ZcWaitDialog { npc_id };
                Some(pkt.to_packet())
            }
            // 全服公告
            DialogueResponse::Announce { message, flag } => {
                // 广播消息到所有在线玩家
                tracing::info!("公告 [flag={}]: {}", flag, message);
                // 公告后继续处理脚本
                let pkt = ZcWaitDialog { npc_id };
                Some(pkt.to_packet())
            }
            // 异步延迟
            DialogueResponse::Sleep(_) => {
                // rAthena 使用 sleep 命令实现异步延迟
                // 当前简化为立即继续
                let pkt = ZcWaitDialog { npc_id };
                Some(pkt.to_packet())
            }
        }
    }

    /// Handle NPC next dialog (0x00B9)
    pub(super) fn handle_npc_next(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzAckNextDialog::from_slice(data)?;
        self.advance_dialogue(player_id, pkt.npc_id)
    }

    /// Handle NPC select menu (0x00B8)
    pub(super) fn handle_npc_select(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzAckSelectMenu::from_slice(data)?;

        let mut dialogues = self.active_dialogues.write();
        let dialogue = dialogues.get_mut(&player_id)?;
        dialogue.handle_input(pkt.select as usize);

        drop(dialogues);
        self.advance_dialogue(player_id, pkt.npc_id)
    }

    /// Handle NPC close dialog (0x0146)
    pub(super) fn handle_npc_close(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzAckCloseDialog::from_slice(data)?;
        self.active_dialogues.write().remove(&player_id);
        tracing::debug!("Player {} closed NPC {} dialogue", player_id, pkt.npc_id);
        None
    }
}
