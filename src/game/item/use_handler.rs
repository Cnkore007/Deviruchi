use crate::game::item::data::{Item, ItemDatabase, ItemType};
use crate::game::item::delay::ItemDelayTracker;
use crate::game::item::effect::{EffectResult, ItemEffect, ItemUseResult};
use crate::game::item::equipment::EquipSlot;
use crate::game::item::inventory::Inventory;
use crate::game::map::Player;
use crate::game::status::{StatusChange, StatusEffect, StatusSource};
use std::sync::Arc;
use uuid::Uuid;

/// 物品使用验证器
#[allow(dead_code)]
pub struct ItemUseValidator {
    /// 冷却追踪器
    delay_tracker: Arc<ItemDelayTracker>,
    /// 允许在战斗中使用
    allow_in_battle: bool,
    /// 允许在死亡状态使用
    allow_while_dead: bool,
    /// 允许在水下使用
    allow_underwater: bool,
}

impl ItemUseValidator {
    pub fn new(delay_tracker: Arc<ItemDelayTracker>) -> Self {
        Self {
            delay_tracker,
            allow_in_battle: true,
            allow_while_dead: false,
            allow_underwater: true,
        }
    }

    /// 验证物品是否可以使用
    pub fn validate(&self, player: &Player, item: &Item) -> ItemUseResult {
        // 检查冷却
        let remaining = self.delay_tracker.remaining_cooldown(player.id, item.id);
        if remaining > 0 {
            return ItemUseResult::Cooldown {
                remaining_ms: remaining,
            };
        }

        // 检查死亡状态
        if !self.allow_while_dead && player.hp() == 0 {
            // 允许复活类物品 - 这里需要检查物品脚本或效果
            // 目前简单处理
        }

        // 检查物品类型
        match item.type_ {
            ItemType::Heal | ItemType::Etc => {
                // 这些类型默认可以使用
            }
            _ => {
                return ItemUseResult::Failed("该物品无法直接使用".to_string());
            }
        }

        ItemUseResult::Success
    }

    /// 设置是否允许战斗中使用
    pub fn set_allow_in_battle(&mut self, allow: bool) {
        self.allow_in_battle = allow;
    }

    /// 设置是否允许死亡时使用
    pub fn set_allow_while_dead(&mut self, allow: bool) {
        self.allow_while_dead = allow;
    }
}

/// 物品使用处理器
pub struct ItemUseHandler {
    /// 数据库引用
    db: Arc<ItemDatabase>,
    /// 冷却追踪器
    delay_tracker: Arc<ItemDelayTracker>,
    /// 验证器
    validator: ItemUseValidator,
}

impl ItemUseHandler {
    pub fn new(db: Arc<ItemDatabase>, delay_tracker: Arc<ItemDelayTracker>) -> Self {
        Self {
            db,
            delay_tracker: delay_tracker.clone(),
            validator: ItemUseValidator::new(delay_tracker),
        }
    }

    /// 使用物品
    pub fn use_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        slot_index: u8,
    ) -> ItemUseResult {
        // 获取物品槽
        let slot = match inventory.slots().get(slot_index as usize) {
            Some(s) if !s.is_empty() => s.clone(),
            _ => return ItemUseResult::Failed("物品槽为空".to_string()),
        };

        // 获取物品数据
        let item = match self.db.get(slot.item_id) {
            Some(i) => i.clone(),
            None => return ItemUseResult::Failed("物品不存在".to_string()),
        };

        // 验证物品是否可以使用
        let validation_result = self.validator.validate(player, &item);
        if !validation_result.is_success() {
            return validation_result;
        }

        // 根据物品类型处理效果
        // 治疗类物品直接使用
        match item.type_ {
            ItemType::Heal => {
                // 恢复HP
                if item.hp_restore > 0 {
                    let current = player.hp();
                    let max = player.max_hp();
                    let new_hp = (current + item.hp_restore as u32).min(max);
                    player.combat_mut().hp = new_hp;
                }
                // 恢复SP
                if item.sp_restore > 0 {
                    let current = player.sp();
                    let max = player.max_sp();
                    let new_sp = (current + item.sp_restore as u32).min(max);
                    player.combat_mut().sp = new_sp;
                }
            }
            _ => {
                // 其他物品类型暂不处理
            }
        }

        // 消耗物品
        if inventory.use_item(slot_index).is_none() {
            return ItemUseResult::Failed("消耗物品失败".to_string());
        }

        ItemUseResult::Success
    }

    /// 检查物品冷却
    pub fn is_on_cooldown(&self, player_id: Uuid, item_id: u16) -> bool {
        self.delay_tracker.is_on_cooldown(player_id, item_id)
    }

    /// 获取物品冷却剩余时间
    pub fn get_cooldown_remaining(&self, player_id: Uuid, item_id: u16) -> u64 {
        self.delay_tracker.remaining_cooldown(player_id, item_id)
    }
}

/// 应用物品效果的辅助函数
pub fn apply_item_effect(
    player: &Player,
    inventory: &mut Inventory,
    effect: &ItemEffect,
    item_id: u16,
) -> ItemUseResult {
    match effect {
        // 传送效果
        ItemEffect::Teleport { map, x, y } | ItemEffect::Scroll { map, x, y } => {
            execute_teleport(player, map, *x, *y)
        }

        // 状态开始
        ItemEffect::StatusStart {
            status,
            val1,
            val2,
            val3,
            duration_ms,
        } => execute_status_start(player, *status, *val1, *val2, *val3, *duration_ms, item_id),

        // 获得物品
        ItemEffect::GetItem {
            item_id: target_id,
            count,
            rate,
        } => execute_get_item(player, inventory, *target_id, *count, *rate),

        // 制作物品
        ItemEffect::Produce(item_id) => execute_produce(player, inventory, *item_id),

        // 使用技能
        ItemEffect::UseSkill { skill_id, level } => execute_use_skill(player, *skill_id, *level),

        // 剥离装备
        ItemEffect::StripArmor | ItemEffect::StripWeapon | ItemEffect::StripAccessory => {
            execute_strip_equipment(player, effect)
        }

        // 隐身
        ItemEffect::Hide => execute_hide(player, item_id),

        // 忍耐/无敌
        ItemEffect::Endure {
            duration_ms,
            is_invincible,
        } => execute_endure(player, *duration_ms, *is_invincible, item_id),

        // 消耗弹药
        ItemEffect::ConsumeAmmo => execute_consume_ammo(player),

        // 伪装
        ItemEffect::Disguise(mob_id) => execute_disguise(player, *mob_id),

        // 心理控制（暂不支持）
        ItemEffect::MindControl => ItemUseResult::Failed("心灵控制暂不支持".to_string()),

        // 其他效果使用默认执行
        _ => {
            let result = effect.apply(player);
            match result {
                EffectResult::Success => ItemUseResult::Success,
                EffectResult::Failed(_) => ItemUseResult::Failed("效果执行失败".to_string()),
                EffectResult::PartialSuccess { msg } => ItemUseResult::Failed(msg),
            }
        }
    }
}

/// 执行传送效果
fn execute_teleport(player: &Player, map: &str, x: i32, y: i32) -> ItemUseResult {
    log::warn!(
        "Item teleport not yet implemented: player={}, map={}, ({}, {})",
        player.id,
        map,
        x,
        y
    );
    ItemUseResult::Failed("传送功能尚未实现".to_string())
}

/// 执行状态效果开始
fn execute_status_start(
    player: &Player,
    status: StatusChange,
    val1: i32,
    val2: i32,
    val3: i32,
    duration_ms: u64,
    item_id: u16,
) -> ItemUseResult {
    let effect = StatusEffect::with_values(
        status,
        duration_ms,
        StatusSource::Item(item_id),
        val1,
        val2,
        val3,
    );
    player.status.add_status(effect);
    ItemUseResult::Success
}

/// 执行获得物品
fn execute_get_item(
    player: &Player,
    inventory: &mut Inventory,
    item_id: u16,
    count: u16,
    rate: u16,
) -> ItemUseResult {
    use rand::Rng;

    // 根据概率决定是否获得物品
    let mut rng = rand::thread_rng();
    let roll = rng.gen_range(0..10000);

    if roll < rate {
        // 检查背包空间
        if inventory.can_add_item(item_id, count) && inventory.add_item(item_id, count) {
            log::info!("Player {} got item {} x{}", player.id, item_id, count);
            return ItemUseResult::Success;
        }
        return ItemUseResult::Failed("背包空间不足".to_string());
    }

    // 概率未中，不获得物品也不算失败
    ItemUseResult::Success
}

/// 执行制作物品
fn execute_produce(player: &Player, inventory: &mut Inventory, item_id: i32) -> ItemUseResult {
    let item_id = item_id as u16;

    // 检查背包空间
    if !inventory.can_add_item(item_id, 1) {
        return ItemUseResult::Failed("背包空间不足".to_string());
    }

    // 添加物品
    if inventory.add_item(item_id, 1) {
        log::info!("Player {} produced item {}", player.id, item_id);
        ItemUseResult::Success
    } else {
        ItemUseResult::Failed("制作失败".to_string())
    }
}

/// 执行使用技能
fn execute_use_skill(player: &Player, skill_id: u16, level: u8) -> ItemUseResult {
    log::warn!(
        "Item skill use not yet implemented: player={}, skill_id={}, level={}",
        player.id,
        skill_id,
        level
    );
    ItemUseResult::Failed("物品技能使用功能尚未实现".to_string())
}

/// 执行剥离装备
fn execute_strip_equipment(player: &Player, effect: &ItemEffect) -> ItemUseResult {
    let equipment = player.equipment.read();

    let slot = match effect {
        ItemEffect::StripArmor => Some(EquipSlot::Body),
        ItemEffect::StripWeapon => Some(EquipSlot::RightHand),
        ItemEffect::StripAccessory => Some(EquipSlot::Accessory1),
        _ => None,
    };

    if let Some(slot) = slot
        && equipment.get(slot).is_some()
    {
        log::warn!("Item strip equipment not yet implemented: player={}, slot={:?}", player.id, slot);
    }

    ItemUseResult::Failed("装备剥离功能尚未实现".to_string())
}

/// 执行隐身
fn execute_hide(player: &Player, item_id: u16) -> ItemUseResult {
    let effect = StatusEffect::new(
        StatusChange::Hide,
        300000, // 5分钟默认
        StatusSource::Item(item_id),
    );
    player.status.add_status(effect);
    ItemUseResult::Success
}

/// 执行忍耐/无敌
fn execute_endure(
    player: &Player,
    duration_ms: u64,
    is_invincible: bool,
    item_id: u16,
) -> ItemUseResult {
    let status_type = if is_invincible {
        StatusChange::Invincible
    } else {
        StatusChange::Haste
    };

    let effect = StatusEffect::new(status_type, duration_ms, StatusSource::Item(item_id));
    player.status.add_status(effect);
    ItemUseResult::Success
}

/// 执行消耗弹药
fn execute_consume_ammo(player: &Player) -> ItemUseResult {
    log::warn!("Ammo consumption not yet implemented: player={}", player.id);
    ItemUseResult::Failed("弹药消耗功能尚未实现".to_string())
}

/// 执行伪装
fn execute_disguise(player: &Player, mob_id: u16) -> ItemUseResult {
    log::warn!("Disguise not yet implemented: player={}, mob_id={}", player.id, mob_id);
    ItemUseResult::Failed("伪装功能尚未实现".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_initial_state() {
        let tracker = Arc::new(ItemDelayTracker::new());
        let validator = ItemUseValidator::new(tracker);

        // 初始状态应该允许使用
        // (需要模拟玩家和物品来完整测试)
    }

    #[test]
    fn test_delay_tracker_integration() {
        let tracker = Arc::new(ItemDelayTracker::new());
        let player_id = Uuid::new_v4();
        let item_id = 501;

        // 开始冷却
        tracker.start_cooldown_with_duration(player_id, item_id, 100);

        // 冷却追踪器应该显示在冷却中
        assert!(tracker.is_on_cooldown(player_id, item_id));
    }
}
