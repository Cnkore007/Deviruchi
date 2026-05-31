use std::collections::HashMap;

// ============================================================
// 脚本命令枚举
// ============================================================

/// 脚本命令
#[derive(Debug, Clone)]
pub enum ScriptCommand {
    // ---- 对话命令 ----
    /// 显示消息
    Mes(String),
    /// 下一行（等待玩家点击）
    Next,
    /// 关闭对话
    Close,
    /// 关闭对话（不清除对话状态，玩家可重新触发 NPC）
    Close2,
    /// 结束脚本
    End,
    /// 选择菜单
    Select(Vec<String>),

    // ---- 传送 ----
    /// 传送玩家到指定地图坐标
    Warp(String, u16, u16),

    // ---- 控制流 ----
    /// 跳转到标签
    Goto(String),
    /// 设置变量（整数或字符串赋值）
    Set(String, ExprValue),
    /// 条件跳转：如果变量等于指定值则跳转
    GotoIf(String, i64, String),
    /// 调用脚本函数（将当前执行位置压栈）
    CallFunc(String, Vec<i64>),
    /// 从函数返回（弹栈恢复执行位置）
    Return,
    /// for 循环开始（条件检查点 + 增量命令序列）
    ForStart(Expr, Vec<ScriptCommand>),
    /// for/while 循环结束（回跳到对应的 ForStart）
    ForEnd,
    /// 跳出当前循环（break）
    Break,
    /// 继续当前循环下一次迭代（continue）
    Continue,

    // ---- 物品操作 ----
    /// 给玩家指定数量的物品
    GetItem(u16, u16),
    /// 从玩家背包删除指定数量的物品
    DelItem(u16, u16),

    // ---- 信息查询（函数式语法，返回值存入变量） ----
    /// 查询玩家持有某物品的数量，结果存入变量
    CountItem(u16, String),
    /// 检查背包能否容纳指定物品，结果存入变量
    CheckWeight(u16, u16, String),
    /// 获取角色 ID（0=char_id, 1=party_id, 2=guild_id, 3=account_id）
    GetCharId(u8, String),
    /// 获取函数参数（按索引）
    GetArg(u16, String),
    /// 获取角色名/公会名/队伍名（0=角色名, 1=公会名, 2=队伍名）
    StrCharInfo(u8, String),
    /// 读取角色属性值
    ReadParam(u16, String),

    // ---- 状态操作 ----
    /// 恢复 HP 和 SP
    Heal(u32, u32),

    // ---- 系统操作 ----
    /// 全服公告
    Announce(String, u8),
    /// 接收玩家输入，存入指定变量
    Input(String),
    /// 延迟执行（毫秒）
    Sleep(u32),

    // ---- 数值操作 ----
    /// 获取 NPC 变量
    GetVariableOfNpc(String, String, String),
    /// 生成随机数 [min, max]，结果存入变量
    Rand(i64, i64, String),

    // ================================================================
    // 新增脚本命令（对应 rAthena 核心 NPC 交互命令）
    // ================================================================

    // ---- 高级对话命令 ----
    /// 显示带选项的菜单（menu "选项1",label1,"选项2",label2,...）
    Menu(Vec<(String, String)>),
    /// 显示带选项的选择框（select "选项1",label1,"选项2",label2,...）
    SelectWithLabels(Vec<(String, String)>),
    /// 显示提示框（prompt "选项1",label1,...）
    Prompt(Vec<(String, String)>),

    // ---- 高级控制流 ----
    /// 调用脚本函数（支持参数传递）
    CallSub(String, Vec<ExprValue>),
    /// 从函数返回值
    ReturnValue(ExprValue),
    /// 获取函数参数（按索引，支持默认值）
    GetArgWithDefault(u16, ExprValue, String),
    /// 条件表达式（三元运算符）
    Conditional(String, Expr, ExprValue, ExprValue),
    /// switch-case 语句
    Switch(String, Vec<(ExprValue, String)>),
    /// case 默认分支
    SwitchDefault(String),

    // ---- 数组操作 ----
    /// 设置数组元素
    SetArray(String, Vec<ExprValue>),
    /// 清空数组
    ClearArray(String, ExprValue),
    /// 复制数组
    CopyArray(String, String),
    /// 获取数组大小
    GetArraySize(String, String),
    /// 获取数组元素
    GetArrayElement(String, String, String),
    /// 设置数组元素
    SetArrayElement(String, String, ExprValue),

    // ---- 字符串操作 ----
    /// 字符串连接
    StrConcat(String, String, String),
    /// 字符串长度
    StrLen(String, String),
    /// 字符串查找
    StrFind(String, String, String),
    /// 字符串截取
    StrSub(String, String, u32, u32),
    /// 字符串替换
    StrReplace(String, String, String, String),
    /// 字符串转整数
    StrToInt(String, String),
    /// 整数转字符串
    IntToStr(String, i64),

    // ---- 数学运算 ----
    /// 绝对值
    Abs(String, i64),
    /// 最大值
    Max(String, i64, i64),
    /// 最小值
    Min(String, i64, i64),
    /// 幂运算
    Pow(String, i64, i64),
    /// 平方根
    Sqrt(String, i64),

    // ---- 玩家信息查询 ----
    /// 获取玩家等级
    GetBaseLevel(String),
    /// 获取玩家职业等级
    GetJobLevel(String),
    /// 获取玩家金币
    GetZeny(String),
    /// 获取玩家 HP
    GetHp(String),
    /// 获取玩家 SP
    GetSp(String),
    /// 获取玩家最大 HP
    GetMaxHp(String),
    /// 获取玩家最大 SP
    GetMaxSp(String),
    /// 获取玩家职业 ID
    GetClass(String),
    /// 获取玩家性别
    GetSex(String),
    /// 获取玩家地图名
    GetMapName(String),
    /// 获取玩家 X 坐标
    GetX(String),
    /// 获取玩家 Y 坐标
    GetY(String),

    // ---- 玩家操作 ----
    /// 设置玩家等级
    SetBaseLevel(u16),
    /// 设置玩家职业等级
    SetJobLevel(u16),
    /// 设置玩家金币
    SetZeny(u32),
    /// 增加玩家金币
    AddZeny(i32),
    /// 恢复 HP
    HealHp(u32),
    /// 恢复 SP
    HealSp(u32),
    /// 百分比恢复
    PercentHeal(u8, u8),
    /// 改变职业
    JobChange(u16),
    /// 重置属性点
    ResetStatus,
    /// 重置技能点
    ResetSkill,
    /// 全部重置
    ResetAll,

    // ---- 物品操作增强 ----
    /// 检查玩家是否拥有物品
    CountItem2(u16, String),
    /// 删除物品（带属性检查）
    DelItem2(u16, u16, String),
    /// 给予物品（带属性）
    GetItem2(u16, u16, String),
    /// 制造物品
    MakeItem(u16, u16),
    /// 检查制造成功率
    CheckCraft(u16, String),

    // ---- 公会操作 ----
    /// 获取公会 ID
    GetGuildId(String),
    /// 获取公会名称
    GetGuildName(String),
    /// 获取公会等级
    GetGuildLevel(String),
    /// 检查是否为公会会长
    IsGuildMaster(String),

    // ---- 队伍操作 ----
    /// 获取队伍 ID
    GetPartyId(String),
    /// 获取队伍名称
    GetPartyName(String),
    /// 检查是否为队长
    IsPartyLeader(String),
    /// 获取队伍成员数量
    GetPartyCount(String),

    // ---- 地图操作 ----
    /// 传送玩家到指定地图
    Warp2(String, u16, u16),
    /// 区域传送（范围内所有玩家）
    AreaWarp(String, u16, u16, u16, u16, String, u16, u16),
    /// 获取地图上玩家数量
    GetMapUsers(String, String),
    /// 获取地图上怪物数量
    GetMapMobs(String, String),

    // ---- 怪物操作 ----
    /// 召唤怪物
    Monster(String, u16, u16, String, u16, String),
    /// 召唤怪物（带属性）
    Monster2(String, u16, u16, String, u16, String, String),
    /// 杀死指定怪物
    KillMonster(String, String),
    /// 杀死地图上所有怪物
    KillMonsterAll(String),

    // ---- NPC 操作 ----
    /// 显示 NPC 对话框
    NpcTalk(String, String),
    /// 关闭 NPC 对话框
    NpcTalkClose(String),
    /// 获取 NPC 名称
    GetNpcName(String, String),
    /// 获取 NPC 位置
    GetNpcPos(String, String, String, String),

    // ---- 时间操作 ----
    /// 获取系统时间
    GetTime(String),
    /// 获取游戏时间
    GetGameTime(String),
    /// 检查是否为白天
    IsDay(String),
    /// 检查是否为晚上
    IsNight(String),

    // ---- 系统操作增强 ----
    /// 显示系统消息
    Message(String),
    /// 显示公告
    Broadcast(String, u8),
    /// 日志记录
    Log(String),
    /// 获取服务器名称
    GetServerName(String),
    /// 获取在线玩家数量
    GetOnlineCount(String),

    // ---- 状态效果操作 ----
    /// 添加状态效果
    AddStatus(u16, u32, u8),
    /// 移除状态效果
    RemoveStatus(u16),
    /// 检查是否有状态效果
    HasStatus(u16, String),

    // ---- 技能操作 ----
    /// 学习技能
    LearnSkill(u16),
    /// 遗忘技能
    ForgetSkill(u16),
    /// 检查技能等级
    GetSkillLevel(u16, String),
    /// 设置技能等级
    SetSkillLevel(u16, u8),

    // ---- 装备操作 ----
    /// 装备物品
    EquipItem(u16),
    /// 卸下装备
    UnequipItem(u16),
    /// 检查是否装备了物品
    IsEquipped(u16, String),
    /// 获取装备位置
    GetEquipSlot(u16, String),

    // ---- 宠物操作 ----
    /// 孵化宠物
    HatchPet(u16),
    /// 召唤宠物
    SummonPet,
    /// 回收宠物
    ReturnPet,
    /// 喂养宠物
    FeedPet(u16),
    /// 获取宠物亲密度
    GetPetIntimacy(String),
    /// 设置宠物亲密度
    SetPetIntimacy(u32),

    // ---- 坐骑操作 ----
    /// 召唤坐骑
    Mount(u16),
    /// 卸下坐骑
    Dismount,
    /// 检查是否骑乘中
    IsMounted(String),

    // ---- 任务操作 ----
    /// 添加任务
    AddQuest(u16),
    /// 完成任务
    CompleteQuest(u16),
    /// 放弃任务
    RemoveQuest(u16),
    /// 检查任务状态
    GetQuestStatus(u16, String),
    /// 设置任务目标
    SetQuestObjective(u16, u16, u16),

    // ---- 成就操作 ----
    /// 解锁成就
    UnlockAchievement(u16),
    /// 检查成就状态
    IsAchievementUnlocked(u16, String),
    /// 获取成就进度
    GetAchievementProgress(u16, String),

    // ---- 公会战操作 ----
    /// 检查是否在公会战中
    IsWoEActive(String),
    /// 获取公会战城堡数量
    GetCastleCount(String),
    /// 获取公会战城堡名称
    GetCastleName(u16, String),

    // ---- 交易操作 ----
    /// 发起交易请求
    TradeRequest(u32),
    /// 接受交易
    TradeAccept,
    /// 拒绝交易
    TradeReject,
    /// 添加交易物品
    TradeAddItem(u16, u16),
    /// 设置交易金币
    TradeSetZeny(u32),

    // ---- 聊天操作 ----
    /// 发送聊天消息
    ChatMessage(String),
    /// 发送私聊消息
    WhisperMessage(String, String),
    /// 创建聊天室
    CreateChatRoom(String, u16, String),
    /// 加入聊天室
    JoinChatRoom(String),
    /// 离开聊天室
    LeaveChatRoom,

    // ---- 邮件操作 ----
    /// 发送邮件
    SendMail(String, String, String),
    /// 读取邮件
    ReadMail(u32, String),
    /// 删除邮件
    DeleteMail(u32),
    /// 附件收取
    GetMailAttachment(u32),

    // ---- 调试命令 ----
    /// 打印变量值
    DebugVar(String),
    /// 断点调试
    DebugBreakpoint,
    /// 获取执行位置
    GetLine(String),

    // ================================================================
    // 扩展脚本命令（对应 rAthena 高级功能）
    // ================================================================

    // ---- 高级物品操作 ----
    /// 检查物品是否被识别
    IsIdentified(u16, String),
    /// 识别物品
    Identify(u16),
    /// 精炼物品
    Refine(u16, u8),
    /// 获取物品精炼等级
    GetRefineLevel(u16, String),
    /// 检查物品是否可精炼
    CanRefine(u16, String),
    /// 获取物品卡片槽位
    GetCardSlot(u16, String),
    /// 插入卡片
    InsertCard(u16, u16),
    /// 移除卡片
    RemoveCard(u16, u16),

    // ---- 高级状态操作 ----
    /// 获取状态效果剩余时间
    GetStatusTime(u16, String),
    /// 设置状态效果持续时间
    SetStatusTime(u16, u32),
    /// 获取状态效果层数
    GetStatusStack(u16, String),
    /// 设置状态效果层数
    SetStatusStack(u16, u8),
    /// 检查状态效果是否激活
    IsStatusActive(u16, String),
    /// 清除所有状态效果
    ClearAllStatus,
    /// 清除负面状态效果
    ClearNegativeStatus,
    /// 清除正面状态效果
    ClearPositiveStatus,

    // ---- 高级技能操作 ----
    /// 获取技能冷却时间
    GetSkillCooldown(u16, String),
    /// 设置技能冷却时间
    SetSkillCooldown(u16, u32),
    /// 检查技能是否在冷却中
    IsSkillOnCooldown(u16, String),
    /// 重置技能冷却
    ResetSkillCooldown(u16),
    /// 获取技能消耗
    GetSkillCost(u16, String),
    /// 检查技能消耗是否足够
    CanPaySkillCost(u16, String),
    /// 获取技能范围
    GetSkillRange(u16, String),
    /// 获取技能伤害
    GetSkillDamage(u16, u8, String),

    // ---- 高级战斗操作 ----
    /// 计算物理伤害
    CalcPhysicalDamage(u16, u16, String),
    /// 计算魔法伤害
    CalcMagicDamage(u16, u16, String),
    /// 计算治愈量
    CalcHealAmount(u16, u8, String),
    /// 获取命中率
    GetHitRate(u16, String),
    /// 获取闪避率
    GetFleeRate(u16, String),
    /// 获取暴击率
    GetCriticalRate(u16, String),
    /// 获取攻击速度
    GetAttackSpeed(String),
    /// 获取移动速度
    GetMoveSpeed(String),

    // ---- 高级地图操作 ----
    /// 获取地图宽度
    GetMapWidth(String, String),
    /// 获取地图高度
    GetMapHeight(String, String),
    /// 检查地图坐标是否可通行
    IsWalkable(String, u16, u16, String),
    /// 获取地图上指定范围的玩家
    GetAreaPlayers(String, u16, u16, u16, String),
    /// 获取地图上指定范围的怪物
    GetAreaMobs(String, u16, u16, u16, String),
    /// 在地图上显示特效
    ShowEffect(String, u16, u16, u16),
    /// 在地图上显示动画
    ShowAnimation(String, u16, u16, u16),

    // ---- 高级 NPC 操作 ----
    /// 设置 NPC 位置
    SetNpcPos(String, u16, u16),
    /// 设置 NPC 方向
    SetNpcDir(String, u16),
    /// 设置 NPC 外观
    SetNpcSprite(String, u16),
    /// 设置 NPC 可见性
    SetNpcVisible(String, bool),
    /// 设置 NPC 可交互性
    SetNpcInteractable(String, bool),
    /// 获取 NPC 状态
    GetNpcStatus(String, String),
    /// 设置 NPC 状态
    SetNpcStatus(String, String),

    // ---- 高级公会操作 ----
    /// 设置公会等级
    SetGuildLevel(u16),
    /// 增加公会经验值
    AddGuildExp(u32),
    /// 获取公会成员数量
    GetGuildMemberCount(String),
    /// 获取公会在线成员数量
    GetGuildOnlineCount(String),
    /// 检查玩家是否在公会中
    IsInGuild(String),
    /// 获取公会公告
    GetGuildNotice(String),
    /// 设置公会公告
    SetGuildNotice(String),
    /// 获取公会技能等级
    GetGuildSkillLevel(u16, String),
    /// 设置公会技能等级
    SetGuildSkillLevel(u16, u8),

    // ---- 高级队伍操作 ----
    /// 获取队伍成员数量
    GetPartyMemberCount(String),
    /// 获取队伍在线成员数量
    GetPartyOnlineCount(String),
    /// 检查玩家是否在队伍中
    IsInParty(String),
    /// 获取队伍经验值分配模式
    GetPartyExpPolicy(String),
    /// 设置队伍经验值分配模式
    SetPartyExpPolicy(u8),
    /// 获取队伍物品分配模式
    GetPartyItemPolicy(String),
    /// 设置队伍物品分配模式
    SetPartyItemPolicy(u8),

    // ---- 高级宠物操作 ----
    /// 获取宠物等级
    GetPetLevel(String),
    /// 获取宠物经验值
    GetPetExp(String),
    /// 增加宠物经验值
    AddPetExp(u32),
    /// 获取宠物饥饿度
    GetPetHunger(String),
    /// 设置宠物饥饿度
    SetPetHunger(u16),
    /// 获取宠物装备
    GetPetEquip(String),
    /// 设置宠物装备
    SetPetEquip(u16),

    // ---- 高级坐骑操作 ----
    /// 获取坐骑等级
    GetMountLevel(String),
    /// 设置坐骑等级
    SetMountLevel(u16),
    /// 获取坐骑经验值
    GetMountExp(String),
    /// 增加坐骑经验值
    AddMountExp(u32),
    /// 获取坐骑速度加成
    GetMountSpeedBonus(String),

    // ---- 高级任务操作 ----
    /// 获取任务目标进度
    GetQuestObjective(u16, u16, String),
    /// 设置任务目标完成
    CompleteQuestObjective(u16, u16),
    /// 检查任务是否完成
    IsQuestComplete(u16, String),
    /// 检查任务是否接受
    IsQuestAccepted(u16, String),
    /// 获取任务奖励
    GetQuestReward(u16, String),
    /// 给予任务奖励
    GiveQuestReward(u16),

    // ---- 高级成就操作 ----
    /// 获取成就进度
    GetAchievementProgress2(u16, String),
    /// 设置成就进度
    SetAchievementProgress(u16, u32),
    /// 检查成就是否解锁
    IsAchievementUnlocked2(u16, String),
    /// 获取成就奖励
    GetAchievementReward(u16, String),
    /// 给予成就奖励
    GiveAchievementReward(u16),

    // ---- 高级公会战操作 ----
    /// 获取公会战状态
    GetWoEStatus(String),
    /// 获取公会战剩余时间
    GetWoETimer(String),
    /// 获取公会战得分
    GetWoEScore(String),
    /// 获取公会战排名
    GetWoERank(String),
    /// 检查公会是否拥有城堡
    IsCastleOwner(u16, String),
    /// 获取城堡防御值
    GetCastleDefense(u16, String),
    /// 设置城堡防御值
    SetCastleDefense(u16, u16),

    // ---- 高级交易操作 ----
    /// 获取交易物品数量
    GetTradeItemCount(String),
    /// 获取交易金币数量
    GetTradeZeny(String),
    /// 检查交易是否完成
    IsTradeComplete(String),
    /// 取消交易
    CancelTrade,

    // ---- 高级聊天操作 ----
    /// 获取聊天室成员数量
    GetChatRoomCount(String),
    /// 获取聊天室标题
    GetChatRoomTitle(String),
    /// 检查玩家是否在聊天室中
    IsInChatRoom(String),
    /// 获取聊天室密码
    GetChatRoomPassword(String),
    /// 设置聊天室密码
    SetChatRoomPassword(String),

    // ---- 高级邮件操作 ----
    /// 获取邮件数量
    GetMailCount(String),
    /// 检查邮件是否有附件
    HasMailAttachment(u32, String),
    /// 获取邮件附件物品
    GetMailItem(u32, u16, String),
    /// 获取邮件附件金币
    GetMailZeny(u32, String),
    /// 标记邮件为已读
    MarkMailRead(u32),

    // ---- 系统信息查询 ----
    /// 获取服务器版本
    GetServerVersion(String),
    /// 获取服务器启动时间
    GetServerUptime(String),
    /// 获取服务器最大在线人数
    GetServerMaxOnline(String),
    /// 获取服务器 TPS
    GetServerTPS(String),
    /// 获取服务器内存使用
    GetServerMemory(String),

    // ---- 高级数学运算 ----
    /// 取模运算
    Mod(i64, i64, String),
    /// 位与运算
    BitAnd(i64, i64, String),
    /// 位或运算
    BitOr(i64, i64, String),
    /// 位异或运算
    BitXor(i64, i64, String),
    /// 位取反运算
    BitNot(i64, String),
    /// 左移运算
    BitShiftLeft(i64, u8, String),
    /// 右移运算
    BitShiftRight(i64, u8, String),

    // ---- 高级字符串操作 ----
    /// 字符串转大写
    StrToUpper(String, String),
    /// 字符串转小写
    StrToLower(String, String),
    /// 字符串去空格
    StrTrim(String, String),
    /// 字符串分割
    StrSplit(String, String, String),
    /// 字符串格式化
    StrFormat(String, Vec<ExprValue>, String),
    /// 字符串比较
    StrCompare(String, String, String),
    /// 字符串包含检查
    StrContains(String, String, String),

    // ---- 高级数组操作 ----
    /// 数组排序
    SortArray(String),
    /// 数组反转
    ReverseArray(String),
    /// 数组查找
    FindInArray(String, ExprValue, String),
    /// 数组去重
    UniqueArray(String),
    /// 数组合并
    MergeArray(String, String),
    /// 数组切片
    SliceArray(String, u32, u32, String),

    // ---- 高级控制流 ----
    /// 条件执行（if-else）
    If(Expr, Vec<ScriptCommand>, Vec<ScriptCommand>),
    /// while 循环
    While(Expr, Vec<ScriptCommand>),
    /// do-while 循环
    DoWhile(Vec<ScriptCommand>, Expr),
    /// for-each 循环
    ForEach(String, String, Vec<ScriptCommand>),
    /// try-catch 异常处理
    TryCatch(Vec<ScriptCommand>, String, Vec<ScriptCommand>),
    /// 抛出异常
    Throw(String),
    /// 断言
    Assert(Expr, String),

    // ---- 高级函数操作 ----
    /// 匿名函数
    Lambda(Vec<String>, Vec<ScriptCommand>),
    /// 函数映射
    Map(String, String, Vec<ScriptCommand>),
    /// 函数过滤
    Filter(String, String, Vec<ScriptCommand>),
    /// 函数归约
    Reduce(String, String, ExprValue, Vec<ScriptCommand>),
    /// 函数柯里化
    Curry(String, Vec<ExprValue>, String),

    // ---- 高级事件操作 ----
    /// 注册事件监听器
    OnEvent(String, Vec<ScriptCommand>),
    /// 触发事件
    EmitEvent(String, Vec<ExprValue>),
    /// 移除事件监听器
    RemoveEventListener(String),
    /// 检查事件监听器是否存在
    HasEventListener(String, String),

    // ---- 高级定时器操作 ----
    /// 设置定时器
    SetTimer(u32, Vec<ScriptCommand>),
    /// 设置循环定时器
    SetInterval(u32, Vec<ScriptCommand>),
    /// 清除定时器
    ClearTimer(String),
    /// 暂停定时器
    PauseTimer(String),
    /// 恢复定时器
    ResumeTimer(String),

    // ---- 高级网络操作 ----
    /// 发送 HTTP 请求
    HttpRequest(String, String, String, String),
    /// 发送 WebSocket 消息
    WsSend(String, String),
    /// 检查网络连接状态
    IsConnected(String),

    // ---- 高级数据库操作 ----
    /// 执行 SQL 查询
    SqlQuery(String, String),
    /// 获取查询结果
    SqlFetch(String, String),
    /// 执行 SQL 更新
    SqlExecute(String, String),
    /// 获取受影响行数
    SqlAffectedRows(String),

    // ---- 高级缓存操作 ----
    /// 设置缓存
    CacheSet(String, ExprValue, u32),
    /// 获取缓存
    CacheGet(String, String),
    /// 删除缓存
    CacheDelete(String),
    /// 检查缓存是否存在
    CacheHas(String, String),
    /// 清空缓存
    CacheClear,

    // ---- 高级日志操作 ----
    /// 记录调试日志
    LogDebug(String),
    /// 记录信息日志
    LogInfo(String),
    /// 记录警告日志
    LogWarn(String),
    /// 记录错误日志
    LogError(String),
    /// 记录性能日志
    LogPerformance(String, u64),

    // ---- 高级配置操作 ----
    /// 获取配置值
    GetConfig(String, String),
    /// 设置配置值
    SetConfig(String, ExprValue),
    /// 检查配置是否存在
    HasConfig(String, String),
    /// 重新加载配置
    ReloadConfig,
    /// 获取配置节
    GetConfigSection(String, String),

    // ---- 高级权限操作 ----
    /// 检查权限
    HasPermission(String, String),
    /// 授予权限
    GrantPermission(String, String),
    /// 撤销权限
    RevokePermission(String, String),
    /// 获取权限列表
    GetPermissions(String, String),
    /// 检查 GM 权限
    IsGM(String),
    /// 获取 GM 等级
    GetGMLevel(String),

    // ---- 高级安全操作 ----
    /// 检查封禁状态
    IsBanned(String),
    /// 封禁玩家
    BanPlayer(String, u32, String),
    /// 解封玩家
    UnbanPlayer(String),
    /// 检查 IP 封禁
    IsIPBanned(String),
    /// 封禁 IP
    BanIP(String, u32, String),
    /// 解封 IP
    UnbanIP(String),

    // ---- 高级统计操作 ----
    /// 获取玩家在线时长
    GetPlayTime(String),
    /// 获取玩家击杀数
    GetKillCount(String),
    /// 获取玩家死亡数
    GetDeathCount(String),
    /// 获取玩家经验值获取量
    GetExpGained(String),
    /// 获取玩家金币获取量
    GetZenyGained(String),

    // ---- 高级排名操作 ----
    /// 获取玩家排名
    GetPlayerRank(String, String),
    /// 获取公会排名
    GetGuildRank(String, String),
    /// 获取排行榜数据
    GetLeaderboard(String, u32, String),
    /// 更新排名
    UpdateRank(String, u32),

    // ---- 高级商城操作 ----
    /// 获取商城物品列表
    GetCashShopItems(String),
    /// 购买商城物品
    BuyCashItem(u16, u32),
    /// 获取玩家点数
    GetCashPoints(String),
    /// 增加玩家点数
    AddCashPoints(u32),
    /// 消耗玩家点数
    ConsumeCashPoints(u32, String),

    // ---- 高级 Kafra 操作 ----
    /// 打开仓库
    OpenStorage,
    /// 获取仓库物品数量
    GetStorageItemCount(u16, String),
    /// 存入物品
    StorageAdd(u16, u16),
    /// 取出物品
    StorageRemove(u16, u16),
    /// 获取仓库容量
    GetStorageCapacity(String),

    // ---- 高级拍卖操作 ----
    /// 获取拍卖列表
    GetAuctionList(String),
    /// 上架拍卖
    AuctionAddItem(u16, u16, u32),
    /// 竞拍
    AuctionBid(u32, u32),
    /// 获取拍卖信息
    GetAuctionInfo(u32, String),
    /// 领取拍卖物品
    AuctionClaimItem(u32),

    // ---- 高级摆摊操作 ----
    /// 打开摆摊界面
    OpenVending,
    /// 获取摆摊物品列表
    GetVendingItems(String),
    /// 购买摆摊物品
    BuyVendingItem(u32, u16),
    /// 获取摆摊信息
    GetVendingInfo(String),
    /// 关闭摆摊
    CloseVending,

    // ---- 高级副本操作 ----
    /// 创建副本
    CreateInstance(u16, String),
    /// 进入副本
    EnterInstance(u16),
    /// 离开副本
    LeaveInstance,
    /// 获取副本信息
    GetInstanceInfo(String),
    /// 检查副本是否活跃
    IsInstanceActive(String),

    // ---- 高级战场操作 ----
    /// 加入战场队列
    JoinBattleground(u16),
    /// 离开战场
    LeaveBattleground,
    /// 获取战场信息
    GetBattlegroundInfo(String),
    /// 获取战场得分
    GetBattlegroundScore(String),
    /// 检查是否在战场中
    IsInBattleground(String),
}

// ============================================================
// 脚本值类型（支持整数和字符串变量）
// ============================================================

/// 脚本中的值类型，用于 set 命令的右值
#[derive(Debug, Clone)]
pub enum ExprValue {
    /// 整数字面量
    Int(i64),
    /// 字符串字面量
    Str(String),
    /// 变量引用（按名称从上下文查找）
    Var(String),
}

// ============================================================
// 表达式类型（用于 for/while 条件判断）
// ============================================================

/// 简单条件表达式，用于 for/while 循环条件
#[derive(Debug, Clone)]
pub enum Expr {
    /// 变量与整数比较：(变量名, 比较运算符, 整数值)
    CompareVar(String, CompareOp, i64),
    /// 变量与变量比较：(左变量名, 比较运算符, 右变量名)
    CompareVarVar(String, CompareOp, String),
    /// 纯变量真值判断（非零为真）
    VarTruthy(String),
    /// 纯整数真值判断
    IntTruthy(i64),
}

/// 比较运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CompareOp {
    /// 执行比较操作
    pub fn eval(self, left: i64, right: i64) -> bool {
        match self {
            CompareOp::Eq => left == right,
            CompareOp::Ne => left != right,
            CompareOp::Lt => left < right,
            CompareOp::Le => left <= right,
            CompareOp::Gt => left > right,
            CompareOp::Ge => left >= right,
        }
    }
}

impl Expr {
    /// 根据上下文变量求值，返回布尔结果
    pub fn eval(&self, variables: &HashMap<String, i64>) -> bool {
        match self {
            Expr::CompareVar(var_name, op, value) => {
                let var_val = variables.get(var_name).copied().unwrap_or(0);
                op.eval(var_val, *value)
            }
            Expr::CompareVarVar(left_name, op, right_name) => {
                let left_val = variables.get(left_name).copied().unwrap_or(0);
                let right_val = variables.get(right_name).copied().unwrap_or(0);
                op.eval(left_val, right_val)
            }
            Expr::VarTruthy(var_name) => {
                variables.get(var_name).copied().unwrap_or(0) != 0
            }
            Expr::IntTruthy(value) => *value != 0,
        }
    }
}

// ============================================================
// 调用栈帧
// ============================================================

/// 函数调用栈帧，记录返回位置和参数
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// 返回到的命令索引
    pub return_index: usize,
    /// 传递给函数的参数列表
    pub args: Vec<i64>,
}

// ============================================================
// 循环上下文（用于 break/continue 跳转）
// ============================================================

/// 循环上下文，记录循环的条件检查点和结束位置
#[derive(Debug, Clone)]
pub struct LoopContext {
    /// ForStart 命令的索引（continue 跳转目标）
    pub condition_index: usize,
    /// ForEnd 命令的索引（break 跳转目标）
    pub end_index: usize,
}

// ============================================================
// 脚本上下文（NPC 脚本执行时可访问的玩家数据）
// ============================================================

/// NPC 脚本可访问的玩家上下文信息
/// 当前为 stub 实现，未来将连接到真实的玩家数据系统
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// 角色 ID
    pub char_id: u32,
    /// 角色名
    pub char_name: String,
    /// 公会名
    pub guild_name: String,
    /// 队伍名
    pub party_name: String,
    /// Base Level
    pub base_level: u16,
    /// Job Level
    pub job_level: u16,
    /// Zeny 数量
    pub zeny: u32,
    /// 当前 HP
    pub current_hp: u32,
    /// 最大 HP
    pub max_hp: u32,
    /// 当前 SP
    pub current_sp: u32,
    /// 最大 SP
    pub max_sp: u32,
    /// 物品数量映射（item_id -> amount）
    pub inventory: std::collections::HashMap<u16, u16>,
    /// NPC 变量映射（variable_name -> value）
    pub npc_variables: std::collections::HashMap<String, i32>,
    /// 是否需要广播
    pub need_broadcast: bool,
    /// 广播消息
    pub broadcast_message: String,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self {
            char_id: 100001,
            char_name: "Player".to_string(),
            guild_name: String::new(),
            party_name: String::new(),
            base_level: 1,
            job_level: 1,
            zeny: 0,
            current_hp: 100,
            max_hp: 100,
            current_sp: 50,
            max_sp: 50,
            inventory: std::collections::HashMap::new(),
            npc_variables: std::collections::HashMap::new(),
            need_broadcast: false,
            broadcast_message: String::new(),
        }
    }
}

// ============================================================
// 脚本节点 & 解析错误（保持原有结构不变）
// ============================================================

/// 脚本节点
#[derive(Debug, Clone)]
pub struct ScriptNode {
    pub commands: Vec<ScriptCommand>,
    pub labels: HashMap<String, usize>,
}

/// NPC 脚本
#[derive(Debug, Clone)]
pub struct NpcScript {
    pub npc_id: u32,
    pub script: ScriptNode,
}

/// 解析错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 行号（1-based）
    pub line: usize,
    /// 原始行内容
    pub source: String,
    /// 错误描述
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "第 {} 行解析错误: {} (原文: '{}')", self.line, self.message, self.source)
    }
}

impl std::error::Error for ParseError {}
