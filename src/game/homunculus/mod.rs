//! 生命体 (Homunculus) 系统

pub mod data;
pub mod manager;

pub use data::{
    EvolutionStage, Homunculus, HomunculusDatabase, HomunculusRace, HomunculusTemplate,
    HomunculusType,
};
pub use manager::{HomunculusError, HomunculusManager};
