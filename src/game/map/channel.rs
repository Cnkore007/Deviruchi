use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use uuid::Uuid;

static ENTITY_ID_MAP: Lazy<DashMap<Uuid, u32>> = Lazy::new(DashMap::new);
static ENTITY_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

pub fn get_entity_id(uuid: &Uuid) -> u32 {
    *ENTITY_ID_MAP
        .entry(*uuid)
        .or_insert_with(|| ENTITY_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn remove_entity_id(uuid: &Uuid) {
    ENTITY_ID_MAP.remove(uuid);
}

pub fn map_channel_name(map_name: &str) -> String {
    let mut s = String::with_capacity(4 + map_name.len());
    s.push_str("map:");
    s.push_str(map_name);
    s
}

pub fn party_channel_name(party_id: Uuid) -> String {
    let id_str = party_id.to_string();
    let mut s = String::with_capacity(7 + id_str.len());
    s.push_str("party:");
    s.push_str(&id_str);
    s
}

pub fn guild_channel_name(guild_id: Uuid) -> String {
    let id_str = guild_id.to_string();
    let mut s = String::with_capacity(6 + id_str.len());
    s.push_str("guild:");
    s.push_str(&id_str);
    s
}

#[derive(Debug, Clone)]
pub enum ChatType {
    Map,
    Party,
    Whisper,
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    PlayerEnter {
        player_id: Uuid,
        name: String,
        x: u16,
        y: u16,
        job: u16,
        level: u16,
        hp: u32,
        max_hp: u32,
    },
    PlayerLeave {
        player_id: Uuid,
    },
    PlayerMove {
        player_id: Uuid,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
    },
    PlayerAttack {
        attacker_id: Uuid,
        target_id: Uuid,
        damage: u32,
        is_crit: bool,
        killed: bool,
    },
    PlayerUseSkill {
        caster_id: Uuid,
        skill_id: u32,
        target_id: Option<Uuid>,
        x: u16,
        y: u16,
    },
    PlayerChat {
        player_id: Uuid,
        message: String,
        chat_type: ChatType,
    },
    PlayerDeath {
        player_id: Uuid,
    },
    PlayerDirectionChange {
        player_id: Uuid,
        direction: u16,
    },
    PlayerRevive {
        player_id: Uuid,
        x: u16,
        y: u16,
    },
    MobSpawn {
        mob_id: Uuid,
        mob_type: u32,
        x: u16,
        y: u16,
    },
    MobMove {
        mob_id: Uuid,
        to_x: u16,
        to_y: u16,
    },
    MobDeath {
        mob_id: Uuid,
        killer_id: Uuid,
    },
    ItemDrop {
        item_id: u32,
        x: u16,
        y: u16,
        amount: u16,
    },
    ItemPickup {
        player_id: Uuid,
        item_id: u32,
        amount: u16,
    },
    ItemDespawn {
        item_id: u32,
        x: u16,
        y: u16,
    },
    MobDamage {
        mob_id: Uuid,
        attacker_id: Uuid,
        damage: u32,
        is_crit: bool,
    },
    MobHpUpdate {
        mob_id: Uuid,
        hp: u32,
        max_hp: u32,
    },
}

impl GameEvent {
    /// 将事件序列化为 rAthena 格式的网络包字节
    ///
    /// 根据事件类型构建对应的客户端可见包：
    /// - 0x0078: EntityAppear（实体出现，玩家/怪物共用）
    /// - 0x007a: EntityDisappear（实体消失）
    /// - 0x0086: EntityMove（实体移动）
    /// - 其他事件暂返回空字节（由上层单独处理）
    pub fn to_packet_bytes(&self) -> Vec<u8> {
        match self {
            // ==================== 玩家进入：0x0078 EntityAppear ====================
            // rAthena 格式：packet_id(u16) + entity_id(u32) + walk_speed(u16) +
            // body_state(u16) + health_state(u16) + effect_state(u32) +
            // job(u16) + head(u16) + weapon(u16) + accessory(u16) +
            // move_start_time(u32) + accessory2(u16) + accessory3(u16) +
            // head_palette(u16) + body_palette(u16) + head_dir(u16) +
            // robe(u16) + guild_id(u32) + emblem_id(u32) +
            // manner(u16) + karma(u16) + sex(u8) +
            // x(u16) + y(u16) + dir(u8) + x(u16) + y(u16) + dir(u8)
            GameEvent::PlayerEnter {
                player_id, x, y, ..
            } => {
                let entity_id = get_entity_id(player_id);
                build_entity_appear_packet(entity_id, 0, *x, *y, 0, 0)
            }

            // ==================== 玩家离开：0x007a EntityDisappear ====================
            // 格式：packet_id(u16) + entity_id(u32) + reason(u8)
            GameEvent::PlayerLeave { player_id } => {
                let entity_id = get_entity_id(player_id);
                build_entity_disappear_packet(entity_id)
            }

            // ==================== 玩家移动：0x0086 EntityMove ====================
            // 格式：packet_id(u16) + entity_id(u32) +
            // from_x(u16) + from_y(u16) + dest_x(u16) + dest_y(u16) +
            // move_start_time(u32)
            GameEvent::PlayerMove {
                player_id,
                from_x,
                from_y,
                to_x,
                to_y,
            } => {
                let entity_id = get_entity_id(player_id);
                build_entity_move_packet(entity_id, *from_x, *from_y, *to_x, *to_y)
            }

            // ==================== 怪物生成：0x0078 EntityAppear ====================
            // entity_type=6 表示怪物，job 字段使用 mob_type（怪物外观 ID）
            GameEvent::MobSpawn {
                mob_id, mob_type, x, y,
            } => {
                let entity_id = get_entity_id(mob_id);
                // mob_type 作为 job 字段传入，entity_type=6
                build_entity_appear_packet(entity_id, 6, *x, *y, *mob_type as u16, 0)
            }

            // ==================== 怪物死亡：0x007a EntityDisappear ====================
            GameEvent::MobDeath { mob_id, .. } => {
                let entity_id = get_entity_id(mob_id);
                build_entity_disappear_packet(entity_id)
            }

            // ==================== 怪物移动：0x0086 EntityMove ====================
            GameEvent::MobMove {
                mob_id,
                to_x,
                to_y,
            } => {
                let entity_id = get_entity_id(mob_id);
                // 怪物移动没有 from 信息，使用 to 作为起始点（客户端会插值）
                build_entity_move_packet(entity_id, *to_x, *to_y, *to_x, *to_y)
            }

            // ==================== 其他事件暂不生成包 ====================
            _ => vec![],
        }
    }

    pub fn source_player_id(&self) -> Option<Uuid> {
        match self {
            GameEvent::PlayerEnter { player_id, .. } => Some(*player_id),
            GameEvent::PlayerLeave { player_id } => Some(*player_id),
            GameEvent::PlayerMove { player_id, .. } => Some(*player_id),
            GameEvent::PlayerAttack { attacker_id, .. } => Some(*attacker_id),
            GameEvent::PlayerUseSkill { caster_id, .. } => Some(*caster_id),
            GameEvent::PlayerChat { player_id, .. } => Some(*player_id),
            GameEvent::PlayerDeath { player_id } => Some(*player_id),
            GameEvent::PlayerDirectionChange { player_id, .. } => Some(*player_id),
            GameEvent::PlayerRevive { player_id, .. } => Some(*player_id),
            GameEvent::MobSpawn { .. } => None,
            GameEvent::MobMove { .. } => None,
            GameEvent::MobDeath { killer_id, .. } => Some(*killer_id),
            GameEvent::ItemDrop { .. } => None,
            GameEvent::ItemPickup { player_id, .. } => Some(*player_id),
            GameEvent::ItemDespawn { .. } => None,
            GameEvent::MobDamage { attacker_id, .. } => Some(*attacker_id),
            GameEvent::MobHpUpdate { .. } => None,
        }
    }

    pub fn position(&self) -> Option<(u16, u16)> {
        match self {
            GameEvent::PlayerEnter { x, y, .. } => Some((*x, *y)),
            GameEvent::PlayerMove { to_x, to_y, .. } => Some((*to_x, *to_y)),
            GameEvent::PlayerUseSkill { x, y, .. } => Some((*x, *y)),
            GameEvent::PlayerRevive { x, y, .. } => Some((*x, *y)),
            GameEvent::MobSpawn { x, y, .. } => Some((*x, *y)),
            GameEvent::MobMove { to_x, to_y, .. } => Some((*to_x, *to_y)),
            GameEvent::ItemDrop { x, y, .. } => Some((*x, *y)),
            GameEvent::ItemDespawn { x, y, .. } => Some((*x, *y)),
            _ => None,
        }
    }
}

const VISION_RADIUS: u16 = 14;

/// 构建 0x0078 EntityAppear 包（实体出现）
///
/// # 参数
/// - `entity_id`: 客户端实体 ID
/// - `entity_type`: 实体类型（0=玩家, 6=怪物）
/// - `x`, `y`: 坐标
/// - `mob_type`: 怪物外观 ID（仅怪物有效，玩家为 0）
/// - `dir`: 朝向
pub fn build_entity_appear_packet(
    entity_id: u32,
    entity_type: u16,
    x: u16,
    y: u16,
    mob_type: u16,
    dir: u8,
) -> Vec<u8> {
    // 包长度：2(packet_id) + 4(entity_id) + 2(walk_speed) + 2(body_state) +
    // 2(health_state) + 4(effect_state) + 2(job) + 2(head) + 2(weapon) +
    // 2(accessory) + 4(move_start_time) + 2(accessory2) + 2(accessory3) +
    // 2(head_palette) + 2(body_palette) + 2(head_dir) + 2(robe) +
    // 4(guild_id) + 4(emblem_id) + 2(manner) + 2(karma) + 1(sex) +
    // 2(x) + 2(y) + 1(dir) + 2(x2) + 2(y2) + 1(dir2) = 63 字节
    let mut pkt = Vec::with_capacity(63);
    pkt.extend_from_slice(&0x0078u16.to_le_bytes());    // packet_id
    pkt.extend_from_slice(&entity_id.to_le_bytes());     // entity_id
    pkt.extend_from_slice(&150u16.to_le_bytes());        // walk_speed (默认 150)
    pkt.extend_from_slice(&0u16.to_le_bytes());          // body_state
    pkt.extend_from_slice(&0u16.to_le_bytes());          // health_state
    pkt.extend_from_slice(&0u32.to_le_bytes());          // effect_state
    let job = if entity_type == 6 { mob_type } else { 0 };
    pkt.extend_from_slice(&job.to_le_bytes());           // job (怪物外观/玩家职业)
    pkt.extend_from_slice(&0u16.to_le_bytes());          // head
    pkt.extend_from_slice(&0u16.to_le_bytes());          // weapon
    pkt.extend_from_slice(&0u16.to_le_bytes());          // accessory
    pkt.extend_from_slice(&0u32.to_le_bytes());          // move_start_time
    pkt.extend_from_slice(&0u16.to_le_bytes());          // accessory2
    pkt.extend_from_slice(&0u16.to_le_bytes());          // accessory3
    pkt.extend_from_slice(&0u16.to_le_bytes());          // head_palette
    pkt.extend_from_slice(&0u16.to_le_bytes());          // body_palette
    pkt.extend_from_slice(&0u16.to_le_bytes());          // head_dir
    pkt.extend_from_slice(&0u16.to_le_bytes());          // robe
    pkt.extend_from_slice(&0u32.to_le_bytes());          // guild_id
    pkt.extend_from_slice(&0u32.to_le_bytes());          // emblem_id
    pkt.extend_from_slice(&0u16.to_le_bytes());          // manner
    pkt.extend_from_slice(&0u16.to_le_bytes());          // karma
    pkt.push(0u8);                                       // sex
    pkt.extend_from_slice(&x.to_le_bytes());             // x
    pkt.extend_from_slice(&y.to_le_bytes());             // y
    pkt.push(dir);                                       // dir
    // 0x0078 包末尾还有重复的坐 x2, y2, dir2（站位数据，与当前位置相同）
    pkt.extend_from_slice(&x.to_le_bytes());             // x2
    pkt.extend_from_slice(&y.to_le_bytes());             // y2
    pkt.push(dir);                                       // dir2
    pkt
}

/// 构建 0x007a EntityDisappear 包（实体消失）
///
/// 格式：packet_id(u16) + entity_id(u32) + reason(u8)
/// reason=0 表示正常消失（离开视野/死亡）
pub fn build_entity_disappear_packet(entity_id: u32) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(7);
    pkt.extend_from_slice(&0x007au16.to_le_bytes());  // packet_id
    pkt.extend_from_slice(&entity_id.to_le_bytes());  // entity_id
    pkt.push(0u8);                                    // reason (0=正常消失)
    pkt
}

/// 构建 0x0086 EntityMove 包（实体移动）
///
/// 格式：packet_id(u16) + entity_id(u32) +
/// from_x(u16) + from_y(u16) + dest_x(u16) + dest_y(u16) +
/// move_start_time(u32)
pub fn build_entity_move_packet(
    entity_id: u32,
    from_x: u16,
    from_y: u16,
    dest_x: u16,
    dest_y: u16,
) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(20);
    pkt.extend_from_slice(&0x0086u16.to_le_bytes());       // packet_id
    pkt.extend_from_slice(&entity_id.to_le_bytes());       // entity_id
    // from_x 和 from_y 需要编码为 3 字节格式（x9 位 + y9 位 + dir3 位）
    let from_xy = encode_pos3(from_x, from_y, 0);
    pkt.extend_from_slice(&from_xy);
    // dest_x 和 dest_y 同样编码
    let dest_xy = encode_pos3(dest_x, dest_y, 0);
    pkt.extend_from_slice(&dest_xy);
    pkt.extend_from_slice(&0u32.to_le_bytes());            // move_start_time
    pkt
}

/// rAthena 位置编码：将 x(u16) + y(u16) + dir(u8) 编码为 3 字节
///
/// 编码方式：bytes = [x_lo, (x_hi << 2) | (y_lo >> 6), (y_mid << 4) | dir]
fn encode_pos3(x: u16, y: u16, dir: u8) -> [u8; 3] {
    [
        (x & 0xFF) as u8,
        ((x >> 8) | ((y & 0x03) << 2)) as u8,
        ((y >> 2) | ((dir as u16 & 0x07) << 6)) as u8,
    ]
}

pub struct Subscriber {
    pub player_id: Uuid,
    pub sender: mpsc::UnboundedSender<Vec<u8>>,
    pub pos_x: u16,
    pub pos_y: u16,
}

pub struct Channel {
    pub name: String,
    pub subscribers: HashMap<Uuid, Subscriber>,
}

pub struct ChannelBus {
    channels: RwLock<HashMap<String, Channel>>,
}

impl Default for ChannelBus {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelBus {
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe a player to a channel
    pub fn subscribe(
        &self,
        channel_name: &str,
        player_id: Uuid,
        sender: mpsc::UnboundedSender<Vec<u8>>,
        x: u16,
        y: u16,
    ) {
        let mut channels = self.channels.write();
        let channel = channels
            .entry(channel_name.to_string())
            .or_insert_with(|| Channel {
                name: channel_name.to_string(),
                subscribers: HashMap::new(),
            });
        channel.subscribers.insert(
            player_id,
            Subscriber {
                player_id,
                sender,
                pos_x: x,
                pos_y: y,
            },
        );
    }

    /// Unsubscribe a player from a channel
    pub fn unsubscribe(&self, channel_name: &str, player_id: &Uuid) {
        let mut channels = self.channels.write();
        if let Some(channel) = channels.get_mut(channel_name) {
            channel.subscribers.remove(player_id);
        }
    }

    /// Update subscriber position
    pub fn update_position(&self, channel_name: &str, player_id: &Uuid, x: u16, y: u16) {
        let channels = self.channels.read();
        if let Some(channel) = channels.get(channel_name)
            && channel.subscribers.contains_key(player_id)
        {
            drop(channels);
            let mut channels = self.channels.write();
            if let Some(channel) = channels.get_mut(channel_name)
                && let Some(sub) = channel.subscribers.get_mut(player_id)
            {
                sub.pos_x = x;
                sub.pos_y = y;
            }
        }
    }

    /// Publish an event to a channel, filtering by vision radius
    /// For party channels (starting with "party:"), no vision filtering
    pub fn publish(&self, channel_name: &str, event: &GameEvent, packet: Vec<u8>) {
        let channels = self.channels.read();
        let Some(channel) = channels.get(channel_name) else {
            return;
        };

        let is_party_channel = channel_name.starts_with("party:");
        let event_pos = event.position();
        let source_player_id = event.source_player_id();

        for sub in channel.subscribers.values() {
            // Don't send to self
            if source_player_id == Some(sub.player_id) {
                continue;
            }

            // Vision filtering for map channels
            if !is_party_channel && let Some((ex, ey)) = event_pos {
                let dx = (sub.pos_x as i32 - ex as i32).unsigned_abs() as u16;
                let dy = (sub.pos_y as i32 - ey as i32).unsigned_abs() as u16;
                if dx > VISION_RADIUS || dy > VISION_RADIUS {
                    continue;
                }
            }

            // Send packet, remove subscriber if send fails (disconnected)
            if sub.sender.send(packet.clone()).is_err() {
                // Will be cleaned up on next unsubscribe
            }
        }
    }

    /// Send raw packet bytes to a specific player across all channels
    pub fn send_to_player(&self, player_id: &Uuid, packet: Vec<u8>) -> bool {
        let channels = self.channels.read();
        for channel in channels.values() {
            if let Some(sub) = channel.subscribers.get(player_id) {
                return sub.sender.send(packet).is_ok();
            }
        }
        false
    }

    /// Get channel subscriber count
    pub fn subscriber_count(&self, channel_name: &str) -> usize {
        let channels = self.channels.read();
        channels
            .get(channel_name)
            .map(|c| c.subscribers.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn make_subscriber(
        _x: u16,
        _y: u16,
    ) -> (
        Uuid,
        mpsc::UnboundedSender<Vec<u8>>,
        mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        let player_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();
        (player_id, tx, rx)
    }

    #[test]
    fn subscribe_adds_subscriber_to_channel() {
        let bus = ChannelBus::new();
        let (player_id, tx, _rx) = make_subscriber(10, 10);
        bus.subscribe("map_1", player_id, tx, 10, 10);
        assert_eq!(bus.subscriber_count("map_1"), 1);
    }

    #[test]
    fn unsubscribe_removes_subscriber() {
        let bus = ChannelBus::new();
        let (player_id, tx, _rx) = make_subscriber(10, 10);
        bus.subscribe("map_1", player_id, tx, 10, 10);
        assert_eq!(bus.subscriber_count("map_1"), 1);
        bus.unsubscribe("map_1", &player_id);
        assert_eq!(bus.subscriber_count("map_1"), 0);
    }

    #[test]
    fn publish_sends_to_subscribers_within_vision_radius() {
        let bus = ChannelBus::new();
        let (p1_id, tx1, mut rx1) = make_subscriber(10, 10);
        let (p2_id, tx2, mut rx2) = make_subscriber(15, 10); // within radius
        let (p3_id, tx3, mut rx3) = make_subscriber(100, 100); // outside radius

        bus.subscribe("map_1", p1_id, tx1, 10, 10);
        bus.subscribe("map_1", p2_id, tx2, 15, 10);
        bus.subscribe("map_1", p3_id, tx3, 100, 100);

        let event = GameEvent::PlayerMove {
            player_id: p1_id,
            from_x: 10,
            from_y: 10,
            to_x: 11,
            to_y: 10,
        };
        bus.publish("map_1", &event, vec![1, 2, 3]);

        // p1 is the source, should not receive
        assert!(rx1.try_recv().is_err());
        // p2 is within vision radius, should receive
        assert!(rx2.try_recv().is_ok());
        // p3 is outside vision radius, should not receive
        assert!(rx3.try_recv().is_err());
    }

    #[test]
    fn publish_does_not_send_to_event_source_player() {
        let bus = ChannelBus::new();
        let (p1_id, tx1, mut rx1) = make_subscriber(10, 10);
        let (p2_id, tx2, mut rx2) = make_subscriber(10, 10);

        bus.subscribe("map_1", p1_id, tx1, 10, 10);
        bus.subscribe("map_1", p2_id, tx2, 10, 10);

        let event = GameEvent::PlayerEnter {
            player_id: p1_id,
            name: "Alice".to_string(),
            x: 10,
            y: 10,
            job: 1,
            level: 1,
            hp: 100,
            max_hp: 100,
        };
        bus.publish("map_1", &event, vec![42]);

        assert!(rx1.try_recv().is_err());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn party_channel_bypasses_vision_filtering() {
        let bus = ChannelBus::new();
        let (p1_id, tx1, mut rx1) = make_subscriber(10, 10);
        let (p2_id, tx2, mut rx2) = make_subscriber(100, 100); // far away

        bus.subscribe("party:abc", p1_id, tx1, 10, 10);
        bus.subscribe("party:abc", p2_id, tx2, 100, 100);

        let event = GameEvent::PlayerChat {
            player_id: p1_id,
            message: "hello".to_string(),
            chat_type: ChatType::Party,
        };
        bus.publish("party:abc", &event, vec![1]);

        // p1 is source, should not receive
        assert!(rx1.try_recv().is_err());
        // p2 is far away but party channel bypasses vision filter
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn subscriber_count_returns_correct_count() {
        let bus = ChannelBus::new();
        assert_eq!(bus.subscriber_count("map_1"), 0);

        let (p1_id, tx1, _) = make_subscriber(0, 0);
        let (p2_id, tx2, _) = make_subscriber(0, 0);
        bus.subscribe("map_1", p1_id, tx1, 0, 0);
        assert_eq!(bus.subscriber_count("map_1"), 1);
        bus.subscribe("map_1", p2_id, tx2, 0, 0);
        assert_eq!(bus.subscriber_count("map_1"), 2);

        bus.unsubscribe("map_1", &p1_id);
        assert_eq!(bus.subscriber_count("map_1"), 1);

        // Non-existent channel returns 0
        assert_eq!(bus.subscriber_count("nonexistent"), 0);
    }

    // ==================== GameEvent 序列化测试 ====================

    #[test]
    fn player_enter_to_packet_bytes_returns_0x0078() {
        let player_id = Uuid::new_v4();
        let event = GameEvent::PlayerEnter {
            player_id,
            name: "Test".to_string(),
            x: 100,
            y: 200,
            job: 1,
            level: 10,
            hp: 500,
            max_hp: 500,
        };
        let pkt = event.to_packet_bytes();

        // 0x0078 包长度应为 63 字节
        assert_eq!(pkt.len(), 63);
        // 验证 packet_id = 0x0078
        assert_eq!(pkt[0], 0x78);
        assert_eq!(pkt[1], 0x00);
        // 验证 entity_id
        let actual_eid = u32::from_le_bytes([pkt[2], pkt[3], pkt[4], pkt[5]]);
        assert!(actual_eid > 0);
        // 验证 walk_speed = 150
        let walk_speed = u16::from_le_bytes([pkt[6], pkt[7]]);
        assert_eq!(walk_speed, 150);
        // 验证坐标（末尾 x2, y2 位置：偏移 58-59 和 60-61）
        let x = u16::from_le_bytes([pkt[58], pkt[59]]);
        let y = u16::from_le_bytes([pkt[60], pkt[61]]);
        assert_eq!(x, 100);
        assert_eq!(y, 200);
    }

    #[test]
    fn player_leave_to_packet_bytes_returns_0x007a() {
        let player_id = Uuid::new_v4();
        let event = GameEvent::PlayerLeave { player_id };
        let pkt = event.to_packet_bytes();

        // 0x007a 包长度应为 7 字节
        assert_eq!(pkt.len(), 7);
        // packet_id = 0x007a
        assert_eq!(pkt[0], 0x7a);
        assert_eq!(pkt[1], 0x00);
        // reason = 0
        assert_eq!(pkt[6], 0);
    }

    #[test]
    fn player_move_to_packet_bytes_returns_0x0086() {
        let player_id = Uuid::new_v4();
        let event = GameEvent::PlayerMove {
            player_id,
            from_x: 50,
            from_y: 60,
            to_x: 55,
            to_y: 65,
        };
        let pkt = event.to_packet_bytes();

        // 0x0086 包：packet_id(2) + entity_id(4) + from(3) + dest(3) + move_start_time(4) = 16 字节
        assert_eq!(pkt.len(), 16);
        // packet_id = 0x0086
        assert_eq!(pkt[0], 0x86);
        assert_eq!(pkt[1], 0x00);
    }

    #[test]
    fn mob_spawn_to_packet_bytes_returns_0x0078_with_entity_type_6() {
        let mob_id = Uuid::new_v4();
        let event = GameEvent::MobSpawn {
            mob_id,
            mob_type: 1001,
            x: 150,
            y: 250,
        };
        let pkt = event.to_packet_bytes();

        // 0x0078 包长度应为 63 字节
        assert_eq!(pkt.len(), 63);
        // packet_id = 0x0078
        assert_eq!(pkt[0], 0x78);
        assert_eq!(pkt[1], 0x00);
        // 验证 job 字段 = mob_type = 1001
        let job = u16::from_le_bytes([pkt[16], pkt[17]]);
        assert_eq!(job, 1001);
    }

    #[test]
    fn mob_death_to_packet_bytes_returns_0x007a() {
        let mob_id = Uuid::new_v4();
        let killer_id = Uuid::new_v4();
        let event = GameEvent::MobDeath { mob_id, killer_id };
        let pkt = event.to_packet_bytes();

        // 0x007a 包长度应为 7 字节
        assert_eq!(pkt.len(), 7);
        assert_eq!(pkt[0], 0x7a);
        assert_eq!(pkt[1], 0x00);
    }

    #[test]
    fn unimplemented_events_return_empty_vec() {
        let player_id = Uuid::new_v4();

        // PlayerAttack 未实现序列化
        let event = GameEvent::PlayerAttack {
            attacker_id: player_id,
            target_id: Uuid::new_v4(),
            damage: 100,
            is_crit: false,
            killed: false,
        };
        assert!(event.to_packet_bytes().is_empty());

        // PlayerChat 未实现序列化
        let event = GameEvent::PlayerChat {
            player_id,
            message: "hello".to_string(),
            chat_type: ChatType::Map,
        };
        assert!(event.to_packet_bytes().is_empty());

        // ItemDrop 未实现序列化
        let event = GameEvent::ItemDrop {
            item_id: 1001,
            x: 10,
            y: 20,
            amount: 1,
        };
        assert!(event.to_packet_bytes().is_empty());
    }

    #[test]
    fn get_entity_id_is_consistent_for_same_uuid() {
        let uuid = Uuid::new_v4();
        let eid1 = get_entity_id(&uuid);
        let eid2 = get_entity_id(&uuid);
        assert_eq!(eid1, eid2);
    }

    #[test]
    fn get_entity_id_returns_unique_ids_for_different_uuids() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let eid1 = get_entity_id(&uuid1);
        let eid2 = get_entity_id(&uuid2);
        assert_ne!(eid1, eid2);
    }

    #[test]
    fn encode_pos3_roundtrip() {
        // 测试 encode_pos3 的编码结果长度
        let encoded = encode_pos3(100, 200, 3);
        assert_eq!(encoded.len(), 3);
        // x=100 的低 8 位 = 100
        assert_eq!(encoded[0], 100);
    }
}
