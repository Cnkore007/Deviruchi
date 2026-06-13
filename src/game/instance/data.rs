//! Instance system data structures

use std::time::Instant;

/// Instance entity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EntityType {
    #[default]
    Monster,
    NPC,
    Portal,
    TreasureChest,
}

/// Instance entity
#[derive(Debug, Clone)]
pub struct InstanceEntity {
    pub entity_type: EntityType,
    pub entity_id: u32,
    pub position: (u16, u16),
}

impl InstanceEntity {
    pub fn new(entity_type: EntityType, entity_id: u32, x: u16, y: u16) -> Self {
        Self {
            entity_type,
            entity_id,
            position: (x, y),
        }
    }
}

/// Instance timers
#[derive(Debug, Clone, Default)]
pub struct InstanceTimers {
    pub remaining_time: u32,
    pub total_time: u32,
    pub idle_timeout: u32,
}

impl InstanceTimers {
    pub fn new(total_time: u32, idle_timeout: u32) -> Self {
        Self {
            remaining_time: total_time,
            total_time,
            idle_timeout,
        }
    }

    pub fn tick(&mut self, seconds: u32) {
        if self.remaining_time >= seconds {
            self.remaining_time -= seconds;
        } else {
            self.remaining_time = 0;
        }
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_time == 0
    }
}

/// Instance state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstanceState {
    #[default]
    Waiting,
    Active,
    Completed,
    Expired,
    Aborted,
}

impl std::fmt::Display for InstanceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceState::Waiting => write!(f, "Waiting"),
            InstanceState::Active => write!(f, "Active"),
            InstanceState::Completed => write!(f, "Completed"),
            InstanceState::Expired => write!(f, "Expired"),
            InstanceState::Aborted => write!(f, "Aborted"),
        }
    }
}

/// Instance type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstanceType {
    #[default]
    PvM,
    PvP,
    Quest,
    Tutorial,
    Event,
}

impl std::fmt::Display for InstanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceType::PvM => write!(f, "PvM"),
            InstanceType::PvP => write!(f, "PvP"),
            InstanceType::Quest => write!(f, "Quest"),
            InstanceType::Tutorial => write!(f, "Tutorial"),
            InstanceType::Event => write!(f, "Event"),
        }
    }
}

/// Instance objective type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstanceObjectiveType {
    #[default]
    KillAllMobs,
    KillBoss,
    CollectItem,
    Survive,
    Escort,
}

/// Instance objective
#[derive(Debug, Clone)]
pub struct InstanceObjective {
    pub id: u32,
    pub objective_type: InstanceObjectiveType,
    pub target: u16,
    pub target_count: u32,
    pub current_count: u32,
    pub description: String,
}

impl InstanceObjective {
    pub fn new(
        id: u32,
        objective_type: InstanceObjectiveType,
        target: u16,
        target_count: u32,
        description: &str,
    ) -> Self {
        Self {
            id,
            objective_type,
            target,
            target_count,
            current_count: 0,
            description: description.to_string(),
        }
    }

    pub fn is_completed(&self) -> bool {
        self.current_count >= self.target_count
    }

    pub fn progress_percent(&self) -> u8 {
        if self.target_count == 0 {
            return 100;
        }
        ((self.current_count as f64 / self.target_count as f64) * 100.0) as u8
    }

    pub fn update_progress(&mut self, count: u32) {
        self.current_count = self.current_count.saturating_add(count);
        if self.current_count > self.target_count {
            self.current_count = self.target_count;
        }
    }

    pub fn set_progress(&mut self, count: u32) {
        self.current_count = count.min(self.target_count);
    }
}

/// Instance data
#[derive(Debug, Clone)]
pub struct Instance {
    pub id: u32,
    pub name: String,
    pub map_name: String,
    pub instance_type: InstanceType,
    pub state: InstanceState,
    pub participants: Vec<u32>,
    pub max_participants: u16,
    pub timers: InstanceTimers,
    pub spawned_entities: Vec<InstanceEntity>,
    pub objectives: Vec<InstanceObjective>,
    pub created_at: Instant,
    pub leader_char_id: u32,
    pub template_id: u32,
}

impl Instance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        name: &str,
        map_name: &str,
        instance_type: InstanceType,
        max_participants: u16,
        duration_secs: u32,
        idle_timeout_secs: u32,
        leader_char_id: u32,
        template_id: u32,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            map_name: map_name.to_string(),
            instance_type,
            state: InstanceState::Waiting,
            participants: Vec::new(),
            max_participants,
            timers: InstanceTimers::new(duration_secs, idle_timeout_secs),
            spawned_entities: Vec::new(),
            objectives: Vec::new(),
            created_at: Instant::now(),
            leader_char_id,
            template_id,
        }
    }

    pub fn add_participant(&mut self, char_id: u32) -> bool {
        if self.participants.len() < self.max_participants as usize
            && !self.participants.contains(&char_id)
        {
            self.participants.push(char_id);
            true
        } else {
            false
        }
    }

    pub fn remove_participant(&mut self, char_id: u32) -> bool {
        if let Some(pos) = self.participants.iter().position(|&id| id == char_id) {
            self.participants.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn is_participant(&self, char_id: u32) -> bool {
        self.participants.contains(&char_id)
    }

    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.participants.len() >= self.max_participants as usize
    }

    pub fn start(&mut self) {
        self.state = InstanceState::Active;
    }

    pub fn complete(&mut self) {
        self.state = InstanceState::Completed;
    }

    pub fn expire(&mut self) {
        self.state = InstanceState::Expired;
    }

    pub fn abort(&mut self) {
        self.state = InstanceState::Aborted;
    }

    pub fn all_objectives_completed(&self) -> bool {
        self.objectives.iter().all(|o| o.is_completed())
    }

    pub fn spawn_entity(&mut self, entity: InstanceEntity) {
        self.spawned_entities.push(entity);
    }

    pub fn despawn_entity(&mut self, entity_id: u32) -> bool {
        if let Some(pos) = self
            .spawned_entities
            .iter()
            .position(|e| e.entity_id == entity_id)
        {
            self.spawned_entities.remove(pos);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_create() {
        let mut instance = Instance::new(
            1,
            "Test Dungeon",
            "dungeon01",
            InstanceType::PvM,
            5,
            3600,
            300,
            100,
            1,
        );

        assert_eq!(instance.id, 1);
        assert_eq!(instance.state, InstanceState::Waiting);
        assert!(instance.participants.is_empty());
        assert!(!instance.is_full());
    }

    #[test]
    fn test_participant_management() {
        let mut instance = Instance::new(1, "Test", "map", InstanceType::PvM, 3, 3600, 300, 100, 1);

        assert!(instance.add_participant(1));
        assert!(instance.add_participant(2));
        assert!(!instance.add_participant(1)); // Already added
        assert_eq!(instance.participants.len(), 2);

        assert!(instance.is_participant(1));
        assert!(!instance.is_participant(99));

        assert!(instance.remove_participant(1));
        assert!(!instance.remove_participant(1)); // Already removed
        assert_eq!(instance.participants.len(), 1);
    }

    #[test]
    fn test_instance_capacity() {
        let mut instance = Instance::new(1, "Test", "map", InstanceType::PvM, 2, 3600, 300, 100, 1);

        assert!(instance.add_participant(1));
        assert!(instance.add_participant(2));
        assert!(!instance.add_participant(3)); // Full
        assert!(instance.is_full());
    }

    #[test]
    fn test_instance_state_transitions() {
        let mut instance = Instance::new(1, "Test", "map", InstanceType::PvM, 5, 3600, 300, 100, 1);

        assert_eq!(instance.state, InstanceState::Waiting);

        instance.start();
        assert_eq!(instance.state, InstanceState::Active);

        instance.complete();
        assert_eq!(instance.state, InstanceState::Completed);
    }

    #[test]
    fn test_objectives() {
        let mut obj = InstanceObjective::new(
            1,
            InstanceObjectiveType::KillAllMobs,
            1001,
            10,
            "Defeat all monsters",
        );

        assert!(!obj.is_completed());
        assert_eq!(obj.progress_percent(), 0);

        obj.update_progress(5);
        assert_eq!(obj.current_count, 5);
        assert_eq!(obj.progress_percent(), 50);

        obj.update_progress(10); // Should cap at 10
        assert!(obj.is_completed());
        assert_eq!(obj.progress_percent(), 100);
    }

    #[test]
    fn test_timers() {
        let mut timers = InstanceTimers::new(3600, 300);

        assert_eq!(timers.remaining_time, 3600);
        assert!(!timers.is_expired());

        timers.tick(60);
        assert_eq!(timers.remaining_time, 3540);

        timers.tick(10000);
        assert!(timers.is_expired());
    }

    #[test]
    fn test_entity_spawn() {
        let mut instance = Instance::new(1, "Test", "map", InstanceType::PvM, 5, 3600, 300, 100, 1);

        let entity = InstanceEntity::new(EntityType::Monster, 1, 100, 200);
        instance.spawn_entity(entity);

        assert_eq!(instance.spawned_entities.len(), 1);
        assert!(instance.despawn_entity(1));
        assert!(instance.spawned_entities.is_empty());
    }
}
