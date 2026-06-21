use crate::game::script::commands::{
    CallFrame, ExprValue, LoopContext, ScriptCommand, ScriptContext, ScriptNode,
};
use std::collections::HashMap;
use uuid::Uuid;

/// NPC 对话状态
pub struct NpcDialogueState {
    pub player_id: Uuid,
    pub npc_id: u32,
    pub script: ScriptNode,
    pub current_index: usize,
    pub pending_next: bool,
    /// 脚本整数变量存储（变量名 -> 值）
    pub variables: HashMap<String, i64>,
    /// 脚本字符串变量存储（变量名 -> 值）
    pub string_variables: HashMap<String, String>,
    /// 最近一次 Select 调用的玩家选择（1-based，0 表示未选择）
    last_select: usize,
    /// 函数调用栈
    call_stack: Vec<CallFrame>,
    /// 循环上下文栈（支持嵌套循环）
    loop_stack: Vec<LoopContext>,
    /// 玩家上下文信息（stub，未来将连接到真实玩家数据）
    pub context: ScriptContext,
    /// 标记对话是否因 close2 而暂停（不清除状态）
    paused_by_close2: bool,
}

impl NpcDialogueState {
    pub fn new(player_id: Uuid, npc_id: u32, script: ScriptNode) -> Self {
        Self {
            player_id,
            npc_id,
            script,
            current_index: 0,
            pending_next: false,
            variables: HashMap::new(),
            string_variables: HashMap::new(),
            last_select: 0,
            call_stack: Vec::new(),
            loop_stack: Vec::new(),
            context: ScriptContext::default(),
            paused_by_close2: false,
        }
    }

    /// 创建带有玩家上下文的 NPC 对话状态
    pub fn with_context(
        player_id: Uuid,
        npc_id: u32,
        script: ScriptNode,
        context: ScriptContext,
    ) -> Self {
        Self {
            player_id,
            npc_id,
            script,
            current_index: 0,
            pending_next: false,
            variables: HashMap::new(),
            string_variables: HashMap::new(),
            last_select: 0,
            call_stack: Vec::new(),
            loop_stack: Vec::new(),
            context,
            paused_by_close2: false,
        }
    }

    /// 获取整数变量值
    pub fn get_variable(&self, name: &str) -> Option<i64> {
        self.variables.get(name).copied()
    }

    /// 设置整数变量值
    pub fn set_variable(&mut self, name: String, value: i64) {
        self.variables.insert(name, value);
    }

    /// 获取字符串变量值
    pub fn get_string_variable(&self, name: &str) -> Option<&String> {
        self.string_variables.get(name)
    }

    /// 设置字符串变量值
    pub fn set_string_variable(&mut self, name: String, value: String) {
        self.string_variables.insert(name, value);
    }

    /// 是否因 close2 暂停（玩家可重新触发 NPC 继续对话）
    pub fn is_paused_by_close2(&self) -> bool {
        self.paused_by_close2
    }

    /// 恢复 close2 暂停状态（玩家重新触发 NPC 时调用）
    pub fn resume_from_close2(&mut self) {
        self.paused_by_close2 = false;
    }

    /// 处理当前命令，返回需要显示的内容
    pub fn process(&mut self) -> DialogueResponse {
        if self.current_index >= self.script.commands.len() {
            return DialogueResponse::Closed;
        }

        let cmd = &self.script.commands[self.current_index].clone();

        match cmd {
            // ============================================================
            // 对话命令
            // ============================================================
            ScriptCommand::Mes(msg) => {
                self.current_index += 1;
                DialogueResponse::Message(msg.clone())
            }

            ScriptCommand::Next => {
                self.pending_next = true;
                self.current_index += 1;
                DialogueResponse::Pending
            }

            ScriptCommand::Close | ScriptCommand::End => DialogueResponse::Closed,

            // close2：关闭对话但不清除状态，玩家可重新触发 NPC 继续
            ScriptCommand::Close2 => {
                self.current_index += 1;
                self.paused_by_close2 = true;
                DialogueResponse::Closed
            }

            ScriptCommand::Select(options) => {
                self.current_index += 1;
                DialogueResponse::Select(options.clone())
            }

            // ============================================================
            // 传送
            // ============================================================
            ScriptCommand::Warp(map, x, y) => {
                self.current_index += 1;
                DialogueResponse::Warp {
                    map: map.clone(),
                    x: *x,
                    y: *y,
                }
            }

            // ============================================================
            // 控制流
            // ============================================================
            ScriptCommand::Goto(label) => {
                if let Some(&idx) = self.script.labels.get(label) {
                    self.current_index = idx;
                } else {
                    tracing::warn!(
                        "脚本标签 '{}' 未找到，跳过 goto 命令 (NPC: {})",
                        label,
                        self.npc_id
                    );
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            ScriptCommand::Set(var_name, expr_value) => {
                match expr_value {
                    ExprValue::Int(value) => {
                        self.variables.insert(var_name.clone(), *value);
                    }
                    ExprValue::Str(value) => {
                        self.string_variables
                            .insert(var_name.clone(), value.clone());
                    }
                    ExprValue::Var(ref_var) => {
                        // 尝试从整数变量复制
                        if let Some(val) = self.variables.get(ref_var).copied() {
                            self.variables.insert(var_name.clone(), val);
                        }
                        // 尝试从字符串变量复制
                        if let Some(val) = self.string_variables.get(ref_var).cloned() {
                            self.string_variables.insert(var_name.clone(), val);
                        }
                    }
                }
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::GotoIf(var_name, target_value, label) => {
                let current_value = self.variables.get(var_name).copied().unwrap_or(0);
                if current_value == *target_value {
                    if let Some(&idx) = self.script.labels.get(label) {
                        self.current_index = idx;
                    } else {
                        tracing::warn!("条件跳转标签 '{}' 未找到 (NPC: {})", label, self.npc_id);
                        self.current_index += 1;
                    }
                } else {
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            // ============================================================
            // 函数调用
            // ============================================================
            ScriptCommand::CallFunc(func_name, args) => {
                // 通过标签查找函数入口
                if let Some(&func_idx) = self.script.labels.get(func_name) {
                    // 将当前执行位置压栈（返回到 callfunc 的下一条命令）
                    self.call_stack.push(CallFrame {
                        return_index: self.current_index + 1,
                        args: args.clone(),
                    });
                    self.current_index = func_idx;
                } else {
                    tracing::warn!("脚本函数 '{}' 未找到 (NPC: {})", func_name, self.npc_id);
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            ScriptCommand::Return => {
                if let Some(frame) = self.call_stack.pop() {
                    self.current_index = frame.return_index;
                } else {
                    // 没有调用栈帧，当作脚本结束
                    tracing::warn!("return 时调用栈为空 (NPC: {})", self.npc_id);
                    return DialogueResponse::Closed;
                }
                DialogueResponse::Continue
            }

            // ============================================================
            // 循环控制
            // ============================================================
            ScriptCommand::ForStart(expr, _inc_cmds) => {
                // 检查循环条件
                if expr.eval(&self.variables) {
                    // 条件为真，记录循环上下文并进入循环体
                    // end_index 在进入循环时暂设为 0，遇到 ForEnd 时修正
                    self.loop_stack.push(LoopContext {
                        condition_index: self.current_index,
                        end_index: 0, // 将在遇到 ForEnd 时设置
                    });
                    self.current_index += 1;
                } else {
                    // 条件为假，跳过整个循环体
                    // 查找对应的 ForEnd
                    let mut depth = 1;
                    let mut search_idx = self.current_index + 1;
                    while search_idx < self.script.commands.len() && depth > 0 {
                        match &self.script.commands[search_idx] {
                            ScriptCommand::ForStart(_, _) => depth += 1,
                            ScriptCommand::ForEnd => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        search_idx += 1;
                    }
                    if depth == 0 {
                        self.current_index = search_idx + 1; // 跳过 ForEnd
                    } else {
                        tracing::warn!("for 循环缺少 endloop (NPC: {})", self.npc_id);
                        return DialogueResponse::Closed;
                    }
                }
                DialogueResponse::Continue
            }

            ScriptCommand::ForEnd => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    let condition_index = loop_ctx.condition_index;
                    // 获取增量命令并执行
                    if let ScriptCommand::ForStart(_, inc_cmds) =
                        &self.script.commands[condition_index].clone()
                    {
                        // 执行增量命令
                        for inc_cmd in inc_cmds {
                            self.execute_simple_command(inc_cmd);
                        }
                    }
                    // 跳回条件检查
                    self.current_index = condition_index;
                } else {
                    tracing::warn!("endloop 缺少对应的循环开始 (NPC: {})", self.npc_id);
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            ScriptCommand::Break => {
                if let Some(loop_ctx) = self.loop_stack.pop() {
                    if loop_ctx.end_index > 0 {
                        // end_index 已设置（由匹配的 ForEnd 设置过）
                        self.current_index = loop_ctx.end_index + 1;
                    } else {
                        // end_index 未设置，需要查找对应的 ForEnd
                        let mut depth = 1;
                        let mut search_idx = self.current_index + 1;
                        while search_idx < self.script.commands.len() && depth > 0 {
                            match &self.script.commands[search_idx] {
                                ScriptCommand::ForStart(_, _) => depth += 1,
                                ScriptCommand::ForEnd => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            search_idx += 1;
                        }
                        if depth == 0 {
                            self.current_index = search_idx + 1;
                        } else {
                            tracing::warn!("break 时找不到匹配的 endloop (NPC: {})", self.npc_id);
                            return DialogueResponse::Closed;
                        }
                    }
                } else {
                    tracing::warn!("break 不在循环内部 (NPC: {})", self.npc_id);
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            ScriptCommand::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last() {
                    let condition_index = loop_ctx.condition_index;
                    // 获取增量命令并执行（continue 也要执行增量）
                    if let ScriptCommand::ForStart(_, inc_cmds) =
                        &self.script.commands[condition_index].clone()
                    {
                        for inc_cmd in inc_cmds {
                            self.execute_simple_command(inc_cmd);
                        }
                    }
                    // 跳回条件检查
                    self.current_index = condition_index;
                } else {
                    tracing::warn!("continue 不在循环内部 (NPC: {})", self.npc_id);
                    self.current_index += 1;
                }
                DialogueResponse::Continue
            }

            // ============================================================
            // 物品操作
            // ============================================================
            ScriptCommand::GetItem(item_id, amount) => {
                // 添加物品到背包
                let current = self.context.inventory.entry(*item_id).or_insert(0);
                *current = current.saturating_add(*amount);
                tracing::info!(
                    "NPC {} 给予玩家 {} 个物品 {}，当前数量: {}",
                    self.npc_id,
                    amount,
                    item_id,
                    current
                );
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::DelItem(item_id, amount) => {
                // 从背包删除物品
                let mut should_remove = false;
                let remaining;
                if let Some(current) = self.context.inventory.get_mut(item_id) {
                    if *current >= *amount {
                        *current -= *amount;
                        remaining = *current;
                        if *current == 0 {
                            should_remove = true;
                        }
                        tracing::info!(
                            "NPC {} 从玩家背包删除 {} 个物品 {}，剩余: {}",
                            self.npc_id,
                            amount,
                            item_id,
                            remaining
                        );
                    } else {
                        tracing::warn!(
                            "NPC {} 尝试删除 {} 个物品 {}，但玩家只有 {} 个",
                            self.npc_id,
                            amount,
                            item_id,
                            current
                        );
                    }
                } else {
                    tracing::warn!(
                        "NPC {} 尝试删除物品 {}，但玩家没有该物品",
                        self.npc_id,
                        item_id
                    );
                }
                if should_remove {
                    self.context.inventory.remove(item_id);
                }
                DialogueResponse::Continue
            }

            // ============================================================
            // 信息查询（函数式语法）
            // ============================================================
            ScriptCommand::CountItem(item_id, result_var) => {
                // 从玩家背包查询物品数量
                let count: i64 = self.context.inventory.get(item_id).copied().unwrap_or(0) as i64;
                tracing::debug!("countitem({}) = {}", item_id, count);
                self.variables.insert(result_var.clone(), count);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::CheckWeight(item_id, amount, result_var) => {
                // 检查玩家负重是否能容纳物品
                // 简化实现：基于最大负重和当前负重计算
                let can_carry: i64 = 1; // TODO: 需要物品重量数据
                tracing::debug!("checkweight({}, {}) = {}", item_id, amount, can_carry);
                self.variables.insert(result_var.clone(), can_carry);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::GetCharId(type_id, result_var) => {
                let value: i64 = match type_id {
                    0 => self.context.char_id as i64,    // char_id
                    1 => self.context.party_id as i64,   // party_id
                    2 => self.context.guild_id as i64,   // guild_id
                    3 => self.context.account_id as i64, // account_id
                    _ => {
                        tracing::warn!("getcharid: 未知类型 {} (NPC: {})", type_id, self.npc_id);
                        0
                    }
                };
                self.variables.insert(result_var.clone(), value);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::GetArg(index, result_var) => {
                let value = if let Some(frame) = self.call_stack.last() {
                    frame.args.get(*index as usize).copied().unwrap_or(0)
                } else {
                    tracing::warn!("getarg: 调用栈为空，无法获取参数 (NPC: {})", self.npc_id);
                    0
                };
                self.variables.insert(result_var.clone(), value);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::StrCharInfo(type_id, result_var) => {
                let value = match type_id {
                    0 => self.context.char_name.clone(),  // 角色名
                    1 => self.context.guild_name.clone(), // 公会名
                    2 => self.context.party_name.clone(), // 队伍名
                    _ => {
                        tracing::warn!("strcharinfo: 未知类型 {} (NPC: {})", type_id, self.npc_id);
                        String::new()
                    }
                };
                self.string_variables.insert(result_var.clone(), value);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::ReadParam(param_id, result_var) => {
                // rAthena 参数 ID 常量
                // 11=BaseLevel, 12=JobLevel, 13=MaxHP, 14=MaxSP
                // 5=Str, 6=Agi, 7=Vit, 8=Int, 9=Dex, 10=Luk
                let value: i64 = match param_id {
                    5 => self.context.str as i64,  // Str
                    6 => self.context.agi as i64,  // Agi
                    7 => self.context.vit as i64,  // Vit
                    8 => self.context.int_ as i64, // Int
                    9 => self.context.dex as i64,  // Dex
                    10 => self.context.luk as i64, // Luk
                    11 => self.context.base_level as i64,
                    12 => self.context.job_level as i64,
                    13 => self.context.max_hp as i64, // MaxHP
                    14 => self.context.max_sp as i64, // MaxSP
                    _ => {
                        tracing::warn!(
                            "readparam: 未知参数 ID {} (NPC: {})",
                            param_id,
                            self.npc_id
                        );
                        0
                    }
                };
                self.variables.insert(result_var.clone(), value);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            // ============================================================
            // 系统操作
            // ============================================================

            // ============================================================
            // 状态操作
            // ============================================================
            ScriptCommand::Heal(hp, sp) => {
                // 恢复 HP/SP
                self.context.current_hp = (self.context.current_hp + hp).min(self.context.max_hp);
                self.context.current_sp = (self.context.current_sp + sp).min(self.context.max_sp);
                tracing::info!(
                    "NPC {} 恢复玩家 HP+{} SP+{}，当前 HP: {}/{}, SP: {}/{}",
                    self.npc_id,
                    hp,
                    sp,
                    self.context.current_hp,
                    self.context.max_hp,
                    self.context.current_sp,
                    self.context.max_sp
                );
                self.current_index += 1;
                DialogueResponse::Continue
            }

            // ============================================================
            // 系统操作
            // ============================================================
            ScriptCommand::Announce(message, flag) => {
                // 设置广播标志，由调用方处理实际广播
                tracing::info!("公告 [flag={}]: {} (NPC: {})", flag, message, self.npc_id);
                self.context.need_broadcast = true;
                self.context.broadcast_message = message.clone();
                self.current_index += 1;
                DialogueResponse::Announce {
                    message: message.clone(),
                    flag: *flag,
                }
            }

            ScriptCommand::Input(var_name) => {
                self.current_index += 1;
                DialogueResponse::Input(var_name.clone())
            }

            ScriptCommand::Sleep(ms) => {
                self.current_index += 1;
                DialogueResponse::Sleep(*ms)
            }

            // ============================================================
            // 数值操作
            // ============================================================
            ScriptCommand::GetVariableOfNpc(var_name, npc_name, result_var) => {
                // TODO: 连接到 NPC 变量系统，从指定 NPC 读取变量
                // 当前返回 0 作为 stub
                tracing::debug!(
                    "getvariableofnpc(\"{}\", \"{}\") = 0 (TODO: NPC 变量系统集成)",
                    var_name,
                    npc_name
                );
                self.variables.insert(result_var.clone(), 0);
                self.current_index += 1;
                DialogueResponse::Continue
            }

            ScriptCommand::Rand(min, max, result_var) => {
                // 使用标准库随机数
                let value = if *min <= *max {
                    // 使用简单的伪随机
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;
                    let hash = seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(self.npc_id as u64);
                    let range = (*max - *min + 1) as u64;
                    *min + (hash % range) as i64
                } else {
                    *max
                };
                self.variables.insert(result_var.clone(), value);
                self.current_index += 1;
                DialogueResponse::Continue
            }
            // 新增命令的处理（暂时跳过）
            _ => {
                tracing::debug!("execute_command: 暂未实现命令 {:?}", cmd);
                self.current_index += 1;
                DialogueResponse::Continue
            }
        }
    }

    /// 执行简单命令（用于循环增量等内联执行场景）
    /// 仅处理不产生对话响应的命令（如 set）
    fn execute_simple_command(&mut self, cmd: &ScriptCommand) {
        match cmd {
            ScriptCommand::Set(var_name, expr_value) => match expr_value {
                ExprValue::Int(value) => {
                    self.variables.insert(var_name.clone(), *value);
                }
                ExprValue::Str(value) => {
                    self.string_variables
                        .insert(var_name.clone(), value.clone());
                }
                ExprValue::Var(ref_var) => {
                    if let Some(val) = self.variables.get(ref_var).copied() {
                        self.variables.insert(var_name.clone(), val);
                    }
                    if let Some(val) = self.string_variables.get(ref_var).cloned() {
                        self.string_variables.insert(var_name.clone(), val);
                    }
                }
            },
            _ => {
                // 其他命令在内联执行场景中忽略
                tracing::debug!("execute_simple_command: 忽略命令 {:?}", cmd);
            }
        }
    }

    /// 处理玩家输入（如选择菜单、next 点击）
    /// input: 1-based 选择索引（rAthena 协议发送 1-based 索引）
    /// 自动将选择结果存储到 @menu 变量，脚本可通过 goto_if "@menu", N, "label" 分支
    pub fn handle_input(&mut self, input: usize) -> DialogueResponse {
        self.pending_next = false;
        self.last_select = input;
        // rAthena 约定：select 命令的选择结果存储到 @menu 变量
        self.variables.insert("@menu".to_string(), input as i64);
        self.process()
    }

    /// 处理文本输入（用于 input 命令）
    pub fn handle_text_input(&mut self, text: &str) -> DialogueResponse {
        self.pending_next = false;
        // 存储输入值到变量
        self.string_variables
            .insert("@input_result".to_string(), text.to_string());
        self.process()
    }

    /// 获取最近一次 Select 的选择结果
    pub fn last_select(&self) -> usize {
        self.last_select
    }

    /// 是否仍在对话中
    pub fn is_active(&self) -> bool {
        self.current_index < self.script.commands.len() && !self.paused_by_close2
    }
}

/// 对话响应
#[derive(Debug, Clone)]
pub enum DialogueResponse {
    /// 显示消息
    Message(String),
    /// 选择菜单
    Select(Vec<String>),
    /// 等待 next
    Pending,
    /// 继续下一条
    Continue,
    /// 传送
    Warp { map: String, x: u16, y: u16 },
    /// 关闭对话（包括 close 和 close2）
    Closed,
    /// 接收玩家文本输入
    Input(String),
    /// 全服公告
    Announce { message: String, flag: u8 },
    /// 延迟执行
    Sleep(u32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::script::parse_script;
    use uuid::Uuid;

    // ---- 原有测试（适配新的 Set 命令签名） ----

    #[test]
    fn test_simple_dialogue() {
        let script = parse_script(
            r#"
            mes "Hello!";
            next;
            mes "How are you?";
            close;
        "#,
        );

        let state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        assert!(state.is_active());
    }

    #[test]
    fn test_dialogue_process_mes() {
        let script = parse_script(r#"mes "Welcome!";"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Welcome!"),
            _ => panic!("Expected Message response"),
        }
    }

    #[test]
    fn test_dialogue_process_warp() {
        let script = parse_script(r#"warp "prontera", 150, 183;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        match response {
            DialogueResponse::Warp { map, x, y } => {
                assert_eq!(map, "prontera");
                assert_eq!(x, 150);
                assert_eq!(y, 183);
            }
            _ => panic!("Expected Warp response"),
        }
    }

    #[test]
    fn test_dialogue_select() {
        let script = parse_script(r#"select("Yes","No");"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        match response {
            DialogueResponse::Select(options) => {
                assert_eq!(options.len(), 2);
                assert_eq!(options[0], "Yes");
                assert_eq!(options[1], "No");
            }
            _ => panic!("Expected Select response"),
        }
    }

    #[test]
    fn test_dialogue_close() {
        let script = parse_script(
            r#"
            mes "Goodbye!";
            close;
        "#,
        );

        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let _ = state.process();
        let response = state.process();
        match response {
            DialogueResponse::Closed => {}
            _ => panic!("Expected Closed response"),
        }
    }

    #[test]
    fn test_set_stores_variable() {
        let script = parse_script(r#"set "$talked", 1;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        assert!(matches!(response, DialogueResponse::Continue));
        assert_eq!(state.get_variable("$talked"), Some(1));
    }

    #[test]
    fn test_set_multiple_variables() {
        let script = parse_script(
            r#"
            set "$count", 0;
            set "$name", 100;
            "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        state.process();
        state.process();
        assert_eq!(state.get_variable("$count"), Some(0));
        assert_eq!(state.get_variable("$name"), Some(100));
    }

    #[test]
    fn test_goto_uses_label() {
        let script = parse_script(
            r#"
            goto "skip";
            mes "This should be skipped";
        skip:
            mes "Arrived at skip";
            close;
            "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        let response = state.process();
        assert!(matches!(response, DialogueResponse::Continue));

        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Arrived at skip"),
            other => panic!("期望 Message(\"Arrived at skip\")，实际: {:?}", other),
        }
    }

    #[test]
    fn test_set_then_goto() {
        let script = parse_script(
            r#"
            set "$done", 1;
            goto "end_part";
            mes "Skipped";
        end_part:
            mes "Done!";
            close;
            "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        state.process();
        assert_eq!(state.get_variable("$done"), Some(1));

        state.process();

        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Done!"),
            other => panic!("期望 Message(\"Done!\"), 实际: {:?}", other),
        }
    }

    #[test]
    fn test_variable_default_none() {
        let script = parse_script(r#"mes "Hi";"#);
        let state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        assert_eq!(state.get_variable("$nonexistent"), None);
    }

    #[test]
    fn test_goto_if_condition_true() {
        let mut state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            parse_script(
                r#"
            set "$done", 1;
            goto_if "$done", 1, "skip";
            mes "Should be skipped";
        skip:
            mes "Jumped!";
            close;
            "#,
            ),
        );

        state.process();
        state.process();
        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Jumped!"),
            other => panic!("期望 Message(\"Jumped!\"), 实际: {:?}", other),
        }
    }

    #[test]
    fn test_goto_if_condition_false() {
        let mut state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            parse_script(
                r#"
            set "$done", 0;
            goto_if "$done", 1, "skip";
            mes "Not skipped";
        skip:
            mes "End";
            close;
            "#,
            ),
        );

        state.process();
        state.process();
        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Not skipped"),
            other => panic!("期望 Message(\"Not skipped\"), 实际: {:?}", other),
        }
    }

    #[test]
    fn test_goto_if_undefined_variable() {
        let mut state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            parse_script(
                r#"
            goto_if "$undefined", 1, "skip";
            mes "Variable undefined, no jump";
            close;
            "#,
            ),
        );

        let response = state.process();
        assert!(matches!(response, DialogueResponse::Continue));

        let response = state.process();
        match response {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Variable undefined, no jump"),
            other => panic!("期望不跳转，实际: {:?}", other),
        }
    }

    #[test]
    fn test_select_choice_affects_flow() {
        let mut state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            parse_script(
                r#"
            mes "Do you want to warp?";
            select("Yes","No");
            set "$choice", 1;
            goto_if "$choice", 1, "do_warp";
            mes "Okay, goodbye!";
            close;
        do_warp:
            mes "Warping!";
            warp "prontera", 150, 183;
            close;
            "#,
            ),
        );

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(_)));

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Select(_)));

        let resp = state.handle_input(1);
        assert!(matches!(resp, DialogueResponse::Continue));
        assert_eq!(state.last_select(), 1);

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        let resp = state.process();
        match resp {
            DialogueResponse::Message(msg) => assert_eq!(msg, "Warping!"),
            other => panic!("期望 Message(\"Warping!\"), 实际: {:?}", other),
        }

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Warp { .. }));
    }

    #[test]
    fn test_multi_step_dialogue_with_next() {
        let mut state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            parse_script(
                r#"
            mes "Step 1";
            next;
            mes "Step 2";
            next;
            mes "Step 3";
            close;
            "#,
            ),
        );

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Step 1"));

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Pending));

        let resp = state.handle_input(0);
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Step 2"));

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Pending));

        let resp = state.handle_input(0);
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Step 3"));

        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
    }

    #[test]
    fn test_empty_script_returns_closed() {
        let script = parse_script("");
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
    }

    #[test]
    fn test_goto_loops() {
        let mut state = NpcDialogueState::new(
            Uuid::new_v4(),
            1,
            parse_script(
                r#"
            set "$i", 0;
        loop_start:
            set "$i", 1;
            goto_if "$i", 3, "loop_end";
            set "$i", 1;
            goto "loop_start";
        loop_end:
            mes "Loop done";
            close;
            "#,
            ),
        );

        state.process();
        state.process();
        assert_eq!(state.get_variable("$i"), Some(1));
    }

    // ---- 新增命令测试 ----

    #[test]
    fn test_close2_preserves_state() {
        let script = parse_script(
            r#"
            mes "Hello!";
            close2;
            mes "This should not run until resumed";
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // 显示消息
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(_)));

        // close2 暂停对话
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
        assert!(state.is_paused_by_close2());
        assert!(!state.is_active());

        // 变量应该被保留
        state.set_variable("$test".to_string(), 42);
        assert_eq!(state.get_variable("$test"), Some(42));

        // 恢复对话
        state.resume_from_close2();
        assert!(state.is_active());

        let resp = state.process();
        assert!(
            matches!(resp, DialogueResponse::Message(ref s) if s == "This should not run until resumed")
        );
    }

    #[test]
    fn test_getitem_command() {
        let script = parse_script(r#"getitem 501, 10;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        // getitem 当前为 stub，返回 Continue
        assert!(matches!(resp, DialogueResponse::Continue));
    }

    #[test]
    fn test_delitem_command() {
        let script = parse_script(r#"delitem 501, 5;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
    }

    #[test]
    fn test_countitem_stores_result() {
        let script = parse_script(r#"countitem(501);"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        // stub 返回 0
        assert_eq!(state.get_variable("@countitem_result"), Some(0));
    }

    #[test]
    fn test_checkweight_stores_result() {
        let script = parse_script(r#"checkweight(501, 10);"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        // stub 返回 1（可以容纳）
        assert_eq!(state.get_variable("@checkweight_result"), Some(1));
    }

    #[test]
    fn test_getcharid_stores_result() {
        let script = parse_script(r#"getcharid(0);"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        // 默认 char_id = 100001
        assert_eq!(state.get_variable("@charid_result"), Some(100001));
    }

    #[test]
    fn test_getarg_from_call_stack() {
        let script = parse_script(
            r#"
        my_func:
            getarg(0);
            mes "Done";
            return;
        entry:
            callfunc "my_func"(42);
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // 跳转到 entry 标签
        // 注意：解析后 entry 标签的索引需要正确
        // 直接从 entry 开始执行
        // 找到 entry 标签位置
        let entry_idx = *state.script.labels.get("entry").unwrap();
        state.current_index = entry_idx;

        // callfunc "my_func"(42)
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // getarg(0) -> 应返回 42
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        assert_eq!(state.get_variable("@getarg_result"), Some(42));
    }

    #[test]
    fn test_strcharinfo_stores_result() {
        let script = parse_script(r#"strcharinfo(0);"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        state.context.char_name = "TestPlayer".to_string();
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        assert_eq!(
            state.get_string_variable("@strcharinfo_result"),
            Some(&"TestPlayer".to_string())
        );
    }

    #[test]
    fn test_readparam_stores_result() {
        let script = parse_script(r#"readparam(11);"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        state.context.base_level = 50;
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        assert_eq!(state.get_variable("@readparam_result"), Some(50));
    }

    #[test]
    fn test_heal_command() {
        let script = parse_script(r#"heal 100, 50;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        // heal 当前为 stub，返回 Continue
        assert!(matches!(resp, DialogueResponse::Continue));
    }

    #[test]
    fn test_announce_returns_response() {
        let script = parse_script(r#"announce "Hello everyone!", 0;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        match resp {
            DialogueResponse::Announce { message, flag } => {
                assert_eq!(message, "Hello everyone!");
                assert_eq!(flag, 0);
            }
            _ => panic!("期望 Announce 响应"),
        }
    }

    #[test]
    fn test_input_returns_response() {
        let script = parse_script(r#"input "$my_input";"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        match resp {
            DialogueResponse::Input(var) => {
                assert_eq!(var, "$my_input");
            }
            _ => panic!("期望 Input 响应"),
        }
    }

    #[test]
    fn test_sleep_returns_response() {
        let script = parse_script(r#"sleep 1000;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        match resp {
            DialogueResponse::Sleep(ms) => {
                assert_eq!(ms, 1000);
            }
            _ => panic!("期望 Sleep 响应"),
        }
    }

    #[test]
    fn test_rand_stores_result() {
        let script = parse_script(r#"rand(1, 100);"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        let value = state.get_variable("@rand_result").unwrap();
        assert!(
            value >= 1 && value <= 100,
            "随机数 {} 不在 [1, 100] 范围内",
            value
        );
    }

    #[test]
    fn test_set_var_reference() {
        let script = parse_script(
            r#"
            set "$a", 99;
            set "$b", "$a";
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        state.process(); // set "$a", 99
        state.process(); // set "$b", "$a"
        assert_eq!(state.get_variable("$a"), Some(99));
        assert_eq!(state.get_variable("$b"), Some(99));
    }

    #[test]
    fn test_getvariableofnpc_stores_result() {
        let script = parse_script(r#"getvariableofnpc("$myvar", "SomeNPC");"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));
        // stub 返回 0
        assert_eq!(state.get_variable("@npcvar_result"), Some(0));
    }

    // ---- 函数调用测试 ----

    #[test]
    fn test_callfunc_and_return() {
        let script = parse_script(
            r#"
            callfunc "my_func"();
            mes "Back from func";
            close;
        my_func:
            mes "Inside func";
            return;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // callfunc "my_func"() -> 跳转到 my_func 标签
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // mes "Inside func"
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Inside func"));

        // return -> 回到 callfunc 的下一条命令
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // mes "Back from func"
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Back from func"));

        // close
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
    }

    #[test]
    fn test_return_without_call_stack_closes() {
        let script = parse_script(r#"return;"#);
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
    }

    // ---- 循环控制测试 ----

    #[test]
    fn test_for_loop_basic() {
        let script = parse_script(
            r#"
            set "$i", 0;
            while ("$i" < 3);
            set "$i", "$i";
            endloop;
            mes "Done";
            close;
        "#,
        );
        // 注意：while + endloop 需要配合使用
        // 这里验证循环条件检查能正确工作
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // set "$i", 0
        state.process();
        assert_eq!(state.get_variable("$i"), Some(0));

        // while ("$i" < 3) -> 条件为真，进入循环
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // 验证循环上下文已入栈
        assert_eq!(state.loop_stack.len(), 1);
    }

    #[test]
    fn test_while_loop_condition_false() {
        let script = parse_script(
            r#"
            set "$i", 10;
            while ("$i" < 3);
            mes "Should not reach here";
            endloop;
            mes "Skipped loop";
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // set "$i", 10
        state.process();

        // while ("$i" < 3) -> 条件为假（10 < 3 = false），跳过循环
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // mes "Skipped loop"
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "Skipped loop"));
    }

    #[test]
    fn test_break_exits_loop() {
        let script = parse_script(
            r#"
            set "$i", 0;
            while ("$i" < 10);
            break;
            endloop;
            mes "After break";
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // set "$i", 0
        state.process();

        // while -> 条件为真
        state.process();

        // break -> 跳出循环
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // mes "After break"
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "After break"));
    }

    #[test]
    fn test_continue_skips_to_condition() {
        let script = parse_script(
            r#"
            set "$i", 0;
            while ("$i" < 3);
            continue;
            endloop;
            mes "Done";
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // set "$i", 0
        state.process();

        // while -> 条件为真
        state.process();

        // continue -> 跳回条件检查
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // 应回到 while 条件检查（while 在 index 1，即 ForStart）
        assert_eq!(state.current_index, 1);
    }

    // ---- 完整 NPC 脚本流程测试 ----

    #[test]
    fn test_npc_quest_flow() {
        let script = parse_script(
            r#"
            mes "欢迎来到新手村!";
            mes "你需要收集 5 个红色药水。";
            next;
            countitem(501);
            goto_if "@countitem_result", 5, "quest_done";
            mes "你还没有收集够，继续加油!";
            close;
        quest_done:
            mes "太好了！你完成了任务!";
            getitem 601, 1;
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // mes "欢迎来到新手村!"
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "欢迎来到新手村!"));

        // mes "你需要收集..."
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(_)));

        // next
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Pending));

        // 模拟玩家点击（handle_input 内部会调用 process()，处理 countitem）
        state.handle_input(0);

        // handle_input 已处理 countitem(501)，验证结果存入变量
        assert_eq!(state.get_variable("@countitem_result"), Some(0));

        // goto_if "@countitem_result", 5, "quest_done" -> 0 != 5，不跳转
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // mes "你还没有收集够..."
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s.contains("没有收集够")));

        // close
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
    }

    #[test]
    fn test_npc_trading_flow() {
        let script = parse_script(
            r#"
            mes "你想买什么?";
            select("红色药水 - 50z","蓝色药水 - 100z");
            goto_if "@menu", 1, "buy_red";
            goto_if "@menu", 2, "buy_blue";
            close;
        buy_red:
            mes "给你红色药水!";
            getitem 501, 10;
            close;
        buy_blue:
            mes "给你蓝色药水!";
            getitem 502, 5;
            close;
        "#,
        );
        let mut state = NpcDialogueState::new(Uuid::new_v4(), 1, script);

        // mes
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(_)));

        // select
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Select(_)));

        // 玩家选择第 1 项（handle_input 内部调用 process()，处理 goto_if 并跳转到 buy_red）
        state.handle_input(1);

        // 已跳转到 buy_red，下一条是 mes "给你红色药水!"
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Message(ref s) if s == "给你红色药水!"));

        // getitem 501, 10
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Continue));

        // close
        let resp = state.process();
        assert!(matches!(resp, DialogueResponse::Closed));
    }
}
