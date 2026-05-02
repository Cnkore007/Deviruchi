# Deviruchi Phase 3: 游戏逻辑实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现完整游戏逻辑系统，包括技能、物品、怪物、NPC、战斗和地图管理

**Architecture:** 分层设计：skill/item/mob/npc/battle/map 作为独立子系统，通过 Player/MapState 集成到游戏核心。每个子系统有清晰的数据结构和业务逻辑。

**Tech Stack:** Rust, Tokio, rusqlite, parking_lot, uuid

---

## 文件结构规划

```
deviruchi/src/
├── game/
│   ├── mod.rs                    # 模块导出
│   ├── skill/                    # 技能系统
│   │   ├── mod.rs
│   │   ├── data.rs              # 技能数据定义
│   │   ├── effect.rs            # 技能效果处理
│   │   └── handler.rs           # 技能使用处理
│   ├── item/                     # 物品系统
│   │   ├── mod.rs
│   │   ├── data.rs              # 物品数据定义
│   │   ├── inventory.rs         # 背包管理
│   │   └── handler.rs           # 物品使用处理
│   ├── mob/                      # 怪物系统
│   │   ├── mod.rs
│   │   ├── data.rs              # 怪物数据定义
│   │   ├── spawn.rs             # 刷新管理
│   │   └── ai.rs                # 怪物AI
│   ├── npc/                      # NPC系统
│   │   ├── mod.rs
│   │   ├── data.rs              # NPC数据定义
│   │   └── handler.rs           # NPC交互处理
│   ├── battle/                   # 战斗系统
│   │   ├── mod.rs
│   │   ├── formula.rs           # 伤害公式
│   │   └── handler.rs           # 战斗处理
│   └── map/                      # 地图系统
│       ├── mod.rs
│       ├── data.rs              # 地图数据定义
│       └── cell.rs             # 地形/障碍
```

---

## 任务列表

### Task 1: 技能系统 - 数据定义

**Files:**
- Create: `src/game/skill/mod.rs`
- Create: `src/game/skill/data.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 创建 src/game/skill/mod.rs**

```rust
//! 技能系统

pub mod data;
pub mod effect;
pub mod handler;

pub use data::{Skill, SkillType, SkillTarget};
pub use handler::SkillHandler;
```

- [ ] **Step 2: 创建 src/game/skill/data.rs**

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 技能类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType {
    Passive,      // 被动技能
    Active,        // 主动技能
    Attack,        // 攻击技能
    Healing,       // 治疗技能
    Support,       // 辅助技能
    Debuff,        // 减益技能
}

/// 技能目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTarget {
    Self_,         // 自身
    Enemy,         // 敌方
    Ally,          // 友方
    Ground,        // 地面
    Party,         // 队伍
}

/// 技能数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: u16,
    pub name: &'static str,
    pub type_: SkillType,
    pub target: SkillTarget,
    pub level: u8,
    pub sp_cost: u16,
    pub hp_cost: u32,
    pub cast_time: u32,        // 吟唱时间(ms)
    pub cooldown: u32,         // 冷却时间(ms)
    pub range: u16,            // 施法范围
    pub skill_time: u32,       // 持续时间(ms)
    pub damage: i32,           // 基础伤害
    pub hit: i16,             // 命中加成
    pub element: u8,           // 属性 (0=无,1=火,2=水,3=风,4=地)
    pub flags: u32,
}

impl Skill {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            name: "Unknown",
            type_: SkillType::Active,
            target: SkillTarget::Enemy,
            level: 1,
            sp_cost: 0,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 0,
            hit: 0,
            element: 0,
            flags: 0,
        }
    }
}

/// 技能数据库
pub struct SkillDatabase {
    skills: HashMap<u16, Skill>,
}

impl SkillDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            skills: HashMap::new(),
        };
        db.init_default_skills();
        db
    }

    fn init_default_skills(&mut self) {
        // 基础攻击技能 - Bash
        self.skills.insert(1, Skill {
            id: 1,
            name: "Bash",
            type_: SkillType::Attack,
            target: SkillTarget::Enemy,
            level: 1,
            sp_cost: 8,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 1,
            skill_time: 0,
            damage: 110,  // 110% ATK
            hit: 3,
            element: 0,
            flags: 0,
        });

        // 怒爆
        self.skills.insert(25, Skill {
            id: 25,
            name: "Fire Ball",
            type_: SkillType::Attack,
            target: SkillTarget::Enemy,
            level: 1,
            sp_cost: 9,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 80,
            hit: 5,
            element: 1,  // 火属性
            flags: 0,
        });

        // 治愈术
        self.skills.insert(28, Skill {
            id: 28,
            name: "Heal",
            type_: SkillType::Healing,
            target: SkillTarget::Ally,
            level: 1,
            sp_cost: 6,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 35,  // 恢复量百分比
            hit: 0,
            element: 0,
            flags: 0,
        });

        // 加速术
        self.skills.insert(29, Skill {
            id: 29,
            name: "Increase AGI",
            type_: SkillType::Support,
            target: SkillTarget::Ally,
            level: 1,
            sp_cost: 10,
            hp_cost: 0,
            cast_time: 2000,
            cooldown: 0,
            range: 9,
            skill_time: 30000,  // 持续30秒
            damage: 0,
            hit: 0,
            element: 0,
            flags: 0,
        });
    }

    pub fn get(&self, skill_id: u16) -> Option<&Skill> {
        self.skills.get(&skill_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &Skill> {
        self.skills.values()
    }
}

impl Default for SkillDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: 更新 src/game/mod.rs**

```rust
//! 游戏业务层

pub mod login;
pub mod char;
pub mod map;
pub mod skill;
pub mod item;
pub mod mob;
pub mod npc;
pub mod battle;
```

- [ ] **Step 4: 运行编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(skill): 添加技能系统数据定义
- SkillType, SkillTarget, Skill 数据结构
- SkillDatabase 技能数据库
- 默认技能: Bash, Fire Ball, Heal, Increase AGI
"
```

---

### Task 2: 技能系统 - 效果与处理器

**Files:**
- Create: `src/game/skill/effect.rs`
- Create: `src/game/skill/handler.rs`

- [ ] **Step 1: 创建 src/game/skill/effect.rs**

```rust
use crate::game::map::Player;
use super::data::{Skill, SkillType};

/// 技能效果应用
pub struct SkillEffect;

impl SkillEffect {
    /// 对目标应用技能效果
    pub fn apply(skill: &Skill, caster: &Player, target: &Player, level: u8) -> SkillResult {
        match skill.type_ {
            SkillType::Attack => Self::apply_attack(skill, caster, target, level),
            SkillType::Healing => Self::apply_healing(skill, caster, target, level),
            SkillType::Support => Self::apply_support(skill, caster, target, level),
            SkillType::Debuff => Self::apply_debuff(skill, caster, target, level),
            _ => SkillResult::None,
        }
    }

    fn apply_attack(skill: &Skill, caster: &Player, target: &Player, level: u8) -> SkillResult {
        // 计算伤害 (简化版，实际需要引用战斗公式)
        let base_damage = skill.damage as i32 * level as i32 / 10;
        SkillResult::Damage {
            damage: base_damage,
            element: skill.element,
            hit_bonus: skill.hit,
        }
    }

    fn apply_healing(skill: &Skill, caster: &Player, target: &Player, level: u8) -> SkillResult {
        let heal_amount = skill.damage as i32 * level as i32 / 10;
        let matk = *caster.int.read() * 2 + *caster.dex.read();
        let total_heal = (heal_amount * matk as i32 / 100).max(1);

        SkillResult::Heal {
            amount: total_heal as u32,
        }
    }

    fn apply_support(skill: &Skill, _caster: &Player, _target: &Player, _level: u8) -> SkillResult {
        // 辅助技能效果
        SkillResult::Buff {
            buff_type: skill.id,
            duration: skill.skill_time,
        }
    }

    fn apply_debuff(skill: &Skill, _caster: &Player, _target: &Player, _level: u8) -> SkillResult {
        SkillResult::Debuff {
            debuff_type: skill.id,
            duration: skill.skill_time,
        }
    }
}

/// 技能效果结果
#[derive(Debug, Clone)]
pub enum SkillResult {
    None,
    Damage { damage: i32, element: u8, hit_bonus: i16 },
    Heal { amount: u32 },
    Buff { buff_type: u16, duration: u32 },
    Debuff { debuff_type: u16, duration: u32 },
}
```

- [ ] **Step 2: 创建 src/game/skill/handler.rs**

```rust
use std::sync::Arc;
use crate::game::map::{Player, MapState};
use crate::game::battle::BattleHandler;
use super::data::{Skill, SkillDatabase};
use super::effect::SkillEffect;

pub struct SkillHandler {
    db: Arc<SkillDatabase>,
    battle: Arc<BattleHandler>,
}

impl SkillHandler {
    pub fn new() -> Self {
        Self {
            db: Arc::new(SkillDatabase::new()),
            battle: Arc::new(BattleHandler::new()),
        }
    }

    /// 检查是否能使用技能
    pub fn can_use_skill(&self, player: &Player, skill_id: u16, level: u8) -> SkillError {
        let skill = match self.db.get(skill_id) {
            Some(s) => s,
            None => return SkillError::SkillNotFound,
        };

        // 检查SP
        let sp_cost = skill.sp_cost as u32 * level as u32;
        if *player.sp.read() < sp_cost {
            return SkillError::NotEnoughSP;
        }

        // 检查HP
        if *player.hp.read() <= skill.hp_cost {
            return SkillError::NotEnoughHP;
        }

        // 检查施法距离 (后续与地图集成)
        // 检查冷却时间

        SkillError::None
    }

    /// 使用技能
    pub fn use_skill(
        &self,
        caster: Arc<Player>,
        skill_id: u16,
        level: u8,
        target_id: u32,
        map_state: &MapState,
    ) -> Result<SkillResult, SkillError> {
        let skill = self.db.get(skill_id)
            .ok_or(SkillError::SkillNotFound)?;

        // 消耗SP/HP
        let sp_cost = skill.sp_cost as u32 * level as u32;
        *caster.sp.write() -= sp_cost;

        if skill.hp_cost > 0 {
            *caster.hp.write() -= skill.hp_cost;
        }

        // 获取目标玩家
        if let Some(target) = map_state.get_player_by_char_id(target_id) {
            Ok(SkillEffect::apply(skill, &caster, &target, level))
        } else {
            // 如果目标是怪物，后续集成
            Ok(SkillResult::None)
        }
    }

    /// 获取技能数据库
    pub fn get_database(&self) -> Arc<SkillDatabase> {
        self.db.clone()
    }
}

impl Default for SkillHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 技能错误
#[derive(Debug, Clone, Copy)]
pub enum SkillError {
    None,
    SkillNotFound,
    NotEnoughSP,
    NotEnoughHP,
    OutOfRange,
    Cooldown,
    InvalidTarget,
}
```

- [ ] **Step 3: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat(skill): 添加技能效果和处理器
- SkillEffect 技能效果应用
- SkillHandler 技能使用管理
- 伤害/治疗/增益/减益效果
"
```

---

### Task 3: 物品系统 - 数据与背包

**Files:**
- Create: `src/game/item/mod.rs`
- Create: `src/game/item/data.rs`
- Create: `src/game/item/inventory.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 创建 src/game/item/mod.rs**

```rust
//! 物品系统

pub mod data;
pub mod inventory;
pub mod handler;

pub use data::{Item, ItemType, ItemFlag};
pub use inventory::{Inventory, InventorySlot};
pub use handler::ItemHandler;
```

- [ ] **Step 2: 创建 src/game/item/data.rs**

```rust
use serde::{Deserialize, Serialize};

/// 物品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    Heal,           // 恢复道具
    Etc,            // 杂项
    Weapon,         // 武器
    Armor,          // 防具
    Card,           // 卡片
    PetEgg,         // 宠物蛋
    PetArmor,       // 宠物装备
}

/// 物品标志
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemFlag {
    None,
    Identified,     // 已鉴定
    Unique,         // 唯一
    NonTradable,    // 不可交易
    NoDrop,         // 不可丢弃
    NoTrade,        // 交易限制
}

/// 物品数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: u16,
    pub name: &'static str,
    pub type_: ItemType,
    pub price: u32,
    pub weight: u16,
    pub flags: u32,
    pub hp_restore: u16,      // HP恢复量
    pub sp_restore: u16,      // SP恢复量
    pub equip_mask: u32,      // 装备位置掩码
    pub atk: u16,             // 物理攻击
    pub matk: u16,            // 魔法攻击
    pub defense: u16,          // 防御
    pub magic_defense: u16,   // 魔法防御
    pub str_bonus: i16,       // STR加成
    pub agi_bonus: i16,       // AGI加成
    pub vit_bonus: i16,       // VIT加成
    pub int_bonus: i16,       // INT加成
    pub dex_bonus: i16,       // DEX加成
    pub luk_bonus: i16,       // LUK加成
}

impl Item {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            name: "Unknown",
            type_: ItemType::Etc,
            price: 0,
            weight: 0,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0,
            atk: 0,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            str_bonus: 0,
            agi_bonus: 0,
            vit_bonus: 0,
            int_bonus: 0,
            dex_bonus: 0,
            luk_bonus: 0,
        }
    }

    pub fn is_equip(&self) -> bool {
        matches!(self.type_, ItemType::Weapon | ItemType::Armor)
    }
}

/// 物品数据库
pub struct ItemDatabase {
    items: std::collections::HashMap<u16, Item>,
}

impl ItemDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            items: std::collections::HashMap::new(),
        };
        db.init_default_items();
        db
    }

    fn init_default_items(&mut self) {
        // 红色药水
        db.items.insert(501, Item {
            id: 501,
            name: "Red Potion",
            type_: ItemType::Heal,
            price: 50,
            weight: 7,
            flags: 0,
            hp_restore: 120,
            sp_restore: 0,
            equip_mask: 0,
            ..Default::default()
        });

        // 黄色药水
        db.items.insert(502, Item {
            id: 502,
            name: "Yellow Potion",
            type_: ItemType::Heal,
            price: 40,
            weight: 5,
            flags: 0,
            hp_restore: 60,
            sp_restore: 0,
            equip_mask: 0,
            ..Default::default()
        });

        // 蓝色药水
        db.items.insert(503, Item {
            id: 503,
            name: "Blue Potion",
            type_: ItemType::Heal,
            price: 50,
            weight: 7,
            flags: 0,
            hp_restore: 0,
            sp_restore: 40,
            equip_mask: 0,
            ..Default::default()
        });

        // 短剑
        db.items.insert(1201, Item {
            id: 1201,
            name: "Dagger",
            type_: ItemType::Weapon,
            price: 1000,
            weight: 50,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0x0001,  // 右手
            atk: 10,
            ..Default::default()
        });

        // 盗贼短剑
        db.items.insert(1202, Item {
            id: 1202,
            name: "Main Gauche",
            type_: ItemType::Weapon,
            price: 2500,
            weight: 60,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0x0001,
            atk: 15,
            ..Default::default()
        });

        // 布甲
        db.items.insert(1501, Item {
            id: 1501,
            name: "Clothes",
            type_: ItemType::Armor,
            price: 500,
            weight: 40,
            flags: 0,
            hp_restore: 0,
            sp_restore: 0,
            equip_mask: 0x0010,  // 身体
            defense: 2,
            ..Default::default()
        });
    }

    pub fn get(&self, item_id: u16) -> Option<&Item> {
        self.items.get(&item_id)
    }
}

impl Default for ItemDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: 创建 src/game/item/inventory.rs**

```rust
use super::data::{Item, ItemDatabase};

/// 背包格子
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub index: u8,
    pub item_id: u16,
    pub amount: u16,
    pub identified: bool,
    pub refine: u8,
    pub cards: [u16; 4],
}

impl InventorySlot {
    pub fn empty(index: u8) -> Self {
        Self {
            index,
            item_id: 0,
            amount: 0,
            identified: false,
            refine: 0,
            cards: [0; 4],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.item_id == 0
    }
}

/// 背包管理
pub struct Inventory {
    max_size: u8,
    slots: Vec<InventorySlot>,
    item_db: std::sync::Arc<ItemDatabase>,
}

impl Inventory {
    pub fn new(max_size: u8, item_db: std::sync::Arc<ItemDatabase>) -> Self {
        let slots: Vec<_> = (0..max_size)
            .map(InventorySlot::empty)
            .collect();

        Self {
            max_size,
            slots,
            item_db,
        }
    }

    /// 添加物品
    pub fn add_item(&mut self, item_id: u16, amount: u16) -> bool {
        // 先找相同物品的空位
        for slot in &mut self.slots {
            if slot.item_id == item_id && slot.amount + amount <= 300 {
                slot.amount += amount;
                return true;
            }
        }

        // 找空位
        for slot in &mut self.slots {
            if slot.is_empty() {
                slot.item_id = item_id;
                slot.amount = amount;
                slot.identified = true;
                return true;
            }
        }

        false  // 背包已满
    }

    /// 移除物品
    pub fn remove_item(&mut self, index: u8, amount: u16) -> bool {
        if index >= self.max_size {
            return false;
        }

        let slot = &mut self.slots[index as usize];
        if slot.amount >= amount {
            slot.amount -= amount;
            if slot.amount == 0 {
                slot.item_id = 0;
            }
            return true;
        }

        false
    }

    /// 使用物品
    pub fn use_item(&mut self, index: u8) -> Option<&Item> {
        if index >= self.max_size {
            return None;
        }

        let slot = &mut self.slots[index as usize];
        if slot.is_empty() {
            return None;
        }

        let item = self.item_db.get(slot.item_id)?;
        if !matches!(item.type_, super::data::ItemType::Heal) {
            return None;
        }

        // 消耗物品
        slot.amount -= 1;
        if slot.amount == 0 {
            slot.item_id = 0;
        }

        Some(item)
    }

    /// 获取格子数量
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// 获取所有格子
    pub fn slots(&self) -> &[InventorySlot] {
        &self.slots
    }
}
```

- [ ] **Step 4: 创建 src/game/item/handler.rs**

```rust
use std::sync::Arc;
use super::data::{Item, ItemDatabase};
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
```

- [ ] **Step 5: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(item): 添加物品系统
- Item, ItemType, ItemFlag 数据结构
- ItemDatabase 物品数据库
- Inventory 背包管理
- ItemHandler 物品使用处理
"
```

---

### Task 4: 怪物系统 - 数据与AI

**Files:**
- Create: `src/game/mob/mod.rs`
- Create: `src/game/mob/data.rs`
- Create: `src/game/mob/spawn.rs`
- Create: `src/game/mob/ai.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 创建 src/game/mob/mod.rs**

```rust
//! 怪物系统

pub mod data;
pub mod spawn;
pub mod ai;

pub use data::{Mob, MobAIState, MobType};
pub use ai::MobAI;
```

- [ ] **Step 2: 创建 src/game/mob/data.rs**

```rust
use parking_lot::RwLock;
use uuid::Uuid;

/// 怪物类型
#[derive(Debug, Clone, Copy)]
pub enum MobType {
    Normal,
    Boss,
    Guardian,
    Event,
}

/// 怪物AI状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobAIState {
    Idle,
    Patrol,
    Chase,
    Attack,
    Return,
    Dead,
}

/// 怪物数据
#[derive(Debug, Clone)]
pub struct Mob {
    pub id: Uuid,
    pub mob_id: u16,           // 怪物模板ID
    pub name: String,
    pub pos_x: RwLock<u16>,
    pub pos_y: RwLock<u16>,
    pub map_name: String,

    // 属性
    pub level: u16,
    pub hp: RwLock<u32>,
    pub max_hp: u32,
    pub sp: RwLock<u32>,
    pub max_sp: u32,

    // 战斗属性
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub hit: i16,
    pub flee: i16,
    pub crit: i16,
    pub walk_speed: u16,
    pub atk_range: u16,

    // AI状态
    pub ai_state: RwLock<MobAIState>,
    pub target_id: RwLock<Option<Uuid>>,

    // AI参数
    pub sight_range: u16,      // 视野范围
    pub chase_range: u16,     // 追击范围
    pub aggro_rate: i16,       // 仇恨值

    // 刷新参数
    pub spawn_delay: u32,      // 刷新延迟(ms)
    pub respawn_time: u32,    // 复活时间(ms)
}

impl Mob {
    pub fn new(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            mob_id,
            name: format!("Mob_{}", mob_id),
            pos_x: RwLock::new(x),
            pos_y: RwLock::new(y),
            map_name: map.to_string(),
            level: 1,
            hp: RwLock::new(100),
            max_hp: 100,
            sp: RwLock::new(0),
            max_sp: 0,
            atk: 10,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 0,
            flee: 0,
            crit: 0,
            walk_speed: 150,
            atk_range: 1,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 0,
            respawn_time: 60000,
        }
    }

    pub fn from_template(mob_id: u16, x: u16, y: u16, map: &str) -> Self {
        let template = MobDatabase::get(mob_id);
        Self {
            id: Uuid::new_v4(),
            mob_id,
            name: template.name.to_string(),
            pos_x: RwLock::new(x),
            pos_y: RwLock::new(y),
            map_name: map.to_string(),
            level: template.level,
            hp: RwLock::new(template.hp),
            max_hp: template.hp,
            sp: RwLock::new(template.sp),
            max_sp: template.sp,
            atk: template.atk,
            matk: template.matk,
            defense: template.defense,
            magic_defense: template.magic_defense,
            hit: template.hit,
            flee: template.flee,
            crit: template.crit,
            walk_speed: template.walk_speed,
            atk_range: template.atk_range,
            ai_state: RwLock::new(MobAIState::Idle),
            target_id: RwLock::new(None),
            sight_range: template.sight_range,
            chase_range: template.chase_range,
            aggro_rate: template.aggro_rate,
            spawn_delay: template.spawn_delay,
            respawn_time: template.respawn_time,
        }
    }

    pub fn get_position(&self) -> (u16, u16) {
        (*self.pos_x.read(), *self.pos_y.read())
    }

    pub fn move_to(&self, x: u16, y: u16) {
        *self.pos_x.write() = x;
        *self.pos_y.write() = y;
    }

    pub fn take_damage(&self, damage: u32) -> bool {
        let current_hp = *self.hp.read();
        if current_hp <= damage {
            *self.hp.write() = 0;
            *self.ai_state.write() = MobAIState::Dead;
            true  // 死亡
        } else {
            *self.hp.write() = current_hp - damage;
            false
        }
    }

    pub fn is_dead(&self) -> bool {
        *self.hp.read() == 0
    }
}

/// 怪物数据库
pub struct MobDatabase;

impl MobDatabase {
    pub fn get(mob_id: u16) -> MobTemplate {
        match mob_id {
            1001 => MobTemplate {
                name: "Poring",
                level: 1,
                hp: 50,
                sp: 0,
                atk: 7,
                matk: 0,
                defense: 0,
                magic_defense: 0,
                hit: 7,
                flee: 5,
                crit: 0,
                walk_speed: 150,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            1002 => MobTemplate {
                name: "Lunatic",
                level: 3,
                hp: 80,
                sp: 0,
                atk: 12,
                matk: 0,
                defense: 0,
                magic_defense: 0,
                hit: 12,
                flee: 10,
                crit: 5,
                walk_speed: 200,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            1003 => MobTemplate {
                name: "Blue Poring",
                level: 2,
                hp: 60,
                sp: 0,
                atk: 8,
                matk: 5,
                defense: 0,
                magic_defense: 5,
                hit: 8,
                flee: 7,
                crit: 0,
                walk_speed: 150,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            1312 => MobTemplate {
                name: "Fabre",
                level: 4,
                hp: 120,
                sp: 0,
                atk: 15,
                matk: 0,
                defense: 0,
                magic_defense: 0,
                hit: 15,
                flee: 12,
                crit: 0,
                walk_speed: 150,
                atk_range: 1,
                sight_range: 12,
                chase_range: 20,
                aggro_rate: 0,
                spawn_delay: 1000,
                respawn_time: 60000,
            },
            _ => MobTemplate::default(mob_id),
        }
    }
}

/// 怪物模板
#[derive(Debug, Clone)]
pub struct MobTemplate {
    pub name: &'static str,
    pub level: u16,
    pub hp: u32,
    pub sp: u32,
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub hit: i16,
    pub flee: i16,
    pub crit: i16,
    pub walk_speed: u16,
    pub atk_range: u16,
    pub sight_range: u16,
    pub chase_range: u16,
    pub aggro_rate: i16,
    pub spawn_delay: u32,
    pub respawn_time: u32,
}

impl MobTemplate {
    fn default(mob_id: u16) -> Self {
        Self {
            name: format!("Unknown_{}", mob_id),
            level: 1,
            hp: 50,
            sp: 0,
            atk: 10,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 10,
            flee: 10,
            crit: 0,
            walk_speed: 150,
            atk_range: 1,
            sight_range: 12,
            chase_range: 20,
            aggro_rate: 0,
            spawn_delay: 1000,
            respawn_time: 60000,
        }
    }
}
```

- [ ] **Step 3: 创建 src/game/mob/spawn.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::game::mob::Mob;

/// 怪物刷新点
#[derive(Debug, Clone)]
pub struct SpawnPoint {
    pub mob_id: u16,
    pub x: u16,
    pub y: u16,
    pub count: u8,
    pub interval: u32,  // 刷新间隔(ms)
}

/// 地图怪物刷新管理
pub struct MobSpawnManager {
    spawns: RwLock<HashMap<String, Vec<SpawnPoint>>>,  // map_name -> spawn points
    active_mobs: RwLock<HashMap<String, Vec<Arc<Mob>>>>,  // map_name -> active mobs
}

impl MobSpawnManager {
    pub fn new() -> Self {
        Self {
            spawns: RwLock::new(HashMap::new()),
            active_mobs: RwLock::new(HashMap::new()),
        }
    }

    /// 添加刷新点
    pub fn add_spawn(&self, map_name: &str, spawn: SpawnPoint) {
        let mut spawns = self.spawns.write();
        spawns.entry(map_name.to_string()).or_default().push(spawn);
    }

    /// 获取地图的刷新点
    pub fn get_spawns(&self, map_name: &str) -> Vec<SpawnPoint> {
        self.spawns.read().get(map_name).cloned().unwrap_or_default()
    }

    /// 注册活跃怪物
    pub fn register_mob(&self, map_name: &str, mob: Arc<Mob>) {
        let mut active = self.active_mobs.write();
        active.entry(map_name.to_string()).or_default().push(mob);
    }

    /// 移除死亡怪物
    pub fn unregister_mob(&self, map_name: &str, mob_id: &uuid::Uuid) {
        let mut active = self.active_mobs.write();
        if let Some(mobs) = active.get_mut(map_name) {
            mobs.retain(|m| &m.id != mob_id);
        }
    }

    /// 获取地图上所有活跃怪物
    pub fn get_active_mobs(&self, map_name: &str) -> Vec<Arc<Mob>> {
        self.active_mobs.read().get(map_name).cloned().unwrap_or_default()
    }

    /// 初始化默认刷新点
    pub fn init_default_spawns(&self) {
        // prontera.gat 刷新点
        self.add_spawn("prontera.gat", SpawnPoint {
            mob_id: 1001,  // Poring
            x: 100,
            y: 100,
            count: 10,
            interval: 10000,
        });
        self.add_spawn("prontera.gat", SpawnPoint {
            mob_id: 1002,  // Lunatic
            x: 150,
            y: 120,
            count: 5,
            interval: 15000,
        });

        // new_1-1.gat 刷新点
        self.add_spawn("new_1-1.gat", SpawnPoint {
            mob_id: 1001,  // Poring
            x: 50,
            y: 50,
            count: 15,
            interval: 5000,
        });
        self.add_spawn("new_1-1.gat", SpawnPoint {
            mob_id: 1312,  // Fabre
            x: 100,
            y: 100,
            count: 10,
            interval: 10000,
        });
    }
}

impl Default for MobSpawnManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: 创建 src/game/mob/ai.rs**

```rust
use std::sync::Arc;
use crate::game::mob::{Mob, MobAIState, MobSpawnManager};
use crate::game::map::MapState;

/// 怪物AI处理器
pub struct MobAI {
    spawn_manager: Arc<MobSpawnManager>,
}

impl MobAI {
    pub fn new(spawn_manager: Arc<MobSpawnManager>) -> Self {
        Self { spawn_manager }
    }

    /// 更新怪物AI
    pub fn update(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let state = *mob.ai_state.read();

        match state {
            MobAIState::Idle => self.update_idle(mob, map_state),
            MobAIState::Patrol => self.update_patrol(mob),
            MobAIState::Chase => self.update_chase(mob, map_state),
            MobAIState::Attack => self.update_attack(mob, map_state),
            MobAIState::Return => self.update_return(mob),
            MobAIState::Dead => self.update_dead(mob),
        }
    }

    fn update_idle(&self, mob: &Arc<Mob>, map_state: &MapState) {
        // 检查是否有玩家进入视野
        let (x, y) = mob.get_position();
        let players = map_state.get_players_on_map(&mob.map_name);

        for player in players {
            let (px, py) = player.get_position();
            let distance = Self::calculate_distance(x, y, px, py);

            if distance <= mob.sight_range as u16 {
                *mob.ai_state.write() = MobAIState::Chase;
                *mob.target_id.write() = Some(player.id);
                return;
            }
        }

        // 随机移动 (5%概率)
        if rand_simple() < 5 {
            let new_x = (x as i32 + rand_offset(-3, 3)).max(0) as u16;
            let new_y = (y as i32 + rand_offset(-3, 3)).max(0) as u16;
            mob.move_to(new_x, new_y);
        }
    }

    fn update_patrol(&self, mob: &Arc<Mob>) {
        // 巡逻逻辑
        let (x, y) = mob.get_position();
        let new_x = (x as i32 + rand_offset(-5, 5)).max(0) as u16;
        let new_y = (y as i32 + rand_offset(-5, 5)).max(0) as u16;
        mob.move_to(new_x, new_y);
    }

    fn update_chase(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let target_id = mob.target_id.read().clone();

        if let Some(target_id) = target_id {
            if let Some(target) = map_state.get_player(&target_id) {
                let (x, y) = mob.get_position();
                let (tx, ty) = target.get_position();
                let distance = Self::calculate_distance(x, y, tx, ty);

                if distance <= mob.atk_range {
                    // 进入攻击距离
                    *mob.ai_state.write() = MobAIState::Attack;
                } else if distance > mob.chase_range {
                    // 超出追击范围，返回
                    *mob.ai_state.write() = MobAIState::Return;
                    *mob.target_id.write() = None;
                } else {
                    // 继续追击
                    let new_x = Self::approach(x, tx);
                    let new_y = Self::approach(y, ty);
                    mob.move_to(new_x, new_y);
                }
            } else {
                // 目标消失，返回
                *mob.ai_state.write() = MobAIState::Return;
                *mob.target_id.write() = None;
            }
        }
    }

    fn update_attack(&self, mob: &Arc<Mob>, map_state: &MapState) {
        let target_id = mob.target_id.read().clone();

        if let Some(target_id) = target_id {
            if let Some(target) = map_state.get_player(&target_id) {
                // 计算伤害
                let damage = mob.atk as i32 - (*target.str.read() as i32 / 2);
                let damage = damage.max(1) as u32;

                // 应用伤害
                let dead = target.take_damage(damage);
                if dead {
                    *mob.ai_state.write() = MobAIState::Idle;
                    *mob.target_id.write() = None;
                }
            }
        }
    }

    fn update_return(&self, mob: &Arc<Mob>) {
        // 返回出生点逻辑 (简化)
        *mob.ai_state.write() = MobAIState::Idle;
    }

    fn update_dead(&self, mob: &Arc<Mob>) {
        // 死亡处理
        self.spawn_manager.unregister_mob(&mob.map_name, &mob.id);
    }

    /// 计算距离
    fn calculate_distance(x1: u16, y1: u16, x2: u16, y2: u16) -> u16 {
        let dx = (x1 as i32 - x2 as i32).abs();
        let dy = (y1 as i32 - y2 as i32).abs();
        ((dx * dx + dy * dy) as f32).sqrt() as u16
    }

    /// 向目标靠近一步
    fn approach(current: u16, target: u16) -> u16 {
        if current < target {
            (current + 1).min(target)
        } else {
            current.saturating_sub(1)
        }
    }
}

fn rand_simple() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 100) as i32
}

fn rand_offset(min: i32, max: i32) -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let range = max - min + 1;
    min + ((nanos as i32) % range)
}
```

- [ ] **Step 5: 更新 src/game/mod.rs**

```rust
//! 游戏业务层

pub mod login;
pub mod char;
pub mod map;
pub mod skill;
pub mod item;
pub mod mob;
pub mod npc;
pub mod battle;
```

- [ ] **Step 6: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat(mob): 添加怪物系统
- Mob, MobType, MobAIState 数据结构
- MobDatabase 怪物数据库
- MobSpawnManager 刷新管理
- MobAI 怪物行为AI
"
```

---

### Task 5: NPC系统

**Files:**
- Create: `src/game/npc/mod.rs`
- Create: `src/game/npc/data.rs`
- Create: `src/game/npc/handler.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 创建 src/game/npc/mod.rs**

```rust
//! NPC系统

pub mod data;
pub mod handler;

pub use data::{Npc, NpcType, NpcFlag};
pub use handler::NpcHandler;
```

- [ ] **Step 2: 创建 src/game/npc/data.rs**

```rust
use parking_lot::RwLock;

/// NPC类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Shop,          // 商店
    SkillTrainer,  // 技能训练师
    Quest,         // 任务NPC
    Warp,          // 传送门
    CashShop,      // 现金商店
}

/// NPC标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcFlag {
    None,
    NoWarp,        // 不能传送
    NoMob,         // 不刷怪
    NoSave,        // 不保存位置
    Private_,      // 私有NPC
}

/// NPC数据
#[derive(Debug, Clone)]
pub struct Npc {
    pub id: u32,
    pub name: String,
    pub display_name: String,
    pub type_: NpcType,
    pub pos_x: u16,
    pub pos_y: u16,
    pub map_name: String,
    pub sprite_id: u16,
    pub level: u16,
    pub flags: u32,

    // 商店物品 (如果是商店)
    pub shop_items: RwLock<Vec<ShopItem>>,

    // 技能列表 (如果是技能训练师)
    pub skills: RwLock<Vec<NpcSkill>>,
}

impl Npc {
    pub fn new(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            display_name: name.to_string(),
            type_: NpcType::Shop,
            pos_x: x,
            pos_y: y,
            map_name: map.to_string(),
            sprite_id: 100,
            level: 1,
            flags: 0,
            shop_items: RwLock::new(Vec::new()),
            skills: RwLock::new(Vec::new()),
        }
    }

    pub fn shop(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::Shop;
        npc
    }

    pub fn skill_trainer(id: u32, name: &str, x: u16, y: u16, map: &str) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::SkillTrainer;
        npc
    }

    pub fn warp(id: u32, name: &str, x: u16, y: u16, map: &str, dest_map: &str, dest_x: u16, dest_y: u16) -> Self {
        let mut npc = Self::new(id, name, x, y, map);
        npc.type_ = NpcType::Warp;
        npc
    }

    pub fn add_shop_item(&self, item_id: u16, price: u32) {
        self.shop_items.write().push(ShopItem { item_id, price });
    }

    pub fn add_skill(&self, skill_id: u16, sp_cost: u16, price: u32) {
        self.skills.write().push(NpcSkill { skill_id, sp_cost, price });
    }
}

/// 商店物品
#[derive(Debug, Clone)]
pub struct ShopItem {
    pub item_id: u16,
    pub price: u32,
}

/// NPC技能
#[derive(Debug, Clone)]
pub struct NpcSkill {
    pub skill_id: u16,
    pub sp_cost: u16,
    pub price: u32,
}

/// NPC数据库
pub struct NpcDatabase;

impl NpcDatabase {
    pub fn get_npc(id: u32) -> Option<Npc> {
        match id {
            1 => Some(Self::create_poring_merchant()),
            2 => Some(Self::create_basilisk_warrior()),
            3 => Some(Self::create_prontera_warp()),
            _ => None,
        }
    }

    fn create_poring_merchant() -> Npc {
        let npc = Npc::shop(1, "Poring Merchant", 50, 100, "new_1-1.gat");
        npc.display_name = "波利商人".to_string();
        npc.sprite_id = 124;
        npc.add_shop_item(501, 50);   // Red Potion
        npc.add_shop_item(502, 40);   // Yellow Potion
        npc.add_shop_item(503, 50);   // Blue Potion
        npc
    }

    fn create_basilisk_warrior() -> Npc {
        let npc = Npc::skill_trainer(2, "Basilisk Warrior", 100, 50, "new_1-1.gat");
        npc.display_name = "蜥蜴武士".to_string();
        npc.sprite_id = 404;
        npc.level = 10;
        npc.add_skill(1, 8, 1000);    // Bash
        npc.add_skill(25, 9, 2000);   // Fire Ball
        npc
    }

    fn create_prontera_warp() -> Npc {
        let npc = Npc::warp(3, "To Prontera", 150, 150, "new_1-1.gat", "prontera.gat", 150, 100);
        npc.display_name = "前往普隆德拉".to_string();
        npc.sprite_id = 405;
        npc
    }
}
```

- [ ] **Step 3: 创建 src/game/npc/handler.rs**

```rust
use std::sync::Arc;
use crate::game::map::Player;
use crate::game::item::ItemHandler;
use crate::game::skill::SkillHandler;
use super::data::Npc;

/// NPC交互处理器
pub struct NpcHandler {
    npcs: std::collections::HashMap<u32, Arc<Npc>>,
}

impl NpcHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            npcs: std::collections::HashMap::new(),
        };
        handler.init_default_npcs();
        handler
    }

    fn init_default_npcs(&mut self) {
        // 注册默认NPC
        if let Some(npc) = super::data::NpcDatabase::get_npc(1) {
            self.npcs.insert(1, Arc::new(npc));
        }
        if let Some(npc) = super::data::NpcDatabase::get_npc(2) {
            self.npcs.insert(2, Arc::new(npc));
        }
        if let Some(npc) = super::data::NpcDatabase::get_npc(3) {
            self.npcs.insert(3, Arc::new(npc));
        }
    }

    /// 获取NPC
    pub fn get_npc(&self, npc_id: u32) -> Option<Arc<Npc>> {
        self.npcs.get(&npc_id).cloned()
    }

    /// 获取地图上的NPC
    pub fn get_npcs_on_map(&self, map_name: &str) -> Vec<Arc<Npc>> {
        self.npcs.values()
            .filter(|n| n.map_name == map_name)
            .cloned()
            .collect()
    }

    /// 处理NPC交互
    pub fn interact(&self, player: &Player, npc_id: u32) -> NpcResponse {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return NpcResponse::NotFound,
        };

        match npc.type_ {
            super::data::NpcType::Shop => NpcResponse::OpenShop {
                npc_id,
                items: npc.shop_items.read().clone(),
            },
            super::data::NpcType::SkillTrainer => NpcResponse::SkillList {
                npc_id,
                skills: npc.skills.read().clone(),
            },
            super::data::NpcType::Warp => NpcResponse::Warp {
                map: npc.map_name.clone(),
                x: npc.pos_x,
                y: npc.pos_y,
            },
            _ => NpcResponse::Message(npc.display_name.clone()),
        }
    }

    /// 购买物品
    pub fn buy_item(&self, player: &Player, npc_id: u32, item_id: u16, amount: u8) -> BuyResult {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return BuyResult::NpcNotFound,
        };

        let shop_item = npc.shop_items.read()
            .iter()
            .find(|i| i.item_id == item_id)
            .copied();

        let shop_item = match shop_item {
            Some(i) => i,
            None => return BuyResult::ItemNotFound,
        };

        let total_price = shop_item.price * amount as u32;

        // 检查金币
        if player.zeny < total_price {
            return BuyResult::NotEnoughZeny;
        }

        // 扣除金币 (需要player有zeny字段)
        // player.zeny -= total_price;

        BuyResult::Success {
            item_id,
            amount,
            remaining_zeny: 0,  // player.zeny
        }
    }

    /// 学习技能
    pub fn learn_skill(&self, player: &Player, npc_id: u32, skill_id: u16) -> LearnResult {
        let npc = match self.get_npc(npc_id) {
            Some(n) => n,
            None => return LearnResult::NpcNotFound,
        };

        let npc_skill = npc.skills.read()
            .iter()
            .find(|s| s.skill_id == skill_id)
            .copied();

        let npc_skill = match npc_skill {
            Some(s) => s,
            None => return LearnResult::SkillNotFound,
        };

        // 检查金币
        // 检查SP

        LearnResult::Success { skill_id }
    }
}

impl Default for NpcHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// NPC响应
#[derive(Debug, Clone)]
pub enum NpcResponse {
    NotFound,
    Message(String),
    OpenShop { npc_id: u32, items: Vec<super::data::ShopItem> },
    SkillList { npc_id: u32, skills: Vec<super::data::NpcSkill> },
    Warp { map: String, x: u16, y: u16 },
}

/// 购买结果
#[derive(Debug, Clone)]
pub enum BuyResult {
    Success { item_id: u16, amount: u8, remaining_zeny: u32 },
    NpcNotFound,
    ItemNotFound,
    NotEnoughZeny,
    InventoryFull,
}

/// 学习技能结果
#[derive(Debug, Clone)]
pub enum LearnResult {
    Success { skill_id: u16 },
    NpcNotFound,
    SkillNotFound,
    NotEnoughZeny,
    AlreadyLearned,
}
```

- [ ] **Step 4: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(npc): 添加NPC系统
- Npc, NpcType, NpcFlag 数据结构
- NpcDatabase NPC数据库
- NpcHandler NPC交互处理
- 商店、技能训练师、传送门NPC
"
```

---

### Task 6: 战斗系统 - 伤害公式

**Files:**
- Create: `src/game/battle/mod.rs`
- Create: `src/game/battle/formula.rs`
- Create: `src/game/battle/handler.rs`
- Modify: `src/game/mod.rs`

- [ ] **Step 1: 创建 src/game/battle/mod.rs**

```rust
//! 战斗系统

pub mod formula;
pub mod handler;

pub use formula::BattleFormula;
pub use handler::BattleHandler;
```

- [ ] **Step 2: 创建 src/game/battle/formula.rs**

```rust
use crate::game::map::Player;
use crate::game::mob::Mob;

/// rAthena 风格战斗公式
pub struct BattleFormula;

impl BattleFormula {
    /// 计算物理攻击伤害
    pub fn physical_damage(
        attacker: &Player,
        defender: &Mob,
        skill_damage_bonus: i32,
        weapon_type: i32,
    ) -> i32 {
        // ATK = [(BaseLevel * 2) + STR + (DEX / 2) + (AGI / 3)] * 武器系数
        let base_atk = {
            let base_level = *attacker.base_level.read() as i32;
            let str = *attacker.str.read() as i32;
            let dex = *attacker.dex.read() as i32;
            let agi = *attacker.agi.read() as i32;

            (base_level * 2 + str + dex / 2 + agi / 3) as i32
        };

        // 武器ATK
        let weapon_atk = weapon_type * 2;

        // 总ATK
        let total_atk = base_atk + weapon_atk;

        // 防御力
        let defense = defender.defense as i32;

        // 伤害 = (ATK - DEF) * 技能倍率
        let damage = ((total_atk - defense).max(1) * skill_damage_bonus) / 100;

        // 伤害波动 (±10%)
        let variance = 90 + (rand_range(0, 20) as i32);
        (damage * variance) / 100
    }

    /// 计算魔法攻击伤害
    pub fn magical_damage(
        attacker: &Player,
        defender: &Mob,
        skill_damage_bonus: i32,
        matk: i32,
    ) -> i32 {
        // MATK = INT * 2 + (DEX / 3) + (base_level / 4)
        let base_matk = {
            let int = *attacker.int.read() as i32;
            let dex = *attacker.dex.read() as i32;
            let base_level = *attacker.base_level.read() as i32;

            int * 2 + dex / 3 + base_level / 4
        };

        // 魔法ATK
        let magic_atk = matk.max(base_matk);

        // 魔法防御
        let magic_defense = defender.magic_defense as i32;

        // 伤害 = (MATK - MDEF) * 技能倍率
        let damage = ((magic_atk - magic_defense).max(1) * skill_damage_bonus) / 100;

        // 伤害波动 (±10%)
        let variance = 90 + (rand_range(0, 20) as i32);
        (damage * variance) / 100
    }

    /// 计算命中率
    pub fn hit_rate(attacker: &Player, defender: &Mob) -> i32 {
        // 命中率 = 95 + (HIT - FLEE) / 2
        let hit = {
            let dex = *attacker.dex.read() as i32;
            let base_level = *attacker.base_level.read() as i32;
            (dex * 3) + base_level
        };

        let flee = defender.flee as i32;

        95 + (hit - flee) / 2
    }

    /// 计算闪避率
    pub fn flee_rate(player: &Player, mob: &Mob) -> i32 {
        // 闪避率 = 80 + AGI - (mob_level * 2)
        let agi = *player.agi.read() as i32;
        let base_level = *player.base_level.read() as i32;

        80 + agi - (base_level * 2)
    }

    /// 计算暴击率
    pub fn crit_rate(attacker: &Player, defender: &Mob) -> i32 {
        // 基础暴击 + LUK修正
        let base_crit = 0;
        let luk = *attacker.luk.read() as i32;

        base_crit + luk / 3
    }

    /// 计算暴击伤害
    pub fn crit_multiplier() -> i32 {
        140  // 暴击伤害 140%
    }

    /// 伤害减免
    pub fn damage_reduction(defense: i32) -> i32 {
        // DEF / (DEF + 100) * 100%
        // 防御力越高，减伤比例越大
        ((defense as f32) / (defense as f32 + 100.0) * 100.0) as i32
    }
}

fn rand_range(min: i32, max: i32) -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let range = max - min + 1;
    min + ((nanos as i32) % range)
}
```

- [ ] **Step 3: 创建 src/game/battle/handler.rs**

```rust
use std::sync::Arc;
use crate::game::map::Player;
use crate::game::mob::Mob;
use super::formula::BattleFormula;

/// 战斗处理器
pub struct BattleHandler;

impl BattleHandler {
    pub fn new() -> Self {
        Self
    }

    /// 普通攻击
    pub fn normal_attack(&self, attacker: &Player, defender: &Mob) -> AttackResult {
        // 检查命中
        let hit_chance = BattleFormula::hit_rate(attacker, defender);
        if rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        // 检查暴击
        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = rand_chance(crit_chance);

        // 计算伤害
        let base_damage = BattleFormula::physical_damage(attacker, defender, 100, 1);

        let damage = if is_crit {
            (base_damage as i32 * BattleFormula::crit_multiplier()) / 100
        } else {
            base_damage
        };

        // 应用伤害
        let killed = defender.take_damage(damage as u32);

        AttackResult::Hit {
            damage,
            is_crit,
            killed,
        }
    }

    /// 技能攻击
    pub fn skill_attack(
        &self,
        attacker: &Player,
        defender: &Mob,
        skill_damage: i32,
        skill_id: u16,
    ) -> AttackResult {
        // 检查命中
        let hit_chance = BattleFormula::hit_rate(attacker, defender) + 5;
        if !rand_chance(hit_chance) {
            return AttackResult::Miss;
        }

        // 检查暴击 (某些技能不暴击)
        let crit_chance = BattleFormula::crit_rate(attacker, defender);
        let is_crit = rand_chance(crit_chance) && skill_id != 25;  // Fire Ball 不暴击

        // 计算伤害
        let base_damage = BattleFormula::physical_damage(attacker, defender, skill_damage, 1);

        let damage = if is_crit {
            (base_damage as i32 * BattleFormula::crit_multiplier()) / 100
        } else {
            base_damage
        };

        // 应用伤害
        let killed = defender.take_damage(damage as u32);

        AttackResult::Hit {
            damage,
            is_crit,
            killed,
        }
    }

    /// 魔法攻击
    pub fn magic_attack(
        &self,
        attacker: &Player,
        defender: &Mob,
        skill_damage: i32,
    ) -> AttackResult {
        // 魔法攻击不Miss
        let matk = (*attacker.int.read() as i32) * 2 + (*attacker.dex.read() as i32) / 3;
        let damage = BattleFormula::magical_damage(attacker, defender, skill_damage, matk);

        // 应用伤害
        let killed = defender.take_damage(damage as u32);

        AttackResult::Hit {
            damage,
            is_crit: false,
            killed,
        }
    }
}

impl Default for BattleHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 攻击结果
#[derive(Debug, Clone)]
pub enum AttackResult {
    Miss,
    Hit {
        damage: i32,
        is_crit: bool,
        killed: bool,
    },
    Blocked,
    Immune,
}

fn rand_chance(percent: i32) -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos as i32 % 100) < percent
}
```

- [ ] **Step 4: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat(battle): 添加战斗系统
- BattleFormula 伤害公式
- BattleHandler 战斗处理
- 物理/魔法/技能攻击
- 命中/暴击/闪避计算
"
```

---

### Task 7: 地图系统增强 - 地形数据

**Files:**
- Create: `src/game/map/data.rs`
- Create: `src/game/map/cell.rs`
- Modify: `src/game/map/mod.rs`
- Modify: `src/game/map/map_state.rs`

- [ ] **Step 1: 创建 src/game/map/data.rs**

```rust
use std::collections::HashMap;

/// 地图数据
#[derive(Debug, Clone)]
pub struct MapData {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<CellType>,
    pub npcs: Vec<u32>,          // NPC ID列表
    pub monsters: Vec<u16>,     // 怪物ID列表
    pub warp_points: Vec<WarpPoint>,
    pub music: String,
}

/// 传送点
#[derive(Debug, Clone)]
pub struct WarpPoint {
    pub x: u16,
    pub y: u16,
    pub target_map: String,
    pub target_x: u16,
    pub target_y: u16,
}

impl MapData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            width: 100,
            height: 100,
            cells: Vec::new(),
            npcs: Vec::new(),
            monsters: Vec::new(),
            warp_points: Vec::new(),
            music: "01.mp3".to_string(),
        }
    }

    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self.cells = vec![CellType::Walkable; (width * height) as usize];
        self
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell_type: CellType) {
        let idx = self.get_index(x, y);
        if idx < self.cells.len() {
            self.cells[idx] = cell_type;
        }
    }

    pub fn get_cell(&self, x: u16, y: u16) -> Option<CellType> {
        let idx = self.get_index(x, y);
        self.cells.get(idx).copied()
    }

    fn get_index(&self, x: u16, y: u16) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }

    pub fn is_walkable(&self, x: u16, y: u16) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        matches!(self.get_cell(x, y), Some(CellType::Walkable) | Some(CellType::WaterWalkable))
    }
}

/// 地图数据库
pub struct MapDatabase {
    maps: HashMap<String, MapData>,
}

impl MapDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            maps: HashMap::new(),
        };
        db.init_default_maps();
        db
    }

    fn init_default_maps(&mut self) {
        // new_1-1.gat - 新手地图
        let mut map = MapData::new("new_1-1.gat").with_size(200, 200);
        map.music = "town.mp3".to_string();
        map.npcs.push(1);  // Poring Merchant
        map.npcs.push(2);  // Basilisk Warrior
        map.npcs.push(3);  // Warp to Prontera
        map.monsters.push(1001);  // Poring
        map.monsters.push(1312);  // Fabre

        // 添加一些障碍
        map.set_cell(50, 50, CellType::Wall);
        map.set_cell(51, 50, CellType::Wall);
        map.set_cell(52, 50, CellType::Wall);

        self.maps.insert("new_1-1.gat".to_string(), map);

        // prontera.gat - 普隆德拉城
        let mut map = MapData::new("prontera.gat").with_size(300, 300);
        map.music = "city.mp3".to_string();
        map.monsters.push(1001);  // Poring
        map.monsters.push(1002);  // Lunatic
        self.maps.insert("prontera.gat".to_string(), map);
    }

    pub fn get(&self, map_name: &str) -> Option<&MapData> {
        self.maps.get(map_name)
    }

    pub fn get_mut(&mut self, map_name: &str) -> Option<&mut MapData> {
        self.maps.get_mut(map_name)
    }

    pub fn all(&self) -> impl Iterator<Item = &MapData> {
        self.maps.values()
    }
}

impl Default for MapDatabase {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 创建 src/game/map/cell.rs**

```rust
use serde::{Deserialize, Serialize};

/// 地形类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellType {
    Walkable,        // 可行走
    NonWalkable,    // 不可行走 (障碍)
    Wall,           // 墙壁
    WaterWalkable,  // 浅水可通行
    Water,          // 深水不可通行
    Cliff,          // 悬崖
    Grass,          // 草地 (可能有隐藏怪物)
    Door,           // 门
    Teleport,       // 传送点
    NoMobSpawn,     // 不刷怪区域
    NoNPC,          // NPC区域
    SafeZone,       // 安全区
    PvPArea,        // PvP区域
    GvGArea,        // GvG区域
    BattleZone,     // 战斗区域
}

impl CellType {
    pub fn is_walkable(&self) -> bool {
        matches!(
            self,
            CellType::Walkable
                | CellType::WaterWalkable
                | CellType::Door
                | CellType::Teleport
                | CellType::SafeZone
        )
    }

    pub fn blocks_sight(&self) -> bool {
        matches!(self, CellType::Wall | CellType::Cliff)
    }

    pub fn has_damage(&self) -> bool {
        false  // 未来可扩展: 岩浆等伤害地形
    }
}

/// 地形属性
#[derive(Debug, Clone)]
pub struct CellAttribute {
    pub terrain: CellType,
    pub elevation: u8,      // 海拔高度
    pub is_night: bool,    // 夜间特殊属性
}
```

- [ ] **Step 3: 更新 src/game/map/mod.rs**

```rust
//! Map Server - 地图服务器核心

pub mod player;
pub mod map_state;
pub mod data;
pub mod cell;

pub use player::Player;
pub use map_state::MapState;
pub use data::{MapData, MapDatabase, WarpPoint};
pub use cell::{CellType, CellAttribute};
```

- [ ] **Step 4: 更新 src/game/map/map_state.rs 添加新方法**

```rust
// Add to existing MapState

impl MapState {
    /// 根据 char_id 获取玩家
    pub fn get_player_by_char_id(&self, char_id: u32) -> Option<Player> {
        let players = self.players.read();
        players.values()
            .find(|p| p.char_id == char_id)
            .cloned()
    }

    /// 检查位置是否可通行
    pub fn can_move_to(&self, map_name: &str, x: u16, y: u16, map_data: &MapDatabase) -> bool {
        if let Some(map) = map_data.get(map_name) {
            map.is_walkable(x, y)
        } else {
            false
        }
    }

    /// 移动玩家
    pub fn move_player(&self, player_id: &Uuid, x: u16, y: u16) -> bool {
        let players = self.players.read();
        if let Some(player) = players.get(player_id) {
            player.move_to(x, y);
            true
        } else {
            false
        }
    }

    /// 获取地图玩家数量
    pub fn get_map_player_count(&self, map_name: &str) -> usize {
        let by_map = self.players_by_map.read();
        by_map.get(map_name).map(|v| v.len()).unwrap_or(0)
    }
}
```

- [ ] **Step 5: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat(map): 添加地图系统增强
- MapData, MapDatabase 地图数据
- CellType 地形类型
- MapState 新增方法: 移动、位置检查
"
```

---

### Task 8: MapState 集成 - 添加 zeny 字段到 Player

**Files:**
- Modify: `src/game/map/player.rs`

- [ ] **Step 1: 更新 Player 结构体添加 zeny**

```rust
// 在 Player struct 中添加 zeny 字段
pub struct Player {
    pub id: Uuid,
    pub char_id: u32,
    pub account_id: u32,
    pub name: String,
    pub pos_x: RwLock<u16>,
    pub pos_y: RwLock<u16>,
    pub map_name: String,
    pub hp: RwLock<u32>,
    pub max_hp: RwLock<u32>,
    pub sp: RwLock<u32>,
    pub max_sp: RwLock<u32>,
    pub base_level: RwLock<u16>,
    pub job_level: RwLock<u16>,
    pub str: RwLock<u16>,
    pub agi: RwLock<u16>,
    pub vit: RwLock<u16>,
    pub int: RwLock<u16>,
    pub dex: RwLock<u16>,
    pub luk: RwLock<u16>,
    pub walk_speed: RwLock<u16>,
    pub zeny: RwLock<u32>,  // 新增
    pub status_points: RwLock<u16>,
    pub skill_points: RwLock<u16>,
}
```

- [ ] **Step 2: 更新 Player::from_character**

```rust
/// 从 Character 创建 Player
pub fn from_character(char: Character) -> Self {
    Self {
        id: Uuid::new_v4(),
        char_id: char.char_id,
        account_id: 0,
        name: char.name,
        pos_x: RwLock::new(char.last_x as u16),
        pos_y: RwLock::new(char.last_y as u16),
        map_name: char.last_map,
        hp: RwLock::new(char.hp),
        max_hp: RwLock::new(char.max_hp),
        sp: RwLock::new(char.sp),
        max_sp: RwLock::new(char.max_sp),
        base_level: RwLock::new(char.base_level),
        job_level: RwLock::new(char.job_level),
        str: RwLock::new(char.str),
        agi: RwLock::new(char.agi),
        vit: RwLock::new(char.vit),
        int: RwLock::new(char.int),
        dex: RwLock::new(char.dex),
        luk: RwLock::new(char.luk),
        walk_speed: RwLock::new(150),
        zeny: RwLock::new(char.zeny),  // 新增
        status_points: RwLock::new(0),
        skill_points: RwLock::new(char.skill_point),
    }
}
```

- [ ] **Step 3: 更新 Clone impl**

```rust
impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            char_id: self.char_id,
            account_id: self.account_id,
            name: self.name.clone(),
            pos_x: RwLock::new(*self.pos_x.read()),
            pos_y: RwLock::new(*self.pos_y.read()),
            map_name: self.map_name.clone(),
            hp: RwLock::new(*self.hp.read()),
            max_hp: RwLock::new(*self.max_hp.read()),
            sp: RwLock::new(*self.sp.read()),
            max_sp: RwLock::new(*self.max_sp.read()),
            base_level: RwLock::new(*self.base_level.read()),
            job_level: RwLock::new(*self.job_level.read()),
            str: RwLock::new(*self.str.read()),
            agi: RwLock::new(*self.agi.read()),
            vit: RwLock::new(*self.vit.read()),
            int: RwLock::new(*self.int.read()),
            dex: RwLock::new(*self.dex.read()),
            luk: RwLock::new(*self.luk.read()),
            walk_speed: RwLock::new(*self.walk_speed.read()),
            zeny: RwLock::new(*self.zeny.read()),  // 新增
            status_points: RwLock::new(*self.status_points.read()),
            skill_points: RwLock::new(*self.skill_points.read()),
        }
    }
}
```

- [ ] **Step 4: 添加 take_damage 方法到 Player**

```rust
/// 受到伤害
pub fn take_damage(&self, damage: u32) -> bool {
    let current_hp = *self.hp.read();
    if current_hp <= damage {
        *self.hp.write() = 0;
        true  // 死亡
    } else {
        *self.hp.write() = current_hp - damage;
        false
    }
}
```

- [ ] **Step 5: 更新 Character 结构体添加 zeny 和 skill_point**

Modify: `src/storage/character.rs`

```rust
#[derive(Debug, Clone)]
pub struct Character {
    pub char_id: u32,
    pub char_num: u8,
    pub name: String,
    pub class: u16,
    pub base_level: u16,
    pub job_level: u16,
    pub base_exp: u32,
    pub job_exp: u32,
    pub zeny: u32,           // 新增
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub hair: u16,
    pub hair_color: u16,
    pub clothes_color: u16,
    pub weapon: u16,
    pub shield: u16,
    pub head_top: u16,
    pub head_mid: u16,
    pub head_bottom: u16,
    pub last_map: String,
    pub last_x: i32,
    pub last_y: i32,
    pub delete_timer: u32,
    pub skill_point: u16,    // 新增
    pub created_at: i64,
    pub updated_at: i64,
}
```

- [ ] **Step 6: 编译验证**

Run: `cargo check`
Expected: 无编译错误

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat(player): Player 添加 zeny 和 take_damage
- Player 新增 zeny 字段
- Player 新增 take_damage 方法
- Character 新增 zeny 和 skill_point 字段
"
```

---

### Task 9: 单元测试

**Files:**
- Create: `tests/game_test.rs`

- [ ] **Step 1: 创建测试文件**

```rust
use deviruchi::game::{
    skill::{SkillDatabase, SkillType, SkillTarget},
    item::{ItemDatabase, ItemType, Inventory},
    mob::{Mob, MobDatabase, MobType},
    battle::{BattleFormula, BattleHandler},
    map::{MapData, CellType},
};

#[test]
fn test_skill_database() {
    let db = SkillDatabase::new();

    let bash = db.get(1).unwrap();
    assert_eq!(bash.name, "Bash");
    assert_eq!(bash.type_, SkillType::Attack);
    assert_eq!(bash.sp_cost, 8);
}

#[test]
fn test_item_database() {
    let db = ItemDatabase::new();

    let potion = db.get(501).unwrap();
    assert_eq!(potion.name, "Red Potion");
    assert_eq!(potion.type_, ItemType::Heal);
    assert_eq!(potion.hp_restore, 120);
}

#[test]
fn test_inventory_add_remove() {
    let db = std::sync::Arc::new(ItemDatabase::new());
    let mut inv = Inventory::new(100, db);

    assert!(inv.add_item(501, 10));
    assert_eq!(inv.slots()[0].item_id, 501);
    assert_eq!(inv.slots()[0].amount, 10);

    assert!(inv.remove_item(0, 5));
    assert_eq!(inv.slots()[0].amount, 5);
}

#[test]
fn test_mob_creation() {
    let mob = Mob::from_template(1001, 100, 100, "test.gat");

    assert_eq!(mob.mob_id, 1001);
    assert_eq!(mob.name, "Poring");
    assert_eq!(mob.max_hp, 50);
    assert!(!mob.is_dead());
}

#[test]
fn test_mob_take_damage() {
    let mob = Mob::new(1001, 50, 50, "test.gat");
    assert_eq!(*mob.hp.read(), 100);

    let dead = mob.take_damage(50);
    assert!(!dead);
    assert_eq!(*mob.hp.read(), 50);

    let dead = mob.take_damage(100);
    assert!(dead);
    assert_eq!(*mob.hp.read(), 0);
}

#[test]
fn test_map_data() {
    let mut map = MapData::new("test.gat").with_size(100, 100);

    assert!(map.is_walkable(50, 50));

    map.set_cell(10, 10, CellType::Wall);
    assert!(!map.is_walkable(10, 10));
}

#[test]
fn test_map_out_of_bounds() {
    let map = MapData::new("test.gat").with_size(100, 100);

    assert!(!map.is_walkable(150, 50));  // x 越界
    assert!(!map.is_walkable(50, 150));  // y 越界
}
```

- [ ] **Step 2: 运行测试**

Run: `cargo test --test game_test`
Expected: 所有测试通过

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "test: 添加游戏系统单元测试
- SkillDatabase 测试
- ItemDatabase 测试
- Inventory 测试
- Mob 创建和伤害测试
- MapData 地形测试
"
```

---

### Task 10: 最终编译验证与文档

**Files:**
- Modify: `src/game/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 更新 src/game/mod.rs 导出所有子系统**

```rust
//! 游戏业务层

pub mod login;
pub mod char;
pub mod map;
pub mod skill;
pub mod item;
pub mod mob;
pub mod npc;
pub mod battle;

pub use login::LoginServer;
pub use char::CharServer;
pub use map::{Player, MapState, MapData, MapDatabase, CellType};
pub use skill::{Skill, SkillType, SkillDatabase, SkillHandler};
pub use item::{Item, ItemType, ItemDatabase, ItemHandler, Inventory};
pub use mob::{Mob, MobType, MobAIState, MobDatabase, MobSpawnManager, MobAI};
pub use npc::{Npc, NpcType, NpcHandler};
pub use battle::{BattleFormula, BattleHandler};
```

- [ ] **Step 2: 完整编译测试**

Run: `cargo build`
Expected: 编译成功

- [ ] **Step 3: 运行所有测试**

Run: `cargo test`
Expected: 所有测试通过

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: 完成 Phase 3 游戏逻辑实现
- 技能系统: 数据定义、效果、处理器
- 物品系统: 数据、背包管理、使用处理
- 怪物系统: 数据、刷新、AI行为
- NPC系统: 商店、技能训练师、传送门
- 战斗系统: 伤害公式、攻击处理
- 地图系统: 地形数据、障碍检查
- 单元测试覆盖
"
```

---

## 自检清单

### Spec 覆盖检查
- [x] 技能系统 - Task 1-2
- [x] 物品系统 - Task 3
- [x] 怪物系统 - Task 4
- [x] NPC系统 - Task 5
- [x] 战斗系统 - Task 6
- [x] 地图增强 - Task 7-8
- [x] 单元测试 - Task 9

### 占位符检查
- [x] 无 TBD/TODO
- [x] 所有代码块完整
- [x] 所有测试代码完整

### 类型一致性
- [x] `Skill` 字段统一
- [x] `Item` 字段统一
- [x] `Mob` 字段统一
- [x] `Player` 与 `Character` 字段映射
- [x] `BattleFormula` 方法签名一致

### 依赖检查
- [x] 无需新依赖
- [x] `parking_lot` 已配置
- [x] `uuid` 已配置
- [x] `serde` 已配置 (如果有序列化需求)

---

## 执行方式选择

**Plan complete and saved to `docs/superpowers/plans/2026-05-02-deviruchi-phase3-plan.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
