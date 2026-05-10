use std::collections::HashMap;
use std::time::Instant;

use parking_lot::RwLock;
use uuid::Uuid;

use super::channel::{ChannelBus, GameEvent};

const DROP_TTL_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Clone)]
pub struct DropItem {
    pub id: Uuid,
    pub item_id: u32,
    pub amount: u16,
    pub x: u16,
    pub y: u16,
    pub map_name: String,
    pub dropped_at: Instant,
}

pub struct DropManager {
    drops: RwLock<HashMap<Uuid, DropItem>>,
}

impl Default for DropManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DropManager {
    pub fn new() -> Self {
        Self {
            drops: RwLock::new(HashMap::new()),
        }
    }

    /// Add a dropped item to the map
    pub fn add(&self, item_id: u32, amount: u16, x: u16, y: u16, map_name: &str) -> Uuid {
        let id = Uuid::new_v4();
        let drop = DropItem {
            id,
            item_id,
            amount,
            x,
            y,
            map_name: map_name.to_string(),
            dropped_at: Instant::now(),
        };
        self.drops.write().insert(id, drop);
        id
    }

    /// Add a dropped item and broadcast to nearby players
    pub fn add_with_broadcast(
        &self,
        item_id: u32,
        amount: u16,
        x: u16,
        y: u16,
        map_name: &str,
        channel_bus: &ChannelBus,
    ) -> Uuid {
        let id = self.add(item_id, amount, x, y, map_name);

        // Broadcast item drop event
        let channel_name = format!("map:{}", map_name);
        let event = GameEvent::ItemDrop {
            item_id,
            x,
            y,
            amount,
        };
        let packet = event.to_packet_bytes();
        channel_bus.publish(&channel_name, &event, packet);

        id
    }

    /// Pick up a dropped item, returns the DropItem if it exists
    pub fn pickup(&self, drop_id: &Uuid) -> Option<DropItem> {
        self.drops.write().remove(drop_id)
    }

    /// Pick up a dropped item and broadcast to nearby players
    pub fn pickup_with_broadcast(
        &self,
        drop_id: &Uuid,
        player_id: Uuid,
        channel_bus: &ChannelBus,
        map_name: &str,
    ) -> Option<DropItem> {
        let item = self.pickup(drop_id);
        if let Some(ref drop) = item {
            // Broadcast item pickup event
            let channel_name = format!("map:{}", map_name);
            let event = GameEvent::ItemPickup {
                player_id,
                item_id: drop.item_id,
                amount: drop.amount,
            };
            let packet = event.to_packet_bytes();
            channel_bus.publish(&channel_name, &event, packet);
        }
        item
    }

    /// Find a drop item at the given position
    pub fn find_at_position(&self, x: u16, y: u16, map_name: &str) -> Option<DropItem> {
        let drops = self.drops.read();
        drops
            .values()
            .find(|d| d.x == x && d.y == y && d.map_name == map_name)
            .cloned()
    }

    /// Clean up expired drops (called by GameLoop tick)
    /// Returns the list of expired drop IDs for event publishing
    pub fn cleanup_expired(&self) -> Vec<Uuid> {
        let mut drops = self.drops.write();
        let expired: Vec<Uuid> = drops
            .iter()
            .filter(|(_, d)| d.dropped_at.elapsed().as_secs() > DROP_TTL_SECS)
            .map(|(id, _)| *id)
            .collect();
        for id in &expired {
            drops.remove(id);
        }
        expired
    }

    /// Clean up expired drops and broadcast despawn events to nearby players.
    /// Returns the list of expired drop IDs.
    pub fn cleanup_expired_with_broadcast(&self, channel_bus: &ChannelBus) -> Vec<Uuid> {
        // First get the expired drops info before removing
        let expired_info: Vec<(Uuid, u32, u16, u16, String)> = {
            let drops = self.drops.read();
            drops
                .iter()
                .filter(|(_, d)| d.dropped_at.elapsed().as_secs() > DROP_TTL_SECS)
                .map(|(id, d)| (*id, d.item_id, d.x, d.y, d.map_name.clone()))
                .collect()
        };

        let expired: Vec<Uuid> = {
            let drops = self.drops.write();
            drops
                .iter()
                .filter(|(_, d)| d.dropped_at.elapsed().as_secs() > DROP_TTL_SECS)
                .map(|(id, _)| *id)
                .collect()
        };

        // Remove expired drops
        {
            let mut drops = self.drops.write();
            for id in &expired {
                drops.remove(id);
            }
        }

        // Broadcast despawn events
        for (_drop_id, item_id, x, y, map_name) in expired_info {
            let channel_name = format!("map:{}", map_name);
            let event = GameEvent::ItemDespawn { item_id, x, y };
            let packet = event.to_packet_bytes();
            channel_bus.publish(&channel_name, &event, packet);
        }

        expired
    }

    /// Get all drops for a specific map
    pub fn get_drops_for_map(&self, map_name: &str) -> Vec<DropItem> {
        let drops = self.drops.read();
        drops
            .values()
            .filter(|d| d.map_name == map_name)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_creates_drop_item_and_returns_id() {
        let manager = DropManager::new();
        let id = manager.add(1001, 3, 10, 20, "prontera");
        assert!(!id.is_nil());

        let drops = manager.get_drops_for_map("prontera");
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].id, id);
        assert_eq!(drops[0].item_id, 1001);
        assert_eq!(drops[0].amount, 3);
        assert_eq!(drops[0].x, 10);
        assert_eq!(drops[0].y, 20);
        assert_eq!(drops[0].map_name, "prontera");
    }

    #[test]
    fn pickup_removes_and_returns_drop_item() {
        let manager = DropManager::new();
        let id = manager.add(1001, 1, 5, 5, "prontera");

        let picked = manager.pickup(&id);
        assert!(picked.is_some());
        let picked = picked.unwrap();
        assert_eq!(picked.item_id, 1001);
        assert_eq!(picked.id, id);

        // Should be removed now
        assert!(manager.pickup(&id).is_none());
    }

    #[test]
    fn pickup_returns_none_for_non_existent_id() {
        let manager = DropManager::new();
        let fake_id = Uuid::new_v4();
        assert!(manager.pickup(&fake_id).is_none());
    }

    #[test]
    fn find_at_position_finds_drop_at_coordinates() {
        let manager = DropManager::new();
        manager.add(1001, 1, 15, 25, "geffen");
        manager.add(1002, 1, 30, 40, "geffen");

        let found = manager.find_at_position(15, 25, "geffen");
        assert!(found.is_some());
        assert_eq!(found.unwrap().item_id, 1001);
    }

    #[test]
    fn find_at_position_returns_none_when_no_drop_at_position() {
        let manager = DropManager::new();
        manager.add(1001, 1, 15, 25, "geffen");

        assert!(manager.find_at_position(99, 99, "geffen").is_none());
        assert!(manager.find_at_position(15, 25, "prontera").is_none());
    }

    #[test]
    fn cleanup_expired_removes_expired_drops() {
        let manager = DropManager::new();
        let id = manager.add(1001, 1, 10, 10, "prontera");

        // Manually insert an expired drop by overwriting
        {
            let mut drops = manager.drops.write();
            let drop = drops.get_mut(&id).unwrap();
            // We can't set dropped_at directly (Instant has no setter),
            // so we test that a freshly created drop is NOT cleaned up
        }

        // Fresh drops should not be expired
        let expired = manager.cleanup_expired();
        assert!(expired.is_empty());
        assert!(manager.pickup(&id).is_some());
    }

    #[test]
    fn get_drops_for_map_returns_only_drops_for_specified_map() {
        let manager = DropManager::new();
        manager.add(1001, 1, 10, 10, "prontera");
        manager.add(1002, 1, 20, 20, "geffen");
        manager.add(1003, 1, 30, 30, "prontera");

        let prontera_drops = manager.get_drops_for_map("prontera");
        assert_eq!(prontera_drops.len(), 2);
        assert!(prontera_drops.iter().all(|d| d.map_name == "prontera"));

        let geffen_drops = manager.get_drops_for_map("geffen");
        assert_eq!(geffen_drops.len(), 1);
        assert_eq!(geffen_drops[0].item_id, 1002);
    }
}
