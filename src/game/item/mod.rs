//! 物品系统

pub mod card;
pub mod data;
pub mod delay;
pub mod effect;
pub mod effect_config;
pub mod equipment;
pub mod handler;
pub mod integration;
pub mod inventory;
pub mod refine;
pub mod result;
pub mod script;
pub mod use_handler;
pub mod yaml_loader;

pub use card::{CardBonus, CardResult, CardSystem};
pub use data::{Item, ItemDatabase, ItemFlag, ItemType};
pub use delay::{GlobalItemDelayManager, ItemDelay, ItemDelayTracker};
pub use effect::{
    EffectError, EffectResult, ItemEffect, ItemUseResult as UseResult, StatType, parse_item_script,
};
pub use effect_config::{ItemEffectConfig, ItemEffectDatabase, ItemEffectType, ItemRequirements};
pub use equipment::{EquipSlot, Equipment};
pub use handler::ItemHandler;
pub use integration::ItemIntegrationHandler;
pub use inventory::{Inventory, InventorySlot};
pub use refine::{RefineBonus, RefineResult, RefineSystem};
pub use result::ItemUseResult;
pub use use_handler::{ItemUseHandler, ItemUseValidator};
pub use yaml_loader::ItemDbLoader;
