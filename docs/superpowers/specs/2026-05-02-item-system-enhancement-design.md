# 物品系统完善设计文档

> 完善Deviruchi物品系统：Zeny货币、商店购买/出售、装备系统、YAML配置、重量系统

---

## 目标

实现完整的物品系统功能，支持经济循环、装备穿戴、重量限制和配置加载。

---

## 1. Zeny货币系统

### 1.1 Player结构体扩展

```rust
pub struct Player {
    // ... 现有字段 ...
    pub zeny: RwLock<u32>,  // 金币数量 (0 ~ 999,999,999)
}
```

### 1.2 数据库变更

```sql
ALTER TABLE characters ADD COLUMN zeny INTEGER DEFAULT 0;
```

### 1.3 ZenyManager

```rust
pub struct ZenyManager;

impl ZenyManager {
    /// 增加Zeny，返回实际增加数量
    pub fn add(player: &Player, amount: u32) -> u32;
    
    /// 扣除Zeny，返回是否成功
    pub fn sub(player: &Player, amount: u32) -> bool;
    
    /// 检查是否足够
    pub fn can_spend(player: &Player, amount: u32) -> bool;
}
```

---

## 2. 商店系统

### 2.1 数据结构

```rust
pub struct ShopItem {
    pub item_id: u16,
    pub buy_price: u32,   // NPC卖给玩家的价格
    pub sell_price: u32,  // NPC收购价格 (通常是buy_price/2)
}

pub struct Item {
    // ... 现有字段 ...
    pub buy_price: u32,
    pub sell_price: u32,
}
```

### 2.2 购买流程

1. 验证NPC存在且有商店功能
2. 验证物品在商店列表中
3. 计算总价 = buy_price × amount
4. 检查玩家Zeny是否足够
5. 检查玩家背包空间
6. 扣除Zeny，添加物品到背包
7. 发送购买成功包

### 2.3 出售流程

1. 验证玩家背包中有该物品
2. 计算获得Zeny = sell_price × amount
3. 检查是否会超过Zeny上限
4. 移除背包物品
5. 增加Zeny
6. 发送出售成功包

---

## 3. 装备系统

### 3.1 装备槽位

```rust
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
    pub fn from_mask(mask: u32) -> Vec<EquipSlot>;
    pub fn to_mask(&self) -> u32;
}
```

### 3.2 装备管理

```rust
pub struct Equipment {
    slots: HashMap<EquipSlot, InventorySlot>,
}

impl Equipment {
    pub fn new() -> Self;
    pub fn equip(&mut self, slot: EquipSlot, item: InventorySlot) -> Option<InventorySlot>;
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<InventorySlot>;
    pub fn get(&self, slot: EquipSlot) -> Option<&InventorySlot>;
}
```

### 3.3 装备属性计算

```rust
pub struct EquipStats {
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub str: i16,
    pub agi: i16,
    pub vit: i16,
    pub int: i16,
    pub dex: i16,
    pub luk: i16,
}

impl Player {
    /// 重算装备属性加成
    pub fn recalc_equip_stats(&self) -> EquipStats;
    
    /// 获取最终属性（基础+装备）
    pub fn get_total_stats(&self) -> TotalStats;
}
```

### 3.4 装备限制检查

```rust
pub struct Item {
    // ... 现有字段 ...
    pub required_level: u16,
    pub required_job: Vec<JobType>,
    pub required_gender: Option<Gender>,
}

pub fn can_equip(player: &Player, item: &Item, slot: EquipSlot) -> bool {
    // 检查职业
    // 检查等级
    // 检查性别
    // 检查装备槽位兼容性
}
```

---

## 4. 重量系统

### 4.1 数据扩展

```rust
pub struct Player {
    // ... 现有字段 ...
    pub current_weight: RwLock<u32>,  // 当前负重 (0.1单位)
    pub max_weight: RwLock<u32>,      // 最大负重
}

pub struct Inventory {
    // ... 现有字段 ...
    total_weight: u32,
}
```

### 4.2 重量计算

```rust
impl Inventory {
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
    
    /// 检查能否添加物品（重量限制）
    pub fn can_carry_weight(&self, item_id: u16, amount: u16) -> bool;
    
    /// 获取当前重量
    pub fn total_weight(&self) -> u32;
}

impl Player {
    /// 计算最大负重
    pub fn calc_max_weight(&self) -> u32 {
        // 基础20000 + STR*200 (单位: 0.1)
        20000 + (*self.str.read() as u32) * 200
    }
    
    /// 更新重量（同步Player和Inventory）
    pub fn update_weight(&self, inventory: &Inventory);
}
```

### 4.3 超重状态

```rust
pub fn is_overweight(player: &Player) -> bool {
    let current = *player.current_weight.read();
    let max = *player.max_weight.read();
    current > max * 50 / 100  // 超过50%为超重
}

pub fn is_overweight_90(player: &Player) -> bool {
    let current = *player.current_weight.read();
    let max = *player.max_weight.read();
    current > max * 90 / 100  // 超过90%为严重超重
}
```

### 4.4 交易重量检查

```rust
/// 交易前检查双方重量容量
fn can_trade_items(
    player1: &Player,
    inv1: &Inventory,
    items1: &[TradeItem],
    player2: &Player,
    inv2: &Inventory,
    items2: &[TradeItem],
) -> Result<(), TradeError> {
    // 计算双方将要增加的重量
    let weight_gain_1 = calc_items_weight(items2, item_db);
    let weight_gain_2 = calc_items_weight(items1, item_db);
    
    // 检查是否超重
    if inv1.total_weight() + weight_gain_1 > *player1.max_weight.read() {
        return Err(TradeError::Overweight(1));
    }
    if inv2.total_weight() + weight_gain_2 > *player2.max_weight.read() {
        return Err(TradeError::Overweight(2));
    }
    
    Ok(())
}
```

---

## 5. YAML配置加载

### 5.1 文件格式

```yaml
# db/item_db.yml
- Id: 501
  Name: Red Potion
  Type: Healing
  Buy: 50
  Sell: 25
  Weight: 7
  HpHeal: 120
  SpHeal: 0
  
- Id: 502
  Name: Yellow Potion
  Type: Healing
  Buy: 40
  Sell: 20
  Weight: 5
  HpHeal: 60
  SpHeal: 0
  
- Id: 1201
  Name: Dagger
  Type: Weapon
  Buy: 1000
  Sell: 500
  Weight: 50
  Atk: 10
  EquipMask: 0x0001  # 右手
  
- Id: 1501
  Name: Clothes
  Type: Armor
  Buy: 500
  Sell: 250
  Weight: 40
  Defense: 2
  EquipMask: 0x0010  # 身体
  Slots: 1            # 卡片槽位数
```

### 5.2 加载器

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct ItemYaml {
    #[serde(rename = "Id")]
    id: u16,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Type")]
    type_: String,
    #[serde(rename = "Buy")]
    buy_price: u32,
    #[serde(rename = "Sell")]
    sell_price: u32,
    #[serde(rename = "Weight")]
    weight: u16,
    // ... 其他字段 ...
}

pub struct ItemDbLoader;

impl ItemDbLoader {
    pub fn load_from_yaml(path: &str) -> Result<ItemDatabase, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let yaml_items: Vec<ItemYaml> = serde_yaml::from_str(&content)?;
        
        let mut db = ItemDatabase::new();
        for y in yaml_items {
            db.insert(y.id, y.into());
        }
        
        Ok(db)
    }
}
```

---

## 6. 扩展物品效果框架

### 6.1 效果类型

```rust
pub enum ItemEffect {
    HealHp(u16),
    HealSp(u16),
    Teleport { map: String, x: u16, y: u16 },
    Buff { stat: StatType, value: i16, duration_secs: u32 },
    LearnSkill(u16),
    OpenStorage,
    // 更多效果...
}

impl ItemEffect {
    /// 执行效果
    pub fn apply(&self, player: &Player) -> EffectResult;
}
```

### 6.2 物品脚本解析（简化版）

```rust
/// 解析脚本字符串生成效果列表
pub fn parse_item_script(script: &str) -> Vec<ItemEffect> {
    // 示例脚本:
    // "item_heal 120, 0;"
    // "warp prontera, 155, 180;"
    
    let mut effects = Vec::new();
    for line in script.split(';') {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() { continue; }
        
        match parts[0] {
            "item_heal" => {
                let hp = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                let sp = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                effects.push(ItemEffect::HealHp(hp));
                effects.push(ItemEffect::HealSp(sp));
            }
            "warp" => {
                // 解析传送...
            }
            // 更多指令...
            _ => {}
        }
    }
    effects
}
```

---

## 7. 数据结构变更汇总

### 7.1 Item结构体完整版

```rust
pub struct Item {
    pub id: u16,
    pub name: &'static str,
    pub type_: ItemType,
    pub buy_price: u32,
    pub sell_price: u32,
    pub weight: u16,
    pub flags: u32,
    
    // 使用效果
    pub hp_restore: u16,
    pub sp_restore: u16,
    
    // 装备属性
    pub equip_mask: u32,
    pub required_level: u16,
    pub required_job: Vec<JobType>,
    pub slots: u8,        // 卡片槽位数
    
    // 战斗属性
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    
    // 属性加成
    pub str_bonus: i16,
    pub agi_bonus: i16,
    pub vit_bonus: i16,
    pub int_bonus: i16,
    pub dex_bonus: i16,
    pub luk_bonus: i16,
    
    // 扩展效果脚本
    pub script: Option<String>,
}
```

### 7.2 Player结构体扩展

```rust
pub struct Player {
    // 基础信息
    pub id: Uuid,
    pub char_id: u32,
    pub name: String,
    
    // 位置和状态
    pub pos_x: RwLock<u16>,
    pub pos_y: RwLock<u16>,
    pub map_name: String,
    
    // 属性
    pub hp: RwLock<u32>,
    pub max_hp: RwLock<u32>,
    pub sp: RwLock<u32>,
    pub max_sp: RwLock<u32>,
    
    // 等级和基础属性
    pub base_level: RwLock<u16>,
    pub job_level: RwLock<u16>,
    pub str: RwLock<u16>,
    pub agi: RwLock<u16>,
    pub vit: RwLock<u16>,
    pub int: RwLock<u16>,
    pub dex: RwLock<u16>,
    pub luk: RwLock<u16>,
    pub walk_speed: RwLock<u16>,
    
    // === 新增字段 ===
    pub zeny: RwLock<u32>,
    pub current_weight: RwLock<u32>,
    pub max_weight: RwLock<u32>,
    pub equipment: RwLock<Equipment>,
}
```

---

## 8. API接口设计

### 8.1 商店协议

```rust
// 客户端 → 服务器
enum ShopPacket {
    BuyItem { npc_id: u32, item_id: u16, amount: u8 },
    SellItem { npc_id: u32, inventory_index: u8, amount: u8 },
    CloseShop,
}

// 服务器 → 客户端
enum ShopResponse {
    BuySuccess { item_id: u16, amount: u8, remaining_zeny: u32 },
    BuyFailed { reason: ShopError },
    SellSuccess { item_id: u16, amount: u8, gained_zeny: u32 },
    SellFailed { reason: ShopError },
}
```

### 8.2 装备协议

```rust
// 客户端 → 服务器
enum EquipPacket {
    EquipItem { inventory_index: u8, slot: EquipSlot },
    UnequipItem { slot: EquipSlot },
    ViewEquipment { player_id: Uuid },
}

// 服务器 → 客户端
enum EquipResponse {
    EquipSuccess { slot: EquipSlot, item_id: u16 },
    EquipFailed { reason: EquipError },
    UnequipSuccess { slot: EquipSlot },
    UnequipFailed { reason: EquipError },
}
```

---

## 9. 测试策略

### 9.1 单元测试

- ZenyManager: 增加、扣除、边界检查
- Inventory: 重量计算、重量限制
- Equipment: 装备/卸下、属性计算
- ItemDbLoader: YAML解析

### 9.2 集成测试

- 完整购买流程
- 完整出售流程
- 装备穿戴流程
- 交易重量检查

---

## 10. 数据库迁移计划

```sql
-- 角色表添加Zeny字段
ALTER TABLE characters ADD COLUMN zeny INTEGER DEFAULT 0;

-- 角色表添加重量字段
ALTER TABLE characters ADD COLUMN current_weight INTEGER DEFAULT 0;
```

---

**设计确认：** 包含Zeny货币、商店购买/出售、装备系统、重量系统、YAML配置加载。交易系统考虑双方重量限制。