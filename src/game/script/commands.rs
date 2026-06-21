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

    // ================================================================
    // rAthena 兼容命令（第一批：核心 NPC 交互）
    // ================================================================

    // ---- rAthena 对话命令 ----
    /// 显示聊天消息（chatmes）
    ChatMes(String),
    /// 显示错误消息（errormes）
    ErrorMes(String),
    /// 显示带颜色的消息（mes with color）
    ColoredMes(String, u32),

    // ---- rAthena 角色操作 ----
    /// 获取角色 ID（attachrid）
    AttachRid(u32),
    /// 分离角色（detachrid）
    DetachRid,
    /// 获取角色 RID
    GetRid(String),
    /// 添加 RID 到列表（addrid）
    AddRid(u32),
    /// 获取在线角色 RID 列表
    GetOnlineRids(String),
    /// 改变角色性别（changesex）
    ChangeSex,
    /// 改变角色职业（changebase）
    ChangeBase(u16),
    /// 改变角色头发（changecharsex）
    ChangeCharSex(u8),

    // ---- rAthena 物品操作 ----
    /// 自动拾取（autoloot）
    AutoLoot(u8),
    /// 检查购物车（checkcart）
    CheckCart(String),
    /// 检查骑乘（checkriding）
    CheckRiding(String),
    /// 检查伪装（checkcloaking）
    CheckCloaking(String),
    /// 检查伪装2（checkchaosstatus）
    CheckChaosStatus(String),
    /// 购买商店物品（buyingstore）
    BuyingStore(u16),
    /// 打开商店（callshop）
    CallShop(u16),
    /// 打破装备（breakequip）
    BreakEquip(u16),
    /// 自动装备（autoequip）
    AutoEquip(u16, u8),
    /// 获取物品数量（countitem）
    /// 获取物品数量2（countitem2）
    /// 获取购物车物品数量（cartcountitem）
    CartCountItem(u16, String),
    /// 获取物品卡片数量（cardscnt）
    CardsCnt(u16, String),

    // ---- rAthena 状态/技能操作 ----
    /// 添加状态效果（bonus）
    Bonus(u16, i32),
    /// 添加状态脚本（bonus_script）
    BonusScript(String, u32),
    /// 清除状态脚本（bonus_script_clear）
    BonusScriptClear,
    /// 获取状态效果信息
    GetBonus(u16, String),
    /// 添加精神球（addspiritball）
    AddSpiritBall(u16, u32),
    /// 删除精神球（delspiritball）
    DelSpiritBall(u16, u32),
    /// 获取精神球数量（countspiritball）
    CountSpiritBall(String),

    // ---- rAthena 怪物操作 ----
    /// 召唤怪物（areamonster）
    AreaMonster(String, u16, u16, u16, String, u16, String),
    /// 怪物使用技能（areamobuseskill）
    AreaMobUseSkill(String, u16, u16, u16, u16, u16),
    /// 添加怪物掉落（addmonsterdrop）
    AddMonsterDrop(u16, u16, u16),
    /// 设置怪物属性
    SetMonsterData(u16, String, i32),
    /// 获取怪物属性
    GetMonsterData(u16, String, String),

    // ---- rAthena 地图操作 ----
    /// 区域公告（areaannounce）
    AreaAnnounce(String, u16, u16, u16, u16, String, u8),
    /// 区域治愈（areapercentheal）
    AreaPercentHeal(String, u16, u16, u16, u16, u8, u8),
    /// 检查单元格（checkcell）
    CheckCell(String, u16, u16, u16, String),
    /// 设置单元格属性（setcell）
    SetCell(String, u16, u16, u16, u16, u8),
    /// 获取单元格属性（getcell）
    GetCell(String, u16, u16, u16, String),

    // ---- rAthena 公会战命令 ----
    /// 检查公会战状态（agitcheck）
    AgitCheck(String),
    /// 检查公会战状态2（agitcheck2）
    AgitCheck2(String),
    /// 检查公会战状态3（agitcheck3）
    AgitCheck3(String),
    /// 开始公会战（agitstart）
    AgitStart,
    /// 开始公会战2（agitstart2）
    AgitStart2,
    /// 开始公会战3（agitstart3）
    AgitStart3,
    /// 结束公会战（agitend）
    AgitEnd,
    /// 结束公会战2（agitend2）
    AgitEnd2,
    /// 结束公会战3（agitend3）
    AgitEnd3,

    // ---- rAthena 频道命令 ----
    /// 创建频道（channel_create）
    ChannelCreate(String, String),
    /// 删除频道（channel_delete）
    ChannelDelete(String),
    /// 加入频道（channel_join）
    ChannelJoin(String),
    /// 踢出频道（channel_kick）
    ChannelKick(String, String),
    /// 封禁频道（channel_ban）
    ChannelBan(String, String),
    /// 解封频道（channel_unban）
    ChannelUnban(String, String),
    /// 频道聊天（channel_chat）
    ChannelChat(String, String),
    /// 设置频道颜色（channel_setcolor）
    ChannelSetColor(String, u32),
    /// 设置频道选项（channel_setopt）
    ChannelSetOpt(String, String, String),
    /// 设置频道密码（channel_setpass）
    ChannelSetPass(String, String),
    /// 设置频道权限组（channel_setgroup）
    ChannelSetGroup(String, String),
    /// 获取频道选项（channel_getopt）
    ChannelGetOpt(String, String, String),

    // ---- rAthena 战场命令 ----
    /// 创建战场（bg_create）
    BgCreate(String, String),
    /// 销毁战场（bg_destroy）
    BgDestroy(u16),
    /// 加入战场（bg_join）
    BgJoin(u16, String),
    /// 离开战场（bg_leave）
    BgLeave,
    /// 传送战场（bg_warp）
    BgWarp(u16, String, u16, u16),
    /// 召唤战场怪物（bg_monster）
    BgMonster(u16, String, u16, u16, String, u16),
    /// 设置战场怪物队伍（bg_monster_set_team）
    BgMonsterSetTeam(u16, u16),
    /// 获取战场数据（bg_get_data）
    BgGetData(u16, String, String),
    /// 获取战场区域用户（bg_getareausers）
    BgGetAreaUsers(u16, String, String),
    /// 获取战场信息（bg_info）
    BgInfo(u16, String),
    /// 预订战场（bg_reserve）
    BgReserve(String),
    /// 取消预订战场（bg_unbook）
    BgUnbook(String),
    /// 设置战场队伍位置（bg_team_setxy）
    BgTeamSetXy(u16, u16, u16),
    /// 更新战场得分（bg_updatescore）
    BgUpdateScore(String, u16, u16),

    // ---- rAthena 宠物命令 ----
    /// 捕获宠物（catchpet）
    CatchPet,
    /// 孵化宠物（birthpet）
    BirthPet(u16),
    /// 获取宠物信息
    GetPetInfo(String),
    /// 设置宠物信息
    SetPetInfo(String, String),

    // ---- rAthena 生命体命令 ----
    /// 获取生命体信息
    GetHomunInfo(String),
    /// 设置生命体信息
    SetHomunInfo(String, String),
    /// 增加生命体亲密度（addhomintimacy）
    AddHomIntimacy(u32),
    /// 获取生命体亲密度
    GetHomIntimacy(String),

    // ---- rAthena 佣兵命令 ----
    /// 获取佣兵信息
    GetMercInfo(String),
    /// 设置佣兵信息
    SetMercInfo(String, String),

    // ---- rAthena 任务命令 ----
    /// 改变任务（changequest）
    ChangeQuest(u16, u16),
    /// 打开任务 UI（open_quest_ui）
    OpenQuestUi(u16),

    // ---- rAthena 成就命令 ----
    /// 添加成就（achievementadd）
    AchievementAdd(u16),
    /// 完成成就（achievementcomplete）
    AchievementComplete(u16),
    /// 检查成就是否存在（achievementexists）
    AchievementExists(u16, String),
    /// 获取成就信息（achievementinfo）
    AchievementInfo(u16, String),
    /// 移除成就（achievementremove）
    AchievementRemove(u16),
    /// 更新成就（achievementupdate）
    AchievementUpdate(u16, String),
    /// 成就条件（achievement_condition）
    AchievementCondition(u16, String),

    // ---- rAthena 声望命令 ----
    /// 增加声望点数（add_reputation_points）
    AddReputationPoints(String, i32),
    /// 获取声望点数
    GetReputationPoints(String, String),

    // ---- rAthena 名声命令 ----
    /// 增加名声（addfame）
    AddFame(u32),
    /// 获取名声
    GetFame(String),

    // ---- rAthena 附魔命令 ----
    /// 获取附魔等级（enchantgradeui）
    EnchantGradeUi(u16),
    /// 设置附魔等级
    SetEnchantGrade(u16, u8),

    // ---- rAthena 合成命令 ----
    /// 合成物品（laphine_synthesis）
    LaphineSynthesis(u16),
    /// 升级物品（laphine_upgrade）
    LaphineUpgrade(u16),

    // ---- rAthena 造型师命令 ----
    /// 打开造型师（openstylist）
    OpenStylist,

    // ---- rAthena 摄像机命令 ----
    /// 获取摄像机信息（camerainfo）
    CameraInfo(String),
    /// 设置摄像机
    SetCamera(u16, u16, u16),

    // ---- rAthena 定时器命令 ----
    /// 添加定时器（addtimer）
    AddTimer(u32, String),
    /// 添加定时器计数（addtimercount）
    AddTimerCount(String, u32),
    /// 附加 NPC 定时器（attachnpctimer）
    AttachNpcTimer(String),
    /// 分离 NPC 定时器
    DetachNpcTimer,

    // ---- rAthena 字符串操作 ----
    /// 字符串转整数（atoi）
    Atoi(String, String),
    /// 十六进制转整数（axtoi）
    Axtoi(String, String),
    /// 获取字符（charat）
    CharAt(String, u32, String),
    /// 检查是否为字母（charisalpha）
    CharIsAlpha(String, String),
    /// 检查是否为小写（charislower）
    CharIsLower(String, String),
    /// 检查是否为大写（charisupper）
    CharIsUpper(String, String),

    // ---- rAthena 数学操作 ----
    /// 限制值范围（cap_value）
    CapValue(i64, i64, i64, String),

    // ---- rAthena 系统命令 ----
    /// 执行 GM 命令（atcommand）
    AtCommand(String),
    /// 绑定 GM 命令（bindatcmd）
    BindAtCmd(String, String),
    /// 唤醒 NPC（awake）
    Awake(String),
    /// 获取基础技能检查（basicskillcheck）
    BasicSkillCheck(String),

    // ---- rAthena 收购商店命令 ----
    /// 打开收购商店
    OpenBuyingStore(u16),
    /// 关闭收购商店
    CloseBuyingStore,
    /// 获取收购商店列表
    GetBuyingStoreList(String),

    // ---- rAthena 搜索商店命令 ----
    /// 搜索商店
    SearchStore(u16, String),
    /// 获取搜索结果
    GetSearchStoreResult(String),

    // ---- rAthena 领养命令 ----
    /// 领养孩子（adopt）
    Adopt(u32),
    /// 检查是否可以领养
    CanAdopt(String),

    // ---- rAthena 自动奖励命令 ----
    /// 设置自动奖励（autobonus）
    AutoBonus(String, u32, u16),
    /// 设置自动奖励2（autobonus2）
    AutoBonus2(String, u32, u16),
    /// 设置自动奖励3（autobonus3）
    AutoBonus3(String, u32, u16),
    /// 清除自动奖励
    ClearAutoBonus,

    // ================================================================
    // rAthena 兼容命令（第二批：高级功能）
    // ================================================================

    // ---- rAthena 脚本控制命令 ----
    /// 调用脚本函数（callfunc）
    CallFunc2(String, Vec<ExprValue>),
    /// 调用脚本子程序（callsub）
    CallSub2(String, Vec<ExprValue>),
    /// 获取函数参数（getarg）
    GetArg2(u16, ExprValue, String),
    /// 返回值（return）
    Return2(ExprValue),
    /// 跳转到标签（goto）
    Goto2(String),
    /// 条件跳转（goto_if）
    GotoIf2(String, Expr, String),

    // ---- rAthena 变量操作命令 ----
    /// 设置变量（set）
    Set2(String, ExprValue),
    /// 获取变量（getvariableofnpc）
    GetVariableOfNpc2(String, String, String),
    /// 设置数组（setarray）
    SetArray2(String, Vec<ExprValue>),
    /// 清空数组（cleararray）
    ClearArray2(String, ExprValue),
    /// 复制数组（copyarray）
    CopyArray2(String, String),
    /// 获取数组大小（getarraysize）
    GetArraySize2(String, String),
    /// 删除数组元素（deletearray）
    DeleteArray(String, u32),
    /// 插入数组元素（insertarray）
    InsertArray(String, u32, ExprValue),

    // ---- rAthena 字符串操作命令 ----
    /// 字符串长度（strlen）
    StrLen2(String, String),
    /// 字符串连接（strcharinfo）
    StrCharInfo2(u8, String),
    /// 字符串查找（charisalpha）
    CharIsAlpha2(String, String),
    /// 字符串截取（substr）
    SubStr(String, u32, u32, String),
    /// 字符串替换（strreplace）
    StrReplace2(String, String, String, String),
    /// 字符串分割（explode）
    Explode(String, String, String),
    /// 字符串连接（implode）
    Implode(String, String, String),
    /// 字符串转大写（strtoupper）
    StrToUpper2(String, String),
    /// 字符串转小写（strtolower）
    StrToLower2(String, String),
    /// 字符串去空格（trim）
    Trim2(String, String),
    /// 字符串格式化（sprintf）
    Sprintf(String, Vec<ExprValue>, String),

    // ---- rAthena 数学操作命令 ----
    /// 绝对值（abs）
    Abs2(i64, String),
    /// 最大值（max）
    Max2(i64, i64, String),
    /// 最小值（min）
    Min2(i64, i64, String),
    /// 幂运算（pow）
    Pow2(i64, i64, String),
    /// 平方根（sqrt）
    Sqrt2(i64, String),
    /// 随机数（rand）
    Rand2(i64, i64, String),
    /// 取模（mod）
    Mod2(i64, i64, String),
    /// 位运算（band/bor/bxor）
    Band2(i64, i64, String),
    Bor2(i64, i64, String),
    Bxor2(i64, i64, String),
    /// 位移运算（shiftl/shiftr）
    Shiftl2(i64, u8, String),
    Shiftr2(i64, u8, String),

    // ---- rAthena 时间操作命令 ----
    /// 获取系统时间（gettime）
    GetTime2(u8, String),
    /// 获取游戏时间（gettime）
    GetGameTime2(u8, String),
    /// 获取时间戳（gettimest）
    GetTimeSt(String),
    /// 检查是否为白天（isday）
    IsDay2(String),
    /// 检查是否为晚上（isnight）
    IsNight2(String),
    /// 获取星期几（getweekday）
    GetWeekday(String),

    // ---- rAthena 消息命令 ----
    /// 显示消息（message）
    Message2(String),
    /// 显示公告（announce）
    Announce2(String, u8),
    /// 区域公告（areaannounce）
    AreaAnnounce2(String, u16, u16, u16, u16, String, u8),
    /// 显示对话框（mes）
    Mes2(String),
    /// 显示下一行（next）
    Next2,
    /// 显示关闭按钮（close）
    /// 显示关闭按钮2（close2）
    Close22,
    /// 显示菜单（menu）
    Menu2(Vec<(String, String)>),
    /// 显示选择框（select）
    Select2(Vec<(String, String)>),
    /// 显示提示框（prompt）
    Prompt2(Vec<(String, String)>),

    // ---- rAthena 输入命令 ----
    /// 接收输入（input）
    Input2(String),
    /// 接收整数输入
    InputInt(String),
    /// 接收字符串输入
    InputStr(String),

    // ---- rAthena 延迟命令 ----
    /// 延迟执行（sleep）
    Sleep2(u32),
    /// 延迟执行2（sleep2）
    Sleep22(u32),
    /// 唤醒 NPC（awake）
    Awake2(String),
    /// 获取延迟状态
    GetSleepTick(String),

    // ---- rAthena NPC 命令 ----
    /// 获取 NPC 名称（getnpcid）
    GetNpcId(String),
    /// 获取 NPC 位置（getnpcpos）
    GetNpcPos2(String, String, String, String),
    /// 设置 NPC 位置
    SetNpcPos2(String, u16, u16),
    /// 设置 NPC 方向
    SetNpcDir2(String, u16),
    /// 设置 NPC 外观
    SetNpcSprite2(String, u16),
    /// 设置 NPC 可见性
    SetNpcVisible2(String, bool),
    /// 设置 NPC 可交互性
    SetNpcInteractable2(String, bool),
    /// 获取 NPC 状态
    GetNpcStatus2(String, String),
    /// 设置 NPC 状态
    SetNpcStatus2(String, String),
    /// 获取 NPC 数据
    GetNpcData(String, String),
    /// 设置 NPC 数据
    SetNpcData(String, String, String),

    // ---- rAthena 怪物命令 ----
    /// 召唤怪物（monster）
    Monster22(String, u16, u16, String, u16, String),
    /// 召唤怪物2（monster2）
    Monster222(String, u16, u16, String, u16, String, String),
    /// 杀死怪物（killmonster）
    KillMonster2(String, String),
    /// 杀死所有怪物（killmonsterall）
    KillMonsterAll2(String),
    /// 获取怪物信息
    GetMonsterInfo2(u16, String, String),
    /// 设置怪物信息
    SetMonsterInfo2(u16, String, i32),
    /// 怪物使用技能（mobuseskill）
    MobUseSkill(u16, u16, u8, u16),
    /// 获取怪物数量（mobcount）
    MobCount(String, String),

    // ---- rAthena 地图命令 ----
    /// 传送玩家（warp）
    Warp22(String, u16, u16),
    /// 区域传送（areawarp）
    AreaWarp2(String, u16, u16, u16, u16, String, u16, u16),
    /// 获取地图名称（strcharinfo）
    GetMapName2(String),
    /// 获取地图上玩家数量（getmapusers）
    GetMapUsers2(String, String),
    /// 获取地图上怪物数量（getmapmobs）
    GetMapMobs2(String, String),
    /// 获取地图宽度
    GetMapWidth2(String, String),
    /// 获取地图高度
    GetMapHeight2(String, String),
    /// 检查地图坐标是否可通行
    IsWalkable2(String, u16, u16, String),
    /// 设置地图属性
    SetMapFlag(String, String, bool),
    /// 获取地图属性
    GetMapFlag(String, String, String),

    // ---- rAthena 玩家命令 ----
    /// 获取玩家等级（baselevel）
    GetBaseLevel2(String),
    /// 获取玩家职业等级（joblevel）
    GetJobLevel2(String),
    /// 设置玩家等级（setbaselevel）
    SetBaseLevel2(u16),
    /// 设置玩家职业等级（setjoblevel）
    SetJobLevel2(u16),
    /// 获取玩家金币（getcharid）
    GetZeny2(String),
    /// 设置玩家金币（setzeny）
    SetZeny2(u32),
    /// 增加玩家金币（getcharid）
    AddZeny2(i32),
    /// 获取玩家 HP（hp）
    GetHp2(String),
    /// 获取玩家 SP（sp）
    GetSp2(String),
    /// 获取玩家最大 HP（maxhp）
    GetMaxHp2(String),
    /// 获取玩家最大 SP（maxsp）
    GetMaxSp2(String),
    /// 恢复 HP（heal）
    Heal2(u32, u32),
    /// 百分比恢复（percentheal）
    PercentHeal2(u8, u8),
    /// 获取玩家职业（class）
    GetClass2(String),
    /// 获取玩家性别（sex）
    GetSex2(String),
    /// 获取玩家位置（getcharid）
    GetCharPos(String, String, String, String),
    /// 获取玩家方向
    GetCharDir(String),
    /// 设置玩家方向
    SetCharDir(u16),
    /// 获取玩家速度
    GetCharSpeed(String),
    /// 设置玩家速度
    SetCharSpeed(u16),

    // ---- rAthena 公会命令 ----
    /// 获取公会 ID（getcharid）
    GetGuildId2(String),
    /// 获取公会名称（getcharid）
    GetGuildName2(String),
    /// 获取公会等级
    GetGuildLevel2(String),
    /// 设置公会等级
    SetGuildLevel2(u16),
    /// 增加公会经验值
    AddGuildExp2(u32),
    /// 获取公会成员数量
    GetGuildMemberCount2(String),
    /// 获取公会在线成员数量
    GetGuildOnlineCount2(String),
    /// 检查是否为公会会长
    IsGuildMaster2(String),
    /// 获取公会公告
    GetGuildNotice2(String),
    /// 设置公会公告
    SetGuildNotice2(String),
    /// 获取公会技能等级
    GetGuildSkillLevel2(u16, String),
    /// 设置公会技能等级
    SetGuildSkillLevel2(u16, u8),

    // ---- rAthena 队伍命令 ----
    /// 获取队伍 ID（getcharid）
    GetPartyId2(String),
    /// 获取队伍名称（getcharid）
    GetPartyName2(String),
    /// 获取队伍成员数量
    GetPartyMemberCount2(String),
    /// 获取队伍在线成员数量
    GetPartyOnlineCount2(String),
    /// 检查是否为队长
    IsPartyLeader2(String),
    /// 获取队伍经验值分配模式
    GetPartyExpPolicy2(String),
    /// 设置队伍经验值分配模式
    SetPartyExpPolicy2(u8),
    /// 获取队伍物品分配模式
    GetPartyItemPolicy2(String),
    /// 设置队伍物品分配模式
    SetPartyItemPolicy2(u8),

    // ---- rAthena 宠物命令 ----
    /// 获取宠物等级
    GetPetLevel2(String),
    /// 获取宠物经验值
    GetPetExp2(String),
    /// 增加宠物经验值
    AddPetExp2(u32),
    /// 获取宠物饥饿度
    GetPetHunger2(String),
    /// 设置宠物饥饿度
    SetPetHunger2(u16),
    /// 获取宠物亲密度
    GetPetIntimacy2(String),
    /// 设置宠物亲密度
    SetPetIntimacy2(u32),
    /// 获取宠物装备
    GetPetEquip2(String),
    /// 设置宠物装备
    SetPetEquip2(u16),

    // ---- rAthena 生命体命令 ----
    /// 获取生命体等级
    GetHomunLevel(String),
    /// 获取生命体经验值
    GetHomunExp(String),
    /// 增加生命体经验值
    AddHomunExp(u32),
    /// 获取生命体亲密度
    GetHomunIntimacy(String),
    /// 设置生命体亲密度
    SetHomunIntimacy(u32),
    /// 获取生命体类型
    GetHomunType(String),
    /// 获取生命体进化状态
    GetHomunEvolution(String),

    // ---- rAthena 佣兵命令 ----
    /// 获取佣兵等级
    GetMercLevel(String),
    /// 获取佣兵经验值
    GetMercExp(String),
    /// 增加佣兵经验值
    AddMercExp(u32),
    /// 获取佣兵忠诚度
    GetMercLoyalty(String),
    /// 设置佣兵忠诚度
    SetMercLoyalty(u32),
    /// 获取佣兵剩余时间
    GetMercTimer(String),

    // ---- rAthena 任务命令 ----
    /// 添加任务（setquest）
    SetQuest(u16),
    /// 完成任务（completequest）
    /// 放弃任务（erasequest）
    EraseQuest(u16),
    /// 检查任务状态（checkquest）
    CheckQuest(u16, String),
    /// 改变任务（changequest）
    ChangeQuest2(u16, u16),
    /// 获取任务信息
    GetQuestInfo(u16, String),
    /// 设置任务信息
    SetQuestInfo(u16, String, String),

    // ---- rAthena 成就命令 ----
    /// 添加成就（achievementadd）
    AchievementAdd2(u16),
    /// 完成成就（achievementcomplete）
    AchievementComplete2(u16),
    /// 检查成就是否存在（achievementexists）
    AchievementExists2(u16, String),
    /// 获取成就信息（achievementinfo）
    AchievementInfo2(u16, String),
    /// 移除成就（achievementremove）
    AchievementRemove2(u16),
    /// 更新成就（achievementupdate）
    AchievementUpdate2(u16, String),

    // ---- rAthena 公会战命令 ----
    /// 检查公会战状态（agitcheck）
    AgitCheck22(String),
    /// 开始公会战（agitstart）
    AgitStart22,
    /// 结束公会战（agitend）
    AgitEnd22,
    /// 获取公会战信息
    GetAgitInfo(String),
    /// 获取公会战城堡信息
    GetCastleInfo(u16, String),

    // ---- rAthena 战场命令 ----
    /// 创建战场（bg_create）
    BgCreate2(String, String),
    /// 销毁战场（bg_destroy）
    BgDestroy2(u16),
    /// 加入战场（bg_join）
    BgJoin2(u16, String),
    /// 离开战场（bg_leave）
    BgLeave2,
    /// 传送战场（bg_warp）
    BgWarp2(u16, String, u16, u16),
    /// 召唤战场怪物（bg_monster）
    BgMonster2(u16, String, u16, u16, String, u16),
    /// 获取战场数据（bg_get_data）
    BgGetData2(u16, String, String),
    /// 获取战场信息（bg_info）
    BgInfo2(u16, String),

    // ---- rAthena 频道命令 ----
    /// 创建频道（channel_create）
    ChannelCreate2(String, String),
    /// 删除频道（channel_delete）
    ChannelDelete2(String),
    /// 加入频道（channel_join）
    ChannelJoin2(String),
    /// 踢出频道（channel_kick）
    ChannelKick2(String, String),
    /// 封禁频道（channel_ban）
    ChannelBan2(String, String),
    /// 解封频道（channel_unban）
    ChannelUnban2(String, String),
    /// 频道聊天（channel_chat）
    ChannelChat2(String, String),
    /// 设置频道颜色（channel_setcolor）
    ChannelSetColor2(String, u32),
    /// 设置频道选项（channel_setopt）
    ChannelSetOpt2(String, String, String),
    /// 设置频道密码（channel_setpass）
    ChannelSetPass2(String, String),
    /// 设置频道权限组（channel_setgroup）
    ChannelSetGroup2(String, String),

    // ---- rAthena 收购商店命令 ----
    /// 打开收购商店
    OpenBuyingStore2(u16),
    /// 关闭收购商店
    CloseBuyingStore2,
    /// 获取收购商店列表
    GetBuyingStoreList2(String),

    // ---- rAthena 搜索商店命令 ----
    /// 搜索商店
    SearchStore2(u16, String),
    /// 获取搜索结果
    GetSearchStoreResult2(String),

    // ---- rAthena 拍卖命令 ----
    /// 获取拍卖列表
    GetAuctionList2(String),
    /// 上架拍卖
    AuctionAddItem2(u16, u16, u32),
    /// 竞拍
    AuctionBid2(u32, u32),
    /// 获取拍卖信息
    GetAuctionInfo2(u32, String),
    /// 领取拍卖物品
    AuctionClaimItem2(u32),

    // ---- rAthena 摆摊命令 ----
    /// 打开摆摊界面
    OpenVending2,
    /// 获取摆摊物品列表
    GetVendingItems2(String),
    /// 购买摆摊物品
    BuyVendingItem2(u32, u16),
    /// 获取摆摊信息
    GetVendingInfo2(String),
    /// 关闭摆摊
    CloseVending2,

    // ---- rAthena 邮件命令 ----
    /// 发送邮件
    SendMail2(String, String, String),
    /// 读取邮件
    ReadMail2(u32, String),
    /// 删除邮件
    DeleteMail2(u32),
    /// 附件收取
    GetMailAttachment2(u32),
    /// 获取邮件数量
    GetMailCount2(String),

    // ---- rAthena 交易命令 ----
    /// 发起交易请求
    TradeRequest2(u32),
    /// 接受交易
    TradeAccept2,
    /// 拒绝交易
    TradeReject2,
    /// 添加交易物品
    TradeAddItem2(u16, u16),
    /// 设置交易金币
    TradeSetZeny2(u32),

    // ---- rAthena 聊天命令 ----
    /// 发送聊天消息
    ChatMessage2(String),
    /// 发送私聊消息
    WhisperMessage2(String, String),
    /// 创建聊天室
    CreateChatRoom2(String, u16, String),
    /// 加入聊天室
    JoinChatRoom2(String),
    /// 离开聊天室
    LeaveChatRoom2,

    // ---- rAthena 副本命令 ----
    /// 创建副本
    CreateInstance2(u16, String),
    /// 进入副本
    EnterInstance2(u16),
    /// 离开副本
    LeaveInstance2,
    /// 获取副本信息
    GetInstanceInfo2(String),
    /// 检查副本是否活跃
    IsInstanceActive2(String),

    // ---- rAthena 商城命令 ----
    /// 获取商城物品列表
    GetCashShopItems2(String),
    /// 购买商城物品
    BuyCashItem2(u16, u32),
    /// 获取玩家点数
    GetCashPoints2(String),
    /// 增加玩家点数
    AddCashPoints2(u32),
    /// 消耗玩家点数
    ConsumeCashPoints2(u32, String),

    // ---- rAthena Kafra 命令 ----
    /// 打开仓库
    OpenStorage2,
    /// 获取仓库物品数量
    GetStorageItemCount2(u16, String),
    /// 存入物品
    StorageAdd2(u16, u16),
    /// 取出物品
    StorageRemove2(u16, u16),
    /// 获取仓库容量
    GetStorageCapacity2(String),

    // ---- rAthena 声望命令 ----
    /// 增加声望点数
    AddReputationPoints2(String, i32),
    /// 获取声望点数
    GetReputationPoints2(String, String),

    // ---- rAthena 名声命令 ----
    /// 增加名声
    AddFame2(u32),
    /// 获取名声
    GetFame2(String),

    // ---- rAthena 附魔命令 ----
    /// 获取附魔等级
    EnchantGradeUi2(u16),
    /// 设置附魔等级
    SetEnchantGrade2(u16, u8),

    // ---- rAthena 合成命令 ----
    /// 合成物品
    LaphineSynthesis2(u16),
    /// 升级物品
    LaphineUpgrade2(u16),

    // ---- rAthena 造型师命令 ----
    /// 打开造型师
    OpenStylist2,

    // ---- rAthena 摄像机命令 ----
    /// 获取摄像机信息
    CameraInfo2(String),
    /// 设置摄像机
    SetCamera2(u16, u16, u16),

    // ---- rAthena 定时器命令 ----
    /// 添加定时器
    AddTimer2(u32, String),
    /// 添加定时器计数
    AddTimerCount2(String, u32),
    /// 附加 NPC 定时器
    AttachNpcTimer2(String),
    /// 分离 NPC 定时器
    DetachNpcTimer2,

    // ---- rAthena 系统命令 ----
    /// 执行 GM 命令
    AtCommand2(String),
    /// 绑定 GM 命令
    BindAtCmd2(String, String),
    /// 获取基础技能检查
    BasicSkillCheck2(String),
    /// 获取服务器信息
    GetServerInfo(String),

    // ---- rAthena 领养命令 ----
    /// 领养孩子
    Adopt2(u32),
    /// 检查是否可以领养
    CanAdopt2(String),

    // ================================================================
    // rAthena 兼容命令（第三批：补齐剩余命令）
    // ================================================================

    // ---- rAthena 高级物品命令 ----
    /// 获取物品 Aegis 名称
    GetItemName(u16, String),
    /// 获取物品描述
    GetItemDesc(u16, String),
    /// 获取物品类型
    GetItemType(u16, String),
    /// 获取物品价格
    GetItemPrice(u16, String),
    /// 获取物品重量
    GetItemWeight(u16, String),
    /// 获取物品攻击
    GetItemAttack(u16, String),
    /// 获取物品防御
    GetItemDefense(u16, String),
    /// 获取物品范围
    GetItemRange(u16, String),
    /// 获取物品槽位数
    GetItemSlot(u16, String),
    /// 获取物品性别限制
    GetItemGender(u16, String),
    /// 获取物品等级限制
    GetItemLevel(u16, String),
    /// 获取物品职业限制
    GetItemJob(u16, String),
    /// 获取物品位置限制
    GetItemLoc(u16, String),
    /// 获取物品卡片数量
    GetItemCard(u16, u8, String),
    /// 获取物品脚本
    GetItemScript(u16, String),
    /// 获取物品装备脚本
    GetItemEquipScript(u16, String),
    /// 获取物品卸下脚本
    GetItemUnequipScript(u16, String),

    // ---- rAthena 高级技能命令 ----
    /// 获取技能名称
    GetSkillName(u16, String),
    /// 获取技能描述
    GetSkillDesc(u16, String),
    /// 获取技能类型
    GetSkillType(u16, String),
    /// 获取技能目标类型
    GetSkillTargetType(u16, String),
    /// 获取技能伤害类型
    GetSkillDamageType(u16, String),
    /// 获取技能元素
    GetSkillElement(u16, String),
    /// 获取技能范围
    GetSkillRange2(u16, String),
    /// 获取技能消耗
    GetSkillCost2(u16, String),
    /// 获取技能冷却时间
    GetSkillCooldown2(u16, String),
    /// 获取技能施法时间
    GetSkillCastTime(u16, String),
    /// 获取技能持续时间
    GetSkillDuration(u16, String),
    /// 获取技能效果
    GetSkillEffect(u16, String),

    // ---- rAthena 高级怪物命令 ----
    /// 获取怪物名称
    GetMonsterName(u16, String),
    /// 获取怪物等级
    GetMonsterLevel(u16, String),
    /// 获取怪物 HP
    GetMonsterHp(u16, String),
    /// 获取怪物攻击
    GetMonsterAttack(u16, String),
    /// 获取怪物防御
    GetMonsterDefense(u16, String),
    /// 获取怪物经验值
    GetMonsterExp(u16, String),
    /// 获取怪物掉落
    GetMonsterDrop(u16, String),
    /// 获取怪物大小
    GetMonsterSize(u16, String),
    /// 获取怪物种族
    GetMonsterRace(u16, String),
    /// 获取怪物元素
    GetMonsterElement(u16, String),
    /// 获取怪物 AI 类型
    GetMonsterAi(u16, String),

    // ---- rAthena 高级状态命令 ----
    /// 获取状态效果名称
    GetStatusName(u16, String),
    /// 获取状态效果描述
    GetStatusDesc(u16, String),
    /// 获取状态效果类型
    GetStatusType(u16, String),
    /// 获取状态效果图标
    GetStatusIcon(u16, String),
    /// 获取状态效果颜色
    GetStatusColor(u16, String),
    /// 获取状态效果持续时间
    GetStatusDuration(u16, String),
    /// 获取状态效果剩余时间
    GetStatusRemaining(u16, String),
    /// 获取状态效果层数
    GetStatusStack2(u16, String),
    /// 获取状态效果来源
    GetStatusSource(u16, String),

    // ---- rAthena 高级战斗命令 ----
    /// 计算物理伤害
    CalcPhysicalDamage2(u16, u16, String),
    /// 计算魔法伤害
    CalcMagicDamage2(u16, u16, String),
    /// 计算治愈量
    CalcHealAmount2(u16, u8, String),
    /// 获取命中率
    GetHitRate2(u16, String),
    /// 获取闪避率
    GetFleeRate2(u16, String),
    /// 获取暴击率
    GetCriticalRate2(u16, String),
    /// 获取攻击速度
    GetAttackSpeed2(String),
    /// 获取移动速度
    GetMoveSpeed2(String),
    /// 获取攻击力
    GetAttack(String),
    /// 获取魔法攻击力
    GetMagicAttack(String),
    /// 获取防御力
    GetDefense(String),
    /// 获取魔法防御力
    GetMagicDefense(String),

    // ---- rAthena 高级地图命令 ----
    /// 获取地图名称
    GetMapName3(String),
    /// 获取地图上玩家数量
    GetMapUsers3(String, String),
    /// 获取地图上怪物数量
    GetMapMobs3(String, String),
    /// 获取地图宽度
    GetMapWidth3(String, String),
    /// 获取地图高度
    GetMapHeight3(String, String),
    /// 检查地图坐标是否可通行
    IsWalkable3(String, u16, u16, String),
    /// 设置地图属性
    SetMapFlag2(String, String, bool),
    /// 获取地图属性
    GetMapFlag2(String, String, String),
    /// 获取地图区域玩家
    GetAreaPlayers2(String, u16, u16, u16, String),
    /// 获取地图区域怪物
    GetAreaMobs2(String, u16, u16, u16, String),
    /// 在地图上显示特效
    ShowEffect2(String, u16, u16, u16),
    /// 在地图上显示动画
    ShowAnimation2(String, u16, u16, u16),

    // ---- rAthena 高级 NPC 命令 ----
    /// 获取 NPC 名称
    GetNpcName2(String, String),
    /// 获取 NPC 位置
    GetNpcPos3(String, String, String, String),
    /// 设置 NPC 位置
    SetNpcPos3(String, u16, u16),
    /// 设置 NPC 方向
    SetNpcDir3(String, u16),
    /// 设置 NPC 外观
    SetNpcSprite3(String, u16),
    /// 设置 NPC 可见性
    SetNpcVisible3(String, bool),
    /// 设置 NPC 可交互性
    SetNpcInteractable3(String, bool),
    /// 获取 NPC 状态
    GetNpcStatus3(String, String),
    /// 设置 NPC 状态
    SetNpcStatus3(String, String),
    /// 获取 NPC 数据
    GetNpcData2(String, String),
    /// 设置 NPC 数据
    SetNpcData2(String, String, String),

    // ---- rAthena 高级公会命令 ----
    /// 获取公会 ID
    GetGuildId3(String),
    /// 获取公会名称
    GetGuildName3(String),
    /// 获取公会等级
    GetGuildLevel3(String),
    /// 设置公会等级
    SetGuildLevel3(u16),
    /// 增加公会经验值
    AddGuildExp3(u32),
    /// 获取公会成员数量
    GetGuildMemberCount3(String),
    /// 获取公会在线成员数量
    GetGuildOnlineCount3(String),
    /// 检查是否为公会会长
    IsGuildMaster3(String),
    /// 获取公会公告
    GetGuildNotice3(String),
    /// 设置公会公告
    SetGuildNotice3(String),
    /// 获取公会技能等级
    GetGuildSkillLevel3(u16, String),
    /// 设置公会技能等级
    SetGuildSkillLevel3(u16, u8),

    // ---- rAthena 高级队伍命令 ----
    /// 获取队伍 ID
    GetPartyId3(String),
    /// 获取队伍名称
    GetPartyName3(String),
    /// 获取队伍成员数量
    GetPartyMemberCount3(String),
    /// 获取队伍在线成员数量
    GetPartyOnlineCount3(String),
    /// 检查是否为队长
    IsPartyLeader3(String),
    /// 获取队伍经验值分配模式
    GetPartyExpPolicy3(String),
    /// 设置队伍经验值分配模式
    SetPartyExpPolicy3(u8),
    /// 获取队伍物品分配模式
    GetPartyItemPolicy3(String),
    /// 设置队伍物品分配模式
    SetPartyItemPolicy3(u8),

    // ---- rAthena 高级宠物命令 ----
    /// 获取宠物等级
    GetPetLevel3(String),
    /// 获取宠物经验值
    GetPetExp3(String),
    /// 增加宠物经验值
    AddPetExp3(u32),
    /// 获取宠物饥饿度
    GetPetHunger3(String),
    /// 设置宠物饥饿度
    SetPetHunger3(u16),
    /// 获取宠物亲密度
    GetPetIntimacy3(String),
    /// 设置宠物亲密度
    SetPetIntimacy3(u32),
    /// 获取宠物装备
    GetPetEquip3(String),
    /// 设置宠物装备
    SetPetEquip3(u16),

    // ---- rAthena 高级生命体命令 ----
    /// 获取生命体等级
    GetHomunLevel2(String),
    /// 获取生命体经验值
    GetHomunExp2(String),
    /// 增加生命体经验值
    AddHomunExp2(u32),
    /// 获取生命体亲密度
    GetHomunIntimacy2(String),
    /// 设置生命体亲密度
    SetHomunIntimacy2(u32),
    /// 获取生命体类型
    GetHomunType2(String),
    /// 获取生命体进化状态
    GetHomunEvolution2(String),

    // ---- rAthena 高级佣兵命令 ----
    /// 获取佣兵等级
    GetMercLevel2(String),
    /// 获取佣兵经验值
    GetMercExp2(String),
    /// 增加佣兵经验值
    AddMercExp2(u32),
    /// 获取佣兵忠诚度
    GetMercLoyalty2(String),
    /// 设置佣兵忠诚度
    SetMercLoyalty2(u32),
    /// 获取佣兵剩余时间
    GetMercTimer2(String),

    // ---- rAthena 高级任务命令 ----
    /// 添加任务
    SetQuest2(u16),
    /// 完成任务
    CompleteQuest2(u16),
    /// 放弃任务
    EraseQuest2(u16),
    /// 检查任务状态
    CheckQuest2(u16, String),
    /// 改变任务
    ChangeQuest3(u16, u16),
    /// 获取任务信息
    GetQuestInfo2(u16, String),
    /// 设置任务信息
    SetQuestInfo2(u16, String, String),

    // ---- rAthena 高级成就命令 ----
    /// 添加成就
    AchievementAdd3(u16),
    /// 完成成就
    AchievementComplete3(u16),
    /// 检查成就是否存在
    AchievementExists3(u16, String),
    /// 获取成就信息
    AchievementInfo3(u16, String),
    /// 移除成就
    AchievementRemove3(u16),
    /// 更新成就
    AchievementUpdate3(u16, String),

    // ---- rAthena 高级公会战命令 ----
    /// 检查公会战状态
    /// 开始公会战
    /// 结束公会战
    /// 获取公会战信息
    GetAgitInfo2(String),
    /// 获取公会战城堡信息
    GetCastleInfo2(u16, String),

    // ---- rAthena 高级战场命令 ----
    /// 创建战场
    BgCreate3(String, String),
    /// 销毁战场
    BgDestroy3(u16),
    /// 加入战场
    BgJoin3(u16, String),
    /// 离开战场
    BgLeave3,
    /// 传送战场
    BgWarp3(u16, String, u16, u16),
    /// 召唤战场怪物
    BgMonster3(u16, String, u16, u16, String, u16),
    /// 获取战场数据
    BgGetData3(u16, String, String),
    /// 获取战场信息
    BgInfo3(u16, String),

    // ---- rAthena 高级频道命令 ----
    /// 创建频道
    ChannelCreate3(String, String),
    /// 删除频道
    ChannelDelete3(String),
    /// 加入频道
    ChannelJoin3(String),
    /// 踢出频道
    ChannelKick3(String, String),
    /// 封禁频道
    ChannelBan3(String, String),
    /// 解封频道
    ChannelUnban3(String, String),
    /// 频道聊天
    ChannelChat3(String, String),
    /// 设置频道颜色
    ChannelSetColor3(String, u32),
    /// 设置频道选项
    ChannelSetOpt3(String, String, String),
    /// 设置频道密码
    ChannelSetPass3(String, String),
    /// 设置频道权限组
    ChannelSetGroup3(String, String),

    // ---- rAthena 高级收购商店命令 ----
    /// 打开收购商店
    OpenBuyingStore3(u16),
    /// 关闭收购商店
    CloseBuyingStore3,
    /// 获取收购商店列表
    GetBuyingStoreList3(String),

    // ---- rAthena 高级搜索商店命令 ----
    /// 搜索商店
    SearchStore3(u16, String),
    /// 获取搜索结果
    GetSearchStoreResult3(String),

    // ---- rAthena 高级拍卖命令 ----
    /// 获取拍卖列表
    GetAuctionList3(String),
    /// 上架拍卖
    AuctionAddItem3(u16, u16, u32),
    /// 竞拍
    AuctionBid3(u32, u32),
    /// 获取拍卖信息
    GetAuctionInfo3(u32, String),
    /// 领取拍卖物品
    AuctionClaimItem3(u32),

    // ---- rAthena 高级摆摊命令 ----
    /// 打开摆摊界面
    OpenVending3,
    /// 获取摆摊物品列表
    GetVendingItems3(String),
    /// 购买摆摊物品
    BuyVendingItem3(u32, u16),
    /// 获取摆摊信息
    GetVendingInfo3(String),
    /// 关闭摆摊
    CloseVending3,

    // ---- rAthena 高级邮件命令 ----
    /// 发送邮件
    SendMail3(String, String, String),
    /// 读取邮件
    ReadMail3(u32, String),
    /// 删除邮件
    DeleteMail3(u32),
    /// 附件收取
    GetMailAttachment3(u32),
    /// 获取邮件数量
    GetMailCount3(String),

    // ---- rAthena 高级交易命令 ----
    /// 发起交易请求
    TradeRequest3(u32),
    /// 接受交易
    TradeAccept3,
    /// 拒绝交易
    TradeReject3,
    /// 添加交易物品
    TradeAddItem3(u16, u16),
    /// 设置交易金币
    TradeSetZeny3(u32),

    // ---- rAthena 高级聊天命令 ----
    /// 发送聊天消息
    ChatMessage3(String),
    /// 发送私聊消息
    WhisperMessage3(String, String),
    /// 创建聊天室
    CreateChatRoom3(String, u16, String),
    /// 加入聊天室
    JoinChatRoom3(String),
    /// 离开聊天室
    LeaveChatRoom3,

    // ---- rAthena 高级副本命令 ----
    /// 创建副本
    CreateInstance3(u16, String),
    /// 进入副本
    EnterInstance3(u16),
    /// 离开副本
    LeaveInstance3,
    /// 获取副本信息
    GetInstanceInfo3(String),
    /// 检查副本是否活跃
    IsInstanceActive3(String),

    // ---- rAthena 高级商城命令 ----
    /// 获取商城物品列表
    GetCashShopItems3(String),
    /// 购买商城物品
    BuyCashItem3(u16, u32),
    /// 获取玩家点数
    GetCashPoints3(String),
    /// 增加玩家点数
    AddCashPoints3(u32),
    /// 消耗玩家点数
    ConsumeCashPoints3(u32, String),

    // ---- rAthena 高级 Kafra 命令 ----
    /// 打开仓库
    OpenStorage3,
    /// 获取仓库物品数量
    GetStorageItemCount3(u16, String),
    /// 存入物品
    StorageAdd3(u16, u16),
    /// 取出物品
    StorageRemove3(u16, u16),
    /// 获取仓库容量
    GetStorageCapacity3(String),

    // ---- rAthena 高级声望命令 ----
    /// 增加声望点数
    AddReputationPoints3(String, i32),
    /// 获取声望点数
    GetReputationPoints3(String, String),

    // ---- rAthena 高级名声命令 ----
    /// 增加名声
    AddFame3(u32),
    /// 获取名声
    GetFame3(String),

    // ---- rAthena 高级附魔命令 ----
    /// 获取附魔等级
    EnchantGradeUi3(u16),
    /// 设置附魔等级
    SetEnchantGrade3(u16, u8),

    // ---- rAthena 高级合成命令 ----
    /// 合成物品
    LaphineSynthesis3(u16),
    /// 升级物品
    LaphineUpgrade3(u16),

    // ---- rAthena 高级造型师命令 ----
    /// 打开造型师
    OpenStylist3,

    // ---- rAthena 高级摄像机命令 ----
    /// 获取摄像机信息
    CameraInfo3(String),
    /// 设置摄像机
    SetCamera3(u16, u16, u16),

    // ---- rAthena 高级定时器命令 ----
    /// 添加定时器
    AddTimer3(u32, String),
    /// 添加定时器计数
    AddTimerCount3(String, u32),
    /// 附加 NPC 定时器
    AttachNpcTimer3(String),
    /// 分离 NPC 定时器
    DetachNpcTimer3,

    // ---- rAthena 高级系统命令 ----
    /// 执行 GM 命令
    AtCommand3(String),
    /// 绑定 GM 命令
    BindAtCmd3(String, String),
    /// 获取基础技能检查
    BasicSkillCheck3(String),
    /// 获取服务器信息
    GetServerInfo2(String),

    // ---- rAthena 高级领养命令 ----
    /// 领养孩子
    Adopt3(u32),
    /// 检查是否可以领养
    CanAdopt3(String),

    // ================================================================
    // rAthena 兼容命令（第四批：最终补齐）
    // ================================================================

    // ---- rAthena 最终物品命令 ----
    /// 获取物品 Aegis 名称
    GetItemName2(u16, String),
    /// 获取物品描述
    GetItemDesc2(u16, String),
    /// 获取物品类型
    GetItemType2(u16, String),
    /// 获取物品价格
    GetItemPrice2(u16, String),
    /// 获取物品重量
    GetItemWeight2(u16, String),
    /// 获取物品攻击
    GetItemAttack2(u16, String),
    /// 获取物品防御
    GetItemDefense2(u16, String),
    /// 获取物品范围
    GetItemRange2(u16, String),
    /// 获取物品槽位数
    GetItemSlot2(u16, String),
    /// 获取物品性别限制
    GetItemGender2(u16, String),
    /// 获取物品等级限制
    GetItemLevel2(u16, String),
    /// 获取物品职业限制
    GetItemJob2(u16, String),
    /// 获取物品位置限制
    GetItemLoc2(u16, String),
    /// 获取物品卡片数量
    GetItemCard2(u16, u8, String),
    /// 获取物品脚本
    GetItemScript2(u16, String),
    /// 获取物品装备脚本
    GetItemEquipScript2(u16, String),
    /// 获取物品卸下脚本
    GetItemUnequipScript2(u16, String),

    // ---- rAthena 最终技能命令 ----
    /// 获取技能名称
    GetSkillName2(u16, String),
    /// 获取技能描述
    GetSkillDesc2(u16, String),
    /// 获取技能类型
    GetSkillType2(u16, String),
    /// 获取技能目标类型
    GetSkillTargetType2(u16, String),
    /// 获取技能伤害类型
    GetSkillDamageType2(u16, String),
    /// 获取技能元素
    GetSkillElement2(u16, String),
    /// 获取技能范围
    GetSkillRange3(u16, String),
    /// 获取技能消耗
    GetSkillCost3(u16, String),
    /// 获取技能冷却时间
    GetSkillCooldown3(u16, String),
    /// 获取技能施法时间
    GetSkillCastTime2(u16, String),
    /// 获取技能持续时间
    GetSkillDuration2(u16, String),
    /// 获取技能效果
    GetSkillEffect2(u16, String),

    // ---- rAthena 最终怪物命令 ----
    /// 获取怪物名称
    GetMonsterName2(u16, String),
    /// 获取怪物等级
    GetMonsterLevel2(u16, String),
    /// 获取怪物 HP
    GetMonsterHp2(u16, String),
    /// 获取怪物攻击
    GetMonsterAttack2(u16, String),
    /// 获取怪物防御
    GetMonsterDefense2(u16, String),
    /// 获取怪物经验值
    GetMonsterExp2(u16, String),
    /// 获取怪物掉落
    GetMonsterDrop2(u16, String),
    /// 获取怪物大小
    GetMonsterSize2(u16, String),
    /// 获取怪物种族
    GetMonsterRace2(u16, String),
    /// 获取怪物元素
    GetMonsterElement2(u16, String),
    /// 获取怪物 AI 类型
    GetMonsterAi2(u16, String),

    // ---- rAthena 最终状态命令 ----
    /// 获取状态效果名称
    GetStatusName2(u16, String),
    /// 获取状态效果描述
    GetStatusDesc2(u16, String),
    /// 获取状态效果类型
    GetStatusType2(u16, String),
    /// 获取状态效果图标
    GetStatusIcon2(u16, String),
    /// 获取状态效果颜色
    GetStatusColor2(u16, String),
    /// 获取状态效果持续时间
    GetStatusDuration2(u16, String),
    /// 获取状态效果剩余时间
    GetStatusRemaining2(u16, String),
    /// 获取状态效果层数
    GetStatusStack3(u16, String),
    /// 获取状态效果来源
    GetStatusSource2(u16, String),

    // ---- rAthena 最终战斗命令 ----
    /// 计算物理伤害
    CalcPhysicalDamage3(u16, u16, String),
    /// 计算魔法伤害
    CalcMagicDamage3(u16, u16, String),
    /// 计算治愈量
    CalcHealAmount3(u16, u8, String),
    /// 获取命中率
    GetHitRate3(u16, String),
    /// 获取闪避率
    GetFleeRate3(u16, String),
    /// 获取暴击率
    GetCriticalRate3(u16, String),
    /// 获取攻击速度
    GetAttackSpeed3(String),
    /// 获取移动速度
    GetMoveSpeed3(String),
    /// 获取攻击力
    GetAttack2(String),
    /// 获取魔法攻击力
    GetMagicAttack2(String),
    /// 获取防御力
    GetDefense2(String),
    /// 获取魔法防御力
    GetMagicDefense2(String),

    // ---- rAthena 最终地图命令 ----
    /// 获取地图名称
    GetMapName4(String),
    /// 获取地图上玩家数量
    GetMapUsers4(String, String),
    /// 获取地图上怪物数量
    GetMapMobs4(String, String),
    /// 获取地图宽度
    GetMapWidth4(String, String),
    /// 获取地图高度
    GetMapHeight4(String, String),
    /// 检查地图坐标是否可通行
    IsWalkable4(String, u16, u16, String),
    /// 设置地图属性
    SetMapFlag3(String, String, bool),
    /// 获取地图属性
    GetMapFlag3(String, String, String),
    /// 获取地图区域玩家
    GetAreaPlayers3(String, u16, u16, u16, String),
    /// 获取地图区域怪物
    GetAreaMobs3(String, u16, u16, u16, String),
    /// 在地图上显示特效
    ShowEffect3(String, u16, u16, u16),
    /// 在地图上显示动画
    ShowAnimation3(String, u16, u16, u16),

    // ---- rAthena 最终 NPC 命令 ----
    /// 获取 NPC 名称
    GetNpcName3(String, String),
    /// 获取 NPC 位置
    GetNpcPos4(String, String, String, String),
    /// 设置 NPC 位置
    SetNpcPos4(String, u16, u16),
    /// 设置 NPC 方向
    SetNpcDir4(String, u16),
    /// 设置 NPC 外观
    SetNpcSprite4(String, u16),
    /// 设置 NPC 可见性
    SetNpcVisible4(String, bool),
    /// 设置 NPC 可交互性
    SetNpcInteractable4(String, bool),
    /// 获取 NPC 状态
    GetNpcStatus4(String, String),
    /// 设置 NPC 状态
    SetNpcStatus4(String, String),
    /// 获取 NPC 数据
    GetNpcData3(String, String),
    /// 设置 NPC 数据
    SetNpcData3(String, String, String),

    // ---- rAthena 最终公会命令 ----
    /// 获取公会 ID
    GetGuildId4(String),
    /// 获取公会名称
    GetGuildName4(String),
    /// 获取公会等级
    GetGuildLevel4(String),
    /// 设置公会等级
    SetGuildLevel4(u16),
    /// 增加公会经验值
    AddGuildExp4(u32),
    /// 获取公会成员数量
    GetGuildMemberCount4(String),
    /// 获取公会在线成员数量
    GetGuildOnlineCount4(String),
    /// 检查是否为公会会长
    IsGuildMaster4(String),
    /// 获取公会公告
    GetGuildNotice4(String),
    /// 设置公会公告
    SetGuildNotice4(String),
    /// 获取公会技能等级
    GetGuildSkillLevel4(u16, String),
    /// 设置公会技能等级
    SetGuildSkillLevel4(u16, u8),

    // ---- rAthena 最终队伍命令 ----
    /// 获取队伍 ID
    GetPartyId4(String),
    /// 获取队伍名称
    GetPartyName4(String),
    /// 获取队伍成员数量
    GetPartyMemberCount4(String),
    /// 获取队伍在线成员数量
    GetPartyOnlineCount4(String),
    /// 检查是否为队长
    IsPartyLeader4(String),
    /// 获取队伍经验值分配模式
    GetPartyExpPolicy4(String),
    /// 设置队伍经验值分配模式
    SetPartyExpPolicy4(u8),
    /// 获取队伍物品分配模式
    GetPartyItemPolicy4(String),
    /// 设置队伍物品分配模式
    SetPartyItemPolicy4(u8),

    // ---- rAthena 最终宠物命令 ----
    /// 获取宠物等级
    GetPetLevel4(String),
    /// 获取宠物经验值
    GetPetExp4(String),
    /// 增加宠物经验值
    AddPetExp4(u32),
    /// 获取宠物饥饿度
    GetPetHunger4(String),
    /// 设置宠物饥饿度
    SetPetHunger4(u16),
    /// 获取宠物亲密度
    GetPetIntimacy4(String),
    /// 设置宠物亲密度
    SetPetIntimacy4(u32),
    /// 获取宠物装备
    GetPetEquip4(String),
    /// 设置宠物装备
    SetPetEquip4(u16),

    // ---- rAthena 最终生命体命令 ----
    /// 获取生命体等级
    GetHomunLevel3(String),
    /// 获取生命体经验值
    GetHomunExp3(String),
    /// 增加生命体经验值
    AddHomunExp3(u32),
    /// 获取生命体亲密度
    GetHomunIntimacy3(String),
    /// 设置生命体亲密度
    SetHomunIntimacy3(u32),
    /// 获取生命体类型
    GetHomunType3(String),
    /// 获取生命体进化状态
    GetHomunEvolution3(String),

    // ---- rAthena 最终佣兵命令 ----
    /// 获取佣兵等级
    GetMercLevel3(String),
    /// 获取佣兵经验值
    GetMercExp3(String),
    /// 增加佣兵经验值
    AddMercExp3(u32),
    /// 获取佣兵忠诚度
    GetMercLoyalty3(String),
    /// 设置佣兵忠诚度
    SetMercLoyalty3(u32),
    /// 获取佣兵剩余时间
    GetMercTimer3(String),

    // ---- rAthena 最终任务命令 ----
    /// 添加任务
    SetQuest3(u16),
    /// 完成任务
    CompleteQuest3(u16),
    /// 放弃任务
    EraseQuest3(u16),
    /// 检查任务状态
    CheckQuest3(u16, String),
    /// 改变任务
    ChangeQuest4(u16, u16),
    /// 获取任务信息
    GetQuestInfo3(u16, String),
    /// 设置任务信息
    SetQuestInfo3(u16, String, String),

    // ---- rAthena 最终成就命令 ----
    /// 添加成就
    AchievementAdd4(u16),
    /// 完成成就
    AchievementComplete4(u16),
    /// 检查成就是否存在
    AchievementExists4(u16, String),
    /// 获取成就信息
    AchievementInfo4(u16, String),
    /// 移除成就
    AchievementRemove4(u16),
    /// 更新成就
    AchievementUpdate4(u16, String),

    // ---- rAthena 最终公会战命令 ----
    /// 检查公会战状态
    AgitCheck4(String),
    /// 开始公会战
    AgitStart4,
    /// 结束公会战
    AgitEnd4,
    /// 获取公会战信息
    GetAgitInfo3(String),
    /// 获取公会战城堡信息
    GetCastleInfo3(u16, String),

    // ---- rAthena 最终战场命令 ----
    /// 创建战场
    BgCreate4(String, String),
    /// 销毁战场
    BgDestroy4(u16),
    /// 加入战场
    BgJoin4(u16, String),
    /// 离开战场
    BgLeave4,
    /// 传送战场
    BgWarp4(u16, String, u16, u16),
    /// 召唤战场怪物
    BgMonster4(u16, String, u16, u16, String, u16),
    /// 获取战场数据
    BgGetData4(u16, String, String),
    /// 获取战场信息
    BgInfo4(u16, String),

    // ---- rAthena 最终频道命令 ----
    /// 创建频道
    ChannelCreate4(String, String),
    /// 删除频道
    ChannelDelete4(String),
    /// 加入频道
    ChannelJoin4(String),
    /// 踢出频道
    ChannelKick4(String, String),
    /// 封禁频道
    ChannelBan4(String, String),
    /// 解封频道
    ChannelUnban4(String, String),
    /// 频道聊天
    ChannelChat4(String, String),
    /// 设置频道颜色
    ChannelSetColor4(String, u32),
    /// 设置频道选项
    ChannelSetOpt4(String, String, String),
    /// 设置频道密码
    ChannelSetPass4(String, String),
    /// 设置频道权限组
    ChannelSetGroup4(String, String),

    // ---- rAthena 最终收购商店命令 ----
    /// 打开收购商店
    OpenBuyingStore4(u16),
    /// 关闭收购商店
    CloseBuyingStore4,
    /// 获取收购商店列表
    GetBuyingStoreList4(String),

    // ---- rAthena 最终搜索商店命令 ----
    /// 搜索商店
    SearchStore4(u16, String),
    /// 获取搜索结果
    GetSearchStoreResult4(String),

    // ---- rAthena 最终拍卖命令 ----
    /// 获取拍卖列表
    GetAuctionList4(String),
    /// 上架拍卖
    AuctionAddItem4(u16, u16, u32),
    /// 竞拍
    AuctionBid4(u32, u32),
    /// 获取拍卖信息
    GetAuctionInfo4(u32, String),
    /// 领取拍卖物品
    AuctionClaimItem4(u32),

    // ---- rAthena 最终摆摊命令 ----
    /// 打开摆摊界面
    OpenVending4,
    /// 获取摆摊物品列表
    GetVendingItems4(String),
    /// 购买摆摊物品
    BuyVendingItem4(u32, u16),
    /// 获取摆摊信息
    GetVendingInfo4(String),
    /// 关闭摆摊
    CloseVending4,

    // ---- rAthena 最终邮件命令 ----
    /// 发送邮件
    SendMail4(String, String, String),
    /// 读取邮件
    ReadMail4(u32, String),
    /// 删除邮件
    DeleteMail4(u32),
    /// 附件收取
    GetMailAttachment4(u32),
    /// 获取邮件数量
    GetMailCount4(String),

    // ---- rAthena 最终交易命令 ----
    /// 发起交易请求
    TradeRequest4(u32),
    /// 接受交易
    TradeAccept4,
    /// 拒绝交易
    TradeReject4,
    /// 添加交易物品
    TradeAddItem4(u16, u16),
    /// 设置交易金币
    TradeSetZeny4(u32),

    // ---- rAthena 最终聊天命令 ----
    /// 发送聊天消息
    ChatMessage4(String),
    /// 发送私聊消息
    WhisperMessage4(String, String),
    /// 创建聊天室
    CreateChatRoom4(String, u16, String),
    /// 加入聊天室
    JoinChatRoom4(String),
    /// 离开聊天室
    LeaveChatRoom4,

    // ---- rAthena 最终副本命令 ----
    /// 创建副本
    CreateInstance4(u16, String),
    /// 进入副本
    EnterInstance4(u16),
    /// 离开副本
    LeaveInstance4,
    /// 获取副本信息
    GetInstanceInfo4(String),
    /// 检查副本是否活跃
    IsInstanceActive4(String),

    // ---- rAthena 最终商城命令 ----
    /// 获取商城物品列表
    GetCashShopItems4(String),
    /// 购买商城物品
    BuyCashItem4(u16, u32),
    /// 获取玩家点数
    GetCashPoints4(String),
    /// 增加玩家点数
    AddCashPoints4(u32),
    /// 消耗玩家点数
    ConsumeCashPoints4(u32, String),

    // ---- rAthena 最终 Kafra 命令 ----
    /// 打开仓库
    OpenStorage4,
    /// 获取仓库物品数量
    GetStorageItemCount4(u16, String),
    /// 存入物品
    StorageAdd4(u16, u16),
    /// 取出物品
    StorageRemove4(u16, u16),
    /// 获取仓库容量
    GetStorageCapacity4(String),

    // ---- rAthena 最终声望命令 ----
    /// 增加声望点数
    AddReputationPoints4(String, i32),
    /// 获取声望点数
    GetReputationPoints4(String, String),

    // ---- rAthena 最终名声命令 ----
    /// 增加名声
    AddFame4(u32),
    /// 获取名声
    GetFame4(String),

    // ---- rAthena 最终附魔命令 ----
    /// 获取附魔等级
    EnchantGradeUi4(u16),
    /// 设置附魔等级
    SetEnchantGrade4(u16, u8),

    // ---- rAthena 最终合成命令 ----
    /// 合成物品
    LaphineSynthesis4(u16),
    /// 升级物品
    LaphineUpgrade4(u16),

    // ---- rAthena 最终造型师命令 ----
    /// 打开造型师
    OpenStylist4,

    // ---- rAthena 最终摄像机命令 ----
    /// 获取摄像机信息
    CameraInfo4(String),
    /// 设置摄像机
    SetCamera4(u16, u16, u16),

    // ---- rAthena 最终定时器命令 ----
    /// 添加定时器
    AddTimer4(u32, String),
    /// 添加定时器计数
    AddTimerCount4(String, u32),
    /// 附加 NPC 定时器
    AttachNpcTimer4(String),
    /// 分离 NPC 定时器
    DetachNpcTimer4,

    // ---- rAthena 最终系统命令 ----
    /// 执行 GM 命令
    AtCommand4(String),
    /// 绑定 GM 命令
    BindAtCmd4(String, String),
    /// 获取基础技能检查
    BasicSkillCheck4(String),
    /// 获取服务器信息
    GetServerInfo3(String),

    // ================================================================
    // rAthena 兼容命令（最终补齐：44 个命令）
    // ================================================================

    // ---- rAthena 最终缺失命令 ----
    /// 获取角色 ID（getcharid）
    GetCharId2(u8, String),
    /// 获取角色名称（strcharinfo）
    StrCharInfo3(u8, String),
    /// 获取角色位置（getcharid）
    GetCharPos2(String, String, String, String),
    /// 获取角色方向
    GetCharDir2(String),
    /// 设置角色方向
    SetCharDir2(u16),
    /// 获取角色速度
    GetCharSpeed2(String),
    /// 设置角色速度
    SetCharSpeed2(u16),
    /// 获取角色 HP
    GetHp3(String),
    /// 获取角色 SP
    GetSp3(String),
    /// 获取角色最大 HP
    GetMaxHp3(String),
    /// 获取角色最大 SP
    GetMaxSp3(String),
    /// 恢复 HP/SP
    Heal3(u32, u32),
    /// 百分比恢复
    PercentHeal3(u8, u8),
    /// 获取角色职业
    GetClass3(String),
    /// 获取角色性别
    GetSex3(String),
    /// 获取角色等级
    GetBaseLevel3(String),
    /// 获取角色职业等级
    GetJobLevel3(String),
    /// 设置角色等级
    SetBaseLevel3(u16),
    /// 设置角色职业等级
    SetJobLevel3(u16),
    /// 获取角色金币
    GetZeny3(String),
    /// 设置角色金币
    SetZeny3(u32),
    /// 增加角色金币
    AddZeny3(i32),
    /// 获取角色经验值
    GetBaseExp(String),
    /// 获取角色职业经验值
    GetJobExp(String),
    /// 增加角色经验值
    AddBaseExp(u32),
    /// 增加角色职业经验值
    AddJobExp(u32),
    /// 获取角色地图
    GetMapName5(String),
    /// 获取角色 X 坐标
    GetX2(String),
    /// 获取角色 Y 坐标
    GetY2(String),
    /// 传送角色
    Warp3(String, u16, u16),
    /// 获取角色状态
    GetCharStatus(String),
    /// 设置角色状态
    SetCharStatus(String, String),
    /// 获取角色装备
    GetCharEquip(String),
    /// 设置角色装备
    SetCharEquip(u16, u16),
    /// 获取角色技能
    GetCharSkills(String),
    /// 设置角色技能
    SetCharSkill(u16, u8),
    /// 获取角色任务
    GetCharQuests(String),
    /// 设置角色任务
    SetCharQuest(u16, u8),
    /// 获取角色成就
    GetCharAchievements(String),
    /// 设置角色成就
    SetCharAchievement(u16, u32),
    /// 获取角色声望
    GetCharReputation(String),
    /// 设置角色声望
    SetCharReputation(String, i32),
    /// 获取角色名声
    GetCharFame(String),
    /// 设置角色名声
    SetCharFame(u32),

    // ================================================================
    // rAthena 兼容命令（最终 26 个命令）
    // ================================================================

    // ---- rAthena 最终补齐命令 ----
    /// 获取角色基础经验值
    GetBaseExp2(String),
    /// 获取角色职业经验值
    GetJobExp2(String),
    /// 增加角色基础经验值
    AddBaseExp2(u32),
    /// 增加角色职业经验值
    AddJobExp2(u32),
    /// 获取角色地图
    GetMapName6(String),
    /// 获取角色 X 坐标
    GetX3(String),
    /// 获取角色 Y 坐标
    GetY3(String),
    /// 传送角色
    Warp4(String, u16, u16),
    /// 获取角色状态
    GetCharStatus2(String),
    /// 设置角色状态
    SetCharStatus2(String, String),
    /// 获取角色装备
    GetCharEquip2(String),
    /// 设置角色装备
    SetCharEquip2(u16, u16),
    /// 获取角色技能
    GetCharSkills2(String),
    /// 设置角色技能
    SetCharSkill2(u16, u8),
    /// 获取角色任务
    GetCharQuests2(String),
    /// 设置角色任务
    SetCharQuest2(u16, u8),
    /// 获取角色成就
    GetCharAchievements2(String),
    /// 设置角色成就
    SetCharAchievement2(u16, u32),
    /// 获取角色声望
    GetCharReputation2(String),
    /// 设置角色声望
    SetCharReputation2(String, i32),
    /// 获取角色名声
    GetCharFame2(String),
    /// 设置角色名声
    SetCharFame2(u32),
    /// 获取角色基础经验值
    GetBaseExp3(String),
    /// 获取角色职业经验值
    GetJobExp3(String),
    /// 增加角色基础经验值
    AddBaseExp3(u32),
    /// 增加角色职业经验值
    AddJobExp3(u32),
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
            Expr::VarTruthy(var_name) => variables.get(var_name).copied().unwrap_or(0) != 0,
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
/// 通过 Player::to_script_context() 从真实玩家数据构建
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// 角色 ID
    pub char_id: u32,
    /// 账户 ID
    pub account_id: u32,
    /// 角色名
    pub char_name: String,
    /// 公会名
    pub guild_name: String,
    /// 公会 ID
    pub guild_id: u32,
    /// 队伍名
    pub party_name: String,
    /// 队伍 ID
    pub party_id: u32,
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
    /// 力量
    pub str: u16,
    /// 敏捷
    pub agi: u16,
    /// 体力
    pub vit: u16,
    /// 智力
    pub int_: u16,
    /// 灵巧
    pub dex: u16,
    /// 幸运
    pub luk: u16,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self {
            char_id: 100001,
            account_id: 2000001,
            char_name: "Player".to_string(),
            guild_name: String::new(),
            guild_id: 0,
            party_name: String::new(),
            party_id: 0,
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
            str: 1,
            agi: 1,
            vit: 1,
            int_: 1,
            dex: 1,
            luk: 1,
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
        write!(
            f,
            "第 {} 行解析错误: {} (原文: '{}')",
            self.line, self.message, self.source
        )
    }
}

impl std::error::Error for ParseError {}
