# Deviruchi vs rAthena 功能完成清单

## 概览

| 分类 | rAthena | Deviruchi 状态 |
|------|---------|---------------|
| 总系统数 | ~45 | 约 30+ |

---

## 一、核心基础设施 ✅

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 登录服务器 | login.cpp | ✅ `src/game/login.rs` | CA_LOGIN, 账号验证 |
| 角色服务器 | char.cpp | ✅ `src/game/char.rs` | 角色列表、创建、选择 |
| 地图服务器 | map.cpp | ✅ `src/game/map/map_server.rs` | 核心地图逻辑 |
| 网络层 | clif.cpp | ✅ `src/network/` | TCP 监听, packet codec |
| 数据库 | SQL | ✅ `src/storage/` | SQLite, schema 初始化 |
| 日志系统 | log.cpp | ✅ `src/core/logging.rs` | tracing 日志，20+ 分类完整实现 |
| 配置管理 | conf/ | ✅ `src/core/config.rs` | TOML 配置，完整配置项，热重载 |

---

## 二、战斗系统 ✅

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 伤害公式 | battle.cpp | ✅ `src/game/battle/formula.rs` | 物理/魔法伤害, 命中率, 爆击率 |
| 战斗处理 | battle.cpp | ✅ `src/game/battle/handler.rs` | AttackResult, MobAttackResult |
| 怪物攻击 | battle.cpp | ⚠️ 部分 | `mob_attack()` 存在，逻辑待完善 |
| 防御计算 | battle.cpp | ✅ 公式已实现 | damage_reduction() |
| ASPD 攻击速度 | status.cpp | ❌ 未实现 | 角色攻击间隔 |
| 伤害flag系统 | battle.cpp | ❌ 未实现 | 无属性伤害标记 |

---

## 三、状态效果系统 (Buff/Debuff) ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 状态效果枚举 | status.hpp | ❌ 未实现 | StatusChange 枚举缺失 |
| ASPD 修改 | status.cpp | ❌ 未实现 | 攻击速度 buff |
| 属性加成 | status.cpp | ❌ 未实现 | Str/Agi/Vit 等加成 |
| 再生效果 | status.cpp | ❌ 未实现 | HP/SP 自然回复 |
| 状态图标 | status.cpp | ❌ 未实现 | 客户端显示 |
| **HP/SP 自然回复** | status.cpp | ❌ **待开发** | 核心功能缺失 |

---

## 四、技能系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 技能数据库 | skill.hpp | ⚠️ `src/game/skill/data.rs` | 基础定义, 4个技能 |
| 技能处理 | skill.cpp | ❌ `src/game/skill/handler.rs` | 空实现 |
| 技能施放 | skill.cpp | ❌ 未实现 | cast_time, cooldown |
| 地面技能 | skill.cpp | ❌ 未实现 | SkillUnit 单元效果 |
| 技能树 | skill.cpp | ❌ 未实现 | 职业技能解锁 |
| 职业技能 | skills/* | ❌ 未实现 | 18个子目录未移植 |
| **技能效果应用** | skill.cpp | ❌ **待开发** | 核心功能缺失 |

---

## 五、物品系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 物品数据库 | itemdb.cpp | ✅ `src/game/item/data.rs` | YAML 加载, 默认物品 |
| 物品类型 | itemdb.cpp | ✅ ItemType enum | Heal/Weapon/Armor 等 |
| 背包管理 | pc.cpp | ✅ `src/game/item/inventory.rs` | add/remove/use |
| 装备管理 | pc.cpp | ✅ `src/game/item/equipment.rs` | 10个装备栏位 |
| 物品使用 | pc.cpp | ⚠️ `src/game/item/effect.rs` | 基础效果解析 |
| 卡片系统 | itemdb.cpp | ❌ 未实现 | 卡片属性附加 |
| 装备精炼 | pc.cpp | ❌ 未实现 | refine 系统 |
| 负重检查 | pc.cpp | ✅ 已实现 | is_overweight() |
| 物品延迟 | itemdb.cpp | ❌ 未实现 | ItemDelay 冷却 |
| 物品脚本 | itemdb.cpp | ⚠️ 基础解析 | 需完善脚本引擎 |

---

## 六、怪物系统 ✅

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 怪物数据库 | mob.cpp | ✅ `src/game/mob/data.rs` | MobTemplate, 4个怪物 |
| 怪物AI | mob.cpp | ✅ `src/game/mob/ai.rs` | 状态机 (Idle/Chase/Attack) |
| 怪物刷新 | mob.cpp | ✅ `src/game/mob/spawn.rs` | 刷新点, 延迟复活 |
| AI 路径寻路 | path.cpp | ✅ `src/game/mob/pathfinder.rs` | A* 寻路 |
| **掉落表系统** | mob.cpp | ✅ `src/game/mob/droptable.rs` | YAML配置, MVP奖励 |
| 怪物技能 | mob.cpp | ❌ 未实现 | MonsterSkill |
| 怪物仇恨 | mob.cpp | ⚠️ damage_log | 基础仇恨追踪 |

---

## 七、组队系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 创建队伍 | party.cpp | ✅ `src/game/party/manager.rs` | create_party() |
| 加入/离开 | party.cpp | ✅ 已实现 | join/leave/kick |
| 队伍聊天 | party.cpp | ⚠️ ChannelBus | 复用 Map channel |
| **经验分配** | party.cpp | ✅ `src/game/battle/exp.rs` | Equal/LevelBased 模式 |
| 物品分配 | party.cpp | ❌ 未实现 | LeaderPick/FreeForAll |
| 队伍预约 | party.cpp | ❌ 未实现 | KRO 匹配系统 |

---

## 八、工会系统 ✅

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 创建工会 | guild.cpp | ✅ `src/game/guild/` | 完整实现 |
| 成员管理 | guild.cpp | ✅ 已实现 | join/leave/expel |
| 工会仓库 | guild.cpp | ✅ `src/game/storage/` | Storage 系统 |
| 工会权限 | guild.cpp | ✅ GuildPermission | Invite/Expel 等 |
| 工会公告 | guild.cpp | ✅ 已实现 | notice |
| 工会技能 | guild.cpp | ❌ 未实现 | GuildSkill |
| 城战(WoE) | guild.cpp | ❌ 未实现 | Castles 城堡系统 |

---

## 九、交易系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 交易请求 | trade.cpp | ✅ `src/game/trade/` | request/accept/reject |
| 物品交易 | trade.cpp | ✅ 已实现 | add_item, confirm |
| Zeny 交易 | trade.cpp | ✅ 已实现 | set_zeny |
| 交易完成 | trade.cpp | ✅ 已实现 | commit/cancel |
| **交易验证** | trade.cpp | ⚠️ 待完善 | 重量/背包验证需加强 |

---

## 十、仓库系统 ✅

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 个人仓库 | storage.cpp | ✅ `src/game/storage/` | StorageSlot, 300格 |
| 仓库管理 | storage.cpp | ✅ StorageManager | 按 char_id 管理 |
| 仓库同步 | intif.cpp | ❌ 未实现 | Char-Map 服务器同步 |

---

## 十一、HP/SP 回复系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| **自然回复** | status.cpp | ❌ **未实现** | 核心功能缺失 |
| 坐下回复 | pc.cpp | ❌ **未实现** | 坐下发呆加速回复 |
| VIT 加成 | status.cpp | ❌ **未实现** | VIT 影响 HP 回复 |
| INT 加成 | status.cpp | ❌ **未实现** | INT 影响 SP 回复 |
| 装备加成 | status.cpp | ❌ **未实现** | 装备影响回复量 |

---

## 十二、坐骑/宠物系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 宠物捕捉 | pet.cpp | ❌ 未实现 | Peco/Ghost |
| 宠物养成 | pet.cpp | ❌ 未实现 | 亲密/饥饿度 |
| 宠物技能 | pet.cpp | ❌ 未实现 | PetSkill |
| 坐骑系统 | pet.cpp | ❌ 未实现 | Peco 骑乘 |

---

## 十三、佣兵/哈比系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 佣兵系统 | mercenary.cpp | ❌ 未实现 | 佣兵契约 |
| 哈比系统 | homunculus.cpp | ❌ 未实现 | 人工生命体 |
| 元素精灵 | elemental.cpp | ❌ 未实现 | 巫师召唤 |

---

## 十四、摆摊系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 摆摊开店 | vending.cpp | ❌ 未实现 | PersonalShop |
| 自动摆摊 | vending.cpp | ❌ 未实现 | AutoVending |
| 收购商店 | buyingstore.cpp | ❌ 未实现 | BuyingStore |
| 商店搜索 | searchstore.cpp | ❌ 未实现 | 搜索功能 |

---

## 十五、NPC/脚本系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| NPC 定义 | npc.cpp | ⚠️ `src/game/npc/data.rs` | 基础定义, 3个NPC |
| NPC 处理 | npc.cpp | ❌ 空实现 | handler 为空 |
| **脚本引擎** | script.cpp | ❌ **未实现** | 核心功能缺失 (787KB) |
| NPC 对话 | script.cpp | ❌ 未实现 | 脚本解析执行 |
| 任务NPC | npc.cpp | ❌ 未实现 | Warp/Shop/Quest |
| 玩家任务 | quest.cpp | ❌ 未实现 | QuestLog |

---

## 十六、地图/传送系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 地图切换 | chrif.cpp | ✅ MapServer | char->map 切换 |
| 传送管理 | - | ✅ `src/game/map/teleport.rs` | WarpService |
| 地图边界 | map.cpp | ✅ 已实现 | MapEdge 检测 |
| **保存点系统** | pc.cpp | ⚠️ 基础 | SavePoint 需持久化 |
| 瞬移服务 | - | ✅ TeleportManager | warp cooldown |
| 地图视野 | map.cpp | ⚠️ 14格 | ChannelBus vision |

---

## 十七、死亡/复活系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 死亡状态 | pc.cpp | ✅ PlayerState::Dead | 已实现 |
| 经验惩罚 | pc.cpp | ✅ 1% 惩罚 | apply_death_penalty() |
| **复活服务** | pc.cpp | ✅ `src/game/map/respawn.rs` | RespawnService |
| Zeny 掉落 | pc.cpp | ✅ 50% 掉落 | drop_zeny_on_death() |
| 复活延迟 | pc.cpp | ⚠️ 待配置 | 复活延迟时间 |

---

## 十八、GM 命令系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| @命令框架 | atcommand.cpp | ❌ 未实现 | 368KB 未移植 |
| 权限系统 | pc_groups.cpp | ❌ 未实现 | PlayerGroup |
| 常用命令 | atcommand.cpp | ❌ 未实现 | @warp/@goto/@hide 等 |

---

## 十九、聊天系统 ⚠️

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 地图聊天 | chat.cpp | ✅ Map channel | ChannelBus |
| 队伍聊天 | chat.cpp | ✅ Party channel | ChannelBus |
| 工会聊天 | chat.cpp | ✅ Guild channel | ChannelBus |
| 私聊 | whisper.cpp | ❌ 未实现 | /w 命令 |
| 大厅聊天 | chat.cpp | ❌ 未实现 | NPC Chat |

---

## 二十、成就/任务系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 成就系统 | achievement.cpp | ❌ 未实现 | 34KB 未移植 |
| 任务系统 | quest.cpp | ❌ 未实现 | QuestLog |
| 每日签到 | pc.hpp | ❌ 未实现 | Attendance |

---

## 二十一、现金商城 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 商城系统 | cashshop.cpp | ❌ 未实现 | 18KB 未移植 |
| Kafra 传送 | storage.cpp | ❌ 未实现 | 仓库/现金商店 |
| 点数系统 | cashshop.cpp | ❌ 未实现 | CashPoint |

---

## 二十二、战场/城战 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 战场系统 | battleground.cpp | ❌ 未实现 | 45KB 未移植 |
| 城战时间 | - | ❌ 未实现 | WoE Schedule |
| 争夺城堡 | guild.cpp | ❌ 未实现 | Castles |

---

## 二十三、副本/副本系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 实例系统 | instance.cpp | ❌ 未实现 | 35KB 未移植 |
| 副本地图 | instance.cpp | ❌ 未实现 | 动态创建 |
| 时间限制 | instance.cpp | ❌ 未实现 | 副本倒计时 |

---

## 二十四、邮件系统 ❌ 缺失

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 邮件系统 | mail.cpp | ❌ 未实现 | 14KB 未移植 |
| 附件物品 | mail.cpp | ❌ 未实现 | MailAttachment |
| 附件 Zeny | mail.cpp | ❌ 未实现 | MailZeny |

---

## 二十五、其他系统

| 功能 | rAthena 文件 | Deviruchi 状态 | 备注 |
|------|-------------|---------------|------|
| 决斗系统 | duel.cpp | ❌ 未实现 | 7KB |
| 氏族系统 | clan.cpp | ❌ 未实现 | 5KB (轻量) |
| 拍卖系统 | auction.cpp | ❌ 未实现 | AuctionHouse |
| 每日抽奖 | itemdb.cpp | ❌ 未实现 | Roulette |
| 宏检测 | pc.hpp | ❌ 未实现 | Anti-Macro |

---

## 优先级排序

### P0 - 核心功能 (必须实现)
1. **HP/SP 自然回复系统** - 玩家基础体验
2. **技能效果应用** - 战斗核心
3. **GM 命令框架** - 服务器管理
4. **脚本引擎基础** - NPC/任务基础

### P1 - 重要功能 (应该实现)
5. **状态效果系统** - Buff/Debuff
6. **物品脚本完善** - 药水/消耗品
7. **仓库服务器同步** - 多地图支持
8. **私聊系统** - 社交基础

### P2 - 扩展功能 (可以延后)
9. 摆摊系统
10. 宠物/坐骑
11. 现金商城
12. 成就/任务

### P3 - 高级功能 (长期规划)
13. 副本系统
14. 战场/城战
15. 佣兵/哈比

### ✅ 已完成 (2026-05-04)
- **日志系统**: 20+ 分类完整实现，分类文件输出，tracing 集成
- **配置管理系统**: TOML 配置，完整配置项（Battle/Drop/Exp/Respawn/Skill/Party/Storage/Chat），热重载支持

---

## 统计总结

| 状态 | 数量 | 占比 |
|------|------|------|
| ✅ 已完成 | 20 | 44% |
| ⚠️ 部分/待完善 | 8 | 18% |
| ❌ 未实现 | 17 | 38% |

**核心战斗系统已完成约 70%，HP/SP 回复、技能应用、GM 命令、脚本引擎等关键功能仍缺失。日志系统和配置管理已完整实现。**
