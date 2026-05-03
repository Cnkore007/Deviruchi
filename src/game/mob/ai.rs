use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use crate::game::mob::{Mob, MobAIState, MobSpawnManager};
use crate::game::mob::pathfinder::Pathfinder;
use crate::game::map::{MapState, ChannelBus, GameEvent, DropManager};
use crate::game::map::data::MapDatabase;
use crate::game::party::PartyManager;
use crate::game::battle::ExpDistributor;

/// 怪物AI处理器
pub struct MobAI {
    spawn_manager: Arc<MobSpawnManager>,
    channel_bus: Arc<ChannelBus>,
    drop_manager: Arc<DropManager>,
    party_manager: Arc<PartyManager>,
    map_database: Arc<MapDatabase>,
}

impl MobAI {
    pub fn new(
        spawn_manager: Arc<MobSpawnManager>,
        channel_bus: Arc<ChannelBus>,
        drop_manager: Arc<DropManager>,
        party_manager: Arc<PartyManager>,
        map_database: Arc<MapDatabase>,
    ) -> Self {
        Self { spawn_manager, channel_bus, drop_manager, party_manager, map_database }
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
            MobAIState::Dead => self.update_dead(mob, map_state),
        }
    }

    fn update_idle(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let (x, y) = mob.get_position();
        let players = map_state.get_players_on_map(&mob.spawn_map);

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
                    mob.path_manager.write().stop_chase();
                } else {
                    // 使用 MobPathManager 寻路
                    let mut pm = mob.path_manager.write();

                    // 检查是否需要重新计算路径
                    let needs_recalc = !pm.is_chasing
                        || pm.target_pos != Some((tx, ty))
                        || pm.path_invalid;

                    if needs_recalc {
                        // 获取地图数据
                        if let Some(map_data) = self.map_database.get(&mob.map_name) {
                            // 创建可通行性检查闭包
                            let is_walkable = |wx: u16, wy: u16| -> bool {
                                map_data.is_walkable(wx, wy)
                            };

                            // 计算新路径
                            if let Some(path) = Pathfinder::find_path(
                                (x, y),
                                (tx, ty),
                                is_walkable,
                                mob.chase_range,
                            ) {
                                pm.start_chase((tx, ty));
                                pm.set_path(path);
                            } else {
                                // 无法找到路径，停止追击
                                pm.stop_chase();
                                drop(pm);
                                *mob.ai_state.write() = MobAIState::Return;
                                *mob.target_id.write() = None;
                                return;
                            }
                        } else {
                            // 没有地图数据，使用简单追击
                            pm.stop_chase();
                        }
                    }

                    // 沿路径移动一步
                    let mut pm = mob.path_manager.write();
                    if let Some(next_pos) = pm.advance_step() {
                        let (nx, ny) = next_pos;
                        // 验证下一步可通行（双重检查）
                        if let Some(map_data) = self.map_database.get(&mob.map_name) {
                            if map_data.is_walkable(nx, ny) {
                                mob.move_to(nx, ny);
                            } else {
                                // 路径失效，需要下次重算
                                pm.invalidate();
                            }
                        }
                    }
                }
            } else {
                *mob.ai_state.write() = MobAIState::Return;
                *mob.target_id.write() = None;
                mob.path_manager.write().stop_chase();
            }
        }
    }

    fn update_attack(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let target_id = mob.target_id.read().clone();

        if let Some(target_id) = target_id {
            if let Some(target) = map_state.get_player(&target_id) {
                let damage = mob.atk as i32 - (*target.str.read() as i32 / 2);
                let damage = damage.max(1) as u32;

                let killed = target.take_damage(damage);

                if killed {
                    // 发布玩家死亡事件
                    let channel_name = format!("map:{}", target.map_name);
                    let event = GameEvent::PlayerDeath {
                        player_id: target_id,
                    };
                    self.channel_bus.publish(&channel_name, &event, vec![]);

                    *mob.ai_state.write() = MobAIState::Idle;
                    *mob.target_id.write() = None;
                }
            } else {
                *mob.ai_state.write() = MobAIState::Return;
                *mob.target_id.write() = None;
            }
        }
    }

    fn update_return(&self, mob: &Arc<Mob>) {
        *mob.ai_state.write() = MobAIState::Idle;
    }

    fn update_dead(&self, mob: &Arc<Mob>, map_state: &MapState) {
        // 首次死亡处理：发布事件 + 掉落 + 经验
        if !*mob.drops_processed.read() {
            *mob.drops_processed.write() = true;

            // 发布 MobDeath 事件
            let killer_id = mob.target_id.read().unwrap_or(Uuid::nil());
            let channel_name = format!("map:{}", mob.spawn_map);
            let event = GameEvent::MobDeath {
                mob_id: mob.id,
                killer_id,
            };
            self.channel_bus.publish(&channel_name, &event, vec![]);

            // 计算并掉落物品
            self.process_drops(mob);

            // 分发经验给击杀者及其队伍
            if !killer_id.is_nil() {
                ExpDistributor::distribute_mob_exp(
                    map_state,
                    &self.party_manager,
                    killer_id,
                    mob.level,
                    mob.base_exp,
                    mob.job_exp,
                );
            }
        }

        // 检查是否可以重生
        let should_respawn = mob.death_time.read().map_or(false, |death_time| {
            Instant::now().duration_since(death_time) >= Duration::from_millis(mob.respawn_time as u64)
        });

        if should_respawn {
            mob.respawn();
        }
    }

    /// 处理怪物掉落
    fn process_drops(&self, mob: &Arc<Mob>) {
        if mob.drops.is_empty() {
            return;
        }

        let (mob_x, mob_y) = mob.get_position();
        let channel_name = format!("map:{}", mob.spawn_map);

        for drop in &mob.drops {
            let roll = rand_u32() % 10000;
            if roll < drop.chance {
                let amount = if drop.min_amount >= drop.max_amount {
                    drop.min_amount
                } else {
                    drop.min_amount + (rand_u32() % ((drop.max_amount - drop.min_amount + 1) as u32)) as u16
                };

                // 添加掉落物到地图
                self.drop_manager.add(
                    drop.item_id,
                    amount,
                    mob_x,
                    mob_y,
                    &mob.spawn_map,
                );

                // 发布 ItemDrop 事件
                let drop_event = GameEvent::ItemDrop {
                    item_id: drop.item_id,
                    x: mob_x,
                    y: mob_y,
                    amount,
                };
                self.channel_bus.publish(&channel_name, &drop_event, vec![]);
            }
        }
    }

    fn calculate_distance(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
        let dx = (x1 as i32 - x2 as i32).abs();
        let dy = (y1 as i32 - y2 as i32).abs();
        ((dx * dx + dy * dy) as f32).sqrt() as u16
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

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos
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
