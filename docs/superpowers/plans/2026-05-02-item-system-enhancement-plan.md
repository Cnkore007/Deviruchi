# 物品系统完善实现计划

> **目标:** 完善Deviruchi物品系统：Zeny货币、商店购买/出售、装备系统、YAML配置、重量系统、交易系统（考虑重量）

**架构:** 扩展现有Item/Inventory/Player结构，新增Equipment管理、重量计算、YAML加载器，实现完整的经济循环

**Tech Stack:** Rust, parking_lot::RwLock, serde_yaml

---

## 任务1: 扩展Item结构 - 拆分价格字段

**Files:**
- Modify: `src/game/item/data.rs:27-48`

- [ ] **Step 1: 修改Item结构体**

```rust
pub struct Item {
    pub id: u16,
    pub name: &'static str,
    pub type_: ItemType,
    pub buy_price: u32,      // 修改: price -> buy_price
    pub sell_price: u32,     // 新增
    pub weight: u16,
    pub flags: u32,
    pub hp_restore: u16,
    pub sp_restore: u16,
    pub equip_mask: u32,
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub str_bonus: i16,
    pub agi_bonus: i16,
    pub vit_bonus: i16,
    pub int_bonus: i16,
    pub dex_bonus: i16,
    pub luk_bonus: i16,
}
```

- [ ] **Step 2: 更新init_default_items函数**

将`price:`改为`buy_price:`，添加`sell_price: price/2`。

- [ ] **Step 3: 编译验证**

```bash
cd /Users/kimmy/Documents/project/Deviruchi && cargo check 2>&1 | head -50
```

Expected: 编译通过

---

## 任务2: 添加装备槽位和装备管理

**Files:**
- Create: `src/game/item/equipment.rs`
- Modify: `src/game/item/mod.rs`

- [ ] **Step 1: 创建装备系统文件**

```rust
use std::collections::HashMap;
use super::inventory::InventorySlot;

/// 装备槽位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    HeadTop,     // 头盔(上)
    HeadMid,     // 头盔(中)
    HeadLow,     // 头盔(下)
    Body,        // 身体
    RightHand,   // 右手武器
    LeftHand,    // 左手(武器/盾牌)
    Robe,        // 披风
    Shoes,       // 鞋子
    Accessory1,  // 饰品1
    Accessory2,  // 饰品2
}

impl EquipSlot {
    /// 从掩码获取槽位列表
    pub fn from_mask(mask: u32) -> Vec<EquipSlot> {
        let mut slots = Vec::new();
        if mask & 0x0001 != 0 { slots.push(EquipSlot::RightHand); }
        if mask & 0x0002 != 0 { slots.push(EquipSlot::LeftHand); }
        if mask & 0x0004 != 0 { slots.push(EquipSlot::HeadTop); }
        if mask & 0x0008 != 0 { slots.push(EquipSlot::Body); }
        if mask & 0x0010 != 0 { slots.push(EquipSlot::HeadLow); }
        if mask & 0x0020 != 0 { slots.push(EquipSlot::Shoes); }
        if mask & 0x0040 != 0 { slots.push(EquipSlot::Accessory1); }
        if mask & 0x0080 != 0 { slots.push(EquipSlot::Accessory2); }
        if mask & 0x0100 != 0 { slots.push(EquipSlot::HeadMid); }
        if mask & 0x0200 != 0 { slots.push(EquipSlot::Robe); }
        slots
    }

    /// 转换为掩码
    pub fn to_mask(&self) -> u32 {
        match self {
            EquipSlot::RightHand => 0x0001,
            EquipSlot::LeftHand => 0x0002,
            EquipSlot::HeadTop => 0x0004,
            EquipSlot::Body => 0x0008,
            EquipSlot::HeadLow => 0x0010,
            EquipSlot::Shoes => 0x0020,
            EquipSlot::Accessory1 => 0x0040,
            EquipSlot::Accessory2 => 0x0080,
            EquipSlot::HeadMid => 0x0100,
            EquipSlot::Robe => 0x0200,
        }
    }
}

/// 装备管理
#[derive(Debug, Clone)]
pub struct Equipment {
    slots: HashMap<EquipSlot, InventorySlot>,
}

impl Equipment {
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    /// 装备物品，返回被替换的旧物品（如果有）
    pub fn equip(&mut self, slot: EquipSlot, item: InventorySlot) -> Option<InventorySlot> {
        self.slots.insert(slot, item)
    }

    /// 卸下装备
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<InventorySlot> {
        self.slots.remove(&slot)
    }

    /// 获取装备
    pub fn get(&self, slot: EquipSlot) -> Option<&InventorySlot> {
        self.slots.get(&slot)
    }

    /// 获取所有装备
    pub fn get_all(&self) -> &HashMap<EquipSlot, InventorySlot> {
        &self.slots
    }

    /// 清空装备
    pub fn clear(&mut self) {
        self.slots.clear();
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: 更新mod.rs**

```rust
pub mod data;
pub mod inventory;
pub mod handler;
pub mod equipment;

pub use data::{Item, ItemType, ItemDatabase};
pub use inventory::{Inventory, InventorySlot};
pub use equipment::{Equipment, EquipSlot};
```

---

## 任务3: Player扩展 - 添加装备和重量字段

**Files:**
- Modify: `src/game/map/player.rs`

- [ ] **Step 1: 添加导入和字段**

```rust
use crate::game::item::{Equipment, EquipSlot, InventorySlot};

pub struct Player {
    // ... 现有字段 ...
    pub zeny: RwLock<u32>,
    pub current_weight: RwLock<u32>,  // 新增: 当前负重 (0.1单位)
    pub max_weight: RwLock<u32>,      // 新增: 最大负重
    pub equipment: RwLock<Equipment>, // 新增: 装备管理
}
```

- [ ] **Step 2: 更新Clone实现**

```rust
impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            // ... 现有字段 ...
            zeny: RwLock::new(*self.zeny.read()),
            current_weight: RwLock::new(*self.current_weight.read()),
            max_weight: RwLock::new(*self.max_weight.read()),
            equipment: RwLock::new(self.equipment.read().clone()),
        }
    }
}
```

- [ ] **Step 3: 更新from_character构造函数**

```rust
pub fn from_character(char: Character) -> Self {
    Self {
        // ... 现有字段 ...
        zeny: RwLock::new(char.zeny as u32),
        current_weight: RwLock::new(0),
        max_weight: RwLock::new(20000 + (char.str as u32) * 200),
        equipment: RwLock::new(Equipment::new()),
    }
}
```

- [ ] **Step 4: 添加重量计算方法**

```rust
impl Player {
    /// 计算最大负重 (基础20000 + STR*200, 单位0.1)
    pub fn calc_max_weight(&self) -> u32 {
        let str = *self.str.read();
        20000 + (str as u32) * 200
    }

    /// 更新最大负重
    pub fn update_max_weight(&self) {
        let new_max = self.calc_max_weight();
        *self.max_weight.write() = new_max;
    }

    /// 检查是否超重(50%)
    pub fn is_overweight(&self) -> bool {
        let current = *self.current_weight.read();
        let max = *self.max_weight.read();
        current > max * 50 / 100
    }

    /// 检查是否严重超重(90%)
    pub fn is_overweight_90(&self) -> bool {
        let current = *self.current_weight.read();
        let max = *self.max_weight.read();
        current > max * 90 / 100
    }
}
```

---

## 任务4: Inventory重量系统

**Files:**
- Modify: `src/game/item/inventory.rs`

- [ ] **Step 1: 添加重量相关字段和方法**

```rust
pub struct Inventory {
    max_size: u8,
    slots: Vec<InventorySlot>,
    item_db: Arc<ItemDatabase>,
    total_weight: u32,  // 新增
}

impl Inventory {
    pub fn new(max_size: u8, item_db: Arc<ItemDatabase>) -> Self {
        let slots: Vec<_> = (0..max_size)
            .map(InventorySlot::empty)
            .collect();

        Self {
            max_size,
            slots,
            item_db,
            total_weight: 0,
        }
    }

    /// 计算总重量
    pub fn calc_weight(&self) -> u32 {
        self.slots.iter()
            .filter(|s| !s.is_empty())
            .map(|s| {
                let item = self.item_db.get(s.item_id).unwrap_or_default();
                (item.weight as u32) * (s.amount as u32)
            })
            .sum()
    }

    /// 获取总重量
    pub fn total_weight(&self) -> u32 {
        self.total_weight
    }

    /// 检查能否添加物品（重量限制）
    pub fn can_carry_weight(&self, item_id: u16, amount: u16, max_weight: u32, current_weight: u32) -> bool {
        let item = self.item_db.get(item_id)?;
        let add_weight = (item.weight as u32) * (amount as u32);
        current_weight + add_weight <= max_weight
    }

    /// 更新重量
    pub fn update_weight(&mut self) {
        self.total_weight = self.calc_weight();
    }

    /// 获取物品重量
    pub fn get_item_weight(&self, item_id: u16) -> u16 {
        self.item_db.get(item_id)
            .map(|i| i.weight)
            .unwrap_or(0)
    }
}
```

- [ ] **Step 2: 更新add_item方法检查重量**

修改`add_item`方法，在添加物品前检查重量（需要传入max_weight和current_weight参数，或由调用方检查）。

---

## 任务5: ZenyManager实现

**Files:**
- Create: `src/game/zeny.rs`

- [ ] **Step 1: 创建ZenyManager**

```rust
use crate::game::map::Player;

pub const MAX_ZENY: u32 = 999_999_999;

pub struct ZenyManager;

impl ZenyManager {
    /// 增加Zeny，返回实际增加数量
    pub fn add(player: &Player, amount: u32) -> u32 {
        let current = *player.zeny.read();
        let can_add = MAX_ZENY - current;
        let actual_add = amount.min(can_add);
        *player.zeny.write() = current + actual_add;
        actual_add
    }

    /// 扣除Zeny，返回是否成功
    pub fn sub(player: &Player, amount: u32) -> bool {
        let current = *player.zeny.read();
        if current >= amount {
            *player.zeny.write() = current - amount;
            true
        } else {
            false
        }
    }

    /// 检查是否足够
    pub fn can_spend(player: &Player, amount: u32) -> bool {
        *player.zeny.read() >= amount
    }

    /// 获取当前Zeny
    pub fn get(player: &Player) -> u32 {
        *player.zeny.read()
    }

    /// 设置Zeny（用于初始化）
    pub fn set(player: &Player, amount: u32) {
        *player.zeny.write() = amount.min(MAX_ZENY);
    }
}
```

- [ ] **Step 2: 创建mod.rs**

```rust
pub mod zeny;
pub use zeny::ZenyManager;
```

---

## 任务6: 完善商店购买/出售逻辑

**Files:**
- Modify: `src/game/npc/data.rs` - 添加sell_price
- Modify: `src/game/npc/handler.rs`

- [ ] **Step 1: 更新ShopItem结构**

```rust
pub struct ShopItem {
    pub item_id: u16,
    pub buy_price: u32,
    pub sell_price: u32,  // 新增
}
```

- [ ] **Step 2: 更新NpcDatabase初始化**

将`add_shop_item`调用改为传入buy_price和sell_price。

- [ ] **Step 3: 完善buy_item函数**

```rust
use crate::game::item::{Inventory, InventorySlot};
use crate::game::zeny::ZenyManager;

pub fn buy_item(
    &self,
    player: &Player,
    inventory: &mut Inventory,
    npc_id: u32,
    item_id: u16,
    amount: u8
) -> BuyResult {
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

    let total_price = shop_item.buy_price * amount as u32;

    // 检查金币
    if !ZenyManager::can_spend(player, total_price) {
        return BuyResult::NotEnoughZeny;
    }

    // 检查重量
    let max_weight = *player.max_weight.read();
    let current_weight = *player.current_weight.read();
    if !inventory.can_carry_weight(item_id, amount as u16, max_weight, current_weight) {
        return BuyResult::Overweight;
    }

    // 检查背包空间
    if !inventory.can_add_item(item_id, amount as u16) {
        return BuyResult::InventoryFull;
    }

    // 扣除金币
    ZenyManager::sub(player, total_price);

    // 添加物品
    inventory.add_item(item_id, amount as u16);
    inventory.update_weight();
    *player.current_weight.write() = inventory.total_weight();

    BuyResult::Success {
        item_id,
        amount,
        remaining_zeny: ZenyManager::get(player),
    }
}
```

- [ ] **Step 4: 添加sell_item函数**

```rust
pub fn sell_item(
    &self,
    player: &Player,
    inventory: &mut Inventory,
    inventory_index: u8,
    amount: u8
) -> SellResult {
    // 获取物品
    let slot = inventory.slots()
        .get(inventory_index as usize)
        .copied()
        .ok_or(SellError::InvalidSlot)?;

    if slot.is_empty() || slot.amount < amount as u16 {
        return SellResult::Failed(SellError::NotEnoughItems);
    }

    // 获取物品数据
    let item = match inventory.get_database().get(slot.item_id) {
        Some(i) => i,
        None => return SellResult::Failed(SellError::InvalidItem),
    };

    let total_gold = item.sell_price * amount as u32;

    // 检查是否会超过Zeny上限
    let current_zeny = ZenyManager::get(player);
    if current_zeny + total_gold > MAX_ZENY {
        return SellResult::Failed(SellError::ZenyOverflow);
    }

    // 移除物品
    if !inventory.remove_item(inventory_index, amount as u16) {
        return SellResult::Failed(SellError::RemoveFailed);
    }

    // 增加Zeny
    ZenyManager::add(player, total_gold);
    inventory.update_weight();
    *player.current_weight.write() = inventory.total_weight();

    SellResult::Success {
        item_id: slot.item_id,
        amount,
        gained_zeny: total_gold,
    }
}
```

- [ ] **Step 5: 添加SellResult和SellError枚举**

```rust
#[derive(Debug, Clone)]
pub enum SellResult {
    Success { item_id: u16, amount: u8, gained_zeny: u32 },
    Failed(SellError),
}

#[derive(Debug, Clone, Copy)]
pub enum SellError {
    InvalidSlot,
    NotEnoughItems,
    InvalidItem,
    ZenyOverflow,
    RemoveFailed,
}
```

- [ ] **Step 6: 添加can_add_item方法到Inventory**

```rust
/// 检查能否添加物品（仅检查空间，不检查重量）
pub fn can_add_item(&self, item_id: u16, amount: u16) -> bool {
    // 先找相同物品的空位
    for slot in &self.slots {
        if slot.item_id == item_id && slot.amount + amount <= 300 {
            return true;
        }
    }

    // 找空位
    for slot in &self.slots {
        if slot.is_empty() {
            return true;
        }
    }

    false
}
```

---

## 任务7: ItemEffect框架

**Files:**
- Create: `src/game/item/effect.rs`
- Modify: `src/game/item/mod.rs`

- [ ] **Step 1: 创建效果系统**

```rust
use crate::game::map::Player;

/// 物品效果类型
#[derive(Debug, Clone)]
pub enum ItemEffect {
    HealHp(u16),
    HealSp(u16),
    DamageHp(u16),       // 伤害类物品
    Teleport { map: String, x: u16, y: u16 },
    Buff { stat: StatType, value: i16, duration_secs: u32 },
    AddZeny(u32),        // 直接加钱
    LearnSkill(u16),     // 学习技能
    OpenStorage,         // 打开仓库
    // 更多效果...
}

#[derive(Debug, Clone, Copy)]
pub enum StatType {
    Str, Agi, Vit, Int, Dex, Luk,
    Atk, Matk, Def, Mdef, Hit, Flee,
    Aspd, Hp, Sp, MaxHp, MaxSp,
}

#[derive(Debug, Clone)]
pub enum EffectResult {
    Success,
    Failed(EffectError),
    PartialSuccess { msg: String },
}

#[derive(Debug, Clone, Copy)]
pub enum EffectError {
    InvalidTarget,
    CooldownNotReady,
    SkillAlreadyLearned,
    CannotUseHere,
    SystemError,
}

impl ItemEffect {
    /// 执行效果
    pub fn apply(&self, player: &Player) -> EffectResult {
        match self {
            ItemEffect::HealHp(amount) => {
                let current = *player.hp.read();
                let max = *player.max_hp.read();
                let new_hp = (current + *amount as u32).min(max);
                *player.hp.write() = new_hp;
                EffectResult::Success
            }
            ItemEffect::HealSp(amount) => {
                let current = *player.sp.read();
                let max = *player.max_sp.read();
                let new_sp = (current + *amount as u32).min(max);
                *player.sp.write() = new_sp;
                EffectResult::Success
            }
            ItemEffect::AddZeny(amount) => {
                use crate::game::zeny::ZenyManager;
                ZenyManager::add(player, *amount);
                EffectResult::Success
            }
            // 其他效果...
            _ => EffectResult::Failed(EffectError::SystemError),
        }
    }
}

/// 从脚本字符串解析效果
pub fn parse_item_script(script: &str) -> Vec<ItemEffect> {
    let mut effects = Vec::new();
    for line in script.split(';') {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() { continue; }

        match parts[0] {
            "item_heal" => {
                let hp = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                let sp = parts.get(2).and_then(|s| s.trim_end_matches(',').parse().ok()).unwrap_or(0);
                if hp > 0 { effects.push(ItemEffect::HealHp(hp)); }
                if sp > 0 { effects.push(ItemEffect::HealSp(sp)); }
            }
            "zeny" => {
                let amount = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                effects.push(ItemEffect::AddZeny(amount));
            }
            // 更多指令...
            _ => {}
        }
    }
    effects
}
```

- [ ] **Step 2: 更新mod.rs导出**

```rust
pub mod equipment;
pub mod effect;

pub use data::{Item, ItemType, ItemFlag, ItemDatabase};
pub use inventory::{Inventory, InventorySlot};
pub use equipment::{Equipment, EquipSlot};
pub use effect::{ItemEffect, EffectResult, EffectError, parse_item_script};
```

---

## 任务8: YAML配置加载

**Files:**
- Create: `db/item_db.yml`
- Create: `src/game/item/yaml_loader.rs`

- [ ] **Step 1: 创建YAML数据文件**

```yaml
# db/item_db.yml
- Id: 501
  Name: Red Potion
  Type: Heal
  BuyPrice: 50
  SellPrice: 25
  Weight: 7
  HpRestore: 120
  SpRestore: 0

- Id: 502
  Name: Yellow Potion
  Type: Heal
  BuyPrice: 40
  SellPrice: 20
  Weight: 5
  HpRestore: 60
  SpRestore: 0

- Id: 503
  Name: Blue Potion
  Type: Heal
  BuyPrice: 50
  SellPrice: 25
  Weight: 7
  HpRestore: 0
  SpRestore: 40

- Id: 1201
  Name: Dagger
  Type: Weapon
  BuyPrice: 1000
  SellPrice: 500
  Weight: 50
  Atk: 10
  EquipMask: 0x0001

- Id: 1202
  Name: Main Gauche
  Type: Weapon
  BuyPrice: 2500
  SellPrice: 1250
  Weight: 60
  Atk: 15
  EquipMask: 0x0001

- Id: 1501
  Name: Clothes
  Type: Armor
  BuyPrice: 500
  SellPrice: 250
  Weight: 40
  Defense: 2
  EquipMask: 0x0008
```

- [ ] **Step 2: 创建YAML加载器**

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::error::Error;
use super::data::{Item, ItemType};

#[derive(Deserialize, Debug)]
struct ItemYaml {
    #[serde(rename = "Id")]
    id: u16,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Type")]
    type_: String,
    #[serde(rename = "BuyPrice")]
    buy_price: u32,
    #[serde(rename = "SellPrice")]
    sell_price: u32,
    #[serde(rename = "Weight")]
    weight: u16,
    #[serde(rename = "HpRestore", default)]
    hp_restore: u16,
    #[serde(rename = "SpRestore", default)]
    sp_restore: u16,
    #[serde(rename = "Atk", default)]
    atk: u16,
    #[serde(rename = "EquipMask", default)]
    equip_mask: u32,
    #[serde(rename = "Defense", default)]
    defense: u16,
}

impl ItemYaml {
    fn to_item(&self) -> Item {
        Item {
            id: self.id,
            name: Box::leak(self.name.clone().into_boxed_str()),
            type_: match self.type_.as_str() {
                "Heal" => ItemType::Heal,
                "Weapon" => ItemType::Weapon,
                "Armor" => ItemType::Armor,
                "Card" => ItemType::Card,
                "PetEgg" => ItemType::PetEgg,
                "PetArmor" => ItemType::PetArmor,
                _ => ItemType::Etc,
            },
            buy_price: self.buy_price,
            sell_price: self.sell_price,
            weight: self.weight,
            flags: 0,
            hp_restore: self.hp_restore,
            sp_restore: self.sp_restore,
            equip_mask: self.equip_mask,
            atk: self.atk,
            matk: 0,
            defense: self.defense,
            magic_defense: 0,
            str_bonus: 0,
            agi_bonus: 0,
            vit_bonus: 0,
            int_bonus: 0,
            dex_bonus: 0,
            luk_bonus: 0,
        }
    }
}

pub struct ItemDbLoader;

impl ItemDbLoader {
    pub fn load_from_yaml(path: &str) -> Result<HashMap<u16, Item>, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let yaml_items: Vec<ItemYaml> = serde_yaml::from_str(&content)?;

        let mut db = HashMap::new();
        for y in yaml_items {
            db.insert(y.id, y.to_item());
        }

        Ok(db)
    }
}
```

- [ ] **Step 3: 修改ItemDatabase支持YAML加载**

```rust
impl ItemDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            items: std::collections::HashMap::new(),
        };
        // 尝试从YAML加载，失败则使用默认物品
        if let Ok(yaml_items) = super::yaml_loader::ItemDbLoader::load_from_yaml("db/item_db.yml") {
            db.items = yaml_items;
        } else {
            db.init_default_items();
        }
        db
    }
    // ...
}
```

- [ ] **Step 4: 更新Cargo.toml**

确保已添加依赖:
```toml
serde_yaml = "0.9"
```

---

## 任务9: 交易系统（考虑重量）

**Files:**
- Create: `src/game/trade/mod.rs`

- [ ] **Step 1: 创建交易模块**

```rust
use std::collections::HashMap;
use uuid::Uuid;
use parking_lot::RwLock;
use crate::game::map::Player;
use crate::game::item::{Inventory, ItemDatabase};
use crate::game::zeny::ZenyManager;

/// 交易物品
#[derive(Debug, Clone, Copy)]
pub struct TradeItem {
    pub inventory_index: u8,
    pub item_id: u16,
    pub amount: u16,
}

/// 交易会话
#[derive(Debug)]
pub struct TradeSession {
    pub id: Uuid,
    pub player1_id: Uuid,
    pub player2_id: Uuid,
    pub items1: RwLock<Vec<TradeItem>>,
    pub items2: RwLock<Vec<TradeItem>>,
    pub zeny1: RwLock<u32>,
    pub zeny2: RwLock<u32>,
    pub confirmed1: RwLock<bool>,
    pub confirmed2: RwLock<bool>,
}

impl TradeSession {
    pub fn new(player1_id: Uuid, player2_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            player1_id,
            player2_id,
            items1: RwLock::new(Vec::new()),
            items2: RwLock::new(Vec::new()),
            zeny1: RwLock::new(0),
            zeny2: RwLock::new(0),
            confirmed1: RwLock::new(false),
            confirmed2: RwLock::new(false),
        }
    }

    /// 检查交易有效性（包括重量）
    pub fn validate(
        &self,
        player1: &Player,
        inv1: &Inventory,
        player2: &Player,
        inv2: &Inventory,
        item_db: &ItemDatabase,
    ) -> Result<(), TradeError> {
        // 计算双方将要增加的重量
        let weight_gain_1 = self.calc_weight_gain(&*self.items2.read(), item_db);
        let weight_gain_2 = self.calc_weight_gain(&*self.items1.read(), item_db);

        let max_weight_1 = *player1.max_weight.read();
        let max_weight_2 = *player2.max_weight.read();
        let current_weight_1 = *player1.current_weight.read();
        let current_weight_2 = *player2.current_weight.read();

        // 检查玩家1是否会超重
        if current_weight_1 + weight_gain_1 > max_weight_1 {
            return Err(TradeError::Overweight(player1.name.clone()));
        }

        // 检查玩家2是否会超重
        if current_weight_2 + weight_gain_2 > max_weight_2 {
            return Err(TradeError::Overweight(player2.name.clone()));
        }

        // 检查Zeny是否足够
        let zeny1 = *self.zeny1.read();
        let zeny2 = *self.zeny2.read();

        if *player1.zeny.read() < zeny1 {
            return Err(TradeError::NotEnoughZeny(player1.name.clone()));
        }
        if *player2.zeny.read() < zeny2 {
            return Err(TradeError::NotEnoughZeny(player2.name.clone()));
        }

        // 检查Zeny是否会导致溢出
        if *player1.zeny.read() - zeny1 + zeny2 > MAX_ZENY {
            return Err(TradeError::ZenyOverflow(player1.name.clone()));
        }
        if *player2.zeny.read() - zeny2 + zeny1 > MAX_ZENY {
            return Err(TradeError::ZenyOverflow(player2.name.clone()));
        }

        Ok(())
    }

    fn calc_weight_gain(&self, items: &[TradeItem], item_db: &ItemDatabase) -> u32 {
        items.iter()
            .map(|item| {
                let item_data = item_db.get(item.item_id).unwrap_or_default();
                (item_data.weight as u32) * (item.amount as u32)
            })
            .sum()
    }

    /// 执行交易
    pub fn execute(
        &self,
        player1: &Player,
        inv1: &mut Inventory,
        player2: &Player,
        inv2: &mut Inventory,
    ) -> Result<(), TradeError> {
        // 交换物品和Zeny
        // ...
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TradeError {
    PlayerNotFound(String),
    InvalidTradeState,
    Overweight(String),
    NotEnoughZeny(String),
    ZenyOverflow(String),
    InventoryFull(String),
    ItemNotFound(String),
}

/// 交易管理器
pub struct TradeManager {
    sessions: RwLock<HashMap<Uuid, TradeSession>>,
}

impl TradeManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// 发起交易请求
    pub fn request_trade(&self, player1_id: Uuid, player2_id: Uuid) -> Uuid {
        let session = TradeSession::new(player1_id, player2_id);
        let id = session.id;
        self.sessions.write().insert(id, session);
        id
    }

    /// 获取交易会话
    pub fn get_session(&self, id: Uuid) -> Option<TradeSession> {
        self.sessions.read().get(&id).cloned()
    }

    /// 结束交易
    pub fn end_trade(&self, id: Uuid) {
        self.sessions.write().remove(&id);
    }
}

impl Default for TradeManager {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 任务10: ItemHandler更新

**Files:**
- Modify: `src/game/item/handler.rs`

- [ ] **Step 1: 添加装备方法**

```rust
use super::equipment::{Equipment, EquipSlot};
use super::effect::{ItemEffect, EffectResult};

impl ItemHandler {
    /// 装备物品
    pub fn equip_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        slot_index: u8,
        equip_slot: EquipSlot
    ) -> EquipResult {
        let item = match inventory.slots().get(slot_index as usize) {
            Some(s) if !s.is_empty() => {
                self.db.get(s.item_id)
                    .ok_or(EquipError::InvalidItem)?
            }
            _ => return EquipResult::Failed(EquipError::InvalidSlot),
        };

        // 检查是否是装备
        if !item.is_equip() {
            return EquipResult::Failed(EquipError::NotEquipable);
        }

        // 检查槽位兼容性
        let valid_slots = EquipSlot::from_mask(item.equip_mask);
        if !valid_slots.contains(&equip_slot) {
            return EquipResult::Failed(EquipError::WrongSlot);
        }

        // 从背包移除
        inventory.remove_item(slot_index, 1);

        // 装备到玩家
        let mut equipment = player.equipment.write();
        let old_item = equipment.equip(
            equip_slot,
            InventorySlot {
                index: slot_index,
                item_id: item.id,
                amount: 1,
                identified: true,
                refine: 0,
                cards: [0; 4],
            }
        );

        // 如果有旧装备，返还到背包
        if let Some(old) = old_item {
            inventory.add_item(old.item_id, 1);
        }

        EquipResult::Success {
            slot: equip_slot,
            item_id: item.id,
        }
    }

    /// 卸下装备
    pub fn unequip_item(
        &self,
        player: &Player,
        inventory: &mut Inventory,
        equip_slot: EquipSlot
    ) -> UnequipResult {
        let mut equipment = player.equipment.write();
        let item = match equipment.unequip(equip_slot) {
            Some(i) => i,
            None => return UnequipResult::Failed(UnequipError::NoItemEquipped),
        };

        // 返还到背包
        if !inventory.add_item(item.item_id, item.amount) {
            // 背包满了，重新装备回去
            equipment.equip(equip_slot, item);
            return UnequipResult::Failed(UnequipError::InventoryFull);
        }

        UnequipResult::Success {
            slot: equip_slot,
            item_id: item.item_id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EquipResult {
    Success { slot: EquipSlot, item_id: u16 },
    Failed(EquipError),
}

#[derive(Debug, Clone, Copy)]
pub enum EquipError {
    InvalidSlot,
    InvalidItem,
    NotEquipable,
    WrongSlot,
    LevelTooLow,
    WrongJob,
}

#[derive(Debug, Clone)]
pub enum UnequipResult {
    Success { slot: EquipSlot, item_id: u16 },
    Failed(UnequipError),
}

#[derive(Debug, Clone, Copy)]
pub enum UnequipError {
    NoItemEquipped,
    InventoryFull,
}
```

---

## 任务11: 集成测试

**Files:**
- Modify: `src/game/item/inventory.rs` (测试)

- [ ] **Step 1: 添加重量测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn weight_calculation_is_correct() {
        let db = Arc::new(ItemDatabase::new());
        let inv = Inventory::new(10, db);
        // 重量计算应该基于物品数据库
        assert_eq!(inv.total_weight(), 0);
    }

    #[test]
    fn can_carry_weight_check() {
        // 测试重量检查逻辑
    }
}
```

---

## 数据库迁移

**Files:**
- Modify: `src/storage/schema.rs` (已存在zeny字段，无需修改)

数据库表`characters`已包含`zeny INTEGER DEFAULT 0`，无需额外迁移。

---

## 执行顺序

1. 任务1: Item价格字段拆分 (基础数据)
2. 任务2: 装备系统
3. 任务3: Player扩展
4. 任务5: ZenyManager (依赖Player)
5. 任务4: Inventory重量 (依赖Item)
6. 任务6: 商店购买/出售 (依赖ZenyManager和重量)
7. 任务7: ItemEffect框架
8. 任务8: YAML配置加载
9. 任务10: ItemHandler更新 (依赖装备系统)
10. 任务9: 交易系统 (依赖重量系统)
11. 任务11: 集成测试

---

**预计总工时:** 2-3小时

**注意事项:**
- 任务3、4、5可以并行进行（它们修改不同文件）
- 任务6依赖任务4和5
- 任务9必须最后实现，依赖所有其他系统
- 每个任务完成后需要编译验证
