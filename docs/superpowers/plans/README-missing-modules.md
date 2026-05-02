# Deviruchi 缺失模块与完善计划总览

> 基于与 rathena 原始代码库的对比分析，总结当前缺失和需要完善的系统。

---

## 当前已实现功能

| 模块 | 状态 | 备注 |
|------|------|------|
| 登录系统 | ✅ 完成 | Token 认证，Session 管理 |
| 角色系统 | ✅ 完成 | 角色创建、选择 |
| 地图系统 | ✅ 完成 | MapServer, ChannelBus 视野同步 |
| 战斗系统 | ✅ 完成 | BattleHandler, BattleFormula |
| 技能系统 | ✅ 完成 | SkillHandler, SkillDatabase |
| 怪物系统 | ✅ 完成 | MobAI, MobSpawnManager |
| NPC系统 | ✅ 完成 | NpcHandler, NpcDatabase |
| 物品系统 | ⚠️ 基础 | Inventory, ItemDatabase |
| 组队系统 | ✅ 完成 | PartyManager |
| 聊天系统 | ✅ 完成 | ChannelBus 支持 Map/Party 聊天 |
| 游戏循环 | ✅ 完成 | GameLoop tick 驱动 |

---

## 缺失模块优先级列表

### 🔴 高优先级（影响核心玩法）

| 模块 | 对应 rathena 文件 | 影响范围 | 计划文件 |
|------|------------------|----------|----------|
| **仓库系统** | storage.cpp | 物品存储、角色进度 | [2026-05-02-storage-system-plan.md](./2026-05-02-storage-system-plan.md) |
| **交易系统** | trade.cpp | 玩家经济、社交 | [2026-05-02-trade-system-plan.md](./2026-05-02-trade-system-plan.md) |

### 🟡 中优先级（重要社交功能）

| 模块 | 对应 rathena 文件 | 影响范围 | 计划文件 |
|------|------------------|----------|----------|
| **公会系统** | guild.cpp, guild.hpp | 社交、公会战 | [2026-05-02-guild-system-plan.md](./2026-05-02-guild-system-plan.md) |
| **任务系统** | quest.cpp, quest.hpp | PvE内容、角色成长 | 待创建 |
| **邮件系统** | mail.cpp | 异步交易、附件 | 待创建 |

### 🟢 低优先级（增强体验）

| 模块 | 对应 rathena 文件 | 影响范围 | 计划文件 |
|------|------------------|----------|----------|
| **摆摊系统** | vending.cpp | 离线经济 | 待创建 |
| **拍卖系统** | auction.cpp | 高级经济 | 待创建 |
| **宠物系统** | pet.cpp | 宠物战斗 | 待创建 |
| **佣兵系统** | mercenary.cpp | 佣兵战斗 | 待创建 |

---

## 需要完善的现有功能

### 1. Mob 掉落表 (Mob Drops)

**现状：**
- `Mob` 结构体缺少掉落表配置
- `DropManager` 存在但无掉落触发逻辑

**需要实现：**
```rust
// src/game/mob/data.rs
pub struct MobDrop {
    pub item_id: u16,
    pub min_amount: u16,
    pub max_amount: u16,
    pub chance: f32,  // 0.0 - 1.0
}

pub struct MobTemplate {
    // ... 现有字段 ...
    pub drops: Vec<MobDrop>,
    pub mvp_drops: Vec<MobDrop>,  // MVP 专属掉落
}
```

**触发点：** `Mob::die()` 中调用 `DropManager::add_from_mob(mob_id, x, y, map)`

### 2. NPC 商店实际逻辑

**现状：**
- `NpcHandler` 有商店框架但无实际购买逻辑
- `ItemDatabase` 存在但无价格信息

**需要实现：**
```rust
// src/game/item/data.rs
pub struct Item {
    // ... 现有字段 ...
    pub buy_price: u32,
    pub sell_price: u32,
}

// src/game/npc/handler.rs
fn handle_buy_item(&self, shop_id: u32, item_id: u16, amount: u16, player: &mut Player) -> bool {
    // 检查价格、背包空间、扣款、添加物品
}
```

### 3. 传送系统

**现状：**
- 地图切换逻辑未实现
- Save Point（存储点）系统缺失

**需要实现：**
```rust
// src/game/map/warp.rs
pub struct WarpPoint {
    pub from_map: String,
    pub from_x: u16,
    pub from_y: u16,
    pub to_map: String,
    pub to_x: u16,
    pub to_y: u16,
}

// src/game/map/player.rs
pub struct Player {
    // ... 现有字段 ...
    pub save_map: String,
    pub save_x: u16,
    pub save_y: u16,
}
```

### 4. 经验值分配系统

**现状：**
- `Party` 有经验分享模式配置
- 无实际经验分配逻辑

**需要实现：**
```rust
// src/game/battle/exp.rs
pub struct ExpDistributor;

impl ExpDistributor {
    pub fn distribute_mob_exp(
        mob: &Mob,
        killer: &Player,
        party: Option<&Party>,
        nearby_players: &[Player],
    ) -> HashMap<Uuid, u32> {
        // 基础经验计算
        // 等级差惩罚
        // 组队分配（Equal/LevelBased）
        // 返回每个玩家获得的经验值
    }
}
```

### 5. 死亡与重生系统

**现状：**
- `GameEvent::PlayerDeath` 存在
- 无重生逻辑

**需要实现：**
```rust
// src/game/map/player.rs
impl Player {
    pub fn die(&mut self) {
        self.state = PlayerState::Dead;
        // 发布 PlayerDeath 事件
        // 如果是 PvP，处理死亡惩罚
    }

    pub fn respawn(&mut self) {
        // 传送回存储点
        self.map = self.save_map.clone();
        self.x = self.save_x;
        self.y = self.save_y;
        // 恢复部分 HP/SP
        self.hp = self.max_hp / 2;
        self.sp = self.max_sp / 2;
        self.state = PlayerState::Idle;
    }
}
```

---

## 配置文件缺失

### 1. 怪物配置 (mob_db.yml)
```yaml
# db/mob_db.yml
- Id: 1001
  Name: Poring
  Hp: 50
  Exp: 20
  JExp: 10
  Drops:
    - Item: 512      # Jellopy
      Rate: 7000     # 70%
    - Item: 909      # Jellopy (另一组)
      Rate: 1000     # 10%
```

### 2. 物品配置 (item_db.yml)
```yaml
# db/item_db.yml
- Id: 501
  Name: Red Potion
  Type: Healing
  Buy: 50
  Sell: 25
  Weight: 100
```

### 3. 技能配置 (skill_db.yml)
```yaml
# db/skill_db.yml
- Id: 1
  Name: NV_BASIC
  MaxLevel: 9
  Type: Passive
```

---

## 执行建议

### 立即执行（高优先级）
1. **仓库系统** - 玩家物品存储是核心功能
2. **交易系统** - 玩家间经济循环必需

### 下一阶段（中优先级）
3. **公会系统** - 社交核心
4. **Mob 掉落表** - 完善 PvE 奖励
5. **NPC 商店** - 经济系统闭环

### 后续完善（低优先级）
6. **任务系统** - PvE 内容扩展
7. **邮件系统** - 异步交互
8. **摆摊系统** - 离线经济

---

## 实施方式

对于每个计划，你可以选择：

1. **Subagent-Driven** - 推荐
   - 每个 Task 分配独立子代理
   - 两阶段审查（规格合规 + 代码质量）
   - 快速迭代

2. **Inline Execution**
   - 当前会话批量执行
   - 适合小改动

---

## 文件索引

| 计划文件 | 模块 | 任务数 | 预估工时 |
|----------|------|--------|----------|
| [2026-05-02-storage-system-plan.md](./2026-05-02-storage-system-plan.md) | 仓库系统 | 9 tasks | 2-3 小时 |
| [2026-05-02-trade-system-plan.md](./2026-05-02-trade-system-plan.md) | 交易系统 | 7 tasks | 2-3 小时 |
| [2026-05-02-guild-system-plan.md](./2026-05-02-guild-system-plan.md) | 公会系统 | 7 tasks | 3-4 小时 |

---

**你想先实施哪个计划？建议从高优先级的仓库系统开始。**
