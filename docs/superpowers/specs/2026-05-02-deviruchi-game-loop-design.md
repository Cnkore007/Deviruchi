# 游戏循环整合设计规格

## 目标

将现有孤立的子系统（Login、Char、Map、Battle、Skill、Item、NPC、Mob）串联为完整可玩的游戏循环，支持多人在线、视野同步、组队和聊天。

## 架构

MapServer 自治 + ChannelBus 事件总线模式。MapServer 处理数据包并调用子系统，ChannelBus 负责跨玩家事件广播（视野同步、组队、聊天）。子系统之间不直接依赖，通过事件解耦。

## 技术栈

- Rust + Tokio async runtime
- parking_lot::RwLock 线程安全状态
- tokio::sync::mpsc 事件发送通道
- uuid::Uuid 实体标识
- SQLite 持久化（复用现有 Database）

---

## 1. Token 认证与 Char→Map 过渡

### TokenStore

```rust
pub struct TokenStore {
    tokens: RwLock<HashMap<String, TokenEntry>>,
}

pub struct TokenEntry {
    pub account_id: u32,
    pub char_id: u32,
    pub created_at: Instant,
}
```

- 内存 HashMap 存储，一次性 token
- Token 生成：16 字节随机数转 hex 字符串
- 有效期 30 秒，过期自动清理

### 完整流程

1. **Login Server (0x0064)** — 验证账号密码，设置 `session.account_id`，返回登录成功
2. **Char Server (0x0065 选角)** — 验证角色归属，生成 token 存入 TokenStore，返回 `HCNotifyZoneServer` 包含 `{map_ip, map_port, token}`
3. **客户端** — 断开 Char 连接，连接 Map Server，发送 `CZEnter(0x007C)` 携带 `{account_id, char_id, token}`
4. **Map Server** — 从 TokenStore 验证 token，匹配 account_id + char_id，验证后删除 token，从 DB 加载 Character 创建 Player，加入 MapState，返回 `ZCAcceptEnter`

### 隔离保证

Login/Char 挂掉不影响 Map — 已在线玩家不受影响，TokenStore 是 Map Server 本地内存。

---

## 2. Session 扩展与 MapServer

### Session 扩展

```rust
pub enum SessionStage {
    Login,
    Char,
    Map,
}

pub struct Session {
    pub id: Uuid,
    pub account_id: Option<u32>,
    pub char_id: Option<u32>,
    pub authenticated: bool,
    pub version: u32,
    pub client_type: u8,
    pub stage: SessionStage,
    pub player_id: Option<Uuid>,
}
```

- `stage` 在每次成功认证后推进：Login→Char→Map
- `player_id` 仅在 stage=Map 时有值，用于从 MapState 查找 Player

### MapServer

```rust
pub struct MapServer {
    db: Arc<Database>,
    token_store: Arc<TokenStore>,
    map_state: Arc<MapState>,
    channel_bus: Arc<ChannelBus>,
    party_manager: Arc<PartyManager>,
    battle_handler: Arc<BattleHandler>,
    skill_handler: Arc<SkillHandler>,
    item_handler: Arc<ItemHandler>,
    npc_handler: Arc<NpcHandler>,
    mob_spawn: Arc<MobSpawnManager>,
    config: Arc<GameConfig>,
}
```

### PacketHandler 扩展

```rust
pub struct PacketHandler {
    login_server: Arc<LoginServer>,
    char_server: Arc<CharServer>,
    map_server: Arc<MapServer>,
}
```

路由逻辑：按 `session.stage` 决定路由目标，不再只看 packet_id。

---

## 3. ChannelBus 事件总线与视野同步

### GameEvent

```rust
pub enum GameEvent {
    PlayerEnter { player_id: Uuid, name: String, x: u16, y: u16, job: u16, level: u16, hp: u32, max_hp: u32 },
    PlayerLeave { player_id: Uuid },
    PlayerMove { player_id: Uuid, from_x: u16, from_y: u16, to_x: u16, to_y: u16 },
    PlayerAttack { attacker_id: Uuid, target_id: Uuid, damage: u32, is_crit: bool, killed: bool },
    PlayerUseSkill { caster_id: Uuid, skill_id: u32, target_id: Option<Uuid>, x: u16, y: u16 },
    PlayerChat { player_id: Uuid, message: String, chat_type: ChatType },
    PlayerDeath { player_id: Uuid },
    PlayerRevive { player_id: Uuid, x: u16, y: u16 },
    MobSpawn { mob_id: Uuid, mob_type: u32, x: u16, y: u16 },
    MobMove { mob_id: Uuid, to_x: u16, to_y: u16 },
    MobDeath { mob_id: Uuid, killer_id: Uuid },
    ItemDrop { item_id: u32, x: u16, y: u16, amount: u16 },
    ItemPickup { player_id: Uuid, item_id: u32, amount: u16 },
}
```

```rust
pub enum ChatType {
    Map,
    Party,
}
```

### ChannelBus

```rust
pub struct ChannelBus {
    channels: RwLock<HashMap<String, Channel>>,
}

pub struct Channel {
    name: String,
    subscribers: HashMap<Uuid, Subscriber>,
}

pub struct Subscriber {
    player_id: Uuid,
    sender: mpsc::UnboundedSender<Vec<u8>>,
    pos_x: u16,
    pos_y: u16,
}
```

### 视野同步流程

1. 玩家进入地图 → 加入频道 `"map:{map_name}"` → 发布 `PlayerEnter`
2. 频道收到事件后，遍历 subscribers，计算与事件源的距离
3. 在视野半径（14格）内的 subscriber，通过 `sender` 发送对应数据包
4. 玩家移动 → 更新 subscriber 中的位置 → 发布 `PlayerMove`
5. 玩家离开/断线 → 从频道移除 → 发布 `PlayerLeave`

### 聊天频道

- `map:{map_name}` — 地图频道，视野内可见
- `party:{party_id}` — 队伍频道，全队可见，不受视野限制
- 玩家可以同时订阅多个频道

---

## 4. 组队系统

### 数据结构

```rust
pub struct Party {
    pub id: Uuid,
    pub name: String,
    pub leader_id: Uuid,
    pub members: HashMap<Uuid, PartyMember>,
    pub exp_share: ExpShareMode,
    pub item_share: ItemShareMode,
}

pub struct PartyMember {
    pub player_id: Uuid,
    pub name: String,
    pub map_name: String,
    pub hp: u32,
    pub max_hp: u32,
    pub online: bool,
}

pub enum ExpShareMode {
    Equal,
    LevelBased,
}

pub enum ItemShareMode {
    LeaderPick,
    FreeForAll,
}
```

### PartyManager

```rust
pub struct PartyManager {
    parties: RwLock<HashMap<Uuid, Party>>,
    player_party: RwLock<HashMap<Uuid, Uuid>>,
    channel_bus: Arc<ChannelBus>,
}
```

### 组队操作

| 操作 | 说明 |
|------|------|
| 创建队伍 | 玩家创建队伍，自动成为队长，订阅 `party:{party_id}` 频道 |
| 邀请加入 | 队长发送邀请，对方确认后加入 |
| 退出队伍 | 移除成员，若队长退出则转让队长 |
| 踢出成员 | 队长踢人 |
| 队伍聊天 | 通过 `party:{party_id}` 频道广播，全队可见 |
| 经验分配 | 击杀 Mob 时，同地图队伍成员按 ExpShareMode 分配经验 |
| Buff 共享 | 队友在同地图时，部分 Buff 可共享给视野内队友 |

---

## 5. 游戏循环 Tick 与 Mob AI 驱动

### GameLoop

```rust
pub struct GameLoop {
    map_state: Arc<MapState>,
    mob_spawn: Arc<MobSpawnManager>,
    mob_ai: Arc<MobAI>,
    channel_bus: Arc<ChannelBus>,
    token_store: Arc<TokenStore>,
    tick_interval: Duration,
}
```

### Tick 流程（每 100ms）

1. **Mob AI 更新** — 遍历所有 active mobs，调用 `MobAI::update()`
   - Idle → 扫描视野内玩家，发现则转 Chase
   - Chase → 向目标移动，进入攻击范围则转 Attack
   - Attack → 对玩家造成伤害，发布 `PlayerAttack` 事件
   - Dead → 发布 `MobDeath`，掉落物品，启动重生计时
2. **技能冷却** — 遍历 MapState 中所有 Player，减少冷却计时器
3. **重生处理** — 检查已死亡 Mob 的重生计时器，到期则重新生成
4. **Token 清理** — 清理 TokenStore 中超过 30 秒的 token

### 掉落物

```rust
pub struct DropItem {
    pub id: Uuid,
    pub item_id: u32,
    pub amount: u16,
    pub x: u16,
    pub y: u16,
    pub map_name: String,
    pub dropped_at: Instant,
}
```

- Mob 死亡后根据 MobTemplate 的 drop 表生成掉落物
- 掉落物存在 MapState 中，5 分钟后自动消失
- 玩家走到掉落物位置可拾取，发布 `ItemPickup` 事件

### 玩家死亡/重生

- HP 归零 → 发布 `PlayerDeath` 事件 → 隐藏玩家模型
- 玩家点击重生 → 传送回存储点（默认 prontera 150x180）→ HP/SP 恢复 50%
- 玩家死亡是否掉落物品由配置 `game.death_drop_items` 控制，默认 false

---

## 6. 数据包路由与新增数据包

### PacketHandler 完整路由表

| Stage | Packet ID | 处理 |
|-------|-----------|------|
| Login | 0x0064 | LoginServer::handle_ca_login |
| Char | 0x0066 | CharServer::handle_char_list |
| Char | 0x0067 | CharServer::handle_create_char |
| Char | 0x0065 | CharServer::handle_select_char |
| Map | 0x007C | MapServer::handle_enter |
| Map | 0x0085 | MapServer::handle_move |
| Map | 0x0112 | MapServer::handle_use_skill |
| Map | 0x0089 | MapServer::handle_attack |
| Map | 0x009B | MapServer::handle_use_item |
| Map | 0x0090 | MapServer::handle_pickup_item |
| Map | 0x0190 | MapServer::handle_npc_interact |
| Map | 0x0100 | MapServer::handle_party_create |
| Map | 0x0101 | MapServer::handle_party_invite |
| Map | 0x0102 | MapServer::handle_party_reply |
| Map | 0x0103 | MapServer::handle_party_leave |
| Map | 0x0109 | MapServer::handle_party_chat |
| Map | 0x010C | MapServer::handle_chat |

### 新增数据包结构体

**客户端→服务器 (C→S)：**

| Packet ID | 名称 | 字段 |
|-----------|------|------|
| 0x0089 | CZRequestAction | {account_id, target_id, action_type} |
| 0x009B | CZUseItem | {index, item_id} |
| 0x0090 | CZRequestPickupItem | {x, y} |
| 0x0190 | CZContactNpc | {npc_id, action} |
| 0x0100 | CZMakeParty | {party_name} |
| 0x0101 | CZReqPartyInvite | {target_account_id} |
| 0x0102 | CZReqPartyJoin | {party_id, accept} |
| 0x0103 | CZLeaveParty | {} |
| 0x0109 | CZPartyChat | {message} |
| 0x010C | CZChatMessage | {message} |

**服务器→客户端 (S→C)：**

| Packet ID | 名称 | 字段 |
|-----------|------|------|
| 0x02D6 | ZCNotifyAct | {src_id, dst_id, damage, is_crit, action} |
| 0x02D7 | ZCNotifyDropItem | {item_id, x, y, amount} |
| 0x02D8 | ZCNotifyPickupItem | {player_id, item_id, amount} |
| 0x0104 | ZCPartyInfo | {party_id, party_name, members[]} |
| 0x0105 | ZCPartyMemberInfo | {player_id, name, hp, max_hp, online} |
| 0x0106 | ZCPartyInvite | {party_id, party_name, leader_name} |
| 0x0108 | ZCPartyChat | {player_name, message} |
| 0x010A | ZCNotifyPlayerDeath | {player_id} |
| 0x010B | ZCNotifyPlayerRevive | {player_id, x, y} |
| 0x0083 | HCNotifyZoneServer | {map_ip, map_port, token} |

### Char Server 修改

`handle_select_char` 返回 `HCNotifyZoneServer` 包含 `{map_ip, map_port, token}` 而非 `vec![0]`。

---

## 7. 文件结构

### 新增文件

```
src/
  game/
    map/
      map_server.rs       — MapServer 核心，处理地图数据包
      channel.rs          — ChannelBus 事件总线
      drop_item.rs        — 掉落物管理
    party/
      mod.rs              — 模块入口
      manager.rs          — PartyManager 组队管理
      data.rs             — Party/PartyMember 数据结构
    token.rs              — TokenStore 认证 token 管理
  protocol/
    party_packets.rs      — 组队数据包结构体
```

### 修改文件

| 文件 | 修改内容 |
|------|----------|
| `src/network/session.rs` | 新增 stage、player_id 字段 |
| `src/network/handler.rs` | 加入 MapServer，按 stage 路由 |
| `src/game/char.rs` | handle_select_char 返回 HCNotifyZoneServer |
| `src/game/map/mod.rs` | 新增 map_server/channel/drop_item 子模块 |
| `src/game/mod.rs` | 新增 party、token 子模块 |
| `src/core/config.rs` | 新增 GameConfig (death_drop_items) |
| `src/core/mod.rs` | 创建 MapServer 并注入 PacketHandler，启动 GameLoop tick |
| `src/protocol/map_packets.rs` | 新增数据包结构体 |
| `src/network/packet.rs` | 新增 packet ID 常量 |

### 模块依赖关系

```
MapServer
  ├── TokenStore (认证)
  ├── MapState (玩家/Mob/掉落物)
  ├── ChannelBus (事件广播)
  │     └── 视野过滤 → 发送数据包
  ├── PartyManager (组队)
  │     └── ChannelBus (队伍频道)
  ├── BattleHandler (战斗)
  ├── SkillHandler (技能)
  ├── ItemHandler (物品)
  ├── NpcHandler (NPC)
  └── MobSpawnManager + MobAI (怪物)
```

---

## 8. 配置扩展

```toml
[game]
death_drop_items = false    # 玩家死亡是否掉落物品
```

```rust
pub struct GameConfig {
    pub death_drop_items: bool,
}
```

---

## 9. 错误处理

- Token 验证失败 → 断开连接，返回 `SCNotifyBan(0x0081)`
- 无效 stage 收到不属于该 stage 的数据包 → 忽略，记录 warn 日志
- 玩家操作越权（如非队长踢人）→ 返回错误提示数据包
- 频道发送失败（连接已断）→ 自动移除 subscriber
