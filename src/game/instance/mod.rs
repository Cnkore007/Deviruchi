//! Instance system - private dungeon instances for groups of players

pub mod data;
pub mod manager;
pub mod template;

pub use data::{
    EntityType, Instance, InstanceEntity, InstanceObjective, InstanceObjectiveType, InstanceState,
    InstanceTimers, InstanceType,
};

pub use template::{
    InstanceMobSpawn, InstanceNpc, InstancePortal, InstanceTemplate, InstanceTemplateDatabase,
};

pub use manager::{InstanceError, InstanceManager};
