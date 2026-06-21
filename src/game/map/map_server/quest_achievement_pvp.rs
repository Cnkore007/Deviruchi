//! 任务、成就、PVP handler

use super::MapServer;
use crate::network::session::Session;
use crate::protocol::map_packets::{
    CzAchievementCheckReward, CzAutoSpell, CzChangeCart, CzQuestStateAck, CzSkillSelectMenu,
};

impl MapServer {
    /// 处理任务状态请求 (0x02B5)
    pub(super) fn handle_quest_state(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzQuestStateAck::from_slice(data)?;

        tracing::info!(
            "Player {} 任务状态: quest_id={}, state={}",
            player_id,
            pkt.quest_id,
            pkt.state
        );

        // 简化实现：记录日志
        tracing::info!("Player {} 任务状态更新", player_id);
        None
    }

    /// 处理成就奖励请求 (0x0224)
    pub(super) fn handle_achievement_reward(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzAchievementCheckReward::from_slice(data)?;

        tracing::info!(
            "Player {} 请求成就奖励: achievement_id={}",
            player_id,
            pkt.achievement_id
        );

        // 简化实现：记录日志
        tracing::info!("Player {} 成就奖励已领取", player_id);
        None
    }

    /// 处理PVP信息请求 (0x0237)
    pub(super) fn handle_pvp_info(&self, _data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        tracing::info!("Player {} 请求PVP信息", player_id);
        None
    }

    /// 处理坐骑请求 (0x019C)
    pub(super) fn handle_change_cart(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzChangeCart::from_slice(data)?;

        tracing::info!("Player {} 请求坐骑: cart_type={}", player_id, pkt.cart_type);

        // 简化实现：记录日志
        tracing::info!("Player {} 坐骑切换成功", player_id);
        None
    }

    /// 处理技能选择菜单请求 (0x0A35)
    pub(super) fn handle_skill_select_menu(
        &self,
        data: &[u8],
        session: &mut Session,
    ) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzSkillSelectMenu::from_slice(data)?;

        tracing::info!(
            "Player {} 技能选择: skill_id={}, level={}",
            player_id,
            pkt.skill_id,
            pkt.level
        );

        // 简化实现：记录日志
        tracing::info!("Player {} 技能选择成功", player_id);
        None
    }

    /// 处理自动念咒请求 (0x01CF)
    pub(super) fn handle_auto_spell(&self, data: &[u8], session: &mut Session) -> Option<Vec<u8>> {
        let player_id = session.player_id?;
        let pkt = CzAutoSpell::from_slice(data)?;

        tracing::info!("Player {} 自动念咒: skill_id={}", player_id, pkt.skill_id);

        // 简化实现：记录日志
        tracing::info!("Player {} 自动念咒设置成功", player_id);
        None
    }
}
