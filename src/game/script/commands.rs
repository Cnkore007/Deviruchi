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
