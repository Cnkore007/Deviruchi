# Deviruchi 三功能完善设计

> **目标：** 完成 Homunculus/Mercenary 持久化、Mob/NPC YAML 完整解析、MySQL 支持三个待完善功能
>
> **架构：** 引入 DatabaseBackend trait 作为数据库抽象层，SQLite 和 MySQL 双后端实现。Homunculus/Mercenary 实时持久化。Mob/NPC YAML 扩展 rAthena 数据映射。
>
> **技术栈：** Rust, rusqlite, mysql (feature gate), serde_yaml, parking_lot

---

## 1. DatabaseBackend 抽象层

### 1.1 Trait 定义

```rust
/// 数据库后端抽象 trait
pub trait DatabaseBackend: Send + Sync + 'static {
    /// 执行 SQL（无返回值）
    fn execute(&self, sql: &str) -> Result<()>;

    /// 带参数执行 SQL，返回影响行数
    fn execute_params(&self, sql: &str, params: &[&dyn Value]) -> Result<usize>;

    /// 查询多行
    fn query_rows(&self, sql: &str, params: &[&dyn Value]) -> Result<Vec<Row>>;

    /// 查询单行
    fn query_row<F, T>(&self, sql: &str, params: &[&dyn Value], f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> Result<T>;

    /// 查询可选单行
    fn query_row_optional<F, T>(&self, sql: &str, params: &[&dyn Value], f: F) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> Result<T>;

    /// 事务
    fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>;

    /// 最后插入的行 ID
    fn last_insert_rowid(&self) -> i64;

    /// 批量执行（用于迁移）
    fn execute_batch(&self, sql: &str) -> Result<()>;

    /// UPSERT 便捷方法（方言差异自动处理）
    fn upsert(&self, table: &str, columns: &[&str], values: &[&dyn Value], conflict_cols: &[&str]) -> Result<usize>;
}
```

### 1.2 Value 类型

```rust
/// 数据库值类型，替代 rusqlite::ToSql / mysql::Value
pub enum Value {
    Null,
    Int(i32),
    BigInt(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}
```

### 1.3 Row 类型

```rust
/// 数据库行，提供统一的列访问接口
pub struct Row {
    columns: Vec<(String, Value)>,
}

impl Row {
    pub fn get_i32(&self, index: usize) -> Result<i32> { ... }
    pub fn get_i64(&self, index: usize) -> Result<i64> { ... }
    pub fn get_f64(&self, index: usize) -> Result<f64> { ... }
    pub fn get_string(&self, index: usize) -> Result<String> { ... }
    pub fn get_blob(&self, index: usize) -> Result<Vec<u8>> { ... }
    pub fn get_optional_i32(&self, index: usize) -> Result<Option<i32>> { ... }
    // ...
}
```

### 1.4 TransactionOps trait

```rust
/// 事务内操作
pub trait TransactionOps {
    fn execute(&self, sql: &str) -> Result<()>;
    fn execute_params(&self, sql: &str, params: &[&dyn Value]) -> Result<usize>;
    fn execute_batch(&self, sql: &str) -> Result<()>;
    fn last_insert_rowid(&self) -> i64;
}
```

### 1.5 Database 结构改造

```rust
/// 数据库操作入口
pub struct Database {
    backend: Arc<dyn DatabaseBackend>,
}

impl Database {
    pub fn new(config: &DatabaseConfig) -> Result<Self> {
        let backend = create_database_backend(config)?;
        Ok(Self { backend })
    }

    pub fn execute(&self, sql: &str) -> Result<()> {
        self.backend.execute(sql)
    }

    pub fn execute_params(&self, sql: &str, params: &[&dyn Value]) -> Result<usize> {
        self.backend.execute_params(sql, params)
    }

    pub fn query_rows(&self, sql: &str, params: &[&dyn Value]) -> Result<Vec<Row>> {
        self.backend.query_rows(sql, params)
    }

    pub fn query_row<F, T>(&self, sql: &str, params: &[&dyn Value], f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> Result<T>,
    {
        self.backend.query_row(sql, params, f)
    }

    pub fn query_row_optional<F, T>(&self, sql: &str, params: &[&dyn Value], f: F) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> Result<T>,
    {
        self.backend.query_row_optional(sql, params, f)
    }

    pub fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn TransactionOps) -> Result<T>,
    {
        self.backend.with_transaction(f)
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.backend.last_insert_rowid()
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        self.backend.execute_batch(sql)
    }
}
```

### 1.6 配置扩展

```rust
pub struct DatabaseConfig {
    pub backend: DatabaseBackendType,
    pub sqlite: SqliteConfig,
    pub mysql: MySqlConfig,
}

pub enum DatabaseBackendType {
    Sqlite,
    MySql,
}

pub struct SqliteConfig {
    pub path: String,
    pub wal_mode: bool,
    pub busy_timeout_ms: u32,
}

pub struct MySqlConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub pool_size: u32,
}
```

### 1.7 工厂函数

```rust
pub fn create_database_backend(config: &DatabaseConfig) -> Result<Arc<dyn DatabaseBackend>> {
    match config.backend {
        DatabaseBackendType::Sqlite => {
            let backend = SqliteBackend::new(&config.sqlite)?;
            Ok(Arc::new(backend))
        }
        DatabaseBackendType::MySql => {
            let backend = MySqlBackend::new(&config.mysql)?;
            Ok(Arc::new(backend))
        }
    }
}
```

---

## 2. Homunculus/Mercenary 持久化

### 2.1 Migration v4: 扩展 homunculus 表

```sql
-- 扩展列
ALTER TABLE homunculus ADD COLUMN race TEXT DEFAULT 'None';
ALTER TABLE homunculus ADD COLUMN element TEXT DEFAULT 'Neutral';
ALTER TABLE homunculus ADD COLUMN element_level INTEGER DEFAULT 1;
ALTER TABLE homunculus ADD COLUMN evolution_stage TEXT DEFAULT 'Base';
ALTER TABLE homunculus ADD COLUMN skill_points INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN atk INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN matk INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN defense INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN magic_defense INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN hit INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN flee INTEGER DEFAULT 0;
ALTER TABLE homunculus ADD COLUMN walk_speed INTEGER DEFAULT 200;
ALTER TABLE homunculus ADD COLUMN attack_delay INTEGER DEFAULT 1000;

-- 技能表
CREATE TABLE IF NOT EXISTS homunculus_skills (
    homun_id INTEGER NOT NULL,
    skill_id INTEGER NOT NULL,
    skill_level INTEGER DEFAULT 1,
    PRIMARY KEY (homun_id, skill_id)
);
```

### 2.2 Migration v5: 扩展 mercenaries 表

```sql
-- 扩展列
ALTER TABLE mercenaries ADD COLUMN defense INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN magic_defense INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN str INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN agi INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN vit INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN int INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN dex INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN luk INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN hit INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN flee INTEGER DEFAULT 0;
ALTER TABLE mercenaries ADD COLUMN walk_speed INTEGER DEFAULT 200;
ALTER TABLE mercenaries ADD COLUMN attack_range INTEGER DEFAULT 1;
ALTER TABLE mercenaries ADD COLUMN contract_cost INTEGER DEFAULT 0;

-- 技能表
CREATE TABLE IF NOT EXISTS mercenary_skills (
    mercenary_id INTEGER NOT NULL,
    skill_id INTEGER NOT NULL,
    skill_level INTEGER DEFAULT 1,
    PRIMARY KEY (mercenary_id, skill_id)
);
```

### 2.3 HomunculusManager 改造

```rust
pub struct HomunculusManager {
    db: Arc<dyn DatabaseBackend>,
    instances: RwLock<HashMap<u32, Homunculus>>,
    summoned: RwLock<HashMap<u32, u32>>,
    next_id: AtomicU32,
}

impl HomunculusManager {
    /// 从数据库加载角色的所有生命体
    pub fn load_for_character(&self, char_id: u32) -> Result<Vec<Homunculus>> {
        // SELECT * FROM homunculus WHERE owner_id = ?
        // SELECT * FROM homunculus_skills WHERE homun_id IN (...)
    }

    /// 创建生命体（INSERT + 内存）
    pub fn create(&self, char_id: u32, homun_type: HomunculusType) -> Result<u32> {
        // 1. INSERT INTO homunculus
        // 2. INSERT INTO homunculus_skills (初始技能)
        // 3. 写入内存
    }

    /// 喂食（UPDATE hunger, intimacy）
    pub fn feed(&self, homun_id: u32) -> Result<()> {
        // 1. 修改内存
        // 2. UPDATE homunculus SET hunger=?, intimacy=? WHERE homun_id=?
    }

    /// 增加经验（可能触发升级）
    pub fn add_exp(&self, homun_id: u32, exp: u64) -> Result<bool> {
        // 1. 修改内存（含升级逻辑）
        // 2. UPDATE homunculus SET level=?, exp=?, hp=?, sp=?, str=?, ... WHERE homun_id=?
    }

    /// 进化
    pub fn evolve(&self, homun_id: u32) -> Result<()> {
        // 1. 修改内存
        // 2. UPDATE homunculus SET evolution_stage=?, evolved=1, ... WHERE homun_id=?
    }

    /// 学习技能
    pub fn learn_skill(&self, homun_id: u32, skill_id: u16) -> Result<()> {
        // 1. INSERT INTO homunculus_skills
        // 2. 更新内存
    }

    /// 召唤（记录到 summoned 表）
    pub fn summon(&self, char_id: u32, homun_id: u32) -> Result<()> {
        // 1. 内存记录
        // 2. 可选：持久化召唤状态
    }

    /// 解散
    pub fn dismiss(&self, char_id: u32) -> Result<()> {
        // 1. 清除内存
        // 2. 可选：清除持久化召唤状态
    }
}
```

### 2.4 MercenaryManager 改造

同 HomunculusManager 模式，额外增加：

```rust
impl MercenaryManager {
    /// 合约到期自动解散（DELETE）
    pub fn check_contracts(&self) -> Result<()> {
        // 1. 检查所有已召唤佣兵的 contract_end
        // 2. 到期的执行 DELETE + dismiss
    }
}
```

---

## 3. MySQL 后端实现

### 3.1 依赖配置

```toml
[dependencies]
mysql = { version = "25", optional = true }
rusqlite = { version = "0.31", optional = true }

[features]
default = ["sqlite"]
sqlite = ["rusqlite"]
mysql-backend = ["dep:mysql"]
```

### 3.2 SqliteBackend 实现

```rust
pub struct SqliteBackend {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteBackend {
    pub fn new(config: &SqliteConfig) -> Result<Self> {
        let conn = rusqlite::Connection::open(&config.path)?;
        if config.wal_mode {
            conn.execute_batch("PRAGMA journal_mode=WAL")?;
        }
        conn.busy_timeout(Duration::from_millis(config.busy_timeout_ms as u64))?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}

impl DatabaseBackend for SqliteBackend {
    fn execute_params(&self, sql: &str, params: &[&dyn Value]) -> Result<usize> {
        let conn = self.conn.lock();
        let rusqlite_params: Vec<Box<dyn rusqlite::ToSql>> = params.iter()
            .map(|v| v.to_rusqlite())
            .collect();
        let mut stmt = conn.prepare(sql)?;
        Ok(stmt.execute(rusqlite::params_from_iter(rusqlite_params.iter()))?)
    }

    fn upsert(&self, table: &str, columns: &[&str], values: &[&dyn Value], conflict_cols: &[&str]) -> Result<usize> {
        // INSERT OR REPLACE INTO table (cols) VALUES (vals)
        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        );
        self.execute_params(&sql, values)
    }
    // ...
}
```

### 3.3 MySqlBackend 实现

```rust
pub struct MySqlBackend {
    pool: mysql::Pool,
}

impl MySqlBackend {
    pub fn new(config: &MySqlConfig) -> Result<Self> {
        let opts = mysql::OptsBuilder::new()
            .ip_or_hostname(Some(&config.host))
            .tcp_port(config.port)
            .db_name(Some(&config.database))
            .user(Some(&config.user))
            .pass(Some(&config.password));
        let pool = mysql::Pool::new(opts)?;
        Ok(Self { pool })
    }
}

impl DatabaseBackend for MySqlBackend {
    fn execute_params(&self, sql: &str, params: &[&dyn Value]) -> Result<usize> {
        let mut conn = self.pool.get_conn()?;
        let mysql_params: Vec<mysql::Value> = params.iter()
            .map(|v| v.to_mysql())
            .collect();
        let result = conn.exec_iter(sql, mysql_params)?;
        Ok(result.affected_rows() as usize)
    }

    fn execute_batch(&self, sql: &str) -> Result<()> {
        let mut conn = self.pool.get_conn()?;
        for stmt in sql.split(';').filter(|s| !s.trim().is_empty()) {
            conn.query_drop(stmt)?;
        }
        Ok(())
    }

    fn upsert(&self, table: &str, columns: &[&str], values: &[&dyn Value], conflict_cols: &[&str]) -> Result<usize> {
        // INSERT INTO table (cols) VALUES (vals) ON DUPLICATE KEY UPDATE col1=VALUES(col1), ...
        let update_cols: Vec<String> = columns.iter()
            .filter(|c| !conflict_cols.contains(c))
            .map(|c| format!("{}=VALUES({})", c, c))
            .collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) ON DUPLICATE KEY UPDATE {}",
            table,
            columns.join(", "),
            columns.iter().map(|_| "?").collect::<Vec<_>>().join(", "),
            update_cols.join(", ")
        );
        self.execute_params(&sql, values)
    }
    // ...
}
```

### 3.4 SQL 方言差异

| SQLite | MySQL | 处理方式 |
|--------|-------|---------|
| `INTEGER PRIMARY KEY` | `INT AUTO_INCREMENT PRIMARY KEY` | 迁移 SQL 用通用语法 |
| `INSERT OR REPLACE` | `INSERT ... ON DUPLICATE KEY UPDATE` | `upsert()` 方法自动适配 |
| `PRAGMA` | 不支持 | SQLite 后端特有，MySQL 后端忽略 |
| `?N` 占位符 | `?` 占位符 | 统一用 `?` |
| `INTEGER` 类型 | `INT` / `BIGINT` | 类型映射 |

### 3.5 迁移框架适配

MigrationManager 改用 Database 方法：

```rust
impl MigrationManager {
    pub fn migrate_up(&self, db: &Database) -> Result<u32> {
        db.with_transaction(|tx| {
            tx.execute_batch(up_sql)?;
            tx.execute_params(
                "INSERT INTO schema_version (version, description, applied_at) VALUES (?, ?, ?)",
                &[&version, &desc, &now],
            )?;
            Ok(())
        })
    }
}
```

### 3.6 配置文件

```toml
[database]
backend = "sqlite"  # 或 "mysql"

[database.sqlite]
path = "data/deviruchi.db"
wal_mode = true

[database.mysql]
host = "127.0.0.1"
port = 3306
user = "deviruchi"
password = ""
database = "deviruchi"
```

---

## 4. Mob/NPC YAML 完整解析

### 4.1 MobTemplate 扩展

```rust
pub struct MobTemplate {
    // 现有字段保持不变
    // 新增：
    pub race: MobRace,
    pub mob_type: MobType,
    pub mvp_drops: Vec<MobDrop>,
    pub skills: Vec<MobSkill>,
}

pub enum MobRace {
    Formless, Undead, Brute, Plant, Insect,
    Fish, Demon, DemiHuman, Angel, Dragon,
}
```

### 4.2 MobYamlSkill 反序列化

```rust
#[derive(Deserialize, Debug)]
struct MobYamlSkill {
    Id: u16,
    Level: u8,
    Rate: u32,
    CastTime: u32,
    Delay: u32,
}
```

### 4.3 Modes 完整解析

```rust
fn parse_modes(modes: &Option<HashMap<String, bool>>) -> MobBehaviorFlags {
    let mut flags = MobBehaviorFlags::empty();
    if let Some(m) = modes {
        if m.get("CanMove").copied() == Some(false) { flags |= Immobile; }
        if m.get("CanAttack").copied() == Some(false) { flags |= NoAttack; }
        if m.get("Detector").copied() == Some(true) { flags |= Detector; }
        if m.get("Boss").copied() == Some(true) { flags |= Boss; }
        if m.get("Plant").copied() == Some(true) { flags |= Plant; }
        if m.get("CanChase").copied() == Some(false) { flags |= NoChase; }
    }
    flags
}
```

### 4.4 硬编码值改为 YAML 驱动

| 字段 | 当前硬编码 | 改为 |
|------|-----------|------|
| sight_range | 12 | `SkillRange` 或默认 12 |
| respawn_time | 60000 | YAML 的 `SpawnDelay` 字段 |
| aggro_rate | 0 | 从 AI 类型推导 |
| hit/flee/crit | Dex/Agi/Luk | 保留推导（rAthena 同样如此） |

### 4.5 item_db.yml 加载器

```rust
/// 从 item_db.yml 加载物品名称到 ID 的映射
pub fn load_item_db(path: &str) -> Result<HashMap<String, u32>> {
    let content = fs::read_to_string(path)?;
    let yaml: ItemYamlFile = serde_yaml::from_str(&content)?;
    let mut map = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            map.insert(entry.AegisName, entry.Id);
        }
    }
    Ok(map)
}

/// 混合查找：先查动态映射，再查硬编码回退
pub fn item_name_to_id_dynamic(name: &str, item_map: &HashMap<String, u32>) -> u32 {
    item_map.get(name).copied().unwrap_or_else(|| item_name_to_id(name))
}
```

### 4.6 NPC YAML 扩展

新增 Spawn 和 Event 类型支持：

```yaml
# 带事件触发的 NPC
- Id: 100
  Name: "Warp Portal"
  Type: Warp
  Map: prontera
  X: 150
  Y: 180
  DestMap: geffen
  DestX: 120
  DestY: 100
  Event: OnTouch
  TriggerRadius: 1
```

---

## 5. 执行顺序

```
Phase 1: DatabaseBackend 抽象层（基础）
  ├── 定义 trait + Value + Row + TransactionOps
  ├── 实现 SqliteBackend
  ├── 改造 Database 结构
  ├── 改造 MigrationManager
  └── 迁移所有调用点

Phase 2: Homunculus/Mercenary 持久化
  ├── 扩展 migration v4/v5
  ├── Manager 接收 DatabaseBackend
  ├── 实现持久化方法
  └── 角色登录加载

Phase 3: MySQL 后端实现
  ├── 添加 mysql crate（feature gate）
  ├── 实现 MySqlBackend
  ├── 配置扩展
  └── 方言差异处理

Phase 4: Mob/NPC YAML 完整解析
  ├── MobTemplate 扩展
  ├── parse_modes 完整解析
  ├── item_db.yml 加载器
  └── NPC YAML 扩展
```

---

## 6. 测试策略

### Phase 1 测试
- SqliteBackend 单元测试（execute/query/transaction）
- Value/Row 类型转换测试
- Database 集成测试

### Phase 2 测试
- HomunculusManager CRUD 持久化测试
- MercenaryManager 合约到期测试
- 并发访问测试

### Phase 3 测试
- MySqlBackend 连接/查询测试（需要 MySQL 实例，可选）
- upsert 方言差异测试
- 配置切换测试

### Phase 4 测试
- MobYamlSkill 反序列化测试
- parse_modes 完整测试
- item_db 加载器测试
- Race/Class 解析测试
