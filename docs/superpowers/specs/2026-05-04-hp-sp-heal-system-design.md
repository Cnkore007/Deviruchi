# HP/SP 自然回复系统设计

## 概述

实现玩家自然 HP/SP 回复功能，包括站立回复和坐下加速回复。

## 设计目标

1. 每 6 秒（可配置）触发一次自然回复
2. VIT 属性加成 HP 回复量
3. INT 属性加成 SP 回复量
4. 坐下状态提供额外回复加成
5. 仅在 HP/SP 未满时触发
6. 死亡状态不触发回复

## 回复公式

参考 rAthena 公式：

```
base_hp_heal = 1 + (VIT / 2) + (max_hp / 200)
base_sp_heal = 1 + (INT / 2) + (max_sp / 100)

hp_rate = if sitting { sit_heal_hp_rate } else { natural_heal_hp_rate }  // 3% 或 10%
sp_rate = if sitting { sit_heal_sp_rate } else { natural_heal_sp_rate }  // 3% 或 10%

hp_heal = base_hp_heal + (max_hp * hp_rate / 100)
sp_heal = base_sp_heal + (max_sp * sp_rate / 100)

最终回复量受 threshold 限制（低于 threshold % 才回复）
```

## 组件设计

### 1. Player 扩展

新增字段：
```rust
pub is_sitting: RwLock<bool>,  // 坐下状态
```

### 2. HealService

```rust
pub struct HealService {
    config: Arc<Config>,
}

impl HealService {
    pub fn new(config: Arc<Config>) -> Self;
    pub fn start(&self, map_state: Arc<MapState>);
    fn process_heal(&self, player: &Player) -> (u32, u32);  // 返回 (hp_heal, sp_heal)
    fn calculate_hp_heal(&self, player: &Player, is_sitting: bool) -> u32;
    fn calculate_sp_heal(&self, player: &Player, is_sitting: bool) -> u32;
}
```

### 3. 事件类型

```rust
pub enum GameEvent {
    // ... 现有事件
    PlayerSit { player_id: Uuid },
    PlayerStand { player_id: Uuid },
    PlayerHeal { player_id: Uuid, hp: u32, sp: u32 },
}
```

## 文件结构

```
src/game/
  └── heal/
      ├── mod.rs          # 模块导出
      └── service.rs      # HealService 实现
```

## 实现步骤

### Step 1: 添加 Player 坐下状态字段

修改 `src/game/map/player.rs`：
- 添加 `is_sitting: RwLock<bool>` 字段
- 添加 `sit()` 和 `stand()` 方法

### Step 2: 创建 HealService

创建 `src/game/heal/service.rs`：
- 实现 `HealService::new()`
- 实现 `calculate_hp_heal()` 和 `calculate_sp_heal()`
- 实现 `process_heal()` 遍历所有玩家并应用回复
- 实现 `start()` 启动定时器

### Step 3: 集成到 Core

修改 `src/core/mod.rs`：
- 创建 HealService 实例
- 在服务器启动时调用 `heal_service.start(map_state)`

### Step 4: 添加坐下/站起命令处理

在 PacketHandler 或新增 handler 中：
- 处理玩家坐下/站起请求
- 广播 PlayerSit/PlayerStand 事件

### Step 5: 测试

- 启动服务器
- 验证自然回复触发
- 验证坐下回复加成
- 验证 VIT/INT 加成

## 依赖

- `src/core/config.rs` - BattleConfig
- `src/core/timer.rs` - 定时器系统
- `src/game/map/map_state.rs` - 获取所有在线玩家
