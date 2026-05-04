//! Instance template system

use crate::game::instance::data::{InstanceObjective, InstanceObjectiveType, InstanceType};
use std::collections::HashMap;

/// Mob spawn definition in instance
#[derive(Debug, Clone)]
pub struct InstanceMobSpawn {
    pub mob_id: u16,
    pub x: u16,
    pub y: u16,
    pub respawn_secs: u32,
    pub count: u16,
}

impl InstanceMobSpawn {
    pub fn new(mob_id: u16, x: u16, y: u16, count: u16) -> Self {
        Self {
            mob_id,
            x,
            y,
            respawn_secs: 0,
            count,
        }
    }

    pub fn with_respawn(mut self, respawn_secs: u32) -> Self {
        self.respawn_secs = respawn_secs;
        self
    }
}

/// NPC definition in instance
#[derive(Debug, Clone)]
pub struct InstanceNpc {
    pub npc_id: u16,
    pub x: u16,
    pub y: u16,
    pub name: String,
}

impl InstanceNpc {
    pub fn new(npc_id: u16, x: u16, y: u16, name: &str) -> Self {
        Self {
            npc_id,
            x,
            y,
            name: name.to_string(),
        }
    }
}

/// Portal definition in instance
#[derive(Debug, Clone)]
pub struct InstancePortal {
    pub x: u16,
    pub y: u16,
    pub target_map: String,
    pub target_x: u16,
    pub target_y: u16,
}

impl InstancePortal {
    pub fn new(x: u16, y: u16, target_map: &str, target_x: u16, target_y: u16) -> Self {
        Self {
            x,
            y,
            target_map: target_map.to_string(),
            target_x,
            target_y,
        }
    }
}

/// Instance template - defines the blueprint for an instance
#[derive(Debug, Clone)]
pub struct InstanceTemplate {
    pub id: u32,
    pub name: String,
    pub map_name: String,
    pub instance_type: InstanceType,
    pub min_level: u16,
    pub max_level: u16,
    pub min_players: u16,
    pub max_players: u16,
    pub duration_secs: u32,
    pub idle_timeout_secs: u32,
    pub enter_items: Vec<u16>,
    pub enter_zenny: u32,
    pub cooldown_secs: u32,
    pub mobs: Vec<InstanceMobSpawn>,
    pub npcs: Vec<InstanceNpc>,
    pub portals: Vec<InstancePortal>,
    pub objectives: Vec<InstanceObjective>,
    pub entry_map: String,
    pub entry_x: u16,
    pub entry_y: u16,
}

impl InstanceTemplate {
    pub fn new(id: u32, name: &str, map_name: &str, instance_type: InstanceType) -> Self {
        Self {
            id,
            name: name.to_string(),
            map_name: map_name.to_string(),
            instance_type,
            min_level: 1,
            max_level: 999,
            min_players: 1,
            max_players: 12,
            duration_secs: 3600,
            idle_timeout_secs: 300,
            enter_items: Vec::new(),
            enter_zenny: 0,
            cooldown_secs: 0,
            mobs: Vec::new(),
            npcs: Vec::new(),
            portals: Vec::new(),
            objectives: Vec::new(),
            entry_map: map_name.to_string(),
            entry_x: 0,
            entry_y: 0,
        }
    }

    pub fn with_level_range(mut self, min: u16, max: u16) -> Self {
        self.min_level = min;
        self.max_level = max;
        self
    }

    pub fn with_player_range(mut self, min: u16, max: u16) -> Self {
        self.min_players = min;
        self.max_players = max;
        self
    }

    pub fn with_duration(mut self, seconds: u32) -> Self {
        self.duration_secs = seconds;
        self
    }

    pub fn with_idle_timeout(mut self, seconds: u32) -> Self {
        self.idle_timeout_secs = seconds;
        self
    }

    pub fn with_cooldown(mut self, seconds: u32) -> Self {
        self.cooldown_secs = seconds;
        self
    }

    pub fn with_entry_cost(mut self, zenny: u32, items: Vec<u16>) -> Self {
        self.enter_zenny = zenny;
        self.enter_items = items;
        self
    }

    pub fn with_entry_position(mut self, map: &str, x: u16, y: u16) -> Self {
        self.entry_map = map.to_string();
        self.entry_x = x;
        self.entry_y = y;
        self
    }

    pub fn with_mobs(mut self, mobs: Vec<InstanceMobSpawn>) -> Self {
        self.mobs = mobs;
        self
    }

    pub fn with_objectives(mut self, objectives: Vec<InstanceObjective>) -> Self {
        self.objectives = objectives;
        self
    }

    pub fn add_mob(&mut self, mob: InstanceMobSpawn) {
        self.mobs.push(mob);
    }

    pub fn add_npc(&mut self, npc: InstanceNpc) {
        self.npcs.push(npc);
    }

    pub fn add_portal(&mut self, portal: InstancePortal) {
        self.portals.push(portal);
    }

    pub fn add_objective(&mut self, objective: InstanceObjective) {
        self.objectives.push(objective);
    }

    pub fn has_cooldown(&self) -> bool {
        self.cooldown_secs > 0
    }

    pub fn has_requirements(&self) -> bool {
        self.enter_zenny > 0 || !self.enter_items.is_empty()
    }

    pub fn total_mob_count(&self) -> u32 {
        self.mobs.iter().map(|m| m.count as u32).sum()
    }
}

/// Instance template database
#[derive(Debug, Clone, Default)]
pub struct InstanceTemplateDatabase {
    templates: HashMap<u32, InstanceTemplate>,
}

impl InstanceTemplateDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, template: InstanceTemplate) {
        self.templates.insert(template.id, template);
    }

    pub fn get(&self, id: u32) -> Option<&InstanceTemplate> {
        self.templates.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut InstanceTemplate> {
        self.templates.get_mut(&id)
    }

    pub fn all(&self) -> Vec<&InstanceTemplate> {
        self.templates.values().collect()
    }

    pub fn get_by_type(&self, instance_type: InstanceType) -> Vec<&InstanceTemplate> {
        self.templates
            .values()
            .filter(|t| t.instance_type == instance_type)
            .collect()
    }

    pub fn load_default_templates(&mut self) {
        // Training Dungeon
        self.register(
            InstanceTemplate::new(1, "Training Dungeon", "train_dun01", InstanceType::Tutorial)
                .with_level_range(1, 10)
                .with_player_range(1, 6)
                .with_duration(1800)
                .with_entry_position("new_1-1", 50, 100)
                .with_mobs(vec![
                    InstanceMobSpawn::new(1001, 50, 50, 5),
                    InstanceMobSpawn::new(1002, 80, 60, 3),
                ])
                .with_objectives(vec![InstanceObjective::new(
                    1,
                    InstanceObjectiveType::KillAllMobs,
                    0,
                    8,
                    "Defeat all training monsters",
                )]),
        );

        // Poring Cave
        self.register(
            InstanceTemplate::new(2, "Poring Cave", "poring_c01", InstanceType::PvM)
                .with_level_range(5, 30)
                .with_player_range(1, 6)
                .with_duration(3600)
                .with_cooldown(3600)
                .with_entry_position("new_1-3", 100, 150)
                .with_mobs(vec![
                    InstanceMobSpawn::new(1001, 30, 30, 10),
                    InstanceMobSpawn::new(1002, 50, 50, 8),
                    InstanceMobSpawn::new(1003, 70, 70, 5),
                ])
                .with_objectives(vec![InstanceObjective::new(
                    1,
                    InstanceObjectiveType::KillAllMobs,
                    0,
                    23,
                    "Clear all monsters in the cave",
                )]),
        );

        // Toy Factory
        self.register(
            InstanceTemplate::new(3, "Toy Factory", "xmas_dun01", InstanceType::PvM)
                .with_level_range(30, 60)
                .with_player_range(3, 6)
                .with_duration(3600)
                .with_cooldown(7200)
                .with_entry_position("xmas", 150, 200)
                .with_mobs(vec![
                    InstanceMobSpawn::new(1312, 40, 40, 15),
                    InstanceMobSpawn::new(1313, 60, 60, 10),
                    InstanceMobSpawn::new(1314, 80, 80, 5), // Mini boss
                ])
                .with_objectives(vec![
                    InstanceObjective::new(
                        1,
                        InstanceObjectiveType::KillBoss,
                        1314,
                        1,
                        "Defeat the Toy Factory Guardian",
                    ),
                    InstanceObjective::new(
                        2,
                        InstanceObjectiveType::KillAllMobs,
                        0,
                        25,
                        "Clear all monsters",
                    ),
                ]),
        );

        // Endless Tower (Event)
        self.register(
            InstanceTemplate::new(4, "Endless Tower", "tower_01", InstanceType::Event)
                .with_level_range(50, 99)
                .with_player_range(1, 5)
                .with_duration(1800)
                .with_cooldown(600)
                .with_entry_position("prontera", 150, 150)
                .with_mobs(vec![
                    InstanceMobSpawn::new(1312, 50, 50, 5).with_respawn(60),
                    InstanceMobSpawn::new(1313, 70, 70, 3).with_respawn(60),
                ])
                .with_objectives(vec![
                    InstanceObjective::new(
                        1,
                        InstanceObjectiveType::Survive,
                        0,
                        10,
                        "Survive 10 waves",
                    ),
                    InstanceObjective::new(
                        2,
                        InstanceObjectiveType::CollectItem,
                        7501,
                        1,
                        "Collect the Tower Key",
                    ),
                ]),
        );

        // PvP Arena
        self.register(
            InstanceTemplate::new(5, "PvP Arena", "pvp_n_1-1", InstanceType::PvP)
                .with_level_range(30, 99)
                .with_player_range(2, 10)
                .with_duration(900)
                .with_cooldown(300)
                .with_entry_position("prontera", 160, 170)
                .with_objectives(vec![InstanceObjective::new(
                    1,
                    InstanceObjectiveType::KillAllMobs,
                    0,
                    0,
                    "Last player standing wins",
                )]),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        let template = InstanceTemplate::new(1, "Test Dungeon", "test_map", InstanceType::PvM);

        assert_eq!(template.id, 1);
        assert_eq!(template.name, "Test Dungeon");
        assert_eq!(template.min_level, 1);
        assert_eq!(template.max_players, 12);
        assert!(!template.has_cooldown());
    }

    #[test]
    fn test_template_builder() {
        let template = InstanceTemplate::new(1, "Test", "map", InstanceType::PvM)
            .with_level_range(10, 50)
            .with_player_range(2, 6)
            .with_duration(7200)
            .with_cooldown(3600);

        assert_eq!(template.min_level, 10);
        assert_eq!(template.max_level, 50);
        assert_eq!(template.min_players, 2);
        assert_eq!(template.max_players, 6);
        assert_eq!(template.duration_secs, 7200);
        assert!(template.has_cooldown());
    }

    #[test]
    fn test_add_mobs_and_objectives() {
        let mut template = InstanceTemplate::new(1, "Test", "map", InstanceType::PvM);

        template.add_mob(InstanceMobSpawn::new(1001, 10, 20, 5));
        template.add_mob(InstanceMobSpawn::new(1002, 30, 40, 3));

        template.add_objective(InstanceObjective::new(
            1,
            InstanceObjectiveType::KillAllMobs,
            0,
            8,
            "Clear all monsters",
        ));

        assert_eq!(template.mobs.len(), 2);
        assert_eq!(template.total_mob_count(), 8);
        assert_eq!(template.objectives.len(), 1);
    }

    #[test]
    fn test_template_database() {
        let mut db = InstanceTemplateDatabase::new();

        db.register(InstanceTemplate::new(1, "Test1", "map1", InstanceType::PvM));
        db.register(InstanceTemplate::new(2, "Test2", "map2", InstanceType::PvP));

        assert_eq!(db.get(1).unwrap().name, "Test1");
        assert_eq!(db.get(2).unwrap().name, "Test2");
        assert!(db.get(99).is_none());

        assert_eq!(db.get_by_type(InstanceType::PvM).len(), 1);
        assert_eq!(db.get_by_type(InstanceType::PvP).len(), 1);
        assert!(db.get_by_type(InstanceType::Quest).is_empty());
    }

    #[test]
    fn test_load_default_templates() {
        let mut db = InstanceTemplateDatabase::new();
        db.load_default_templates();

        assert!(db.get(1).is_some()); // Training Dungeon
        assert!(db.get(2).is_some()); // Poring Cave
        assert!(db.get(3).is_some()); // Toy Factory

        let training = db.get(1).unwrap();
        assert_eq!(training.instance_type, InstanceType::Tutorial);
        assert!(training.mobs.len() >= 2);
    }
}
