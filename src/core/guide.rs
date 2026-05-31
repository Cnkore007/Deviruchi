//! 参考文档生成器
//!
//! 从数据库 YAML 文件生成 Markdown 格式的参考文档，
//! 包含物品、怪物、技能、地图列表，方便用户查找 ID 和配置。

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// 生成参考文档
///
/// 读取 db/ 目录下的 YAML 文件，生成 config/guide.md。
/// 每次服务器启动时调用，自动更新文档。
pub fn generate_guide() -> Result<()> {
    let guide_path = "config/guide.md";

    // 确保 config 目录存在
    if let Some(parent) = Path::new(guide_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut content = String::new();

    // 文档头部
    content.push_str("# Deviruchi 参考文档\n\n");
    content.push_str(&format!(
        "> 自动生成于 {}，请勿手动编辑\n\n",
        chrono_now()
    ));
    content.push_str("本文档列出服务器中的物品、怪物、技能和地图数据，方便您在配置时查找 ID 等信息。\n\n");
    content.push_str("---\n\n");

    // 生成各部分
    content.push_str(&generate_item_section());
    content.push_str(&generate_mob_section());
    content.push_str(&generate_skill_section());
    content.push_str(&generate_map_section());

    std::fs::write(guide_path, &content)?;
    tracing::info!("参考文档已生成: {}", guide_path);

    Ok(())
}

/// 获取当前时间字符串
fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 简单格式化时间戳
    format!("timestamp-{}", now)
}

// ============================================================
// 物品部分
// ============================================================

/// rAthena 物品条目（用于 YAML 反序列化）
#[derive(Deserialize, Debug)]
struct ItemFile {
    #[allow(dead_code)]
    Header: Option<serde_yaml::Value>,
    Body: Option<Vec<ItemEntry>>,
}

#[derive(Deserialize, Debug)]
struct ItemEntry {
    Id: u32,
    Name: String,
    #[serde(rename = "Type")]
    type_: Option<String>,
    #[serde(rename = "Buy", default)]
    buy: u32,
    #[serde(rename = "Sell", default)]
    sell: u32,
    #[serde(rename = "Attack", default)]
    attack: u16,
    #[serde(rename = "Defense", default)]
    defense: u16,
}

/// 简化物品条目（来自 item_db.yml）
#[derive(Deserialize, Debug)]
struct SimpleItemEntry {
    Id: u32,
    Name: String,
    #[serde(rename = "Type")]
    type_: Option<String>,
    #[serde(rename = "BuyPrice", default)]
    buy_price: u32,
    #[serde(rename = "SellPrice", default)]
    sell_price: u32,
    #[serde(rename = "HpRestore", default)]
    hp_restore: u16,
    #[serde(rename = "SpRestore", default)]
    sp_restore: u16,
}

fn generate_item_section() -> String {
    let mut section = String::from("## 物品列表\n\n");

    // 收集所有物品
    let mut items_by_type: HashMap<String, Vec<(u32, String, String)>> = HashMap::new();

    // 尝试加载简化格式
    if let Ok(content) = std::fs::read_to_string("db/item_db.yml") {
        if let Ok(items) = serde_yaml::from_str::<Vec<SimpleItemEntry>>(&content) {
            for item in items {
                let type_name = item.type_.clone().unwrap_or_else(|| "Etc".to_string());
                let detail = format!("{} (买:{}, 卖:{})", item.Name, item.buy_price, item.sell_price);
                items_by_type
                    .entry(type_name)
                    .or_default()
                    .push((item.Id, item.Name.clone(), detail));
            }
        }
    }

    // 尝试加载 rAthena 格式（equip/usable/etc）
    let rathena_files = [
        ("db/item_db_equip.yml", "装备"),
        ("db/item_db_usable.yml", "消耗品"),
        ("db/item_db_etc.yml", "杂项"),
    ];

    for (path, category) in &rathena_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            match serde_yaml::from_str::<ItemFile>(&content) {
                Ok(file) => {
                    if let Some(body) = file.Body {
                        let count = body.len();
                        tracing::debug!("从 {} 加载了 {} 个{}", path, count, category);
                        for item in body {
                            let type_name = item.type_.clone().unwrap_or_else(|| "Etc".to_string());
                            let detail = if item.attack > 0 {
                                format!("{} (攻:{}, 买:{})", item.Name, item.attack, item.buy)
                            } else if item.defense > 0 {
                                format!("{} (防:{}, 买:{})", item.Name, item.defense, item.buy)
                            } else {
                                format!("{} (买:{})", item.Name, item.buy)
                            };
                            items_by_type
                                .entry(type_name)
                                .or_default()
                                .push((item.Id, item.Name.clone(), detail));
                        }
                    } else {
                        tracing::warn!("{}: Body 为空", path);
                    }
                }
                Err(e) => {
                    tracing::warn!("解析 {} 失败: {}", path, e);
                }
            }
        } else {
            tracing::debug!("文件不存在: {}", path);
        }
    }

    if items_by_type.is_empty() {
        section.push_str("*未找到物品数据库文件*\n\n");
        return section;
    }

    // 按类型输出
    let type_order = ["Heal", "Weapon", "Armor", "Card", "PetEgg", "PetArmor", "Etc"];
    let type_labels = [
        ("Heal", "恢复道具"),
        ("Weapon", "武器"),
        ("Armor", "防具"),
        ("Card", "卡片"),
        ("PetEgg", "宠物蛋"),
        ("PetArmor", "宠物装备"),
        ("Etc", "杂项"),
    ];

    let type_label_map: HashMap<&str, &str> = type_labels.iter().cloned().collect();

    for type_key in &type_order {
        if let Some(items) = items_by_type.get(*type_key) {
            let label = type_label_map.get(type_key).unwrap_or(type_key);
            section.push_str(&format!("### {} ({})\n\n", label, type_key));
            section.push_str("| ID | 名称 | 详情 |\n");
            section.push_str("|----|------|------|\n");

            let mut sorted = items.clone();
            sorted.sort_by_key(|k| k.0);
            sorted.truncate(200); // 限制每类最多 200 条

            for (id, _name, detail) in &sorted {
                section.push_str(&format!("| {} | {} | {} |\n", id, _name, detail));
            }
            section.push('\n');
        }
    }

    // 统计总数
    let total: usize = items_by_type.values().map(|v| v.len()).sum();
    section.push_str(&format!("*共 {} 个物品*\n\n", total));

    section
}

// ============================================================
// 怪物部分
// ============================================================

/// rAthena 怪物条目
#[derive(Deserialize, Debug)]
struct MobFile {
    #[allow(dead_code)]
    Header: Option<serde_yaml::Value>,
    Body: Option<Vec<MobEntry>>,
}

#[derive(Deserialize, Debug)]
struct MobEntry {
    Id: u16,
    #[serde(rename = "AegisName")]
    #[allow(dead_code)]
    aegis_name: Option<String>,
    Name: String,
    #[serde(rename = "Level", default)]
    level: u16,
    #[serde(rename = "Hp", default)]
    hp: u32,
    #[serde(rename = "BaseExp", default)]
    base_exp: u32,
    #[serde(rename = "JobExp", default)]
    job_exp: u32,
    #[serde(rename = "Attack", default)]
    attack: u16,
    #[serde(rename = "Defense", default)]
    defense: u16,
    #[serde(rename = "Size", default)]
    size: Option<String>,
    #[serde(rename = "Race", default)]
    race: Option<String>,
}

fn generate_mob_section() -> String {
    let mut section = String::from("## 怪物列表\n\n");

    let path = "db/mob_db.yml";
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            section.push_str("*未找到怪物数据库文件*\n\n");
            return section;
        }
    };

    let file: MobFile = match serde_yaml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            section.push_str(&format!("*解析怪物数据库失败: {}*\n\n", e));
            return section;
        }
    };

    let body = match file.Body {
        Some(b) => b,
        None => {
            section.push_str("*怪物数据库为空*\n\n");
            return section;
        }
    };

    section.push_str("| ID | 名称 | 等级 | HP | 基础经验 | 职业经验 | 种族 | 体型 |\n");
    section.push_str("|----|------|------|-----|----------|----------|------|------|\n");

    let mut sorted = body;
    sorted.sort_by_key(|m| m.Id);

    let display_count = sorted.len().min(500); // 最多显示 500 个
    for mob in sorted.iter().take(display_count) {
        let size = mob.size.as_deref().unwrap_or("-");
        let race = mob.race.as_deref().unwrap_or("-");
        section.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            mob.Id, mob.Name, mob.level, mob.hp, mob.base_exp, mob.job_exp, race, size
        ));
    }

    section.push('\n');
    section.push_str(&format!("*共 {} 个怪物（显示前 {} 个）*\n\n", sorted.len(), display_count));

    section
}

// ============================================================
// 技能部分
// ============================================================

/// rAthena 技能条目
#[derive(Deserialize, Debug)]
struct SkillFile {
    #[allow(dead_code)]
    Header: Option<serde_yaml::Value>,
    Body: Option<Vec<SkillEntry>>,
}

#[derive(Deserialize, Debug)]
struct SkillEntry {
    Id: u16,
    Name: String,
    #[serde(rename = "MaxLevel", default)]
    max_level: u8,
    #[serde(rename = "Type", default)]
    type_: Option<String>,
    #[serde(rename = "Description", default)]
    #[allow(dead_code)]
    description: Option<String>,
}

fn generate_skill_section() -> String {
    let mut section = String::from("## 技能列表\n\n");

    let path = "db/skill_db.yml";
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            section.push_str("*未找到技能数据库文件*\n\n");
            return section;
        }
    };

    let file: SkillFile = match serde_yaml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            section.push_str(&format!("*解析技能数据库失败: {}*\n\n", e));
            return section;
        }
    };

    let body = match file.Body {
        Some(b) => b,
        None => {
            section.push_str("*技能数据库为空*\n\n");
            return section;
        }
    };

    section.push_str("| ID | 名称 | 最大等级 | 类型 |\n");
    section.push_str("|----|------|----------|------|\n");

    let mut sorted = body;
    sorted.sort_by_key(|s| s.Id);

    for skill in &sorted {
        let type_name = skill.type_.as_deref().unwrap_or("-");
        section.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            skill.Id, skill.Name, skill.max_level, type_name
        ));
    }

    section.push('\n');
    section.push_str(&format!("*共 {} 个技能*\n\n", sorted.len()));

    section
}

// ============================================================
// 地图部分
// ============================================================

/// 常用地图列表（硬编码，因为地图数据在 GRF 文件中）
const COMMON_MAPS: &[(&str, &str)] = &[
    ("prontera", "普隆德拉（首都）"),
    ("geffen", "吉芬（魔法之城）"),
    ("payon", "裴扬（弓手村）"),
    ("morocc", "梦罗克（沙漠之城）"),
    ("alberta", "阿尔贝塔（港口）"),
    ("aldebaran", "阿尔德巴兰（钟楼之城）"),
    ("izlude", "伊斯鲁德（初心者岛对岸）"),
    ("comodo", "科莫多（海边之城）"),
    ("yuno", "朱诺（学术之城）"),
    ("lighthalzen", "里希塔尔特（工业之城）"),
    ("hugel", "休格尔（山间小镇）"),
    ("rachel", "拉赫尔（神殿之城）"),
    ("veins", "威尼斯（火山之城）"),
    ("einbroch", "恩布罗克（钢铁之城）"),
    ("einbech", "恩贝克（矿山小镇）"),
    ("amatsu", "天津（东方之城）"),
    ("gonryun", "昆仑（仙境之城）"),
    ("louyang", "龙之城（洛阳）"),
    ("ayothaya", "阿尤塔亚（泰国风之城）"),
    ("umbala", "乌姆巴拉（原始部落）"),
    ("niflheim", "尼芙海姆（亡灵之城）"),
    ("lhz_dun01", "里希塔尔特地下实验室 1F"),
    ("lhz_dun02", "里希塔尔特地下实验室 2F"),
    ("lhz_dun03", "里希塔尔特地下实验室 3F"),
    ("prt_maze01", "迷宫森林 1F"),
    ("gef_dun00", "吉芬地下洞穴 1F"),
    ("gef_dun01", "吉芬地下洞穴 2F"),
    ("pay_dun00", "裴扬地下洞穴 1F"),
    ("pay_dun01", "裴扬地下洞穴 2F"),
    ("pay_dun02", "裴扬地下洞穴 3F"),
    ("moc_pryd01", "金字塔 1F"),
    ("moc_pryd02", "金字塔 2F"),
    ("moc_pryd03", "金字塔 3F"),
    ("moc_pryd04", "金字塔 4F"),
    ("treasure01", "沉船 1F"),
    ("treasure02", "沉船 2F"),
    ("c_tower1", "钟楼 1F"),
    ("c_tower2", "钟楼 2F"),
    ("c_tower3", "钟楼 3F"),
    ("c_tower4", "钟楼 4F"),
    ("xmas_dun01", "冰洞 1F"),
    ("xmas_dun02", "冰洞 2F"),
    ("anthell01", "蚂蚁地狱 1F"),
    ("anthell02", "蚂蚁地狱 2F"),
    ("orcsdun01", "兽人地下洞穴 1F"),
    ("orcsdun02", "兽人地下洞穴 2F"),
    ("gld_dun01", "公会副本 1"),
    ("gld_dun02", "公会副本 2"),
    ("gld_dun03", "公会副本 3"),
    ("gld_dun04", "公会副本 4"),
];

fn generate_map_section() -> String {
    let mut section = String::from("## 地图列表\n\n");
    section.push_str("以下为常用地图名称，可用于配置 `respawn.default_map` 等参数。\n\n");
    section.push_str("| 名称 | 说明 |\n");
    section.push_str("|------|------|\n");

    for (name, desc) in COMMON_MAPS {
        section.push_str(&format!("| {} | {} |\n", name, desc));
    }

    section.push('\n');
    section.push_str(&format!("*共 {} 个常用地图*\n\n", COMMON_MAPS.len()));
    section.push_str("> 完整地图数据存储在 GRF 资源文件中，此处仅列出常用地图。\n\n");

    section
}
