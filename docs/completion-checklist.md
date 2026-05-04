审查报告：Deviruchi 服务端代码全面审查
审查日期：2026-05-04

代码规模：168 个 .rs 文件，约 44,240 行

审查维度：未完成代码标记、逻辑缺陷与安全性、并发安全、架构与代码质量、rAthena 功能缺失

一、🔴 严重问题 (Critical)
C1. 战斗伤害 i32→u32 无符号守卫（远程崩溃/漏洞利用）
文件：src/game/battle/handler.rs，第 32-37、70、89、110 行

问题：所有攻击处理函数将 damage: i32 通过 damage as u32 转换为 u32。在 Rust release 模式下，负数 i32 会静默回绕为巨大的 u32 值（如 -1i32 as u32 == 4294967295）。公式中的 .max(1) 守卫在暴击倍率缩放之前被调用，如果 base_damage * crit_multiplier / 100 溢出 i32，结果会变负，导致怪物受到数十亿伤害或被瞬杀。


// handler.rs:32 — 在暴击缩放之前有守卫
let damage = if is_crit {
    (base_damage * BattleFormula::crit_multiplier()) / 100  // 可能溢出 i32
} else {
    base_damage
};
let killed = defender.take_damage(damage as u32);  // 负数回绕为大 u32
C2. 网络包长度下溢 → 索引 Panic（远程 DoS）
文件：src/network/packet.rs，第 46-50 行

问题：如果客户端发送包头长度字段为 1、2 或 3，切片 bytes[4..length as usize] 变成逆序范围（如 bytes[4..1]），在 Rust 中立即 panic 导致服务端崩溃。第 46 行只检查了 bytes.len() < length，没有验证 length >= 4。


// packet.rs:46-50
if bytes.len() < length as usize {  // length=3 时检查通过
    return None;
}
let data = bytes[4..length as usize].to_vec();  // PANIC: 4..3 是逆序范围
C3. 明文密码对比（凭据泄露）
文件：src/game/login.rs，第 67 行


if login.password != account.password_hash {
尽管字段名是 password_hash，但对比是明文字符串比较。整个代码库中没有任何密码哈希（bcrypt/argon2）实现。

C4. Mutex 污染崩溃整个服务端
文件：src/storage/sqlite.rs，第 27、32、41、53、63、69 行

问题：每个数据库操作都调用 .conn.lock().unwrap()。如果任何线程在持有 std::sync::Mutex 时 panic，该 mutex 变为 poisoned，所有后续 DB 操作都会 panic 崩溃整个服务端。共 6 个调用点。

二、🟠 高优先级问题 (High)
H1. 五个未实现的 TODO 函数返回假 Success
文件：src/game/item/use_handler.rs，第 233、315、339、377、384 行

函数	行号	影响
execute_teleport	233	记录日志但从不移动玩家
execute_use_skill	315	记录日志但从不施放技能
execute_strip_equipment	339	记录日志但从不卸下装备
execute_consume_ammo	377	记录日志但从不消耗弹药
execute_disguise	384	记录日志但从不应用伪装
这些函数全部返回 ItemUseResult::Success 但不执行任何实际操作。使用这些效果的道具会静默失效。

H2. from_slice 包解析 Panic（拒绝服务）
文件：src/protocol/char_packets.rs 第 151-166 行、map_packets.rs、party_packets.rs、trade_packets.rs 等

多个 from_slice 实现中的长度检查不足。例如 CHMakeChar::from_slice 检查 offset + 6 但实际读取 offset + 10 字节的两个 u16。畸形包可导致 panic。

H3. GM 指令无权限检查
文件：src/game/map/map_server.rs，第 715-847 行


// Check GM permissions (simplified - in real implementation check account level)
// For now, allow all for testing
@warp、@goto、@summon 等 GM 指令的权限检查被注释掉，任何玩家都可以使用。

H4. 治愈服务变量名 Bug — HP 恢复使用了 SP 修正值
文件：src/game/heal/service.rs，第 95-96 行


let (_, sp_base) = self.apply_all_modifiers(player, base_heal, 0, is_sitting);
new_hp = (current_hp + sp_base).min(max_hp);  // BUG: 用 sp_base 来恢复 HP
apply_all_modifiers 返回 (hp_value, sp_value)，但解构时将 HP 绑定到 _（丢弃）、SP 绑定到 sp_base，然后用 sp_base 来恢复 HP。这意味着 HP 恢复实际使用了 SP 修正链。

H5. 交易系统仅回显——无状态追踪
文件：src/game/map/map_server.rs，第 586-663 行

四个交易处理器（handle_trade_request、handle_trade_ack、handle_trade_add_item、handle_trade_add_zeny、handle_trade_lock）全部只是回显响应，不追踪交易状态、不验证物品是否存在、不检查交易伙伴。item_id 在第 631 行被硬编码为 0。

H6. 静默吞掉的失败
位置	文件	影响
Warp 失败	map_server.rs:225	玩家穿过传送门但传不到目的地
Channel 发送失败	modern_server.rs:181	玩家复活/下线事件丢失
碰撞分类错误	ai.rs	错误日志吞没判断错误
H7. 定时器队列空 pop → Panic
文件：src/core/timer.rs，第 105 行


let entry = queue.pop().unwrap();
如果队列在 peek 和 pop 之间被清空（竞争条件或逻辑错误），unwrap() 会崩溃服务端。

H8. 玩家移动无验证（加速/穿墙）
文件：src/game/map/map_server.rs，第 202-229 行

handle_move 函数接受客户端的任何 (x, y) 坐标，不验证：

坐标在地图边界内
移动距离合理（加速检测）
玩家未死亡/眩晕/冻结
移动路径可通行
三、🟡 中优先级问题 (Medium)
M1. 战斗公式未连线元素/体型修正
文件：src/game/battle/formula.rs

虽然 src/game/battle/element.rs 中已实现完整的元素表（4×10×10 矩阵）和体型修正表（23 种武器×3 种体型），但 BattleFormula::physical_damage() 从未调用 apply_element_and_size_modifier()。

M2. 状态计算可能整数溢出
文件：src/game/battle/formula.rs，第 22、49、73、98 行

所有攻击公式使用未检查的 i32 运算：


base_level * 2 + str + dex / 2 + agi / 3
在 release 模式下，极高的属性值（GM 指令、装备、状态效果）可能溢出 i32 产生负伤害。

M3. Token 验证失败不关闭连接
文件：src/game/map/map_server.rs，第 159-170 行

当 token 验证失败时返回 None，但不会设置 session.authenticated = false 或关闭连接。会话保持在 Map 阶段但没有有效玩家。

M4. 角色删除定时器从未执行
文件：src/game/char.rs

标记为删除的角色有 delete_timer 字段，但没有定时任务执行实际删除。数据库中永远不会清理角色。

M5. 未识别包 ID 静默丢弃
文件：src/game/map/map_server.rs，第 138 行


_ => None,
dispatch match 对未识别的包 ID 返回 None 且无任何警告日志，使得协议调试和漏洞检测几乎不可能。

M6. NPC 交互空操作
文件：src/game/map/map_server.rs，第 378-384 行

所有 NPC 交互包被解析但丢弃，NPC 完全不可用。

M7. 队伍邀请空操作
文件：src/game/map/map_server.rs，第 412-418 行

队伍邀请被解析但从不转发给目标玩家。

四、🔵 架构与并发问题
A1. map_server.rs — 1244 行上帝对象
文件：src/game/map/map_server.rs（1244 行）

直接耦合到 13 个子系统，44 个函数处理所有包类型。应拆分为独立的 handler 模块。

A2. Player 结构体 25+ 独立 RwLock — TOCTOU 漏洞
文件：src/game/map/player.rs，第 15-62 行

Player 结构体有 25 个独立的 RwLock 字段（每个属性一个）。Player::clone() 顺序获取 24 个读锁，在第一个锁和最后一个锁之间值可能被其他线程修改——产生不一致的快照。

A3. 混合使用三种不同的锁类型
锁类型	使用位置
parking_lot::RwLock	40+ 文件，大部分 Player/Manager 字段
std::sync::Mutex	Database.conn、ThreadRng、BattleHandler.rng、ItemDelay
parking_lot::Mutex	仅 TIMER_QUEUE
std::sync::Mutex 有 poison 机制（panic 后会污染），而 parking_lot 锁没有。混合使用可能导致微妙的优先级反转。

A4. 数据库单 Mutex 瓶颈
文件：src/storage/sqlite.rs，第 6-8 行

尽管启用了 WAL 模式（允许并发读），但所有 DB 访问通过单个 Arc<Mutex<Connection>> 串行化。高并发下数据库成为瓶颈。

A5. 存储/背包保存使用逐条 INSERT（无事务）
文件：src/game/storage/repository.rs，第 84-108 行；src/storage/character.rs，第 257-261 行

保存角色时执行多个独立 SQL 操作（UPDATE 角色、DELETE 背包、逐条 INSERT 物品、DELETE 状态、逐条 INSERT 状态）无显式事务。中途崩溃会导致数据不一致。

A6. MockRng 四份完全相同
测试辅助 MockRng 在四个文件中逐行复制：

src/game/battle/handler.rs
src/game/battle/formula.rs
src/game/mob/ai.rs
src/game/mob/droptable.rs
A7. entry + get_mut 双重 lookups
文件：src/game/map/map_state.rs，第 27-29 行


by_map.entry(map_name.clone()).or_default();
if let Some(players) = by_map.get_mut(&map_name) {  // 第二次查找
应写为 by_map.entry(map_name.clone()).or_default().push(player_id);

五、📦 rAthena 功能缺失对比
严重缺失（核心功能）
子系统	现状	缺失程度
技能系统	仅有基础框架，20+ 技能效果存根	🔴 严重
状态效果	结构体存在，excelotick 逻辑和叠加规则缺失	🔴 严重
物品精炼	refiner.rs 和 ingredient.rs 为空	🔴 完全缺失
NPC 系统	handler 存在但是空操作	🔴 不可用
队伍系统	Manager 数据层面存在，邀请处理空操作	🟠 高
公会系统	数据层面存在，未连线到网络处理	🟠 高
PvP/Battleground	框架存在，无实例化逻辑	🟡 中
副本实例	框架存在，无实际副本	🟡 中
任务系统	框架存在，无实际任务	🟡 中
邮件/拍卖	✅ 已实现	✅ 完成
元素/体型表	✅ 已实现但未连线到伤害公式	⚠️ 半完成
卡片系统	✅ 已实现基础	✅ 完成
战斗系统详细差异
物理伤害公式：当前 base_level*2 + str + dex/2 + agi/3，rAthena 用的公式是 statusATK + weaponATK + equipATK + masteries，差异较大
魔法伤害公式：当前 int*2 + dex/3 + base_level/4，rAthena 有更复杂的 MATK 计算链
元素修正：已计算但未应用于最终伤害
完全缺失：Perfect Dodge、Perfect Hit、Crit Shield、Flee 率计算、Aspd 计算
怪物系统详细差异
MobDatabase：仅 4 种硬编码怪物（Poring/Lunatic/Blue Poring/Fabre），rAthena 有 2000+
怪物技能使用：AI 框架有 MobSkill 结构体，但从不施放技能
MVP 系统：MVP 奖励分发逻辑不完整
怪物掉落：YAML 掉落表加载器存在但未集成到实际死亡处理
六、📊 统计总览
严重级别	数量	主题
🔴 Critical	4	整数回绕崩溃、包 panic、明文密码、mutex 污染
🟠 High	8	TODO 假成功、解析 panic、无 GM 认证、变量 bug、假交易、静默失败
🟡 Medium	7	公式不连线、溢出风险、token 不关连接、空操作处理、缺失清理
🔵 Architecture	7	上帝对象、TOCTOU、混合锁、DB 单瓶颈、无事务、代码重复
📦 rAthena Gap	12+	技能/状态/精炼/NPC 核心缺失
七、优先修复建议
第一优先：修复 C2（包 length 下溢 panic）和 C1（伤害 as u32 回绕）——可被外部触发
第二优先：实现 H1 中的 5 个 TODO 函数，连线元素/体型修正到伤害公式
第三优先：添加 GM 权限检查，修复治愈服务变量 bug
第四优先：为数据库操作添加事务包裹，使用 saturating_add 或 checked_add 替代裸算术
第五优先：拆分 map_server.rs，消除上帝对象
请告诉我您想优先处理哪些问题，或者是否要我按照推荐顺序开始实现修复。

<DSMLCOUN xmlns="tool_calls="https://www.anthropic.com/xml/tool_calls">
<DSMLCOUN type="text">我已完成对服务端全部 168 个 .rs 文件（~44,240 行）的全面审查，涵盖六大维度：

未完成代码标记——5 个返回假 Success 的 TODO 函数、8 个完全空的模块
逻辑缺陷与安全性——4 个 Critical 级问题（整数回绕崩溃、包解析 panic、明文密码、mutex 污染）
并发安全——混合使用三种锁类型、25 个独立 RwLock 导致的 TOCTOU 漏洞
架构质量——1244 行上帝对象 map_server.rs、多处代码重复、无事务的数据库操作
rAthena 功能缺失——技能/状态/精炼/NPC 为最大缺失点，元素体型表已实现但未连线到伤害公式
可优化点——双重 HashMap 查找、逐条 INSERT 无批处理、String clone 过多
最关键的两个可被外部触发的安全问题：

包头部 length < 4 时直接 panic 导致服务端崩溃
伤害计算 i32 as u32 负数回绕可导致数十亿伤害或瞬杀
您想优先处理哪些问题？或者我按推荐顺序（安全→功能缺失→架构优化）开始制定修复计划？