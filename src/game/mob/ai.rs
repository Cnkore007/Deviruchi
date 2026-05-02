use std::sync::Arc;
use crate::game::mob::{Mob, MobAIState, MobSpawnManager};
use crate::game::map::MapState;

/// 怪物AI处理器
pub struct MobAI {
    spawn_manager: Arc<MobSpawnManager>,
}

impl MobAI {
    pub fn new(spawn_manager: Arc<MobSpawnManager>) -> Self {
        Self { spawn_manager }
    }

    /// 更新怪物AI
    pub fn update(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let state = *mob.ai_state.read();

        match state {
            MobAIState::Idle => self.update_idle(mob, map_state),
            MobAIState::Patrol => self.update_patrol(mob),
            MobAIState::Chase => self.update_chase(mob, map_state),
            MobAIState::Attack => self.update_attack(mob, map_state),
            MobAIState::Return => self.update_return(mob),
            MobAIState::Dead => self.update_dead(mob),
        }
    }

    fn update_idle(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let (x, y) = mob.get_position();
        let players = map_state.get_players_on_map(&mob.map_name);

        for player in players {
            let (px, py) = player.get_position();
            let distance = Self::calculate_distance(x, y, px, py);

            if distance <= mob.sight_range as u16 {
                *mob.ai_state.write() = MobAIState::Chase;
                *mob.target_id.write() = Some(player.id);
                return;
            }
        }

        if rand_simple() < 5 {
            let new_x = (x as i32 + rand_offset(-3, 3)).max(0) as u16;
            let new_y = (y as i32 + rand_offset(-3, 3)).max(0) as u16;
            mob.move_to(new_x, new_y);
        }
    }

    fn update_patrol(&self, mob: &Arc<Mob>) {
        let (x, y) = mob.get_position();
        let new_x = (x as i32 + rand_offset(-5, 5)).max(0) as u16;
        let new_y = (y as i32 + rand_offset(-5, 5)).max(0) as u16;
        mob.move_to(new_x, new_y);
    }

    fn update_chase(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let target_id = mob.target_id.read().clone();

        if let Some(target_id) = target_id {
            if let Some(target) = map_state.get_player(&target_id) {
                let (x, y) = mob.get_position();
                let (tx, ty) = target.get_position();
                let distance = Self::calculate_distance(x, y, tx, ty);

                if distance <= mob.atk_range {
                    *mob.ai_state.write() = MobAIState::Attack;
                } else if distance > mob.chase_range {
                    *mob.ai_state.write() = MobAIState::Return;
                    *mob.target_id.write() = None;
                } else {
                    let new_x = Self::approach(x, tx);
                    let new_y = Self::approach(y, ty);
                    mob.move_to(new_x, new_y);
                }
            } else {
                *mob.ai_state.write() = MobAIState::Return;
                *mob.target_id.write() = None;
            }
        }
    }

    fn update_attack(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let target_id = mob.target_id.read().clone();

        if let Some(target_id) = target_id {
            if let Some(target) = map_state.get_player(&target_id) {
                let damage = mob.atk as i32 - (*target.str.read() as i32 / 2);
                let damage = damage.max(1) as u32;

                // 直接操作 Player 的 hp RwLock
                let current_hp = *target.hp.read();
                if current_hp <= damage {
                    *target.hp.write() = 0;
                } else {
                    *target.hp.write() = current_hp - damage;
                }

                if *target.hp.read() == 0 {
                    *mob.ai_state.write() = MobAIState::Idle;
                    *mob.target_id.write() = None;
                }
            }
        }
    }

    fn update_return(&self, mob: &Arc<Mob>) {
        *mob.ai_state.write() = MobAIState::Idle;
    }

    fn update_dead(&self, mob: &Arc<Mob>) {
        self.spawn_manager.unregister_mob(&mob.map_name, &mob.id);
    }

    fn calculate_distance(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
        let dx = (x1 as i32 - x2 as i32).abs();
        let dy = (y1 as i32 - y2 as i32).abs();
        ((dx * dx + dy * dy) as f32).sqrt() as u16
    }

    fn approach(current: u16, target: u16) -> u16 {
        if current < target {
            (current + 1).min(target)
        } else {
            current.saturating_sub(1)
        }
    }
}

fn rand_simple() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 100) as i32
}

fn rand_offset(min: i32, max: i32) -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let range = max - min + 1;
    min + ((nanos as i32) % range)
}
