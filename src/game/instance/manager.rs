//! Instance manager - handles instance lifecycle and player management

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::data::{EntityType, Instance, InstanceEntity, InstanceObjectiveType, InstanceState};
use super::template::{InstanceTemplate, InstanceTemplateDatabase};

/// Instance error types
#[derive(Debug, Clone)]
pub enum InstanceError {
    NotFound,
    AlreadyExists,
    TemplateNotFound,
    InstanceFull,
    InstanceNotWaiting,
    InstanceNotActive,
    InvalidState,
    CooldownActive {
        remaining_secs: u32,
    },
    LevelRequirementNotMet {
        min_level: u16,
        max_level: u16,
    },
    PlayerAlreadyInInstance,
    PlayerNotInInstance,
    RequirementsNotMet {
        zenny_required: u32,
        items_required: Vec<u16>,
    },
}

impl std::fmt::Display for InstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceError::NotFound => write!(f, "Instance not found"),
            InstanceError::AlreadyExists => write!(f, "Instance already exists"),
            InstanceError::TemplateNotFound => write!(f, "Instance template not found"),
            InstanceError::InstanceFull => write!(f, "Instance is full"),
            InstanceError::InstanceNotWaiting => write!(f, "Instance is not in waiting state"),
            InstanceError::InstanceNotActive => write!(f, "Instance is not active"),
            InstanceError::InvalidState => write!(f, "Invalid instance state"),
            InstanceError::CooldownActive { remaining_secs } => {
                write!(f, "Cooldown active, {} seconds remaining", remaining_secs)
            }
            InstanceError::LevelRequirementNotMet {
                min_level,
                max_level,
            } => {
                write!(
                    f,
                    "Level requirement not met ({} to {})",
                    min_level, max_level
                )
            }
            InstanceError::PlayerAlreadyInInstance => write!(f, "Player already in an instance"),
            InstanceError::PlayerNotInInstance => write!(f, "Player not in this instance"),
            InstanceError::RequirementsNotMet { zenny_required, .. } => {
                write!(f, "Entry requirements not met (zenny: {})", zenny_required)
            }
        }
    }
}

impl std::error::Error for InstanceError {}

/// Instance manager - main controller for all instances
pub struct InstanceManager {
    templates: Arc<RwLock<InstanceTemplateDatabase>>,
    instances: RwLock<HashMap<u32, Arc<RwLock<Instance>>>>,
    player_instances: RwLock<HashMap<u32, u32>>, // char_id -> instance_id
    cooldowns: RwLock<HashMap<u32, CooldownEntry>>, // char_id -> cooldown data
    next_instance_id: RwLock<u32>,
}

#[derive(Debug, Clone)]
struct CooldownEntry {
    template_id: u32,
    expires_at: std::time::Instant,
}

impl InstanceManager {
    pub fn new() -> Self {
        let templates = InstanceTemplateDatabase::new();
        Self {
            templates: Arc::new(RwLock::new(templates)),
            instances: RwLock::new(HashMap::new()),
            player_instances: RwLock::new(HashMap::new()),
            cooldowns: RwLock::new(HashMap::new()),
            next_instance_id: RwLock::new(1),
        }
    }

    pub fn with_templates(templates: InstanceTemplateDatabase) -> Self {
        Self {
            templates: Arc::new(RwLock::new(templates)),
            instances: RwLock::new(HashMap::new()),
            player_instances: RwLock::new(HashMap::new()),
            cooldowns: RwLock::new(HashMap::new()),
            next_instance_id: RwLock::new(1),
        }
    }

    pub fn load_default_templates(&self) {
        self.templates.write().load_default_templates();
    }

    pub fn register_template(&self, template: InstanceTemplate) {
        self.templates.write().register(template);
    }

    pub fn get_template(&self, template_id: u32) -> Option<InstanceTemplate> {
        self.templates.read().get(template_id).cloned()
    }

    pub fn all_templates(&self) -> Vec<InstanceTemplate> {
        self.templates.read().all().into_iter().cloned().collect()
    }

    fn next_id(&self) -> u32 {
        let mut next = self.next_instance_id.write();
        let id = *next;
        *next += 1;
        id
    }

    pub fn create_instance(
        &self,
        template_id: u32,
        leader_char_id: u32,
    ) -> Result<Instance, InstanceError> {
        let template = self
            .templates
            .read()
            .get(template_id)
            .cloned()
            .ok_or(InstanceError::TemplateNotFound)?;

        let instance_id = self.next_id();

        let mut instance = Instance::new(
            instance_id,
            &template.name,
            &template.map_name,
            template.instance_type,
            template.max_players,
            template.duration_secs,
            template.idle_timeout_secs,
            leader_char_id,
            template_id,
        );

        // Add objectives from template
        for obj in &template.objectives {
            instance.objectives.push(obj.clone());
        }

        let instance = Arc::new(RwLock::new(instance));

        // Check if instance already exists for this template/leader
        {
            let instances = self.instances.read();
            if instances.contains_key(&instance_id) {
                return Err(InstanceError::AlreadyExists);
            }
        }

        // Add leader as participant
        {
            let mut inst = instance.write();
            inst.add_participant(leader_char_id);
        }

        // Store instance
        let instance_clone = instance.clone();
        {
            let mut instances = self.instances.write();
            instances.insert(instance_id, instance);
        }

        // Track player instance
        {
            let mut player_instances = self.player_instances.write();
            player_instances.insert(leader_char_id, instance_id);
        }

        Ok(instance_clone.read().clone())
    }

    pub fn join_instance(&self, instance_id: u32, char_id: u32) -> Result<(), InstanceError> {
        // Check if player is already in another instance
        {
            let player_instances = self.player_instances.read();
            if player_instances.contains_key(&char_id) {
                return Err(InstanceError::PlayerAlreadyInInstance);
            }
        }

        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        {
            let mut inst = instance.write();

            if inst.state != InstanceState::Waiting {
                return Err(InstanceError::InstanceNotWaiting);
            }

            if inst.is_full() {
                return Err(InstanceError::InstanceFull);
            }

            inst.add_participant(char_id);
        }

        // Track player
        {
            let mut player_instances = self.player_instances.write();
            player_instances.insert(char_id, instance_id);
        }

        Ok(())
    }

    pub fn leave_instance(&self, char_id: u32) {
        let instance_id = {
            let player_instances = self.player_instances.read();
            player_instances.get(&char_id).copied()
        };

        if let Some(id) = instance_id
            && let Some(instance) = self.instances.read().get(&id)
        {
            let mut inst = instance.write();
            inst.remove_participant(char_id);

            // If instance becomes empty, it may need to be cleaned up
            if inst.is_empty() && inst.state == InstanceState::Waiting {
                // Could implement idle timeout cleanup here
            }
        }

        // Remove player tracking
        let mut player_instances = self.player_instances.write();
        player_instances.remove(&char_id);
    }

    pub fn start_instance(&self, instance_id: u32) -> Result<(), InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let mut inst = instance.write();

        if inst.state != InstanceState::Waiting {
            return Err(InstanceError::InstanceNotWaiting);
        }

        // Check minimum players requirement
        let template = self
            .templates
            .read()
            .get(inst.template_id)
            .cloned()
            .ok_or(InstanceError::TemplateNotFound)?;

        if inst.participants.len() < template.min_players as usize {
            return Err(InstanceError::InvalidState);
        }

        inst.start();
        Ok(())
    }

    pub fn complete_instance(&self, instance_id: u32) -> Result<(), InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let mut inst = instance.write();

        if inst.state != InstanceState::Active {
            return Err(InstanceError::InstanceNotActive);
        }

        inst.complete();
        Ok(())
    }

    pub fn abort_instance(&self, instance_id: u32) {
        if let Some(instance) = self.instances.write().remove(&instance_id) {
            let inst = instance.read();

            // Remove all participants from tracking
            let mut player_instances = self.player_instances.write();
            for char_id in &inst.participants {
                player_instances.remove(char_id);
            }
        }
    }

    pub fn expire_instance(&self, instance_id: u32) -> Result<(), InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let mut inst = instance.write();
        inst.expire();
        Ok(())
    }

    pub fn remove_instance(&self, instance_id: u32) -> Option<Instance> {
        if let Some(instance) = self.instances.write().remove(&instance_id) {
            // Remove all participants from tracking
            let mut player_instances = self.player_instances.write();
            for char_id in &instance.read().participants {
                player_instances.remove(char_id);
            }
            Some(instance.read().clone())
        } else {
            None
        }
    }

    pub fn get_instance(&self, instance_id: u32) -> Option<Instance> {
        self.instances
            .read()
            .get(&instance_id)
            .map(|i| i.read().clone())
    }

    pub fn get_player_instance(&self, char_id: u32) -> Option<Instance> {
        let instance_id = self.player_instances.read().get(&char_id).copied()?;
        self.get_instance(instance_id)
    }

    pub fn get_player_instance_id(&self, char_id: u32) -> Option<u32> {
        self.player_instances.read().get(&char_id).copied()
    }

    pub fn check_cooldown(&self, char_id: u32, template_id: u32) -> Option<u32> {
        let cooldowns = self.cooldowns.read();
        if let Some(entry) = cooldowns.get(&char_id)
            && entry.template_id == template_id
        {
            let remaining = entry
                .expires_at
                .duration_since(std::time::Instant::now())
                .as_secs() as u32;
            if remaining > 0 {
                return Some(remaining);
            }
        }
        None
    }

    pub fn start_cooldown(&self, char_id: u32, template_id: u32, duration_secs: u32) {
        let mut cooldowns = self.cooldowns.write();
        cooldowns.insert(
            char_id,
            CooldownEntry {
                template_id,
                expires_at: std::time::Instant::now()
                    + std::time::Duration::from_secs(duration_secs as u64),
            },
        );
    }

    pub fn clear_cooldown(&self, char_id: u32, template_id: u32) {
        let mut cooldowns = self.cooldowns.write();
        if let Some(entry) = cooldowns.get(&char_id)
            && entry.template_id == template_id
        {
            cooldowns.remove(&char_id);
        }
    }

    pub fn all_instances(&self) -> Vec<Instance> {
        self.instances
            .read()
            .values()
            .map(|i| i.read().clone())
            .collect()
    }

    pub fn active_instances(&self) -> Vec<Instance> {
        self.instances
            .read()
            .values()
            .filter(|i| {
                let state = i.read().state;
                state == InstanceState::Waiting || state == InstanceState::Active
            })
            .map(|i| i.read().clone())
            .collect()
    }

    pub fn tick_instance(&self, instance_id: u32, seconds: u32) -> Result<(), InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let mut inst = instance.write();
        inst.timers.tick(seconds);

        if inst.timers.is_expired() {
            inst.expire();
            return Err(InstanceError::InvalidState);
        }

        Ok(())
    }

    pub fn update_objective(
        &self,
        instance_id: u32,
        char_id: u32,
        objective_type: InstanceObjectiveType,
        target: u16,
        count: u32,
    ) -> Result<(), InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let mut inst = instance.write();

        // Only active instances can have objectives updated
        if inst.state != InstanceState::Active {
            return Err(InstanceError::InstanceNotActive);
        }

        // Verify player is in instance
        if !inst.is_participant(char_id) {
            return Err(InstanceError::PlayerNotInInstance);
        }

        // Find matching objective
        for obj in &mut inst.objectives {
            if obj.objective_type == objective_type && (obj.target == 0 || obj.target == target) {
                obj.update_progress(count);
                break;
            }
        }

        // Check if all objectives are complete
        if inst.all_objectives_completed() {
            inst.complete();
        }

        Ok(())
    }

    pub fn spawn_mob(
        &self,
        instance_id: u32,
        _mob_id: u16,
        x: u16,
        y: u16,
    ) -> Result<u32, InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let entity_id = Uuid::new_v4().as_u128() as u32;
        let entity = InstanceEntity::new(EntityType::Monster, entity_id, x, y);

        let mut inst = instance.write();
        inst.spawn_entity(entity);

        Ok(entity_id)
    }

    pub fn despawn_mob(&self, instance_id: u32, entity_id: u32) -> Result<(), InstanceError> {
        let instance = {
            let instances = self.instances.read();
            instances
                .get(&instance_id)
                .cloned()
                .ok_or(InstanceError::NotFound)?
        };

        let mut inst = instance.write();
        if inst.despawn_entity(entity_id) {
            Ok(())
        } else {
            Err(InstanceError::NotFound)
        }
    }

    pub fn cleanup_expired_instances(&self) -> Vec<u32> {
        let mut instances = self.instances.write();
        let mut removed = Vec::new();

        instances.retain(|id, inst| {
            let state = inst.read().state;
            let should_remove = state == InstanceState::Expired
                || state == InstanceState::Completed
                || state == InstanceState::Aborted;

            if should_remove {
                // Clear player tracking for participants
                let mut player_instances = self.player_instances.write();
                for char_id in &inst.read().participants {
                    player_instances.remove(char_id);
                }
                removed.push(*id);
            }

            !should_remove
        });

        removed
    }

    pub fn instance_count(&self) -> usize {
        self.instances.read().len()
    }

    pub fn player_count(&self) -> usize {
        self.player_instances.read().len()
    }
}

impl Default for InstanceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::data::InstanceType;
    use super::*;

    fn create_test_manager() -> InstanceManager {
        let mut manager = InstanceManager::new();
        manager.load_default_templates();
        manager
    }

    #[test]
    fn test_create_instance() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();

        assert_eq!(instance.id, 1);
        assert_eq!(instance.name, "Training Dungeon");
        assert_eq!(instance.state, InstanceState::Waiting);
        assert!(instance.is_participant(100));
    }

    #[test]
    fn test_join_instance() {
        let manager = create_test_manager();

        let leader_instance = manager.create_instance(1, 100).unwrap();
        let instance_id = leader_instance.id;

        manager.join_instance(instance_id, 101).unwrap();
        manager.join_instance(instance_id, 102).unwrap();

        let instance = manager.get_instance(instance_id).unwrap();
        assert_eq!(instance.participants.len(), 3);
    }

    #[test]
    fn test_leave_instance() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        let instance_id = instance.id;

        manager.join_instance(instance_id, 101).unwrap();
        manager.leave_instance(101);

        let instance = manager.get_instance(instance_id).unwrap();
        assert!(!instance.is_participant(101));
    }

    #[test]
    fn test_player_already_in_instance() {
        let manager = create_test_manager();

        let instance1 = manager.create_instance(1, 100).unwrap();
        let instance2 = manager.create_instance(2, 101).unwrap();

        // Player 100 is in instance 1
        let result = manager.join_instance(instance2.id, 100);
        assert!(matches!(
            result,
            Err(InstanceError::PlayerAlreadyInInstance)
        ));
    }

    #[test]
    fn test_instance_start() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.start_instance(instance.id).unwrap();

        let updated = manager.get_instance(instance.id).unwrap();
        assert_eq!(updated.state, InstanceState::Active);
    }

    #[test]
    fn test_instance_complete() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.start_instance(instance.id).unwrap();
        manager.complete_instance(instance.id).unwrap();

        let updated = manager.get_instance(instance.id).unwrap();
        assert_eq!(updated.state, InstanceState::Completed);
    }

    #[test]
    fn test_cooldown_tracking() {
        let manager = create_test_manager();

        // Instance 2 has cooldown
        manager.create_instance(2, 100).unwrap();
        manager.start_cooldown(100, 2, 3600);

        let remaining = manager.check_cooldown(100, 2);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > 0);

        // No cooldown for other templates
        let remaining = manager.check_cooldown(100, 1);
        assert!(remaining.is_none());
    }

    #[test]
    fn test_update_objective() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.start_instance(instance.id).unwrap();

        manager
            .update_objective(instance.id, 100, InstanceObjectiveType::KillAllMobs, 0, 1)
            .unwrap();

        let updated = manager.get_instance(instance.id).unwrap();
        assert_eq!(updated.objectives[0].current_count, 1);
    }

    #[test]
    fn test_objective_completion_triggers_completion() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.start_instance(instance.id).unwrap();

        // Complete all objectives
        for _ in 0..8 {
            manager
                .update_objective(instance.id, 100, InstanceObjectiveType::KillAllMobs, 0, 1)
                .unwrap();
        }

        let updated = manager.get_instance(instance.id).unwrap();
        assert_eq!(updated.state, InstanceState::Completed);
    }

    #[test]
    fn test_abort_instance() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.join_instance(instance.id, 101).unwrap();

        manager.abort_instance(instance.id);

        let updated = manager.get_instance(instance.id);
        assert!(updated.is_none()); // Should be cleaned up
    }

    #[test]
    fn test_cleanup_expired_instances() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.start_instance(instance.id).unwrap();
        manager.complete_instance(instance.id).unwrap();

        let removed = manager.cleanup_expired_instances();
        assert_eq!(removed.len(), 1);
        assert!(manager.get_instance(instance.id).is_none());
    }

    #[test]
    fn test_get_player_instance() {
        let manager = create_test_manager();

        let instance = manager.create_instance(1, 100).unwrap();
        manager.join_instance(instance.id, 101).unwrap();

        let player_instance = manager.get_player_instance(101).unwrap();
        assert_eq!(player_instance.id, instance.id);

        let no_instance = manager.get_player_instance(999);
        assert!(no_instance.is_none());
    }

    #[test]
    fn test_instance_full() {
        let mut manager = create_test_manager();

        // Register a template with max 2 players
        manager.register_template(
            InstanceTemplate::new(100, "Small Instance", "test", InstanceType::PvM)
                .with_player_range(1, 2),
        );

        let instance = manager.create_instance(100, 1).unwrap();
        manager.join_instance(instance.id, 2).unwrap();

        let result = manager.join_instance(instance.id, 3);
        assert!(matches!(result, Err(InstanceError::InstanceFull)));
    }
}
