# 核心战斗闭环设计

## 目标

打通玩家↔怪物战斗闭环：GameLoop tick 驱动 MobAI，packet 0x89 调用 BattleHandler，参考 rAthena 实现 dmglog 血条同步和怪物死亡/重生。

## 架构总览

```
Core::run()
  ├── MapDatabase::new() ────→ MapDatabase (Arc)
  ├── MobSpawnManager::new() ────→ MobSpawnManager (Arc)
  ├── init_default_spawns() ────→ 注册刷怪点
  ├── MobAI::new(spawn_manager, channel_bus, drop_manager, party_manager, map_database)
  │     └── MobAI (Arc)
  └── GameLoop::new(map_state, drop_manager, token_store, mob_ai, spawn_manager)
        └── GameLoop (Arc) → tokio::spawn(tick loop)

MapServer (独立创建)
  └── spawn_manager: Arc<MobSpawnManager>
  └── battle_handler: BattleHandler
  └── handle_attack() → BattleHandler + dmglog + 广播

每 100ms tick:
  1. drop_manager.cleanup_expired()
  2. token_store.cleanup_expired()
  3. spawn_manager.get_active_maps() → 每张地图的 mobs → mob_ai.update(mob, map_state)
```

## 数据结构

### Mob 新增字段

```rust
// 伤害记录（参考 rAthena dmglog）
pub dmglog: RwLock<HashMap<Uuid, u32>>, // player_id → total damage dealt
```

### GameEvent 新增

```rust
MobDamage {
    mob_id: Uuid,
    attacker_id: Uuid,
    damage: u32,
    is_crit: bool,
},
MobHpUpdate {
    mob_id: Uuid,
    hp: u32,
    max_hp: u32,
},
```

### 网络协议包

| 包 ID | 名称 | 用途 |
|-------|------|------|
| `0x008D` | ZC_NOTIFY_ACT | 广播伤害动画给周围玩家 |
| `0x0977` | ZC_HP_INFO | 更新怪物血条（仅 dmglog 玩家可见）|

### ZCNotifyAct 结构

```rust
pub struct ZCNotifyAct {
    pub src_id: u32,       // 攻击者 GID
    pub dst_id: u32,       // 目标 GID (mob GID)
    pub damage: u32,        // 伤害值
    pub action: u8,         // 0=普通, 5=暴击
    pub left_damage: u32,   // 分身用，当前填 0
}
```

### ZCMonsterHpBar 结构

```rust
pub struct ZCMonsterHpBar {
    pub mob_id: u32,
    pub hp: u32,
    pub max_hp: u32,
}
```

## 玩家攻击怪物流程

```
packet 0x0089 (CZ_REQUEST_ACT)
  → CZRequestAction { target_id: u32, action_type: u8 }
  → MapServer::handle_attack()
    → lookup MobSpawnManager::get_mob(target_id)
    → validate same map
    → BattleHandler::normal_attack(player, mob)
      → hit check → damage formula → defender.take_damage()
    → mob.add_damage(player_id, damage)
    → if Hit:
        → 广播 ZCNotifyAct (0x8D) via channel_bus (vision filter)
        → if killed:
            → 设置 drops_processed = true
            → 发布 MobDeath 事件
            → process_drops() + ExpDistributor
        → else:
            → 广播 ZCMonsterHpBar (0x977) via channel_bus
              (仅 dmglog 玩家收到)
```

## 怪物 AI 流程（GameLoop tick）

```
GameLoop::tick()
  → 每张地图的活跃怪物:
      MobAI::update(mob, map_state)

MobAI::update() 状态机:
  Idle → 检测视野内玩家 → Chase
  Chase → A* 寻路 → Attack (in range) / Return (out of range)
  Attack → 造成伤害 → 广播 ZCNotifyAct
  Dead → 首次: drops_processed 检查
           → if !processed: 处理掉落/经验
           → 检查重生计时器 → respawn()
```

## 怪物重生

- 怪物死亡时设置 `death_time = Instant::now()`
- `MobAI::update_dead` 检查: `now - death_time >= respawn_time`
- 到期调用 `Mob.respawn()`: 重置 HP/位置/AI状态，**清空 dmglog 和 drops_processed**
- 重生后在地图上重新可见

## 死亡处理防重

`MobAI::update_dead` 中使用 `drops_processed` 标志：
- 玩家攻击击杀时，`MobDeath` 事件已发布，掉落和经验已分配
- GameLoop tick 进入 `update_dead` 时，检测 `drops_processed = true`，跳过掉落处理
- 只检查重生计时器

## 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 血条包发送对象 | 仅 dmglog 玩家 | 与 rAthena 一致 |
| 伤害广播 | 0x8D 广播给所有周围玩家 | rAthena 行为 |
| 重生检查 | GameLoop tick 中 MobAI::update_dead | 与死亡逻辑合一 |
| 死亡首次处理 | 玩家攻击击杀时直接处理 | 避免 tick 延迟 |
| A* 寻路 | 继承 MobPathManager + Pathfinder | 已实现 |

## 文件变更

| 文件 | 变更 |
|------|------|
| `src/game/mob/data.rs` | +dmglog 字段，+add_damage 方法 |
| `src/game/map/channel.rs` | +MobDamage, MobHpUpdate GameEvent |
| `src/protocol/map_packets.rs` | +ZCNotifyAct, ZCMonsterHpBar |
| `src/game/map/data.rs` | 确保 MapDatabase::new() 初始化地图 |
| `src/game/game_loop.rs` | +mob_ai, spawn_manager, tick 更新 mobs |
| `src/game/mod.rs` | 导出 MapDatabase |
| `src/core/mod.rs` | 创建 MapDatabase, MobSpawnManager, MobAI, GameLoop |
| `src/game/map/map_server.rs` | +spawn_manager, battle_handler, 完整 handle_attack |
| `src/game/mob/spawn.rs` | +get_mob, +get_active_maps |
| `src/game/mob/ai.rs` | +drops_processed 防重检查 |
