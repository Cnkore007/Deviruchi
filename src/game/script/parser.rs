use super::commands::{CompareOp, Expr, ExprValue, ParseError, ScriptCommand, ScriptNode};
use std::collections::HashMap;

/// 解析脚本文本（向后兼容，静默忽略错误行）
pub fn parse_script(script_text: &str) -> ScriptNode {
    match parse_script_checked(script_text) {
        Ok(node) => node,
        Err(_) => {
            // 向后兼容：有错误时仍然返回能解析的部分
            let mut commands = Vec::new();
            let mut labels = HashMap::new();
            for line in script_text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with("//") {
                    continue;
                }
                if line.ends_with(':') {
                    let label_name = line.trim_end_matches(':').to_string();
                    labels.insert(label_name, commands.len());
                    continue;
                }
                if let Some(cmd) = parse_line(line) {
                    commands.push(cmd);
                }
            }
            ScriptNode { commands, labels }
        }
    }
}

/// 解析脚本文本（带错误报告）
pub fn parse_script_checked(script_text: &str) -> Result<ScriptNode, Vec<ParseError>> {
    let mut commands = Vec::new();
    let mut labels = HashMap::new();
    let mut errors = Vec::new();

    for (line_idx, line) in script_text.lines().enumerate() {
        let line_num = line_idx + 1; // 1-based 行号
        let trimmed = line.trim();

        // 跳过空行和注释
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // 解析标签
        if trimmed.ends_with(':') {
            let label_name = trimmed.trim_end_matches(':').to_string();
            if label_name.is_empty() {
                errors.push(ParseError {
                    line: line_num,
                    source: trimmed.to_string(),
                    message: "标签名不能为空".to_string(),
                });
            } else {
                labels.insert(label_name, commands.len());
            }
            continue;
        }

        // 解析命令
        match parse_line(trimmed) {
            Some(cmd) => commands.push(cmd),
            None => {
                errors.push(ParseError {
                    line: line_num,
                    source: trimmed.to_string(),
                    message: format!("未知命令或语法错误: '{}'", trimmed),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(ScriptNode { commands, labels })
    } else {
        Err(errors)
    }
}

/// 解析单行命令
/// 注意：检查顺序很重要——必须先检查较长的前缀（如 close2 在 close 之前）
fn parse_line(line: &str) -> Option<ScriptCommand> {
    let line = line.trim().trim_end_matches(';');

    // ================================================================
    // 对话命令
    // ================================================================

    // mes "消息"
    if let Some(msg) = line.strip_prefix("mes ") {
        let msg = extract_string(msg)?;
        return Some(ScriptCommand::Mes(msg));
    }

    // next
    if line == "next" {
        return Some(ScriptCommand::Next);
    }

    // close2（必须在 close 之前检查，避免前缀冲突）
    if line == "close2" {
        return Some(ScriptCommand::Close2);
    }

    // close
    if line == "close" {
        return Some(ScriptCommand::Close);
    }

    // end
    if line == "end" {
        return Some(ScriptCommand::End);
    }

    // select("选项1","选项2",...)
    if let Some(opts) = line.strip_prefix("select(")
        && let Some(opts) = opts.strip_suffix(")")
    {
        let options: Vec<String> = opts
            .split(',')
            .filter_map(|s| extract_string(s.trim()))
            .collect();
        if !options.is_empty() {
            return Some(ScriptCommand::Select(options));
        }
    }

    // ================================================================
    // 传送
    // ================================================================

    // warp "地图", x, y
    if let Some(args) = line.strip_prefix("warp ") {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() >= 3 {
            let map = extract_string(parts[0])?;
            let x: u16 = parts[1].trim().parse().ok()?;
            let y: u16 = parts[2].trim().parse().ok()?;
            return Some(ScriptCommand::Warp(map, x, y));
        }
    }

    // ================================================================
    // 控制流
    // ================================================================

    // goto_if（必须在 goto 之前检查）
    if let Some(args) = line.strip_prefix("goto_if ") {
        let parts: Vec<&str> = args.splitn(3, ',').collect();
        if parts.len() == 3 {
            let var_name = extract_string(parts[0]);
            let value = parts[1].trim().parse::<i64>().ok();
            let label = extract_string(parts[2]);
            if let (Some(name), Some(val), Some(lbl)) = (var_name, value, label) {
                return Some(ScriptCommand::GotoIf(name, val, lbl));
            }
        }
    }

    // goto "label"  或  goto label
    if let Some(label_arg) = line.strip_prefix("goto ") {
        let label_arg = label_arg.trim();
        let label = if label_arg.starts_with('"') && label_arg.ends_with('"') {
            label_arg[1..label_arg.len() - 1].to_string()
        } else {
            label_arg.to_string()
        };
        if !label.is_empty() {
            return Some(ScriptCommand::Goto(label));
        }
    }

    // callfunc "函数名"(参数列表)
    if let Some(args) = line.strip_prefix("callfunc ") {
        return parse_callfunc(args);
    }

    // return
    if line == "return" {
        return Some(ScriptCommand::Return);
    }

    // for (初始化; 条件; 增量) —— 解析为 ForStart（条件 + 增量命令序列）
    // 注意：逐行解析限制下，for 的初始化命令会被丢弃，
    // 脚本作者应在 for 之前手动编写初始化命令。
    // 循环体由 ForStart 和 ForEnd 之间的命令组成，配合 endloop 使用。
    if let Some(rest) = line.strip_prefix("for ")
        && let Some(inner) = rest.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<&str> = inner.splitn(3, ';').collect();
            if parts.len() == 3 {
                let cond_expr = parse_expr(parts[1].trim());
                let step_cmd = parse_line(parts[2].trim());

                let mut inc_cmds = Vec::new();
                if let Some(cmd) = step_cmd {
                    inc_cmds.push(cmd);
                }

                if let Some(expr) = cond_expr {
                    return Some(ScriptCommand::ForStart(expr, inc_cmds));
                }
            }
        }

    // while (条件) —— 等价于 ForStart(条件, 空增量)
    if let Some(rest) = line.strip_prefix("while ")
        && let Some(inner) = rest.strip_prefix('(').and_then(|s| s.strip_suffix(')'))
            && let Some(expr) = parse_expr(inner.trim()) {
                return Some(ScriptCommand::ForStart(expr, Vec::new()));
            }

    // endloop —— 标记循环结束，对应 ForEnd
    if line == "endloop" {
        return Some(ScriptCommand::ForEnd);
    }

    // break
    if line == "break" {
        return Some(ScriptCommand::Break);
    }

    // continue
    if line == "continue" {
        return Some(ScriptCommand::Continue);
    }

    // ================================================================
    // set 命令（支持整数赋值和字符串变量赋值）
    // ================================================================

    // set "$var", value  或  set "$var", "$other_var"
    if let Some(args) = line.strip_prefix("set ") {
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            let var_name = extract_string(parts[0]);
            let value_str = parts[1].trim();
            if let Some(name) = var_name {
                // 尝试解析为整数
                if let Ok(value) = value_str.parse::<i64>() {
                    return Some(ScriptCommand::Set(name, ExprValue::Int(value)));
                }
                // 尝试解析为字符串变量引用
                if let Some(var_ref) = extract_string(value_str) {
                    return Some(ScriptCommand::Set(name, ExprValue::Var(var_ref)));
                }
            }
        }
    }

    // ================================================================
    // 物品操作
    // ================================================================

    // getitem <item_id>, <amount>
    if let Some(args) = line.strip_prefix("getitem ") {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() >= 2 {
            let item_id: u16 = parts[0].trim().parse().ok()?;
            let amount: u16 = parts[1].trim().parse().ok()?;
            return Some(ScriptCommand::GetItem(item_id, amount));
        }
    }

    // delitem <item_id>, <amount>
    if let Some(args) = line.strip_prefix("delitem ") {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() >= 2 {
            let item_id: u16 = parts[0].trim().parse().ok()?;
            let amount: u16 = parts[1].trim().parse().ok()?;
            return Some(ScriptCommand::DelItem(item_id, amount));
        }
    }

    // ================================================================
    // 信息查询（函数式语法）
    // ================================================================

    // countitem(<item_id>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("countitem(").and_then(|s| s.strip_suffix(')')) {
        let item_id: u16 = inner.trim().parse().ok()?;
        // 结果变量名默认为 @countitem_result
        return Some(ScriptCommand::CountItem(item_id, "@countitem_result".to_string()));
    }

    // checkweight(<item_id>, <amount>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("checkweight(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 2 {
            let item_id: u16 = parts[0].trim().parse().ok()?;
            let amount: u16 = parts[1].trim().parse().ok()?;
            return Some(ScriptCommand::CheckWeight(item_id, amount, "@checkweight_result".to_string()));
        }
    }

    // getcharid(<type>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("getcharid(").and_then(|s| s.strip_suffix(')')) {
        let type_id: u8 = inner.trim().parse().ok()?;
        return Some(ScriptCommand::GetCharId(type_id, "@charid_result".to_string()));
    }

    // getarg(<index>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("getarg(").and_then(|s| s.strip_suffix(')')) {
        let index: u16 = inner.trim().parse().ok()?;
        return Some(ScriptCommand::GetArg(index, "@getarg_result".to_string()));
    }

    // strcharinfo(<type>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("strcharinfo(").and_then(|s| s.strip_suffix(')')) {
        let type_id: u8 = inner.trim().parse().ok()?;
        return Some(ScriptCommand::StrCharInfo(type_id, "@strcharinfo_result".to_string()));
    }

    // readparam(<param>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("readparam(").and_then(|s| s.strip_suffix(')')) {
        let param_id: u16 = inner.trim().parse().ok()?;
        return Some(ScriptCommand::ReadParam(param_id, "@readparam_result".to_string()));
    }

    // ================================================================
    // 状态操作
    // ================================================================

    // heal <hp>, <sp>
    if let Some(args) = line.strip_prefix("heal ") {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() >= 2 {
            let hp: u32 = parts[0].trim().parse().ok()?;
            let sp: u32 = parts[1].trim().parse().ok()?;
            return Some(ScriptCommand::Heal(hp, sp));
        }
    }

    // ================================================================
    // 系统操作
    // ================================================================

    // announce "消息", flag
    if let Some(args) = line.strip_prefix("announce ") {
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() >= 2 {
            let msg = extract_string(parts[0])?;
            let flag: u8 = parts[1].trim().parse().ok()?;
            return Some(ScriptCommand::Announce(msg, flag));
        }
    }

    // input "变量名"  或  input $var
    if let Some(args) = line.strip_prefix("input ") {
        let args = args.trim();
        let var_name = if args.starts_with('"') && args.ends_with('"') {
            args[1..args.len() - 1].to_string()
        } else if args.starts_with('$') || args.starts_with('@') || args.starts_with('.') {
            args.to_string()
        } else {
            return None;
        };
        return Some(ScriptCommand::Input(var_name));
    }

    // sleep <milliseconds>
    if let Some(args) = line.strip_prefix("sleep ") {
        let ms: u32 = args.trim().parse().ok()?;
        return Some(ScriptCommand::Sleep(ms));
    }

    // ================================================================
    // 数值操作
    // ================================================================

    // getvariableofnpc "变量名", "NPC名" -> 存入结果变量
    if let Some(inner) = line
        .strip_prefix("getvariableofnpc(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.splitn(2, ',').collect();
        if parts.len() >= 2 {
            let var_name = extract_string(parts[0])?;
            let npc_name = extract_string(parts[1])?;
            return Some(ScriptCommand::GetVariableOfNpc(
                var_name,
                npc_name,
                "@npcvar_result".to_string(),
            ));
        }
    }

    // rand(<min>, <max>) -> 存入结果变量
    if let Some(inner) = line.strip_prefix("rand(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 2 {
            let min: i64 = parts[0].trim().parse().ok()?;
            let max: i64 = parts[1].trim().parse().ok()?;
            return Some(ScriptCommand::Rand(min, max, "@rand_result".to_string()));
        }
    }

    None
}

/// 解析 callfunc 命令
/// 格式: callfunc "函数名"(arg1, arg2, ...)
fn parse_callfunc(args: &str) -> Option<ScriptCommand> {
    let args = args.trim();
    // 找到函数名（带引号或不带引号）
    let (func_name, rest) = if let Some(rest) = args.strip_prefix('"') {
        let end_quote = rest.find('"')?;
        let name = &rest[..end_quote];
        (name.to_string(), &rest[end_quote + 1..])
    } else {
        // 不带引号：找到 '(' 的位置
        let paren_pos = args.find('(')?;
        let name = args[..paren_pos].trim().to_string();
        (name, &args[paren_pos..])
    };

    if func_name.is_empty() {
        return None;
    }

    // 解析参数列表
    let args_str = rest.strip_prefix('(')?.strip_suffix(')')?;
    let call_args: Vec<i64> = if args_str.trim().is_empty() {
        Vec::new()
    } else {
        args_str
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect()
    };

    Some(ScriptCommand::CallFunc(func_name, call_args))
}

/// 解析简单表达式（用于 for/while 条件）
/// 支持: 变量比较整数、变量比较变量、纯变量判断、纯整数判断
fn parse_expr(s: &str) -> Option<Expr> {
    let s = s.trim();

    // 先尝试解析比较表达式（优先于纯字符串匹配，避免 "$a" < "$b" 被误判为字符串）
    // 支持运算符: ==, !=, <, <=, >, >=
    let ops = [
        ("!=", CompareOp::Ne),
        ("==", CompareOp::Eq),
        ("<=", CompareOp::Le),
        (">=", CompareOp::Ge),
        ("<", CompareOp::Lt),
        (">", CompareOp::Gt),
    ];

    for (op_str, op) in &ops {
        if let Some(pos) = s.find(op_str) {
            let left_str = s[..pos].trim();
            let right_str = s[pos + op_str.len()..].trim();

            let left = extract_string(left_str).unwrap_or_else(|| left_str.to_string());
            let right_val = right_str.parse::<i64>();

            if let Ok(right) = right_val {
                return Some(Expr::CompareVar(left, *op, right));
            }
            // 右边也是变量
            let right_var = extract_string(right_str).unwrap_or_else(|| right_str.to_string());
            return Some(Expr::CompareVarVar(left, *op, right_var));
        }
    }

    // 纯引号包裹的变量名："$var" -> VarTruthy
    if let Some(var_name) = extract_string(s) {
        return Some(Expr::VarTruthy(var_name));
    }

    // 尝试解析为纯整数
    if let Ok(val) = s.parse::<i64>() {
        return Some(Expr::IntTruthy(val));
    }

    // 无法解析
    None
}

/// 从字符串提取值（支持双引号包裹和 [NPC Name] 格式）
fn extract_string(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') {
        Some(s[1..s.len() - 1].to_string())
    } else if s.starts_with('[') && s.ends_with(']') {
        // [NPC Name] 格式
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 原有测试（保持不变） ----

    #[test]
    fn test_parse_mes() {
        let node = parse_script(r#"mes "Hello, world!";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Mes(msg) = &node.commands[0] {
            assert_eq!(msg, "Hello, world!");
        } else {
            panic!("Expected Mes command");
        }
    }

    #[test]
    fn test_parse_select() {
        let node = parse_script(r#"select("Option 1","Option 2");"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Select(opts) = &node.commands[0] {
            assert_eq!(opts.len(), 2);
        }
    }

    #[test]
    fn test_parse_warp() {
        let node = parse_script(r#"warp "prontera", 100, 200;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Warp(map, x, y) = &node.commands[0] {
            assert_eq!(map, "prontera");
            assert_eq!(*x, 100);
            assert_eq!(*y, 200);
        }
    }

    #[test]
    fn test_parse_multi_line() {
        let node = parse_script(
            r#"
            mes "Hello!";
            next;
            mes "How are you?";
            close;
        "#,
        );
        assert_eq!(node.commands.len(), 4);
    }

    #[test]
    fn test_parse_goto() {
        let node = parse_script(r#"goto "my_label";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Goto(label) = &node.commands[0] {
            assert_eq!(label, "my_label");
        } else {
            panic!("期望 Goto 命令，实际: {:?}", node.commands[0]);
        }
    }

    #[test]
    fn test_parse_goto_without_quotes() {
        let node = parse_script(r#"goto my_label;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Goto(label) = &node.commands[0] {
            assert_eq!(label, "my_label");
        } else {
            panic!("期望 Goto 命令");
        }
    }

    #[test]
    fn test_parse_set_string_var() {
        let node = parse_script(r#"set "$var", 42;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Set(name, ExprValue::Int(value)) = &node.commands[0] {
            assert_eq!(name, "$var");
            assert_eq!(*value, 42);
        } else {
            panic!("期望 Set 命令，实际: {:?}", node.commands[0]);
        }
    }

    #[test]
    fn test_parse_set_negative_value() {
        let node = parse_script(r#"set "$counter", -5;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Set(name, ExprValue::Int(value)) = &node.commands[0] {
            assert_eq!(name, "$counter");
            assert_eq!(*value, -5);
        } else {
            panic!("期望 Set 命令");
        }
    }

    #[test]
    fn test_parse_goto_and_set_in_script() {
        let script = r#"
            mes "Hello!";
            set "$talked", 1;
            goto "end_label";
        end_label:
            close;
        "#;
        let node = parse_script(script);
        assert_eq!(node.commands.len(), 4);
        assert!(node.labels.contains_key("end_label"));
    }

    #[test]
    fn test_parse_goto_label_with_underscore() {
        let node = parse_script(r#"goto "my_long_label_1";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Goto(label) = &node.commands[0] {
            assert_eq!(label, "my_long_label_1");
        }
    }

    #[test]
    fn test_parse_error_on_invalid_line() {
        let result = parse_script_checked("this is not a valid command;");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 1);
        assert!(errors[0].message.contains("未知命令"));
    }

    #[test]
    fn test_parse_error_reports_line_number() {
        let script = r#"
            mes "Hello";
            invalid_command;
            mes "World";
        "#;
        let result = parse_script_checked(script);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].line >= 2 && errors[0].line <= 4);
    }

    #[test]
    fn test_parse_error_multiple_errors() {
        let script = r#"
            bad_cmd_1;
            mes "OK";
            bad_cmd_2;
        "#;
        let result = parse_script_checked(script);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_parse_valid_script_returns_ok() {
        let script = r#"
            mes "Hello!";
            next;
            mes "World!";
            close;
        "#;
        let result = parse_script_checked(script);
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.commands.len(), 4);
    }

    #[test]
    fn test_parse_goto_if() {
        let node = parse_script(r#"goto_if "$done", 1, "end_label";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::GotoIf(var, val, label) = &node.commands[0] {
            assert_eq!(var, "$done");
            assert_eq!(*val, 1);
            assert_eq!(label, "end_label");
        } else {
            panic!("期望 GotoIf 命令");
        }
    }

    // ---- 新增命令解析测试 ----

    #[test]
    fn test_parse_close2() {
        let node = parse_script(r#"close2;"#);
        assert_eq!(node.commands.len(), 1);
        assert!(matches!(node.commands[0], ScriptCommand::Close2));
    }

    #[test]
    fn test_parse_close2_not_confused_with_close() {
        // close2 必须正确解析，不能被 close 的前缀匹配拦截
        let node = parse_script(r#"close2;"#);
        assert!(matches!(node.commands[0], ScriptCommand::Close2));

        let node = parse_script(r#"close;"#);
        assert!(matches!(node.commands[0], ScriptCommand::Close));
    }

    #[test]
    fn test_parse_getitem() {
        let node = parse_script(r#"getitem 501, 10;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::GetItem(item_id, amount) = &node.commands[0] {
            assert_eq!(*item_id, 501);
            assert_eq!(*amount, 10);
        } else {
            panic!("期望 GetItem 命令");
        }
    }

    #[test]
    fn test_parse_delitem() {
        let node = parse_script(r#"delitem 501, 5;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::DelItem(item_id, amount) = &node.commands[0] {
            assert_eq!(*item_id, 501);
            assert_eq!(*amount, 5);
        } else {
            panic!("期望 DelItem 命令");
        }
    }

    #[test]
    fn test_parse_countitem() {
        let node = parse_script(r#"countitem(501);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::CountItem(item_id, var) = &node.commands[0] {
            assert_eq!(*item_id, 501);
            assert_eq!(var, "@countitem_result");
        } else {
            panic!("期望 CountItem 命令");
        }
    }

    #[test]
    fn test_parse_checkweight() {
        let node = parse_script(r#"checkweight(501, 10);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::CheckWeight(item_id, amount, var) = &node.commands[0] {
            assert_eq!(*item_id, 501);
            assert_eq!(*amount, 10);
            assert_eq!(var, "@checkweight_result");
        } else {
            panic!("期望 CheckWeight 命令");
        }
    }

    #[test]
    fn test_parse_getcharid() {
        let node = parse_script(r#"getcharid(0);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::GetCharId(type_id, var) = &node.commands[0] {
            assert_eq!(*type_id, 0);
            assert_eq!(var, "@charid_result");
        } else {
            panic!("期望 GetCharId 命令");
        }
    }

    #[test]
    fn test_parse_getarg() {
        let node = parse_script(r#"getarg(0);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::GetArg(index, var) = &node.commands[0] {
            assert_eq!(*index, 0);
            assert_eq!(var, "@getarg_result");
        } else {
            panic!("期望 GetArg 命令");
        }
    }

    #[test]
    fn test_parse_strcharinfo() {
        let node = parse_script(r#"strcharinfo(0);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::StrCharInfo(type_id, var) = &node.commands[0] {
            assert_eq!(*type_id, 0);
            assert_eq!(var, "@strcharinfo_result");
        } else {
            panic!("期望 StrCharInfo 命令");
        }
    }

    #[test]
    fn test_parse_readparam() {
        let node = parse_script(r#"readparam(5);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::ReadParam(param_id, var) = &node.commands[0] {
            assert_eq!(*param_id, 5);
            assert_eq!(var, "@readparam_result");
        } else {
            panic!("期望 ReadParam 命令");
        }
    }

    #[test]
    fn test_parse_heal() {
        let node = parse_script(r#"heal 100, 50;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Heal(hp, sp) = &node.commands[0] {
            assert_eq!(*hp, 100);
            assert_eq!(*sp, 50);
        } else {
            panic!("期望 Heal 命令");
        }
    }

    #[test]
    fn test_parse_announce() {
        let node = parse_script(r#"announce "Server maintenance in 10 minutes", 0;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Announce(msg, flag) = &node.commands[0] {
            assert_eq!(msg, "Server maintenance in 10 minutes");
            assert_eq!(*flag, 0);
        } else {
            panic!("期望 Announce 命令");
        }
    }

    #[test]
    fn test_parse_input() {
        let node = parse_script(r#"input "$player_input";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Input(var) = &node.commands[0] {
            assert_eq!(var, "$player_input");
        } else {
            panic!("期望 Input 命令");
        }
    }

    #[test]
    fn test_parse_input_without_quotes() {
        let node = parse_script(r#"input @myvar;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Input(var) = &node.commands[0] {
            assert_eq!(var, "@myvar");
        } else {
            panic!("期望 Input 命令");
        }
    }

    #[test]
    fn test_parse_sleep() {
        let node = parse_script(r#"sleep 1000;"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Sleep(ms) = &node.commands[0] {
            assert_eq!(*ms, 1000);
        } else {
            panic!("期望 Sleep 命令");
        }
    }

    #[test]
    fn test_parse_callfunc() {
        let node = parse_script(r#"callfunc "my_function"(1, 2, 3);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::CallFunc(name, args) = &node.commands[0] {
            assert_eq!(name, "my_function");
            assert_eq!(args, &vec![1, 2, 3]);
        } else {
            panic!("期望 CallFunc 命令");
        }
    }

    #[test]
    fn test_parse_callfunc_no_args() {
        let node = parse_script(r#"callfunc "my_func"();"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::CallFunc(name, args) = &node.commands[0] {
            assert_eq!(name, "my_func");
            assert!(args.is_empty());
        } else {
            panic!("期望 CallFunc 命令");
        }
    }

    #[test]
    fn test_parse_return() {
        let node = parse_script(r#"return;"#);
        assert_eq!(node.commands.len(), 1);
        assert!(matches!(node.commands[0], ScriptCommand::Return));
    }

    #[test]
    fn test_parse_break() {
        let node = parse_script(r#"break;"#);
        assert_eq!(node.commands.len(), 1);
        assert!(matches!(node.commands[0], ScriptCommand::Break));
    }

    #[test]
    fn test_parse_continue() {
        let node = parse_script(r#"continue;"#);
        assert_eq!(node.commands.len(), 1);
        assert!(matches!(node.commands[0], ScriptCommand::Continue));
    }

    #[test]
    fn test_parse_rand() {
        let node = parse_script(r#"rand(1, 100);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Rand(min, max, var) = &node.commands[0] {
            assert_eq!(*min, 1);
            assert_eq!(*max, 100);
            assert_eq!(var, "@rand_result");
        } else {
            panic!("期望 Rand 命令");
        }
    }

    #[test]
    fn test_parse_getvariableofnpc() {
        let node = parse_script(r#"getvariableofnpc("$myvar", "SomeNPC");"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::GetVariableOfNpc(var, npc, result_var) = &node.commands[0] {
            assert_eq!(var, "$myvar");
            assert_eq!(npc, "SomeNPC");
            assert_eq!(result_var, "@npcvar_result");
        } else {
            panic!("期望 GetVariableOfNpc 命令");
        }
    }

    #[test]
    fn test_parse_set_var_ref() {
        // set "$a", "$b" —— 将变量 b 的值赋给变量 a
        let node = parse_script(r#"set "$a", "$b";"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::Set(name, ExprValue::Var(ref_var)) = &node.commands[0] {
            assert_eq!(name, "$a");
            assert_eq!(ref_var, "$b");
        } else {
            panic!("期望 Set 命令（变量引用），实际: {:?}", node.commands[0]);
        }
    }

    // ---- 表达式解析测试 ----

    #[test]
    fn test_parse_expr_var_truthy() {
        let expr = parse_expr(r#""$myvar""#).unwrap();
        if let Expr::VarTruthy(name) = expr {
            assert_eq!(name, "$myvar");
        } else {
            panic!("期望 VarTruthy");
        }
    }

    #[test]
    fn test_parse_expr_int_truthy() {
        let expr = parse_expr("42").unwrap();
        assert!(matches!(expr, Expr::IntTruthy(42)));
    }

    #[test]
    fn test_parse_expr_compare_eq() {
        let expr = parse_expr(r#""$count" == 5"#).unwrap();
        if let Expr::CompareVar(name, op, val) = expr {
            assert_eq!(name, "$count");
            assert_eq!(op, CompareOp::Eq);
            assert_eq!(val, 5);
        } else {
            panic!("期望 CompareVar");
        }
    }

    #[test]
    fn test_parse_expr_compare_lt() {
        let expr = parse_expr(r#""$i" < 10"#).unwrap();
        if let Expr::CompareVar(name, op, val) = expr {
            assert_eq!(name, "$i");
            assert_eq!(op, CompareOp::Lt);
            assert_eq!(val, 10);
        } else {
            panic!("期望 CompareVar");
        }
    }

    #[test]
    fn test_parse_expr_compare_var_var() {
        let expr = parse_expr(r#""$a" < "$b""#).unwrap();
        if let Expr::CompareVarVar(left, op, right) = expr {
            assert_eq!(left, "$a");
            assert_eq!(op, CompareOp::Lt);
            assert_eq!(right, "$b");
        } else {
            panic!("期望 CompareVarVar");
        }
    }

    #[test]
    fn test_parse_while() {
        let node = parse_script(r#"while ("$i" < 10);"#);
        assert_eq!(node.commands.len(), 1);
        if let ScriptCommand::ForStart(expr, inc) = &node.commands[0] {
            // 验证条件表达式
            let vars = HashMap::from([("$i".to_string(), 5)]);
            assert!(expr.eval(&vars));
            // while 循环的增量为空
            assert!(inc.is_empty());
        } else {
            panic!("期望 ForStart 命令");
        }
    }

    #[test]
    fn test_parse_endloop() {
        let node = parse_script(r#"endloop;"#);
        assert_eq!(node.commands.len(), 1);
        assert!(matches!(node.commands[0], ScriptCommand::ForEnd));
    }

    // ---- 完整脚本解析测试 ----

    #[test]
    fn test_full_npc_quest_script() {
        let script = r#"
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
        "#;
        let node = parse_script(script);
        // mes + mes + next + countitem + goto_if + mes + close + mes + getitem + close = 10
        assert_eq!(node.commands.len(), 10);
        assert!(node.labels.contains_key("quest_done"));
    }

    #[test]
    fn test_full_trading_script() {
        let script = r#"
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
        "#;
        let node = parse_script(script);
        assert_eq!(node.commands.len(), 11);
    }
}
