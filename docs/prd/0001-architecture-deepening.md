# Deviruchi 架构深化与修复

## Problem Statement

Deviruchi 服务端存在多个架构问题，阻止了客户端接收到游戏状态更新，并导致关键游戏逻辑（战斗、经验分配）行为不一致。当前代码库约 50% 完成，但核心基础设施存在断裂。

### 核心问题

1. **ChannelBus 广播管道断裂** — `push_tx` 信道被创建但从未连接，事件发布到总线但客户端完全收不到
2. **MobAI 不可驱动** — MobAI 从未被 GameLoop 调用，状态机从未真正运行，Mob 不会巡逻/追击/返回
3. **伤害公式双轨** — MobAI 绕过 BattleFormula 直接计算伤害，和玩家攻击使用不同的公式
4. **非确定性随机数** — 随机函数硬编码 `SystemTime::now().subsec_nanos()`，无法注入测试

## Solution

### 阶段一：修复 ChannelBus 广播管道

将 ChannelBus 订阅者的 sender 路由到 `server.rs` 的 `handle_connection` 循环，使客户端能接收到所有游戏事件推送。

### 阶段二：驱动 MobAI 并统一伤害公式

在 GameLoop 中驱动 MobAI::update，让 Mob 真正执行状态机。同时让 MobAI 使用 BattleFormula，删除内联伤害计算。

### 阶段三：抽象随机数

将 `rand_*` 函数抽象为可注入的 trait，使 AI 和战斗逻辑可测试。

## User Stories

### 频道广播

1. 作为玩家，我进入地图后能看见同地图的其他玩家和 Mob，以便进行社交和战斗
2. 作为玩家，我移动后能实时看到其他玩家的位置变化，以便了解战场态势
3. 作为玩家，我攻击 Mob 后能看到伤害数字和 Mob 的血量变化，以便了解战斗结果
4. 作为玩家，我死亡后能看到系统提示和重生界面，以便继续游戏
5. 作为玩家，我的物品掉落时其他玩家能看到，以便进行拾取竞争

### Mob AI

6. 作为玩家，我在 Mob 视野范围内时，Mob 会主动追击我
7. 作为玩家，我逃离 Mob 视野范围后，Mob 会返回巡逻点
8. 作为玩家，Mob 被击杀后会掉落物品和经验，其他玩家能看到
9. 作为玩家，Mob 死亡后会触发重生计时器，一段时间后重新出现

### 战斗系统

10. 作为玩家，我攻击 Mob 时使用和 rAthena 一致的伤害公式
11. 作为玩家，Mob 攻击我时使用和 rAthena 一致的伤害公式
12. 作为玩家，我获得的经验值符合 rAthena 的等级惩罚规则
13. 作为玩家，我组队时经验值分配符合队伍设置（平均/等级差）

### 架构质量

14. 作为运维，我能在日志中追踪完整的事件链路
15. 作为开发者，我能对战斗公式编写单元测试验证正确性
16. 作为开发者，我能对 MobAI 状态机编写单元测试验证行为

## Implementation Decisions

### 模块修复清单

#### 1. ChannelBus 广播管道

**修改文件**: `src/network/server.rs`, `src/game/map/channel.rs`, `src/game/map/map_server.rs`

**改动**:
- `ChannelBus::subscribe` 返回的 `UnboundedSender` 需要被路由到 `handle_connection`
- 在 `MapServer` 创建订阅时，将 sender 保存到 `Session` 中
- `handle_connection` 的 `select` 循环需要同时处理 `push_rx` 和 `packet_rx`
- `Session` 需要一个字段存储 MapServer 的事件 sender

**新模块**: 无

**接口变更**:
- `Session` 新增字段 `map_event_tx: Option<UnboundedSender<GameEvent>>`
- `ChannelBus::subscribe` 签名不变，但返回值需要被 caller 保存

#### 2. MobAI 驱动

**修改文件**: `src/game/game_loop.rs`, `src/game/map/mob/ai.rs`

**改动**:
- GameLoop 新增 MobAI tick，在 100ms 间隔内调用所有在线 Mob 的 `MobAI::update`
- `MobSpawnManager` 提供 `get_active_mobs()` 方法供 GameLoop 遍历
- MobAI update 需要传入 `MobSpawnManager` 的引用以获取活跃 Mob

**新模块**: 无

#### 3. 统一伤害公式

**修改文件**: `src/game/map/mob/ai.rs`, `src/game/battle/handler.rs`

**改动**:
- `MobAI::update_attack` 改为调用 `BattleHandler::normal_attack`
- 删除 `MobAI::calculate_damage` 方法
- `MobAI` 需要持有 `BattleHandler` 的引用

#### 4. 随机数抽象

**修改文件**: 新建 `src/game/rand.rs`（或 `src/game/map/rng.rs`），修改 `src/game/map/mob/ai.rs`, `src/game/battle/handler.rs`, `src/game/battle/formula.rs`

**新模块**: `GameRng` trait

```rust
pub trait GameRng: Send + Sync {
    fn rand_range(&self, min: u32, max: u32) -> u32;
    fn rand_bool(&self, probability: f32) -> bool;
}
```

**默认实现**: 使用 `rand::thread_rng()` 的实现，供生产使用

**测试实现**: 使用确定性的 `SmallRng`（或自定义 mock），供测试使用

#### 5. MapState 职责拆分（可选，取决于复杂度）

**如需拆分**:
- `SpatialIndex` 新模块：从 MapState 提取空间查询逻辑
- `PlayerRegistry` 新模块：从 MapState 提取玩家注册逻辑
- MapState 保留为协调者，持有 `PlayerRegistry` 和 `SpatialIndex`

如当前阶段工作量过大，此项可延后。

### API 契约变更

- `GameLoop::start()` 不再只清理 TTL，还需要驱动 MobAI
- `MobAI::update` 需要接受 `&dyn GameRng` 参数
- `BattleHandler::new` 需要接受 `Arc<dyn GameRng>` 参数

### 错误模式

- ChannelBus 连接断开时，优雅降级：移除订阅者，继续处理其他玩家
- MobAI 更新出错时（随机数失败、状态机异常），记录日志并跳过本次更新

## Testing Decisions

### 测试策略

**外部行为测试**: 只测试最终效果，不测试实现细节

### 需要测试的模块

#### 1. BattleFormula

**已有测试**: `test_solo_exp_distribution`, `test_level_penalty_reduces_exp`

**补充测试**:
- 物理伤害计算：ATK - VIT/2 公式
- 魔法伤害：MATK * 技能倍率 - MDEF
- 命中率：HIT - FLEE + 命中率基础值
- 必杀率：LUK / 10
- 等级惩罚：5 种差值等级（≤10/≤15/≤20/≤25/>25）的惩罚系数

**测试模式**: 纯函数，使用 mock `GameRng` 返回固定值

#### 2. ExpDistributor

**补充测试**:
- `ExpShareMode::Equal` — 队伍成员平均分配
- `ExpShareMode::LevelBased` — 等级差惩罚分配
- 跨地图成员不分配经验
- 单人击杀：100% 经验

**测试模式**: 创建 mock `PartyManager`，使用确定性的 mock `GameRng`

#### 3. MobAI 状态机

**测试用例**:
- Idle 状态：视野内无玩家，保持 Idle
- Idle → Chase：玩家进入视野，转换到 Chase
- Chase → Attack：进入攻击范围，转换到 Attack
- Chase → Return：玩家离开视野，转换到 Return
- Attack → Dead：Mob HP 归零，触发死亡处理

**测试模式**: 使用 mock `MapState`、`mock `GameRng`（返回确定值）、mock `ChannelBus`

#### 4. ChannelBus 广播

**测试用例**:
- 视野范围内玩家收到事件
- 视野范围外玩家不收到事件
- 队伍频道绕过视野限制

**测试模式**: 使用 mock `Session`，验证 sender 收到预期包

### 优先测试顺序

1. BattleFormula 纯逻辑（最简单，最有价值）
2. ExpDistributor 队伍分配（业务核心）
3. MobAI 状态转换（核心游戏逻辑）
4. ChannelBus 视野过滤（基础设施）

## Out of Scope

- WebSocket/Modern Protocol 实现（规划中）
- 地图 .gat 文件加载（规划中）
- 碰撞检测实现（is_walkable 目前永远返回 true）
- NPC 对话引擎
- HP/SP 回复逻辑（GameLoop 中有 TODO）
- 数据库迁移系统
- 交易实际物品转移
- 自动缩放测试（性能测试）

## Further Notes

### 阶段交付

| 阶段 | 内容 | 阻塞 |
|------|------|------|
| 1 | ChannelBus 管道修复 | 客户端完全无法使用 |
| 2 | MobAI 驱动 | Mob 不会动 |
| 3 | 伤害公式统一 | 战斗数值不一致 |
| 4 | 随机数抽象 + 测试 | AI 不可测试 |

### 依赖关系

- 阶段 2 依赖阶段 1（ChannelBus 需要正常工作才能发布 Mob 事件）
- 阶段 4 可与阶段 2、3 并行进行
- 阶段 3 依赖阶段 4（抽象随机数后更容易测试 BattleHandler）
