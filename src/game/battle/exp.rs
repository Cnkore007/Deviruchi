use uuid::Uuid;
use crate::game::map::MapState;
use crate::game::party::{PartyManager, ExpShareMode};

/// 经验值分配器
pub struct ExpDistributor;

impl ExpDistributor {
    /// 分发怪物经验给击杀者及其队伍
    pub fn distribute_mob_exp(
        map_state: &MapState,
        party_manager: &PartyManager,
        killer_id: Uuid,
        mob_level: u16,
        mob_base_exp: u64,
        mob_job_exp: u64,
    ) {
        // 检查击杀者是否在队伍中
        let party = party_manager.get_player_party(&killer_id);

        match party {
            Some(party) if matches!(party.exp_share, ExpShareMode::Equal | ExpShareMode::LevelBased) => {
                // 队伍经验分享
                Self::distribute_party_exp(
                    map_state,
                    &party,
                    killer_id,
                    mob_level,
                    mob_base_exp,
                    mob_job_exp,
                );
            }
            _ => {
                // 单人经验
                Self::give_exp_to_player(map_state, killer_id, mob_level, mob_base_exp, mob_job_exp);
            }
        }
    }

    /// 组队经验分配
    fn distribute_party_exp(
        map_state: &MapState,
        party: &crate::game::party::Party,
        killer_id: Uuid,
        mob_level: u16,
        mob_base_exp: u64,
        mob_job_exp: u64,
    ) {
        // 收集同地图的在线队员
        let mut nearby: Vec<(Uuid, u16)> = Vec::new();
        for member in &party.members {
            if !member.online || member.player_id == killer_id {
                continue;
            }
            if let Some(player) = map_state.get_player(&member.player_id) {
                nearby.push((member.player_id, *player.base_level.read()));
            }
        }

        // 队伍人数（含击杀者）
        let party_size = (nearby.len() + 1) as u64;

        // 基础经验平分
        let base_per_member = mob_base_exp / party_size;
        let job_per_member = mob_job_exp / party_size;

        match party.exp_share {
            ExpShareMode::Equal => {
                // 等额分配
                for (member_id, _) in &nearby {
                    Self::give_exp_to_player(map_state, *member_id, mob_level, base_per_member, job_per_member);
                }
                Self::give_exp_to_player(map_state, killer_id, mob_level, base_per_member, job_per_member);
            }
            ExpShareMode::LevelBased => {
                // 按等级加权分配
                let killer = map_state.get_player(&killer_id);
                let killer_level = killer.map(|p| *p.base_level.read()).unwrap_or(1) as u64;

                let total_level: u64 = killer_level + nearby.iter().map(|(_, lvl)| *lvl as u64).sum::<u64>();
                if total_level == 0 {
                    return;
                }

                for (member_id, level) in &nearby {
                    let share = mob_base_exp * (*level as u64) / total_level;
                    let job_share = mob_job_exp * (*level as u64) / total_level;
                    Self::give_exp_to_player(map_state, *member_id, mob_level, share, job_share);
                }

                let killer_share = mob_base_exp * killer_level / total_level;
                let killer_job_share = mob_job_exp * killer_level / total_level;
                Self::give_exp_to_player(map_state, killer_id, mob_level, killer_share, killer_job_share);
            }
            _ => unreachable!(),
        }
    }

    /// 给单个玩家经验（含等级惩罚）
    fn give_exp_to_player(
        map_state: &MapState,
        player_id: Uuid,
        mob_level: u16,
        base_exp: u64,
        job_exp: u64,
    ) {
        let player = match map_state.get_player(&player_id) {
            Some(p) => p,
            None => return,
        };

        let player_level = *player.base_level.read() as i32;
        let mob_level = mob_level as i32;
        let level_diff = player_level - mob_level;

        // 等级惩罚系数
        let penalty = if level_diff <= 10 {
            1.0
        } else if level_diff <= 15 {
            0.75
        } else if level_diff <= 20 {
            0.5
        } else if level_diff <= 25 {
            0.25
        } else {
            0.1
        };

        let adjusted_base = (base_exp as f64 * penalty) as u64;
        let adjusted_job = (job_exp as f64 * penalty) as u64;

        // 通过 MapState 原子更新玩家经验
        map_state.add_player_base_exp(&player_id, adjusted_base);
        map_state.add_player_job_exp(&player_id, adjusted_job);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::map::MapState;
    use crate::game::party::PartyManager;
    use std::sync::Arc;

    fn make_test_state() -> (Arc<MapState>, Arc<PartyManager>) {
        (Arc::new(MapState::new()), Arc::new(PartyManager::new()))
    }

    #[test]
    fn test_solo_exp_distribution() {
        let (map_state, party_manager) = make_test_state();
        let player = crate::game::map::Player::from_character(crate::storage::Character {
            char_id: 1,
            char_num: 0,
            name: "Test".to_string(),
            class: 0,
            base_level: 10,
            job_level: 5,
            base_exp: 0,
            job_exp: 0,
            zeny: 0,
            str: 1, agi: 1, vit: 1, int: 1, dex: 1, luk: 1,
            hp: 100, max_hp: 100, sp: 50, max_sp: 50,
            hair: 0, hair_color: 0, clothes_color: 0,
            weapon: 0, shield: 0, head_top: 0, head_mid: 0, head_bottom: 0,
            last_map: "test".to_string(), last_x: 0, last_y: 0,
            delete_timer: 0, created_at: 0, updated_at: 0,
        });
        let player_id = player.id;
        map_state.add_player(player);

        ExpDistributor::distribute_mob_exp(
            &map_state, &party_manager, player_id,
            5, 100, 50,
        );

        let p = map_state.get_player(&player_id).unwrap();
        // level 10 vs mob 5: diff = 5, within 10 -> no penalty
        assert_eq!(*p.base_exp.read(), 100);
        assert_eq!(*p.job_exp.read(), 50);
    }

    #[test]
    fn test_level_penalty_reduces_exp() {
        let (map_state, party_manager) = make_test_state();
        let player = crate::game::map::Player::from_character(crate::storage::Character {
            char_id: 2,
            char_num: 0,
            name: "High".to_string(),
            class: 0,
            base_level: 30,
            job_level: 20,
            base_exp: 0,
            job_exp: 0,
            zeny: 0,
            str: 1, agi: 1, vit: 1, int: 1, dex: 1, luk: 1,
            hp: 100, max_hp: 100, sp: 50, max_sp: 50,
            hair: 0, hair_color: 0, clothes_color: 0,
            weapon: 0, shield: 0, head_top: 0, head_mid: 0, head_bottom: 0,
            last_map: "test".to_string(), last_x: 0, last_y: 0,
            delete_timer: 0, created_at: 0, updated_at: 0,
        });
        let player_id = player.id;
        map_state.add_player(player);

        ExpDistributor::distribute_mob_exp(
            &map_state, &party_manager, player_id,
            5, 100, 100,
        );

        let p = map_state.get_player(&player_id).unwrap();
        // level 30 vs mob 5: diff = 25 -> penalty 0.25
        assert_eq!(*p.base_exp.read(), 25);
        assert_eq!(*p.job_exp.read(), 25);
    }
}
