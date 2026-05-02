//! 物品系统

pub mod data;
pub mod inventory;
pub mod handler;
pub mod equipment;
pub mod effect;
pub mod yaml_loader;

pub use data::{Item, ItemType, ItemFlag, ItemDatabase};
pub use inventory::{Inventory, InventorySlot};
pub use handler::ItemHandler;
pub use equipment::{Equipment, EquipSlot};
pub use effect::{ItemEffect, EffectResult, EffectError, StatType, parse_item_script};
pub use yaml_loader::ItemDbLoader;
