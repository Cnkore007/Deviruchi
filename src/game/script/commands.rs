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
