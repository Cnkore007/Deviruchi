/// 解析命令字符串
/// 输入: "@warp prontera 100 200"
/// 输出: ("warp", ["prontera", "100", "200"])
pub fn parse_command(input: &str) -> (String, Vec<String>) {
    let input = input.trim();

    // 移除所有前缀的 @ 符号
    let input = match input.strip_prefix('@') {
        Some(stripped) => stripped,
        None => return (String::new(), vec![]),
    };

    // 继续移除可能存在的额外 @ 符号 (例如 "@@warp" -> "warp")
    let input = input.trim_start_matches('@');

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), vec![]);
    }

    let command = parts[0].to_lowercase();
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    (command, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let (cmd, args) = parse_command("@warp prontera 100 200");
        assert_eq!(cmd, "warp");
        assert_eq!(args, vec!["prontera", "100", "200"]);
    }

    #[test]
    fn test_parse_command_no_args() {
        let (cmd, args) = parse_command("@heal");
        assert_eq!(cmd, "heal");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_command_uppercase() {
        let (cmd, _args) = parse_command("@WARP prontera");
        assert_eq!(cmd, "warp");
    }

    #[test]
    fn test_parse_command_invalid() {
        let (cmd, args) = parse_command("hello world");
        assert!(cmd.is_empty());
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_command_with_at_prefix() {
        let (cmd, _args) = parse_command("@@warp prontera");
        assert_eq!(cmd, "warp");
    }
}
