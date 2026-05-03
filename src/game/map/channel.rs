use std::collections::HashMap;

use parking_lot::RwLock;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ChatType {
    Map,
    Party,
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
}

impl GameEvent {
    pub fn source_player_id(&self) -> Option<Uuid> {
        match self {
            GameEvent::PlayerEnter { player_id, .. } => Some(*player_id),
            GameEvent::PlayerLeave { player_id } => Some(*player_id),
            GameEvent::PlayerMove { player_id, .. } => Some(*player_id),
            GameEvent::PlayerAttack { attacker_id, .. } => Some(*attacker_id),
            GameEvent::PlayerUseSkill { caster_id, .. } => Some(*caster_id),
            GameEvent::PlayerChat { player_id, .. } => Some(*player_id),
            GameEvent::PlayerDeath { player_id } => Some(*player_id),
            GameEvent::PlayerRevive { player_id, .. } => Some(*player_id),
            GameEvent::MobSpawn { .. } => None,
            GameEvent::MobMove { .. } => None,
            GameEvent::MobDeath { killer_id, .. } => Some(*killer_id),
            GameEvent::ItemDrop { .. } => None,
            GameEvent::ItemPickup { player_id, .. } => Some(*player_id),
            GameEvent::ItemDespawn { .. } => None,
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
        if let Some(channel) = channels.get(channel_name) {
            if channel.subscribers.contains_key(player_id) {
                drop(channels);
                let mut channels = self.channels.write();
                if let Some(channel) = channels.get_mut(channel_name) {
                    if let Some(sub) = channel.subscribers.get_mut(player_id) {
                        sub.pos_x = x;
                        sub.pos_y = y;
                    }
                }
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

        for (_, sub) in &channel.subscribers {
            // Don't send to self
            if source_player_id == Some(sub.player_id) {
                continue;
            }

            // Vision filtering for map channels
            if !is_party_channel {
                if let Some((ex, ey)) = event_pos {
                    let dx = (sub.pos_x as i32 - ex as i32).unsigned_abs() as u16;
                    let dy = (sub.pos_y as i32 - ey as i32).unsigned_abs() as u16;
                    if dx > VISION_RADIUS || dy > VISION_RADIUS {
                        continue;
                    }
                }
            }

            // Send packet, remove subscriber if send fails (disconnected)
            if sub.sender.send(packet.clone()).is_err() {
                // Will be cleaned up on next unsubscribe
            }
        }
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

    fn make_subscriber(_x: u16, _y: u16) -> (Uuid, mpsc::UnboundedSender<Vec<u8>>, mpsc::UnboundedReceiver<Vec<u8>>) {
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
}
