use super::commands::{ScriptCommand, ScriptNode};
use std::collections::HashMap;

/// 解析脚本文本
pub fn parse_script(script_text: &str) -> ScriptNode {
    let mut commands = Vec::new();
    let mut labels = HashMap::new();

    for line in script_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // 解析标签
        if line.ends_with(':') {
            let label_name = line.trim_end_matches(':').to_string();
            labels.insert(label_name, commands.len());
            continue;
        }

        // 解析命令
        if let Some(cmd) = parse_line(line) {
            commands.push(cmd);
        }
    }

    ScriptNode { commands, labels }
}

fn parse_line(line: &str) -> Option<ScriptCommand> {
    let line = line.trim().trim_end_matches(';');

    // mes "消息"
    if let Some(msg) = line.strip_prefix("mes ") {
        let msg = extract_string(msg)?;
        return Some(ScriptCommand::Mes(msg));
    }

    // next
    if line == "next" {
        return Some(ScriptCommand::Next);
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

    None
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
