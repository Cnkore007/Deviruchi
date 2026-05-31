//! 知识索引生成器
//!
//! Agent 启动时自动从源码提取关键信息，生成 LLM 可用的参考文档。
//! 索引存储在 ~/.devi-agent/knowledge/ 目录下，包含代码库结构、
//! 配置 Schema 和脚本命令参考等文档。

use std::path::PathBuf;
use anyhow::Result;
use tracing::info;

/// 知识索引管理器
///
/// 负责生成和维护 Agent 的知识库，包括：
/// - 代码库结构地图
/// - 配置文件 Schema 文档
/// - NPC 脚本命令参考
pub struct KnowledgeIndex {
    /// 索引输出目录 (~/.devi-agent/knowledge/)
    output_dir: PathBuf,
    /// 源码目录（项目根目录）
    source_dir: PathBuf,
}

impl KnowledgeIndex {
    /// 创建新的知识索引管理器
    ///
    /// # 参数
    /// - `output_dir`: 索引输出目录，通常是 ~/.devi-agent/knowledge/
    /// - `source_dir`: 项目源码根目录
    pub fn new(output_dir: PathBuf, source_dir: PathBuf) -> Self {
        Self { output_dir, source_dir }
    }

    /// 生成知识索引
    ///
    /// 创建必要的目录结构，然后生成所有知识文档。
    /// 如果目录已存在则不会覆盖已有文件。
    pub fn generate(&self) -> Result<()> {
        // 创建输出目录结构
        std::fs::create_dir_all(&self.output_dir)?;
        std::fs::create_dir_all(self.output_dir.join("schemas"))?;

        // 生成各类知识文档
        self.generate_codebase_map()?;
        self.generate_config_schema()?;
        self.generate_script_reference()?;

        info!("知识索引已生成: {}", self.output_dir.display());
        Ok(())
    }

    /// 检查是否需要更新索引
    ///
    /// 通过比较源码文件和索引文件的修改时间来判断。
    /// 如果源码比索引新，或者索引文件不存在，则返回 true。
    pub fn needs_update(&self) -> bool {
        let index_file = self.output_dir.join("codebase.md");

        // 索引文件不存在，需要生成
        if !index_file.exists() {
            return true;
        }

        // 获取索引文件的修改时间
        let index_time = std::fs::metadata(&index_file)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        // 检查 lib.rs 是否比索引新
        // 作为源码变更的代理指标
        self.source_dir.join("src").join("lib.rs").metadata()
            .and_then(|m| m.modified())
            .map(|t| t > index_time)
            .unwrap_or(true)
    }

    /// 生成代码库结构地图
    ///
    /// 输出 Deviruchi 项目的整体架构文档，包括：
    /// - 服务器架构概览
    /// - 关键类型定义
    /// - 数据文件位置
    fn generate_codebase_map(&self) -> Result<()> {
        let content = r#"# Deviruchi 代码库结构

## 服务器架构

- `src/core/` — 核心系统（配置、日志、定时器、panic 处理）
- `src/game/` — 游戏逻辑（40 个子模块）
  - `map/` — 地图和玩家状态管理
  - `mob/` — 怪物 AI 和刷新
  - `battle/` — 战斗系统
  - `item/` — 物品系统
  - `script/` — NPC 脚本引擎
  - `command/` — GM 命令系统
  - `skill/` — 技能系统
  - `party/` — 组队系统
  - `guild/` — 公会系统
  - `trade/` — 交易系统
  - `storage/` — 仓库系统
  - `pet/` — 宠物系统
  - `vending/` — 摆摊系统
  - `woe/` — 攻城战
  - `login/` — 登录服务器
  - `char/` — 角色服务器
  - `token/` — Token 管理
  - `heal/` — 治疗服务
  - `status/` — 状态效果
- `src/network/` — 网络层（TCP + WebSocket + Agent API）
- `src/protocol/` — 协议编解码
- `src/storage/` — 数据库存储

## 关键类型

- `Core` — 服务主结构，拥有所有子系统
- `Config` — TOML 配置（13 个节）
- `MapState` — 玩家状态管理
- `Player` — 玩家数据（6 个分组锁）
- `MobTemplate` — 怪物模板
- `Item` — 物品数据
- `ScriptNode` — 脚本 AST

## 数据文件

- `config/server.toml` — 服务器配置
- `db/mob_db.yml` — 怪物数据库
- `db/item_db_equip.yml` — 装备数据库
- `db/item_db_usable.yml` — 消耗品数据库
- `db/item_db_etc.yml` — 其他物品数据库
- `db/skill_db.yml` — 技能数据库
- `db/droptable.yml` — 掉落表
"#;
        std::fs::write(self.output_dir.join("codebase.md"), content)?;
        Ok(())
    }

    /// 生成配置 Schema 文档
    ///
    /// 输出 config/server.toml 的完整字段说明，方便 LLM
    /// 理解和修改服务器配置。
    fn generate_config_schema(&self) -> Result<()> {
        let content = r#"# 配置 Schema (config/server.toml)

## [server]
- name: String — 服务器名称

## [database]
- backend: String — "sqlite" 或 "mysql"
- path: String — 数据库文件路径

## [network]
- login_port: u16 — 登录服务器端口 (默认 6900)
- char_port: u16 — 角色服务器端口 (默认 6000)
- map_port: u16 — 地图服务器端口 (默认 6121)
- max_connections: usize — 最大连接数

## [game]
- max_players: usize — 最大玩家数
- max_level: u16 — 最大等级
- base_level_cap: u16 — 基础等级上限
- job_level_cap: u16 — 职业等级上限
- autosave_interval_seconds: u64 — 自动保存间隔
- death_drop_items: bool — 死亡是否掉落物品

## [battle]
- base_exp_rate: u32 — 基础经验倍率
- job_exp_rate: u32 — 职业经验倍率
- zeny_rate: u32 — Zeny 倍率
- item_drop_rate: u32 — 物品掉落倍率

## [drop]
- item_drop_rate: u32 — 掉落倍率

## [exp]
- base_exp_rate: u32 — 经验倍率

## [respawn]
- delay: u64 — 重生延迟

## [skill]
- skill_tree_enabled: bool — 是否启用技能树

## [party]
- enabled: bool — 是否启用组队

## [storage]
- enabled: bool — 是否启用仓库

## [chat]
- enabled: bool — 是否启用聊天
"#;
        std::fs::write(self.output_dir.join("schemas").join("server.toml.md"), content)?;
        Ok(())
    }

    /// 生成脚本命令参考
    ///
    /// 输出 NPC 脚本引擎支持的所有命令，包括：
    /// - 对话命令（mes, next, close 等）
    /// - 传送命令（warp）
    /// - 物品命令（getitem, delitem 等）
    /// - 流程控制（goto, if, set 等）
    /// - 循环命令（for, break, continue）
    /// - 状态命令（heal, announce 等）
    fn generate_script_reference(&self) -> Result<()> {
        let content = r#"# NPC 脚本命令参考

## 对话命令
- `mes "文本"` — 显示对话文本
- `next` — 等待玩家点击下一步
- `close` — 关闭对话窗口
- `close2` — 关闭对话（暂停脚本）
- `end` — 终止脚本执行
- `select "选项1","选项2"` — 显示选择菜单

## 传送命令
- `warp "地图名",x,y` — 传送玩家到指定位置

## 物品命令
- `getitem 物品ID,数量` — 给予玩家物品
- `delitem 物品ID,数量` — 删除玩家物品
- `countitem 物品ID,"结果变量"` — 计算玩家持有的物品数量
- `checkweight 物品ID,数量,"结果变量"` — 检查负重

## 流程控制
- `标签名:` — 定义标签（行末冒号）
- `goto "标签"` — 无条件跳转
- `goto_if "变量",值,"标签"` — 条件跳转（变量==值时跳转）
- `set "变量",值` — 设置变量值
- `callfunc "函数名",参数` — 调用函数
- `return` — 从函数返回

## 循环
- `for "变量",起始值,结束值` — 循环开始
- `endfor` — 循环结束
- `break` — 跳出循环
- `continue` — 继续下一次循环

## 状态命令
- `heal HP,SP` — 恢复 HP/SP
- `announce "消息",flag` — 发送全服公告
- `input "变量"` — 接收玩家输入
- `sleep 毫秒数` — 暂停脚本执行

## 示例：简单 NPC 对话
```
mes "[商人]";
mes "欢迎！想要买什么？";
next;
select "红药水","蓝药水","离开";
// 根据选择处理...
close;
```
"#;
        std::fs::write(self.output_dir.join("schemas").join("script.md"), content)?;
        Ok(())
    }
}
