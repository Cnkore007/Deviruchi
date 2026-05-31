//! 首次启动配置向导
//!
//! 引导用户完成服务器配置，生成 deviruchi.toml 配置文件。
//! 仅在配置文件不存在时触发。

use anyhow::Result;
use std::io::{self, Write};

use super::config::Config;

/// 配置向导
pub struct SetupWizard;

impl SetupWizard {
    /// 运行配置向导
    pub fn run() -> Result<Config> {
        Self::print_banner();

        let config = Config::default();

        // 步骤 1: 基础设置
        let config = Self::step_basic_settings(config)?;

        // 步骤 2: 游戏模式
        let config = Self::step_game_mode(config)?;

        // 步骤 3: 角色设置
        let config = Self::step_character_settings(config)?;

        // 步骤 4: 战斗与PVP
        let config = Self::step_battle_settings(config)?;

        // 步骤 5: 网络与性能
        let config = Self::step_network_settings(config)?;

        // 步骤 6: 日志与高级
        let config = Self::step_advanced_settings(config)?;

        // 显示配置预览
        Self::show_preview(&config);

        // 确认保存
        loop {
            print!("\n? 确认保存配置? [Y/n/修改]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "" | "y" | "yes" | "是" => return Ok(config),
                "n" | "no" | "否" => {
                    println!("\n已取消配置，使用默认配置启动。\n");
                    return Ok(Config::default());
                }
                "修改" | "edit" | "m" => {
                    println!("\n重新开始配置...\n");
                    return Self::run();
                }
                _ => println!("请输入 Y(是)/N(否)/修改"),
            }
        }
    }

    /// 打印横幅
    fn print_banner() {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                                                            ║");
        println!("║    ██████╗ ███████╗██╗   ██╗██╗██████╗ ██╗   ██╗ ██████╗██╗║");
        println!("║    ██╔══██╗██╔════╝██║   ██║██║██╔══██╗██║   ██║██╔════╝██║║");
        println!("║    ██║  ██║█████╗  ██║   ██║██║██████╔╝██║   ██║██║     ██║║");
        println!("║    ██║  ██║██╔══╝  ╚██╗ ██╔╝██║██╔══██╗██║   ██║██║     ██║║");
        println!("║    ██████╔╝███████╗ ╚████╔╝ ██║██║  ██║╚██████╔╝╚██████╗██║║");
        println!("║    ╚═════╝ ╚══════╝  ╚═══╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚═╝║");
        println!("║                                                            ║");
        println!("║              v0.0.2 — 首次配置向导                          ║");
        println!("║                                                            ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("欢迎使用 Deviruchi！这将引导您完成服务器的初始配置。");
        println!("每个选项都有默认值，直接按 Enter 使用默认值。");
        println!();
    }

    /// 步骤 1: 基础设置
    fn step_basic_settings(mut config: Config) -> Result<Config> {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  步骤 1/6: 基础设置");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // 服务器名称
        config.server.name = Self::input("服务器名称", &config.server.name, None)?;

        // 运行模式
        let mode_options = vec!["全部服务 (推荐)", "仅登录服务器", "仅角色服务器", "仅地图服务器"];
        let mode_idx = Self::select("运行模式", &mode_options, 0)?;
        config.server.mode = match mode_idx {
            0 => super::config::ServerMode::All,
            1 => super::config::ServerMode::Login,
            2 => super::config::ServerMode::Char,
            3 => super::config::ServerMode::Map,
            _ => super::config::ServerMode::All,
        };

        println!();
        Ok(config)
    }

    /// 步骤 2: 游戏模式
    fn step_game_mode(mut config: Config) -> Result<Config> {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  步骤 2/6: 游戏模式");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let preset_options = vec![
            "原版 (1x/1x/1x) — 硬核体验，还原官服",
            "低倍率 (3x/3x/2x) — 传统私服风格",
            "中等倍率 (5x/5x/3x) — 平衡体验 (推荐)",
            "高倍率 (20x/20x/10x) — 快速升级",
            "超高倍率 (100x/100x/50x) — 休闲娱乐",
            "自定义 — 手动设置各项倍率",
        ];
        let preset_idx = Self::select("游戏倍率预设", &preset_options, 2)?;

        let (exp, job_exp, drop, zeny) = match preset_idx {
            0 => (1.0, 1.0, 1.0, 1.0),
            1 => (3.0, 3.0, 2.0, 2.0),
            2 => (5.0, 5.0, 3.0, 2.0),
            3 => (20.0, 20.0, 10.0, 5.0),
            4 => (100.0, 100.0, 50.0, 20.0),
            _ => {
                println!("\n  自定义倍率设置:");
                let e = Self::input_f64("基础经验倍率", 5.0)?;
                let j = Self::input_f64("职业经验倍率", 5.0)?;
                let d = Self::input_f64("物品掉落倍率", 3.0)?;
                let z = Self::input_f64("Zeny掉落倍率", 2.0)?;
                (e, j, d, z)
            }
        };

        config.battle.base_exp_rate = exp;
        config.battle.job_exp_rate = job_exp;
        config.battle.item_drop_rate = drop;
        config.battle.zeny_rate = zeny;

        println!("\n  ✓ 倍率设置: 经验 {}x/{}x, 掉落 {}x, Zeny {}x", exp, job_exp, drop, zeny);
        println!();
        Ok(config)
    }

    /// 步骤 3: 角色设置
    fn step_character_settings(mut config: Config) -> Result<Config> {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  步骤 3/6: 角色设置");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // 最大等级
        let level_options = vec!["50级", "99级 (推荐)", "150级", "200级 (扩展开服)"];
        let level_idx = Self::select("最大基础等级", &level_options, 1)?;
        config.game.max_level = match level_idx {
            0 => 50,
            1 => 99,
            2 => 150,
            3 => 200,
            _ => 99,
        };
        config.game.base_level_cap = config.game.max_level;

        // 职业等级
        let job_options = vec!["25级", "50级 (推荐)", "70级", "99级"];
        let job_idx = Self::select("最大职业等级", &job_options, 1)?;
        config.game.job_level_cap = match job_idx {
            0 => 25,
            1 => 50,
            2 => 70,
            3 => 99,
            _ => 50,
        };

        // 死亡惩罚
        let death_options = vec!["无惩罚 (推荐)", "掉落物品", "损失 1% 经验", "掉落物品 + 损失经验"];
        let death_idx = Self::select("死亡惩罚", &death_options, 0)?;
        config.game.death_drop_items = matches!(death_idx, 1 | 3);

        // 角色名长度
        println!("\n  角色名长度限制 (直接回车使用默认值):");
        config.game.player_name_length_min = Self::input_u8("最小长度", 4)? as u8;
        config.game.player_name_length_max = Self::input_u8("最大长度", 24)? as u8;

        // 公会名长度
        println!("\n  公会名长度限制:");
        config.game.guild_name_length_min = Self::input_u8("最小长度", 4)? as u8;
        config.game.guild_name_length_max = Self::input_u8("最大长度", 24)? as u8;

        println!();
        Ok(config)
    }

    /// 步骤 4: 战斗与PVP
    fn step_battle_settings(mut config: Config) -> Result<Config> {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  步骤 4/6: 战斗与PVP设置");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // PVP模式
        let pvp_options = vec!["关闭 (推荐)", "开启 PVP"];
        let pvp_idx = Self::select("PVP模式", &pvp_options, 0)?;
        config.battle.pvp_mode = pvp_idx == 1;

        if config.battle.pvp_mode {
            config.battle.pvp_damage_rate = Self::input_f64("PVP伤害倍率 (1.0=100%)", 1.0)?;
        }

        // GVG模式
        let gvg_options = vec!["关闭", "开启 GVG (攻城战)"];
        let gvg_idx = Self::select("GVG模式", &gvg_options, 0)?;
        config.battle.gvg_mode = gvg_idx == 1;

        if config.battle.gvg_mode {
            config.battle.gvg_damage_rate = Self::input_f64("GVG伤害倍率 (1.0=100%)", 1.0)?;
        }

        // 自然回复
        println!("\n  自然回复设置:");
        let heal_options = vec!["100% (原版)", "150%", "200% (推荐)", "300%"];
        let heal_idx = Self::select("HP/SP回复倍率", &heal_options, 2)?;
        let heal_rate = match heal_idx {
            0 => 100,
            1 => 150,
            2 => 200,
            3 => 300,
            _ => 200,
        };
        config.battle.natural_heal_hp_rate = heal_rate;
        config.battle.natural_heal_sp_rate = heal_rate;
        config.battle.sit_heal_hp_rate = heal_rate * 2;
        config.battle.sit_heal_sp_rate = heal_rate * 2;

        // 战斗惩罚
        let penalty_options = vec!["开启 (原版体验)", "关闭 (推荐)"];
        let penalty_idx = Self::select("战斗中回复惩罚", &penalty_options, 1)?;
        config.battle.battle_heal_penalty = penalty_idx == 0;
        config.battle.overweight_heal_penalty = penalty_idx == 0;

        println!();
        Ok(config)
    }

    /// 步骤 5: 网络与性能
    fn step_network_settings(mut config: Config) -> Result<Config> {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  步骤 5/6: 网络与性能设置");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // 最大玩家数
        let player_options = vec!["100人", "500人", "1000人 (推荐)", "5000人", "10000人"];
        let player_idx = Self::select("最大在线玩家", &player_options, 2)?;
        config.game.max_players = match player_idx {
            0 => 100,
            1 => 500,
            2 => 1000,
            3 => 5000,
            4 => 10000,
            _ => 1000,
        };

        // 数据库类型
        let db_options = vec![
            "SQLite — 单服务器，无需安装 (推荐)",
            "MySQL — 大型服务器，需提前安装",
        ];
        let db_idx = Self::select("数据库类型", &db_options, 0)?;
        config.database.backend = if db_idx == 0 {
            "sqlite".to_string()
        } else {
            "mysql".to_string()
        };

        // MySQL 配置
        if db_idx == 1 {
            println!("\n  MySQL 配置:");
            config.database.mysql_host = Self::input("主机地址", "127.0.0.1", None)?;
            config.database.mysql_port = Self::input_u16("端口", 3306)?;
            config.database.mysql_user = Self::input("用户名", "deviruchi", None)?;
            config.database.mysql_password = Self::input("密码", "", Some("（留空表示无密码）"))?;
            config.database.mysql_database = Self::input("数据库名", "deviruchi", None)?;
        }

        // 自动保存
        let save_options = vec!["30秒", "60秒 (推荐)", "300秒 (5分钟)", "600秒 (10分钟)"];
        let save_idx = Self::select("自动保存间隔", &save_options, 1)?;
        config.game.autosave_interval_seconds = match save_idx {
            0 => 30,
            1 => 60,
            2 => 300,
            3 => 600,
            _ => 60,
        };

        // 连接超时
        let timeout_options = vec!["60秒", "300秒 (推荐)", "600秒"];
        let timeout_idx = Self::select("连接超时", &timeout_options, 1)?;
        config.game.timeout_seconds = match timeout_idx {
            0 => 60,
            1 => 300,
            2 => 600,
            _ => 300,
        };

        println!();
        Ok(config)
    }

    /// 步骤 6: 日志与高级设置
    fn step_advanced_settings(mut config: Config) -> Result<Config> {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  步骤 6/6: 日志与高级设置");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // 日志级别
        let log_options = vec!["ERROR — 仅错误", "WARN — 警告和错误", "INFO — 常规信息 (推荐)", "DEBUG — 调试信息", "TRACE — 详细追踪"];
        let log_idx = Self::select("日志级别", &log_options, 2)?;
        config.logging.level = match log_idx {
            0 => "error".to_string(),
            1 => "warn".to_string(),
            2 => "info".to_string(),
            3 => "debug".to_string(),
            4 => "trace".to_string(),
            _ => "info".to_string(),
        };

        // 日志输出
        let output_options = vec!["仅控制台", "仅文件", "控制台 + 文件 (推荐)"];
        let output_idx = Self::select("日志输出", &output_options, 2)?;
        config.logging.console = matches!(output_idx, 0 | 2);
        config.logging.enabled = matches!(output_idx, 1 | 2);

        // 日志轮转
        let rotation_options = vec!["按小时轮转 (推荐)", "按天轮转", "不轮转"];
        let rotation_idx = Self::select("日志轮转", &rotation_options, 0)?;
        config.logging.rotation_hourly = rotation_idx == 0;

        // 端口配置
        println!("\n  端口配置 (通常无需修改，直接回车使用默认值):");
        config.network.login_port = Self::input_u16("登录服务器端口", 6900)?;
        config.network.char_port = Self::input_u16("角色服务器端口", 6000)?;
        config.network.map_port = Self::input_u16("地图服务器端口", 6121)?;

        // 最大连接数
        let conn_options = vec!["5000", "10000 (推荐)", "20000", "50000"];
        let conn_idx = Self::select("最大连接数", &conn_options, 1)?;
        config.network.max_connections = match conn_idx {
            0 => 5000,
            1 => 10000,
            2 => 20000,
            3 => 50000,
            _ => 10000,
        };

        println!();
        Ok(config)
    }

    /// 显示配置预览
    fn show_preview(config: &Config) {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  配置预览");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│ [server]                                                    │");
        println!("│ name = \"{}\" {:<45}│", config.server.name, "");
        println!("│ mode = \"{}\" {:<45}│", format!("{:?}", config.server.mode).to_lowercase(), "");
        println!("│                                                             │");
        println!("│ [game]                                                      │");
        println!("│ max_players = {:<47}│", config.game.max_players);
        println!("│ max_level = {:<49}│", config.game.max_level);
        println!("│ job_level_cap = {:<45}│", config.game.job_level_cap);
        println!("│ death_drop_items = {:<42}│", config.game.death_drop_items);
        println!("│                                                             │");
        println!("│ [battle]                                                    │");
        println!("│ base_exp_rate = {:<45}│", config.battle.base_exp_rate);
        println!("│ job_exp_rate = {:<46}│", config.battle.job_exp_rate);
        println!("│ item_drop_rate = {:<44}│", config.battle.item_drop_rate);
        println!("│ zeny_rate = {:<49}│", config.battle.zeny_rate);
        println!("│ pvp_mode = {:<50}│", config.battle.pvp_mode);
        println!("│                                                             │");
        println!("│ [database]                                                  │");
        println!("│ backend = \"{}\" {:<44}│", config.database.backend, "");
        if config.database.backend == "mysql" {
            println!("│ mysql_host = \"{}\" {:<41}│", config.database.mysql_host, "");
        }
        println!("│                                                             │");
        println!("│ [network]                                                   │");
        println!("│ login_port = {:<48}│", config.network.login_port);
        println!("│ char_port = {:<49}│", config.network.char_port);
        println!("│ map_port = {:<50}│", config.network.map_port);
        println!("│                                                             │");
        println!("│ [logging]                                                   │");
        println!("│ level = \"{}\" {:<46}│", config.logging.level, "");
        println!("│ console = {:<51}│", config.logging.console);
        println!("└─────────────────────────────────────────────────────────────┘");
    }

    // ========== 输入辅助函数 ==========

    /// 文本输入
    fn input(prompt: &str, default: &str, hint: Option<&str>) -> Result<String> {
        let hint_str = hint.map(|h| format!(" {}", h)).unwrap_or_default();
        print!("? {} [{}]{}: ", prompt, default, hint_str);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(input.to_string())
        }
    }

    /// 数字输入 (f64)
    fn input_f64(prompt: &str, default: f64) -> Result<f64> {
        print!("? {} [{}]: ", prompt, default);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            Ok(default)
        } else {
            input.parse::<f64>().map_err(|e| anyhow::anyhow!("无效数字: {}", e))
        }
    }

    /// 数字输入 (u8)
    fn input_u8(prompt: &str, default: u8) -> Result<u8> {
        print!("? {} [{}]: ", prompt, default);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            Ok(default)
        } else {
            input.parse::<u8>().map_err(|e| anyhow::anyhow!("无效数字: {}", e))
        }
    }

    /// 数字输入 (u16)
    fn input_u16(prompt: &str, default: u16) -> Result<u16> {
        print!("? {} [{}]: ", prompt, default);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            Ok(default)
        } else {
            input.parse::<u16>().map_err(|e| anyhow::anyhow!("无效数字: {}", e))
        }
    }

    /// 选择列表
    fn select(prompt: &str, options: &[&str], default: usize) -> Result<usize> {
        println!("? {}:", prompt);
        for (i, option) in options.iter().enumerate() {
            let marker = if i == default { "●" } else { "○" };
            println!("  {} {}", marker, option);
        }
        print!("\n  选择 [{}]: ", default + 1);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            Ok(default)
        } else {
            let idx = input.parse::<usize>().map_err(|e| anyhow::anyhow!("无效选择: {}", e))?;
            if idx == 0 || idx > options.len() {
                Err(anyhow::anyhow!("选择超出范围，请输入 1-{}", options.len()))
            } else {
                Ok(idx - 1)
            }
        }
    }
}
