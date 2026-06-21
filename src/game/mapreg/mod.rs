//! 地图全局变量存储
//!
//! 对应 rAthena 的 `src/map/mapreg.cpp`，提供 NPC 脚本全局变量的读写和持久化。
//!
//! 变量命名规则（与 rAthena 一致）：
//! - `$` 前缀：永久整数变量（持久化到数据库）
//! - `$` 前缀 + `$` 后缀：永久字符串变量
//! - `@` 前缀：临时变量（不持久化）

use parking_lot::RwLock;
use std::collections::HashMap;

/// 变量值类型
#[derive(Debug, Clone, PartialEq)]
pub enum VarValue {
    /// 整数值
    Int(i64),
    /// 字符串值
    Str(String),
}

/// 全局变量条目
#[derive(Debug, Clone)]
pub struct MapRegEntry {
    /// 变量名
    pub name: String,
    /// 数组索引（0 表示标量）
    pub index: u32,
    /// 变量值
    pub value: VarValue,
    /// 是否需要保存到数据库
    pub dirty: bool,
}

/// 全局变量存储
///
/// 管理 NPC 脚本使用的全局变量，支持整数和字符串类型。
pub struct MapRegStore {
    /// 整数变量 (name:index -> value)
    int_vars: RwLock<HashMap<String, i64>>,
    /// 字符串变量 (name:index -> value)
    str_vars: RwLock<HashMap<String, String>>,
    /// 脏标记（有未保存的修改）
    dirty: RwLock<bool>,
    /// 自动保存间隔（秒）
    autosave_interval: u64,
}

impl MapRegStore {
    /// 创建空的变量存储
    pub fn new() -> Self {
        Self {
            int_vars: RwLock::new(HashMap::new()),
            str_vars: RwLock::new(HashMap::new()),
            dirty: RwLock::new(false),
            autosave_interval: 300, // 5 分钟
        }
    }

    /// 设置自动保存间隔
    pub fn with_autosave_interval(mut self, seconds: u64) -> Self {
        self.autosave_interval = seconds;
        self
    }

    /// 生成变量键
    fn make_key(name: &str, index: u32) -> String {
        if index == 0 {
            name.to_string()
        } else {
            format!("{}:{}", name, index)
        }
    }

    /// 判断变量名是否为永久变量（需要持久化）
    ///
    /// rAthena 规则：
    /// - `$` 前缀的变量是永久的（如 `$VAR`、`$VAR$`）
    /// - `@` 前缀是临时的（如 `@VAR`、`@@VAR`）
    pub fn is_permanent(name: &str) -> bool {
        name.starts_with('$')
    }

    /// 读取整数变量
    pub fn read_int(&self, name: &str, index: u32) -> i64 {
        let key = Self::make_key(name, index);
        self.int_vars.read().get(&key).copied().unwrap_or(0)
    }

    /// 读取字符串变量
    pub fn read_str(&self, name: &str, index: u32) -> Option<String> {
        let key = Self::make_key(name, index);
        self.str_vars.read().get(&key).cloned()
    }

    /// 设置整数变量
    ///
    /// 如果值为 0 且变量存在，则删除变量（节省内存）。
    /// 对于永久变量，标记为需要保存。
    pub fn set_int(&self, name: &str, index: u32, value: i64) {
        let key = Self::make_key(name, index);
        let is_permanent = Self::is_permanent(name);

        if value == 0 {
            // 值为 0 时删除变量
            self.int_vars.write().remove(&key);
        } else {
            self.int_vars.write().insert(key, value);
        }

        if is_permanent {
            *self.dirty.write() = true;
        }
    }

    /// 设置字符串变量
    ///
    /// 如果值为空或 None，则删除变量。
    pub fn set_str(&self, name: &str, index: u32, value: Option<&str>) {
        let key = Self::make_key(name, index);
        let is_permanent = Self::is_permanent(name);

        match value {
            Some(s) if !s.is_empty() => {
                self.str_vars.write().insert(key, s.to_string());
            }
            _ => {
                self.str_vars.write().remove(&key);
            }
        }

        if is_permanent {
            *self.dirty.write() = true;
        }
    }

    /// 检查是否有未保存的修改
    pub fn is_dirty(&self) -> bool {
        *self.dirty.read()
    }

    /// 清除脏标记
    pub fn clear_dirty(&self) {
        *self.dirty.write() = false;
    }

    /// 获取所有需要保存的整数变量
    pub fn get_persistent_int_vars(&self) -> HashMap<String, i64> {
        self.int_vars
            .read()
            .iter()
            .filter(|(k, _)| Self::is_permanent(k))
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// 获取所有需要保存的字符串变量
    pub fn get_persistent_str_vars(&self) -> HashMap<String, String> {
        self.str_vars
            .read()
            .iter()
            .filter(|(k, _)| Self::is_permanent(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// 从数据库加载永久变量
    pub fn load_from_db(&self, int_vars: HashMap<String, i64>, str_vars: HashMap<String, String>) {
        let mut ints = self.int_vars.write();
        let mut strs = self.str_vars.write();

        for (k, v) in int_vars {
            if v != 0 {
                ints.insert(k, v);
            }
        }

        for (k, v) in str_vars {
            if !v.is_empty() {
                strs.insert(k, v);
            }
        }

        *self.dirty.write() = false;
    }

    /// 清空所有变量
    pub fn clear(&self) {
        self.int_vars.write().clear();
        self.str_vars.write().clear();
        *self.dirty.write() = false;
    }

    /// 重载：清空临时变量，重新加载永久变量
    pub fn reload(&self, int_vars: HashMap<String, i64>, str_vars: HashMap<String, String>) {
        self.clear();
        self.load_from_db(int_vars, str_vars);
    }

    /// 获取整数变量数量
    pub fn int_count(&self) -> usize {
        self.int_vars.read().len()
    }

    /// 获取字符串变量数量
    pub fn str_count(&self) -> usize {
        self.str_vars.read().len()
    }

    /// 获取自动保存间隔
    pub fn autosave_interval(&self) -> u64 {
        self.autosave_interval
    }
}

impl Default for MapRegStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write_int() {
        let store = MapRegStore::new();

        // 读取不存在的变量返回 0
        assert_eq!(store.read_int("$VAR", 0), 0);

        // 设置后可读取
        store.set_int("$VAR", 0, 42);
        assert_eq!(store.read_int("$VAR", 0), 42);

        // 设置为 0 会删除
        store.set_int("$VAR", 0, 0);
        assert_eq!(store.read_int("$VAR", 0), 0);
    }

    #[test]
    fn test_read_write_str() {
        let store = MapRegStore::new();

        // 读取不存在的变量返回 None
        assert_eq!(store.read_str("$VAR$", 0), None);

        // 设置后可读取
        store.set_str("$VAR$", 0, Some("hello"));
        assert_eq!(store.read_str("$VAR$", 0), Some("hello".to_string()));

        // 设置为空会删除
        store.set_str("$VAR$", 0, Some(""));
        assert_eq!(store.read_str("$VAR$", 0), None);

        // 设置为 None 也会删除
        store.set_str("$VAR$", 0, Some("test"));
        store.set_str("$VAR$", 0, None);
        assert_eq!(store.read_str("$VAR$", 0), None);
    }

    #[test]
    fn test_array_index() {
        let store = MapRegStore::new();

        store.set_int("$ARRAY", 0, 100);
        store.set_int("$ARRAY", 1, 200);
        store.set_int("$ARRAY", 2, 300);

        assert_eq!(store.read_int("$ARRAY", 0), 100);
        assert_eq!(store.read_int("$ARRAY", 1), 200);
        assert_eq!(store.read_int("$ARRAY", 2), 300);
    }

    #[test]
    fn test_temporary_vs_permanent() {
        let store = MapRegStore::new();

        // 临时变量不标记脏
        store.set_int("@TEMP", 0, 42);
        assert!(!store.is_dirty());

        // 永久变量标记脏
        store.set_int("$PERM", 0, 42);
        assert!(store.is_dirty());

        store.clear_dirty();
        assert!(!store.is_dirty());
    }

    #[test]
    fn test_persistent_vars() {
        let store = MapRegStore::new();

        store.set_int("$PERM1", 0, 10);
        store.set_int("$PERM2", 1, 20);
        store.set_int("@TEMP", 0, 30);

        let persistent = store.get_persistent_int_vars();
        assert_eq!(persistent.len(), 2);
        assert_eq!(persistent.get("$PERM1"), Some(&10));
        assert_eq!(persistent.get("$PERM2:1"), Some(&20));
    }

    #[test]
    fn test_load_from_db() {
        let store = MapRegStore::new();

        let mut ints = HashMap::new();
        ints.insert("$VAR1".to_string(), 100);
        ints.insert("$VAR2".to_string(), 200);

        let mut strs = HashMap::new();
        strs.insert("$STR1$".to_string(), "hello".to_string());

        store.load_from_db(ints, strs);

        assert_eq!(store.read_int("$VAR1", 0), 100);
        assert_eq!(store.read_int("$VAR2", 0), 200);
        assert_eq!(store.read_str("$STR1$", 0), Some("hello".to_string()));
        assert!(!store.is_dirty());
    }

    #[test]
    fn test_load_skips_zero() {
        let store = MapRegStore::new();

        let mut ints = HashMap::new();
        ints.insert("$ZERO".to_string(), 0);
        ints.insert("$NONZERO".to_string(), 42);

        store.load_from_db(ints, HashMap::new());

        assert_eq!(store.read_int("$ZERO", 0), 0);
        assert_eq!(store.read_int("$NONZERO", 0), 42);
    }

    #[test]
    fn test_reload() {
        let store = MapRegStore::new();

        store.set_int("$OLD", 0, 100);
        assert_eq!(store.read_int("$OLD", 0), 100);

        let mut new_ints = HashMap::new();
        new_ints.insert("$NEW".to_string(), 200);

        store.reload(new_ints, HashMap::new());

        assert_eq!(store.read_int("$OLD", 0), 0);
        assert_eq!(store.read_int("$NEW", 0), 200);
    }

    #[test]
    fn test_clear() {
        let store = MapRegStore::new();

        store.set_int("$VAR", 0, 42);
        store.set_str("$STR$", 0, Some("test"));

        store.clear();

        assert_eq!(store.read_int("$VAR", 0), 0);
        assert_eq!(store.read_str("$STR$", 0), None);
        assert_eq!(store.int_count(), 0);
        assert_eq!(store.str_count(), 0);
    }

    #[test]
    fn test_make_key() {
        assert_eq!(MapRegStore::make_key("$VAR", 0), "$VAR");
        assert_eq!(MapRegStore::make_key("$VAR", 1), "$VAR:1");
        assert_eq!(MapRegStore::make_key("$VAR", 42), "$VAR:42");
    }

    #[test]
    fn test_is_permanent() {
        assert!(MapRegStore::is_permanent("$VAR"));
        assert!(MapRegStore::is_permanent("$VAR$"));
        assert!(!MapRegStore::is_permanent("@TEMP"));
        assert!(!MapRegStore::is_permanent("@@TEMP"));
    }
}
