//! 物品效果集成处理器
//!
//! 负责将物品效果与地图系统、技能系统、传送系统集成

use super::ItemDatabase;
use super::delay::ItemDelayTracker;
use super::effect_config::{ItemEffectDatabase, ItemEffectType};
use super::result::ItemUseResult;
use crate::game::map::teleport::WarpService;
use crate::game::map::{MapState, Player};
use crate::game::skill::SkillHandler;
use crate::game::status::{StatusChange, StatusEffect, StatusSource};
use std::sync::Arc;
use uuid::Uuid;

/// 物品效果集成处理器
pub struct ItemIntegrationHandler {
    /// 物品效果数据库
    effect_db: Arc<ItemEffectDatabase>,
    /// 物品数据库
    item_db: Arc<ItemDatabase>,
    /// 冷却时间追踪器
    delay_tracker: Arc<ItemDelayTracker>,
}

impl ItemIntegrationHandler {
    /// 获取物品数据库
    pub fn item_db(&self) -> Arc<ItemDatabase> {
        self.item_db.clone()
    }

    /// 创建新的处理器
    pub fn new(effect_db: Arc<ItemEffectDatabase>, item_db: Arc<ItemDatabase>) -> Self {
        Self {
            effect_db,
            item_db,
            delay_tracker: Arc::new(ItemDelayTracker::new()),
        }
    }

    /// 创建带有冷却追踪器的处理器
    pub fn with_delay_tracker(
        effect_db: Arc<ItemEffectDatabase>,
        item_db: Arc<ItemDatabase>,
        delay_tracker: Arc<ItemDelayTracker>,
    ) -> Self {
        Self {
            effect_db,
            item_db,
            delay_tracker,
        }
    }

    /// 使用物品
    pub fn use_item(
        &self,
        player: &Player,
        inventory: &mut super::inventory::Inventory,
        item_id: u16,
        warp_service: &WarpService,
        skill_handler: &SkillHandler,
        map_state: &MapState,
    ) -> ItemUseResult {
        // 1. 获取物品效果配置
        let effect_config = match self.effect_db.get(item_id) {
            Some(config) => config,
            None => {
                // 如果没有特殊配置，使用物品数据库中的默认配置
                return self.use_default_item(player, inventory, item_id);
            }
        };

        // 2. 检查使用条件
        if let Some(reason) = effect_config.requirements.check(player) {
            return ItemUseResult::RequirementsNotMet(reason);
        }

        // 3. 检查冷却
        if effect_config.cooldown_ms > 0 {
            let player_id = player.id;
            if self.delay_tracker.is_on_cooldown(player_id, item_id) {
                let remaining = self.delay_tracker.remaining_cooldown(player_id, item_id);
                return ItemUseResult::CooldownActive {
                    remaining_ms: remaining,
                };
            }
        }

        // 4. 应用物品效果
        let result = self.apply_effect(
            player,
            inventory,
            &effect_config.effect_type,
            item_id,
            warp_service,
            skill_handler,
            map_state,
        );

        // 5. 消耗物品数量并启动冷却
        if result.is_success() {
            inventory.use_item(0); // 消耗1个物品
            // 如果有冷却时间，启动冷却
            if effect_config.cooldown_ms > 0 {
                self.delay_tracker.start_cooldown_with_duration(
                    player.id,
                    item_id,
                    effect_config.cooldown_ms,
                );
            }
        }

        result
    }

    /// 使用物品（指定槽位）
    pub fn use_item_at_slot(
        &self,
        player: &Player,
        inventory: &mut super::inventory::Inventory,
        slot_index: u8,
        warp_service: &WarpService,
        skill_handler: &SkillHandler,
        map_state: &MapState,
    ) -> ItemUseResult {
        // 获取物品槽
        let slot = match inventory.slots().get(slot_index as usize) {
            Some(s) if !s.is_empty() => s.clone(),
            _ => return ItemUseResult::ItemNotFound,
        };

        // 获取物品效果配置
        let effect_config = match self.effect_db.get(slot.item_id) {
            Some(config) => config,
            None => {
                return self.use_default_item_at_slot(player, inventory, slot_index);
            }
        };

        // 检查使用条件
        if let Some(reason) = effect_config.requirements.check(player) {
            return ItemUseResult::RequirementsNotMet(reason);
        }

        // 检查冷却
        if effect_config.cooldown_ms > 0 {
            let player_id = player.id;
            if self.delay_tracker.is_on_cooldown(player_id, slot.item_id) {
                let remaining = self
                    .delay_tracker
                    .remaining_cooldown(player_id, slot.item_id);
                return ItemUseResult::CooldownActive {
                    remaining_ms: remaining,
                };
            }
        }

        // 应用效果
        let result = self.apply_effect(
            player,
            inventory,
            &effect_config.effect_type,
            slot.item_id,
            warp_service,
            skill_handler,
            map_state,
        );

        // 消耗物品并启动冷却
        if result.is_success() {
            inventory.use_item(slot_index);
            if effect_config.cooldown_ms > 0 {
                self.delay_tracker.start_cooldown_with_duration(
                    player.id,
                    slot.item_id,
                    effect_config.cooldown_ms,
                );
            }
        }

        result
    }

    /// 应用物品效果
    fn apply_effect(
        &self,
        player: &Player,
        inventory: &mut super::inventory::Inventory,
        effect_type: &ItemEffectType,
        item_id: u16,
        warp_service: &WarpService,
        skill_handler: &SkillHandler,
        map_state: &MapState,
    ) -> ItemUseResult {
        match effect_type {
            ItemEffectType::Teleport { map, x, y } => ItemUseResult::Teleport {
                map: map.clone(),
                x: *x,
                y: *y,
            },

            ItemEffectType::RandomTeleport { range } => {
                ItemUseResult::RandomTeleport { range: *range }
            }

            ItemEffectType::SavePoint => {
                // 需要在外部执行实际传送
                ItemUseResult::Teleport {
                    map: player.map_name.clone(),
                    x: player.get_save_point().0,
                    y: player.get_save_point().1,
                }
            }

            ItemEffectType::HealHp { amount } => {
                self.apply_heal(player, *amount as i32, 0);
                ItemUseResult::Success(format!("HP +{}", amount))
            }

            ItemEffectType::HealSp { amount } => {
                self.apply_heal(player, 0, *amount as i32);
                ItemUseResult::Success(format!("SP +{}", amount))
            }

            ItemEffectType::HealBoth { hp, sp } => {
                self.apply_heal(player, *hp as i32, *sp as i32);
                ItemUseResult::Success(format!("HP +{}, SP +{}", hp, sp))
            }

            ItemEffectType::PercentHeal {
                hp_percent,
                sp_percent,
            } => {
                let max_hp = player.max_hp();
                let max_sp = player.max_sp();
                let _current_hp = player.hp();
                let _current_sp = player.sp();

                let hp_amount = (max_hp as i32 * *hp_percent as i32 / 100).max(0);
                let sp_amount = (max_sp as i32 * *sp_percent as i32 / 100).max(0);

                self.apply_heal(player, hp_amount, sp_amount);
                ItemUseResult::Success(format!("HP +{}%, SP +{}%", hp_percent, sp_percent))
            }

            ItemEffectType::LearnSkill { skill_id } => {
                // 学习技能
                if let Err(e) = skill_handler.learn_skill(player, *skill_id as u16) {
                    tracing::warn!(
                        "Player {} failed to learn skill {}: {:?}",
                        player.name,
                        skill_id,
                        e
                    );
                    return ItemUseResult::Failure(format!("无法学习技能: {:?}", e));
                }
                ItemUseResult::SkillLearned {
                    skill_id: *skill_id,
                }
            }

            ItemEffectType::UseSkill { skill_id, level } => {
                // 执行技能
                let player_arc = Arc::new(Player::clone(player));
                match skill_handler.use_skill(
                    player_arc,
                    *skill_id as u16,
                    *level,
                    0, // 物品触发的技能默认无目标
                    map_state,
                ) {
                    Ok(result) => {
                        tracing::info!(
                            "Player {} used skill {} from item: {:?}",
                            player.name,
                            skill_id,
                            result
                        );
                        ItemUseResult::SkillUsed {
                            skill_id: *skill_id,
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Player {} failed to use skill {}: {:?}",
                            player.name,
                            skill_id,
                            e
                        );
                        ItemUseResult::Failure(format!("无法使用技能: {:?}", e))
                    }
                }
            }

            ItemEffectType::Revive { hp_percent } => {
                // 复活
                if player.hp() == 0 {
                    let max_hp = player.max_hp();
                    let res_hp = max_hp * *hp_percent / 100;
                    player.combat_mut().hp = res_hp;
                    // 移除死亡状态
                    player.status.remove_status(StatusChange::Stone);
                    ItemUseResult::Revive {
                        hp_percent: *hp_percent,
                    }
                } else {
                    ItemUseResult::Failure("只有在死亡状态才能使用".to_string())
                }
            }

            ItemEffectType::SetSavePoint => {
                // 设置存档点
                player.set_save_point();
                ItemUseResult::SavePointSet
            }

            ItemEffectType::ApplyBuff {
                status,
                duration_secs,
                val1,
                val2,
                val3,
            } => {
                let effect = StatusEffect::with_values(
                    *status,
                    duration_secs * 1000,
                    StatusSource::Item(item_id),
                    *val1,
                    *val2,
                    *val3,
                );
                player.status.add_status(effect);
                ItemUseResult::Success(format!("获得 {} 状态 ({}秒)", status.name(), duration_secs))
            }

            ItemEffectType::Script { script } => {
                // 执行自定义脚本
                self.execute_script(
                    player,
                    inventory,
                    script,
                    item_id,
                    warp_service,
                    skill_handler,
                )
            }
        }
    }

    /// 应用治愈效果
    fn apply_heal(&self, player: &Player, hp: i32, sp: i32) {
        // 治愈HP
        if hp != 0 {
            let current = player.hp();
            let max = player.max_hp();
            let new_hp = if hp > 0 {
                (current + hp as u32).min(max)
            } else {
                current.saturating_sub((-hp) as u32)
            };
            player.combat_mut().hp = new_hp;
        }

        // 治愈SP
        if sp != 0 {
            let current = player.sp();
            let max = player.max_sp();
            let new_sp = if sp > 0 {
                (current + sp as u32).min(max)
            } else {
                current.saturating_sub((-sp) as u32)
            };
            player.combat_mut().sp = new_sp;
        }
    }

    /// 执行自定义脚本
    fn execute_script(
        &self,
        player: &Player,
        _inventory: &mut super::inventory::Inventory,
        script: &str,
        item_id: u16,
        _warp_service: &WarpService,
        _skill_handler: &SkillHandler,
    ) -> ItemUseResult {
        use super::script::ItemScript;

        let item_script = ItemScript::parse(script);
        let effects = item_script.execute();

        for effect in effects {
            let result = effect.apply(player);
            match result {
                super::effect::EffectResult::Success => continue,
                super::effect::EffectResult::Failed(msg) => {
                    return ItemUseResult::Failure(msg.to_string());
                }
                super::effect::EffectResult::PartialSuccess { msg } => {
                    return ItemUseResult::Failure(msg);
                }
            }
        }

        ItemUseResult::Success(format!(
            "使用了 {}",
            self.item_db.get(item_id).map(|i| i.name.as_str()).unwrap_or("物品")
        ))
    }

    /// 使用默认物品（没有特殊配置）
    fn use_default_item(
        &self,
        player: &Player,
        inventory: &mut super::inventory::Inventory,
        item_id: u16,
    ) -> ItemUseResult {
        let item = match self.item_db.get(item_id) {
            Some(i) => i,
            None => return ItemUseResult::ItemNotFound,
        };

        // 处理恢复类物品
        match item.type_ {
            super::data::ItemType::Heal => {
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
                inventory.use_item(0);
                ItemUseResult::Success(format!("使用了 {}", item.name))
            }
            _ => ItemUseResult::CannotUse,
        }
    }

    /// 在指定槽位使用默认物品
    fn use_default_item_at_slot(
        &self,
        player: &Player,
        inventory: &mut super::inventory::Inventory,
        slot_index: u8,
    ) -> ItemUseResult {
        let slot = match inventory.slots().get(slot_index as usize) {
            Some(s) if !s.is_empty() => s.clone(),
            _ => return ItemUseResult::ItemNotFound,
        };

        let item = match self.item_db.get(slot.item_id) {
            Some(i) => i,
            None => return ItemUseResult::ItemNotFound,
        };

        // 处理恢复类物品
        match item.type_ {
            super::data::ItemType::Heal => {
                if item.hp_restore > 0 {
                    let current = player.hp();
                    let max = player.max_hp();
                    let new_hp = (current + item.hp_restore as u32).min(max);
                    player.combat_mut().hp = new_hp;
                }
                if item.sp_restore > 0 {
                    let current = player.sp();
                    let max = player.max_sp();
                    let new_sp = (current + item.sp_restore as u32).min(max);
                    player.combat_mut().sp = new_sp;
                }
                inventory.use_item(slot_index);
                ItemUseResult::Success(format!("使用了 {}", item.name))
            }
            _ => ItemUseResult::CannotUse,
        }
    }

    /// 检查物品是否可使用
    pub fn can_use(&self, player: &Player, item_id: u16) -> bool {
        if let Some(config) = self.effect_db.get(item_id) {
            if config.requirements.check(player).is_some() {
                return false;
            }
            // 检查冷却
            if config.cooldown_ms > 0 {
                return !self.delay_tracker.is_on_cooldown(player.id, item_id);
            }
            true
        } else {
            // 检查是否是恢复类物品
            self.item_db
                .get(item_id)
                .map(|item| matches!(item.type_, super::data::ItemType::Heal))
                .unwrap_or(false)
        }
    }

    /// 获取物品效果描述
    pub fn get_effect_description(&self, item_id: u16) -> Option<String> {
        self.effect_db
            .get(item_id)
            .map(|config| config.effect_type.description())
    }

    /// 获取物品冷却时间（毫秒）
    pub fn get_cooldown(&self, item_id: u16) -> Option<u64> {
        self.effect_db
            .get(item_id)
            .map(|config| config.cooldown_ms)
            .filter(|&ms| ms > 0)
    }

    /// 检查物品是否在冷却中
    pub fn is_on_cooldown(&self, player_id: Uuid, item_id: u16) -> bool {
        self.delay_tracker.is_on_cooldown(player_id, item_id)
    }

    /// 获取物品剩余冷却时间
    pub fn get_remaining_cooldown(&self, player_id: Uuid, item_id: u16) -> u64 {
        self.delay_tracker.remaining_cooldown(player_id, item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integration_handler_creation() {
        let effect_db = Arc::new(ItemEffectDatabase::new());
        let item_db = Arc::new(ItemDatabase::new());

        let handler = ItemIntegrationHandler::new(effect_db, item_db);
        assert!(handler.effect_db.get(602).is_some()); // 蝴蝶翅膀
    }

    #[test]
    fn test_get_effect_description() {
        let effect_db = Arc::new(ItemEffectDatabase::new());
        let item_db = Arc::new(ItemDatabase::new());

        let handler = ItemIntegrationHandler::new(effect_db, item_db);

        // 测试传送物品
        let desc = handler.get_effect_description(601); // Fly Wing
        assert!(desc.is_some());
        assert!(desc.unwrap().contains("随机传送"));

        // 测试治愈物品
        let desc = handler.get_effect_description(604); // Red Potion
        assert!(desc.is_some());
        assert!(desc.unwrap().contains("HP"));
    }
}
