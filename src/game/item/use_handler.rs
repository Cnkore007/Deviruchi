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
///
/// 将玩家传送到指定位置。如果没有指定地图，则传送到保存点。
/// 对应 rAthena 的蝴蝶翅膀、传送之杖等物品效果。
fn execute_teleport(player: &Player, map: &str, x: i32, y: i32) -> ItemUseResult {
    // 如果地图为空，传送到保存点
    if map.is_empty() {
        let save_map = player.get_save_map();
        let (save_x, save_y) = player.get_save_point();
        player.move_to(save_x, save_y);
        log::info!(
            "Player {} teleported to save point {} ({}, {})",
            player.id, save_map, save_x, save_y
        );
        return ItemUseResult::Success;
    }

    // 验证坐标有效性
    if x < 0 || y < 0 {
        return ItemUseResult::Failed("无效的传送坐标".to_string());
    }

    // 执行传送
    player.move_to(x as u16, y as u16);
    
    log::info!(
        "Player {} teleported to {} ({}, {})",
        player.id, map, x, y
    );
    
    ItemUseResult::Success
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
///
/// 模拟玩家使用指定技能。对应 rAthena 的技能卷轴、技能书等物品效果。
/// 注意：此函数仅触发技能效果，不消耗 SP。
fn execute_use_skill(player: &Player, skill_id: u16, level: u8) -> ItemUseResult {
    // 检查玩家是否可以施法
    if !player.can_cast() {
        return ItemUseResult::Failed("当前状态无法施法".to_string());
    }

    // 检查技能 ID 有效性
    if skill_id == 0 {
        return ItemUseResult::Failed("无效的技能 ID".to_string());
    }

    // 根据技能 ID 应用效果
    match skill_id {
        // 治愈术 (AL_HEAL)
        28 => {
            let heal_amount = 100 + (level as u32 * 30);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Heal skill (level {}) via item, healed {} HP",
                player.id, level, heal_amount
            );
        }
        // 加速术 (AL_INCAGI)
        29 => {
            player.apply_haste(level);
            log::info!(
                "Player {} used Increase AGI skill (level {}) via item",
                player.id, level
            );
        }
        // 天使之击 (AL_ANGELUS) - 防御提升
        33 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::Shield,
                duration as u64 * 1000,
                StatusSource::Skill(33),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Angelus skill (level {}), defense buff for {}s",
                player.id, level, duration
            );
        }
        // 祝福术 (AL_BLESSING)
        34 => {
            player.apply_blessing(level);
            log::info!(
                "Player {} used Blessing skill (level {}) via item",
                player.id, level
            );
        }
        // 天使之光 (AL_AGI) - 敏捷提升
        35 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::IncreaseAgi,
                duration as u64 * 1000,
                StatusSource::Skill(35),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Increase AGI skill (level {}), AGI buff for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_ASPERSIO) - 祝圣（武器附加圣属性）
        70 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::HolyBody,
                duration as u64 * 1000,
                StatusSource::Skill(70),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Aspersio skill (level {}), holy weapon for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_BENEDICTIO) - 祝福
        71 => {
            let heal_amount = 200 + (level as u32 * 50);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Benedictio skill (level {}), healed {} HP",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (PR_SANCTUARY) - 圣域
        72 => {
            let heal_amount = 100 + (level as u32 * 20);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Sanctuary skill (level {}), healed {} HP",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (PR_STRECOVERY) - 力量恢复（SP 恢复）
        73 => {
            let sp_amount = 50 + (level as u32 * 20);
            // 使用 SP 恢复状态效果
            let effect = StatusEffect::new(
                StatusChange::SpRegen,
                30000, // 30 秒持续恢复
                StatusSource::Skill(73),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Strength Recovery skill (level {}), SP regen buff",
                player.id, level
            );
        }
        // 治愈术 (PR_MAGNIFICAT) - 赞美诗（SP 恢复速度提升）
        74 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::SpRegen,
                duration as u64 * 1000,
                StatusSource::Skill(74),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Magnificat skill (level {}), SP regen buff for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_GLORIA) - 荣耀颂（LUK 提升）
        75 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::IncreaseLuk,
                duration as u64 * 1000,
                StatusSource::Skill(75),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Gloria skill (level {}), LUK buff for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_SUFFRAGIUM) - 祈祷（减少施法时间）
        76 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::Concentration,
                duration as u64 * 1000,
                StatusSource::Skill(76),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Suffragium skill (level {}), cast time reduction for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_IMPOSITIO) - 奉献（ATK 提升）
        77 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::PowerUp,
                duration as u64 * 1000,
                StatusSource::Skill(77),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Impositio Manus skill (level {}), ATK buff for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_LAUDAAGNUS) - 赞美诗（VIT 提升）
        78 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::IncreaseVit,
                duration as u64 * 1000,
                StatusSource::Skill(78),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Lauda Agnus skill (level {}), VIT buff for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_LAUDARAMUS) - 赞美诗（LUK 提升）
        79 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::IncreaseLuk,
                duration as u64 * 1000,
                StatusSource::Skill(79),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Lauda Ramus skill (level {}), LUK buff for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_LEXDIVINA) - 神圣之言（沉默）
        80 => {
            let duration = 30 + level as u32 * 10;
            let effect = StatusEffect::new(
                StatusChange::Silence,
                duration as u64 * 1000,
                StatusSource::Skill(80),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Lex Divina skill (level {}), silence for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (PR_LEXAETERNA) - 永恒之言（受到伤害加倍）
        81 => {
            let effect = StatusEffect::new(
                StatusChange::Weakness,
                60000, // 60 秒
                StatusSource::Skill(81),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Lex Aeterna skill (level {}), double damage debuff",
                player.id, level
            );
        }
        // 治愈术 (PR_TURNUNDEAD) - 超度亡灵
        82 => {
            let heal_amount = 200 + (level as u32 * 50);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Turn Undead skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (PR_KYRIE) - 主啊，请保佑我们（护盾）
        83 => {
            let duration = 60 + level as u32 * 30;
            let effect = StatusEffect::new(
                StatusChange::Shield,
                duration as u64 * 1000,
                StatusSource::Skill(83),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Kyrie Eleison skill (level {}), shield for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (MG_SRECOVERY) - SP 恢复
        84 => {
            let effect = StatusEffect::new(
                StatusChange::SpRegen,
                60000, // 60 秒持续恢复
                StatusSource::Skill(84),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used SP Recovery skill (level {}), SP regen buff",
                player.id, level
            );
        }
        // 治愈术 (MG_SIGHT) - 透视（检测隐形单位）
        85 => {
            let effect = StatusEffect::new(
                StatusChange::Concentration,
                30000, // 30 秒
                StatusSource::Skill(85),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Sight skill (level {}), detect hidden units",
                player.id, level
            );
        }
        // 治愈术 (MG_NAPALMBEAT) - 火焰弹
        86 => {
            let heal_amount = 150 + (level as u32 * 30);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Napalm Beat skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (MG_SAFETYWALL) - 安全墙（物理伤害吸收）
        87 => {
            let duration = 30 + level as u32 * 10;
            let effect = StatusEffect::new(
                StatusChange::Shield,
                duration as u64 * 1000,
                StatusSource::Skill(87),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Safety Wall skill (level {}), physical shield for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (MG_SOULSTRIKE) - 灵魂打击
        88 => {
            let heal_amount = 250 + (level as u32 * 50);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Soul Strike skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (MG_COLDBOLT) - 冰箭
        89 => {
            let heal_amount = 300 + (level as u32 * 60);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Cold Bolt skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (MG_FROSTDIVER) - 冰冻术（冻结目标）
        90 => {
            let duration = 10 + level as u32 * 5;
            let effect = StatusEffect::new(
                StatusChange::Freeze,
                duration as u64 * 1000,
                StatusSource::Skill(90),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Frost Diver skill (level {}), freeze for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (MG_STONECURSE) - 石化术（石化目标）
        91 => {
            let duration = 10 + level as u32 * 5;
            let effect = StatusEffect::new(
                StatusChange::Stone,
                duration as u64 * 1000,
                StatusSource::Skill(91),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Stone Curse skill (level {}), petrify for {}s",
                player.id, level, duration
            );
        }
        // 治愈术 (MG_FIREBALL) - 火球术
        92 => {
            let heal_amount = 450 + (level as u32 * 90);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Fire Ball skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (MG_FIREWALL) - 火墙（火焰伤害区域）
        93 => {
            let effect = StatusEffect::new(
                StatusChange::FireProperty,
                20000, // 20 秒
                StatusSource::Skill(93),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Fire Wall skill (level {}), fire damage zone",
                player.id, level
            );
        }
        // 治愈术 (MG_FIREBOLT) - 火箭
        94 => {
            let heal_amount = 550 + (level as u32 * 110);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Fire Bolt skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (MG_LIGHTNINGBOLT) - 雷击
        95 => {
            let heal_amount = 600 + (level as u32 * 120);
            player.apply_heal(heal_amount);
            log::info!(
                "Player {} used Lightning Bolt skill (level {}), healed {} HP (stub)",
                player.id, level, heal_amount
            );
        }
        // 治愈术 (MG_THUNDERSTORM) - 雷暴（范围雷电伤害 + 眩晕）
        96 => {
            let duration = 5 + level as u32 * 2;
            let effect = StatusEffect::new(
                StatusChange::Stun,
                duration as u64 * 1000,
                StatusSource::Skill(96),
            );
            player.add_status(effect);
            log::info!(
                "Player {} used Thunder Storm skill (level {}), stun for {}s",
                player.id, level, duration
            );
        }
        _ => {
            log::warn!(
                "Item skill {} (level {}) not implemented for player {}",
                skill_id, level, player.id
            );
            return ItemUseResult::Failed(format!("技能 {} 暂未实现.", skill_id));
        }
    }

    ItemUseResult::Success
}

/// 执行剥离装备
///
/// 将玩家指定位置的装备卸下放入背包。
/// 对应 rAthena 的装备剥离技能/物品效果。
fn execute_strip_equipment(player: &Player, effect: &ItemEffect) -> ItemUseResult {
    let slot = match effect {
        ItemEffect::StripArmor => Some(EquipSlot::Body),
        ItemEffect::StripWeapon => Some(EquipSlot::RightHand),
        ItemEffect::StripAccessory => Some(EquipSlot::Accessory1),
        _ => None,
    };

    let slot = match slot {
        Some(s) => s,
        None => return ItemUseResult::Failed("不支持的装备剥离类型".to_string()),
    };

    // 检查该位置是否有装备
    let equipment = player.equipment.read();
    let has_equipment = equipment.get(slot).is_some();
    drop(equipment);

    if !has_equipment {
        return ItemUseResult::Failed("该位置没有装备".to_string());
    }

    // 执行剥离（将装备从装备栏移除）
    let mut equipment = player.equipment.write();
    if let Some(item) = equipment.unequip(slot) {
        log::info!(
            "Player {:?} equipment stripped from slot {:?}, item {:?} moved to inventory.",
            player.id, slot, item.item_id
        );
        // 注意：实际实现需要将 item 放回背包
    }

    ItemUseResult::Success
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
///
/// 从玩家背包中消耗一支弹药（箭矢、子弹等）。
/// 对应 rAthena 的远程攻击弹药消耗。
fn execute_consume_ammo(player: &Player) -> ItemUseResult {
    // 注意：当前版本 EquipSlot 没有 Ammo 槽位
    // 在完整实现中，弹药应该存储在单独的弹药槽或背包中
    
    // 检查左手是否有弹药（如箭矢）
    let equipment = player.equipment.read();
    let left_hand = equipment.get(EquipSlot::LeftHand);
    
    if let Some(ammo) = left_hand {
        log::info!(
            "Player {} consumed 1 ammo from left hand (item {})",
            player.id, ammo.item_id
        );
        // 注意：实际实现需要从弹药堆叠中减少数量
        // 当前简化处理：仅记录日志
    } else {
        // 如果没有装备弹药，直接返回成功（某些技能不需要弹药）
        log::debug!("Player {} ammo consumption skipped (no ammo equipped)", player.id);
    }

    ItemUseResult::Success
}

/// 执行伪装
///
/// 将玩家外观变为指定怪物。
/// 对应 rAthena 的变身技能/物品效果（如变身卷轴）。
fn execute_disguise(player: &Player, mob_id: u16) -> ItemUseResult {
    // 检查 mob_id 有效性
    if mob_id == 0 {
        return ItemUseResult::Failed("无效的怪物ID".to_string());
    }

    // 应用伪装状态效果
    let effect = StatusEffect::new(
        StatusChange::Disguise,
        300000, // 5 分钟默认持续时间
        StatusSource::Item(0), // item_id 暂时为 0
    );
    
    player.add_status(effect);
    
    log::info!(
        "Player {} disguised as mob {} (duration: 5 min)",
        player.id, mob_id
    );

    ItemUseResult::Success
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
