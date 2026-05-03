use std::sync::Arc;
use crate::core::Config;
use crate::game::map::{MapState, Player, PlayerState};

#[derive(Clone)]
pub struct HealService {
    config: Arc<Config>,
}

impl HealService {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn start(&self, map_state: Arc<MapState>) {
        let interval_ms = self.config.battle.natural_heal_interval_ms;
        let service = self.clone();
        let ms = map_state.clone();

        crate::core::timer::Timer::add_interval(
            std::time::Duration::from_millis(interval_ms),
            move || {
                service.process_heal(&ms);
            }
        );

        tracing::info!("HealService started with interval {}ms", interval_ms);
    }

    fn process_heal(&self, map_state: &MapState) {
        let threshold_hp = self.config.battle.natural_heal_threshold_hp;
        let threshold_sp = self.config.battle.natural_heal_threshold_sp;

        // 获取所有地图上的玩家
        let map_names: Vec<String> = {
            let players = map_state.players.read();
            players.values().map(|p| p.map_name.clone()).collect()
        };
        let unique_maps: Vec<String> = map_names.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();

        for map_name in unique_maps {
            let players = map_state.get_players_on_map(&map_name);

            for player in players {
                // 跳过死亡玩家
                if *player.state.read() == PlayerState::Dead {
                    continue;
                }

                let is_sitting = player.is_sitting();
                let current_hp = *player.hp.read();
                let max_hp = *player.max_hp.read();
                let current_sp = *player.sp.read();
                let max_sp = *player.max_sp.read();

                // HP 回复
                if current_hp < max_hp {
                    let hp_threshold = (max_hp * threshold_hp) / 100;
                    if current_hp >= hp_threshold {
                        let heal = self.calculate_hp_heal(&player, is_sitting);
                        let new_hp = (current_hp + heal).min(max_hp);
                        *player.hp.write() = new_hp;
                        tracing::trace!("Player {} healed {} HP (sitting: {})", player.name, heal, is_sitting);
                    }
                }

                // SP 回复
                if current_sp < max_sp {
                    let sp_threshold = (max_sp * threshold_sp) / 100;
                    if current_sp >= sp_threshold {
                        let heal = self.calculate_sp_heal(&player, is_sitting);
                        let new_sp = (current_sp + heal).min(max_sp);
                        *player.sp.write() = new_sp;
                        tracing::trace!("Player {} healed {} SP (sitting: {})", player.name, heal, is_sitting);
                    }
                }
            }
        }
    }

    pub fn calculate_hp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
        let vit = *player.vit.read() as u32;
        let max_hp = *player.max_hp.read();

        // 基础回复: 1 + VIT/2 + max_hp/200
        let base_heal = 1 + (vit / 2) + (max_hp / 200);

        // 百分比回复
        let rate = if is_sitting {
            self.config.battle.sit_heal_hp_rate
        } else {
            self.config.battle.natural_heal_hp_rate
        };
        let rate_heal = (max_hp * rate) / 100;

        base_heal + rate_heal
    }

    pub fn calculate_sp_heal(&self, player: &Player, is_sitting: bool) -> u32 {
        let int = *player.int.read() as u32;
        let max_sp = *player.max_sp.read();

        // 基础回复: 1 + INT/2 + max_sp/100
        let base_heal = 1 + (int / 2) + (max_sp / 100);

        // 百分比回复
        let rate = if is_sitting {
            self.config.battle.sit_heal_sp_rate
        } else {
            self.config.battle.natural_heal_sp_rate
        };
        let rate_heal = (max_sp * rate) / 100;

        base_heal + rate_heal
    }
}
