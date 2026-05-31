//! NPC 聊天模式匹配系统
//!
//! 对应 rAthena 的 `src/map/npc_chat.cpp`，提供 NPC 对玩家聊天消息的正则匹配。
//!
//! 使用方式（对应 rAthena NPC 脚本命令）：
//! - `defpattern set_id, "regex", "label"` — 定义模式
//! - `activatepset set_id` — 激活模式集
//! - `deactivatepset set_id` — 停用模式集（-1 = 停用全部）
//! - `deletepset set_id` — 删除模式集

use std::collections::HashMap;
use parking_lot::RwLock;
use uuid::Uuid;

/// 模式匹配条目
#[derive(Debug, Clone)]
pub struct PatternEntry {
    /// 正则表达式模式
    pub pattern: String,
    /// 匹配后跳转的脚本标签
    pub label: String,
    /// 编译后的正则（延迟初始化）
    compiled: Option<regex::Regex>,
}

impl PatternEntry {
    /// 创建新的模式条目
    pub fn new(pattern: &str, label: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            label: label.to_string(),
            compiled: None,
        }
    }

    /// 编译正则表达式
    pub fn compile(&mut self) -> Result<(), String> {
        if self.compiled.is_some() {
            return Ok(());
        }
        let re = regex::Regex::new(&self.pattern)
            .map_err(|e| format!("Invalid regex '{}': {}", self.pattern, e))?;
        self.compiled = Some(re);
        Ok(())
    }

    /// 尝试匹配文本，返回捕获组
    pub fn matches(&self, text: &str) -> Option<Vec<String>> {
        let re = self.compiled.as_ref()?;
        let caps = re.captures(text)?;
        let mut groups = Vec::new();
        // 第 0 组是整个匹配
        groups.push(caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default());
        // 后续组是捕获组
        for i in 1..caps.len() {
            groups.push(caps.get(i).map(|m| m.as_str().to_string()).unwrap_or_default());
        }
        Some(groups)
    }
}

/// 模式集（可整体激活/停用）
#[derive(Debug, Clone)]
pub struct PatternSet {
    /// 模式集 ID
    pub id: i64,
    /// 是否激活
    pub active: bool,
    /// 模式列表
    pub entries: Vec<PatternEntry>,
}

impl PatternSet {
    /// 创建新的模式集
    pub fn new(id: i64) -> Self {
        Self {
            id,
            active: false,
            entries: Vec::new(),
        }
    }

    /// 添加模式
    pub fn add_pattern(&mut self, pattern: &str, label: &str) -> Result<(), String> {
        let mut entry = PatternEntry::new(pattern, label);
        entry.compile()?;
        self.entries.push(entry);
        Ok(())
    }

    /// 匹配文本，返回第一个匹配的标签和捕获组
    pub fn try_match(&self, text: &str) -> Option<(String, Vec<String>)> {
        if !self.active {
            return None;
        }
        for entry in &self.entries {
            if let Some(groups) = entry.matches(text) {
                return Some((entry.label.clone(), groups));
            }
        }
        None
    }
}

/// 匹配结果
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// NPC ID
    pub npc_id: Uuid,
    /// 跳转的脚本标签
    pub label: String,
    /// 捕获组（p0 = 完整匹配, p1-p9 = 捕获组）
    pub captures: Vec<String>,
}

/// NPC 聊天模式匹配管理器
///
/// 管理所有 NPC 的聊天模式匹配规则。
pub struct NpcChatManager {
    /// NPC 模式集 (npc_id -> pattern_sets)
    npc_patterns: RwLock<HashMap<Uuid, HashMap<i64, PatternSet>>>,
}

impl NpcChatManager {
    /// 创建空的管理器
    pub fn new() -> Self {
        Self {
            npc_patterns: RwLock::new(HashMap::new()),
        }
    }

    /// 定义模式
    ///
    /// 对应 rAthena 的 `defpattern` 命令。
    pub fn define_pattern(
        &self,
        npc_id: Uuid,
        set_id: i64,
        pattern: &str,
        label: &str,
    ) -> Result<(), String> {
        let mut npc_patterns = self.npc_patterns.write();
        let sets = npc_patterns
            .entry(npc_id)
            .or_insert_with(HashMap::new);

        let set = sets
            .entry(set_id)
            .or_insert_with(|| PatternSet::new(set_id));

        set.add_pattern(pattern, label)
    }

    /// 激活模式集
    ///
    /// 对应 rAthena 的 `activatepset` 命令。
    pub fn activate_set(&self, npc_id: Uuid, set_id: i64) {
        let mut npc_patterns = self.npc_patterns.write();
        if let Some(sets) = npc_patterns.get_mut(&npc_id) {
            if let Some(set) = sets.get_mut(&set_id) {
                set.active = true;
            }
        }
    }

    /// 停用模式集
    ///
    /// 对应 rAthena 的 `deactivatepset` 命令。
    /// `set_id = -1` 表示停用该 NPC 的所有模式集。
    pub fn deactivate_set(&self, npc_id: Uuid, set_id: i64) {
        let mut npc_patterns = self.npc_patterns.write();
        if let Some(sets) = npc_patterns.get_mut(&npc_id) {
            if set_id == -1 {
                // 停用所有
                for set in sets.values_mut() {
                    set.active = false;
                }
            } else if let Some(set) = sets.get_mut(&set_id) {
                set.active = false;
            }
        }
    }

    /// 删除模式集
    ///
    /// 对应 rAthena 的 `deletepset` 命令。
    pub fn delete_set(&self, npc_id: Uuid, set_id: i64) {
        let mut npc_patterns = self.npc_patterns.write();
        if let Some(sets) = npc_patterns.get_mut(&npc_id) {
            sets.remove(&set_id);
        }
    }

    /// 处理玩家聊天消息
    ///
    /// 遍历所有激活的模式集，返回第一个匹配结果。
    pub fn process_message(&self, npc_id: Uuid, message: &str) -> Option<MatchResult> {
        let npc_patterns = self.npc_patterns.read();
        let sets = npc_patterns.get(&npc_id)?;

        for set in sets.values() {
            if let Some((label, captures)) = set.try_match(message) {
                return Some(MatchResult {
                    npc_id,
                    label,
                    captures,
                });
            }
        }

        None
    }

    /// 处理全局聊天消息
    ///
    /// 遍历所有 NPC 的激活模式集，返回所有匹配结果。
    pub fn process_global_message(&self, message: &str) -> Vec<MatchResult> {
        let npc_patterns = self.npc_patterns.read();
        let mut results = Vec::new();

        for (&npc_id, sets) in npc_patterns.iter() {
            for set in sets.values() {
                if let Some((label, captures)) = set.try_match(message) {
                    results.push(MatchResult {
                        npc_id,
                        label,
                        captures,
                    });
                }
            }
        }

        results
    }

    /// 清理 NPC 的所有模式
    pub fn clear_npc(&self, npc_id: &Uuid) {
        self.npc_patterns.write().remove(npc_id);
    }

    /// 清理所有
    pub fn clear(&self) {
        self.npc_patterns.write().clear();
    }
}

impl Default for NpcChatManager {
    fn default() -> Self {
        Self::new()
    }
}

// 需要添加 regex 依赖到 Cargo.toml
// regex = "1"

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_entry_compile() {
        let mut entry = PatternEntry::new(r"^hello\s+(\w+)$", "greeting");
        assert!(entry.compile().is_ok());
        assert!(entry.compiled.is_some());
    }

    #[test]
    fn test_pattern_entry_compile_invalid() {
        let mut entry = PatternEntry::new("[invalid", "label");
        assert!(entry.compile().is_err());
    }

    #[test]
    fn test_pattern_entry_matches() {
        let mut entry = PatternEntry::new(r"^(\w+) loves (\w+)$", "love");
        entry.compile().unwrap();

        let result = entry.matches("Alice loves Bob");
        assert!(result.is_some());
        let groups = result.unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0], "Alice loves Bob");
        assert_eq!(groups[1], "Alice");
        assert_eq!(groups[2], "Bob");

        assert!(entry.matches("no match here").is_none());
    }

    #[test]
    fn test_pattern_set() {
        let mut set = PatternSet::new(1);
        set.active = true;
        set.add_pattern(r"hello", "greet").unwrap();
        set.add_pattern(r"bye", "farewell").unwrap();

        let result = set.try_match("hello world");
        assert!(result.is_some());
        let (label, groups) = result.unwrap();
        assert_eq!(label, "greet");
        assert_eq!(groups[0], "hello");

        let result = set.try_match("farewell");
        assert!(result.is_none());
    }

    #[test]
    fn test_pattern_set_inactive() {
        let mut set = PatternSet::new(1);
        set.active = false;
        set.add_pattern(r"hello", "greet").unwrap();

        let result = set.try_match("hello world");
        assert!(result.is_none());
    }

    #[test]
    fn test_npc_chat_manager_define_and_activate() {
        let manager = NpcChatManager::new();
        let npc_id = Uuid::new_v4();

        manager
            .define_pattern(npc_id, 1, r"hello", "greet")
            .unwrap();
        manager.activate_set(npc_id, 1);

        let result = manager.process_message(npc_id, "hello world");
        assert!(result.is_some());
        assert_eq!(result.unwrap().label, "greet");
    }

    #[test]
    fn test_npc_chat_manager_deactivate() {
        let manager = NpcChatManager::new();
        let npc_id = Uuid::new_v4();

        manager
            .define_pattern(npc_id, 1, r"hello", "greet")
            .unwrap();
        manager.activate_set(npc_id, 1);

        manager.deactivate_set(npc_id, 1);
        let result = manager.process_message(npc_id, "hello world");
        assert!(result.is_none());
    }

    #[test]
    fn test_npc_chat_manager_deactivate_all() {
        let manager = NpcChatManager::new();
        let npc_id = Uuid::new_v4();

        manager
            .define_pattern(npc_id, 1, r"hello", "greet")
            .unwrap();
        manager
            .define_pattern(npc_id, 2, r"bye", "farewell")
            .unwrap();
        manager.activate_set(npc_id, 1);
        manager.activate_set(npc_id, 2);

        manager.deactivate_set(npc_id, -1);

        assert!(manager.process_message(npc_id, "hello").is_none());
        assert!(manager.process_message(npc_id, "bye").is_none());
    }

    #[test]
    fn test_npc_chat_manager_delete_set() {
        let manager = NpcChatManager::new();
        let npc_id = Uuid::new_v4();

        manager
            .define_pattern(npc_id, 1, r"hello", "greet")
            .unwrap();
        manager.activate_set(npc_id, 1);

        manager.delete_set(npc_id, 1);
        let result = manager.process_message(npc_id, "hello world");
        assert!(result.is_none());
    }

    #[test]
    fn test_npc_chat_manager_captures() {
        let manager = NpcChatManager::new();
        let npc_id = Uuid::new_v4();

        manager
            .define_pattern(npc_id, 1, r"^(\w+) loves (\w+)$", "love")
            .unwrap();
        manager.activate_set(npc_id, 1);

        let result = manager.process_message(npc_id, "Alice loves Bob");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.label, "love");
        assert_eq!(result.captures.len(), 3);
        assert_eq!(result.captures[1], "Alice");
        assert_eq!(result.captures[2], "Bob");
    }

    #[test]
    fn test_global_message() {
        let manager = NpcChatManager::new();
        let npc1 = Uuid::new_v4();
        let npc2 = Uuid::new_v4();

        manager
            .define_pattern(npc1, 1, r"hello", "greet1")
            .unwrap();
        manager
            .define_pattern(npc2, 1, r"hello", "greet2")
            .unwrap();
        manager.activate_set(npc1, 1);
        manager.activate_set(npc2, 1);

        let results = manager.process_global_message("hello world");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_clear_npc() {
        let manager = NpcChatManager::new();
        let npc_id = Uuid::new_v4();

        manager
            .define_pattern(npc_id, 1, r"hello", "greet")
            .unwrap();
        manager.activate_set(npc_id, 1);

        manager.clear_npc(&npc_id);
        let result = manager.process_message(npc_id, "hello");
        assert!(result.is_none());
    }
}
