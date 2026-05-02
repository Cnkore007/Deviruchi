use std::sync::Arc;
use super::data::ItemDatabase;
use super::inventory::Inventory;
use crate::game::map::Player;

pub struct ItemHandler {
    db: Arc<ItemDatabase>,
}

impl ItemHandler {
    pub fn new() -> Self {
        Self {
            db: Arc::new(ItemDatabase::new()),
        }
    }

    /// 使用背包中的物品
    pub fn use_item(&self, player: &Player, inventory: &mut Inventory, slot_index: u8) -> ItemUseResult {
        let item = match inventory.use_item(slot_index) {
            Some(i) => i,
            None => return ItemUseResult::Failed(ItemUseError::InvalidSlot),
        };

        // 应用物品效果
        if item.hp_restore > 0 {
            let current_hp = *player.hp.read();
            let max_hp = *player.max_hp.read();
            let new_hp = (current_hp + item.hp_restore as u32).min(max_hp);
            *player.hp.write() = new_hp;
        }

        if item.sp_restore > 0 {
            let current_sp = *player.sp.read();
            let max_sp = *player.max_sp.read();
            let new_sp = (current_sp + item.sp_restore as u32).min(max_sp);
            *player.sp.write() = new_sp;
        }

        ItemUseResult::Success {
            hp_restored: item.hp_restore,
            sp_restored: item.sp_restore,
        }
    }

    /// 获取物品数据库
    pub fn get_database(&self) -> Arc<ItemDatabase> {
        self.db.clone()
    }

    /// 创建背包
    pub fn create_inventory(&self, max_size: u8) -> Inventory {
        Inventory::new(max_size, self.db.clone())
    }
}

impl Default for ItemHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 物品使用结果
#[derive(Debug, Clone)]
pub enum ItemUseResult {
    Success {
        hp_restored: u16,
        sp_restored: u16,
    },
    Failed(ItemUseError),
}

/// 物品使用错误
#[derive(Debug, Clone, Copy)]
pub enum ItemUseError {
    InvalidSlot,
    NotUsable,
    InventoryFull,
}
