#![allow(dead_code)]
#![allow(non_snake_case)]

//! NPC YAML 数据加载器
//!
//! 从自定义 YAML 格式加载 NPC 模板数据。
//! YAML 格式示例:
//! ```yaml
//! npcs:
//!   - id: 1
//!     name: "Poring Merchant"
//!     display_name: "波利商人"
//!     type: shop
//!     map: new_1-1.gat
//!     x: 50
//!     y: 100
//!     sprite_id: 124
//!     script: |
//!       mes "[Poring Merchant]"
//!       mes "Welcome!"
//!     shop_items:
//!       - item_id: 501
//!         buy_price: 50
//!         sell_price: 25
//! ```

use super::data::{Npc, NpcEvent, NpcType};
use serde::Deserialize;
use std::collections::HashMap;

/// NPC YAML 文件结构
#[derive(Deserialize, Debug)]
struct NpcYamlFile {
    npcs: Vec<NpcYamlEntry>,
}

/// NPC YAML 条目
#[derive(Deserialize, Debug)]
struct NpcYamlEntry {
    id: u32,
    name: String,
    display_name: Option<String>,
    #[serde(rename = "type")]
    npc_type: String,
    map: String,
    x: u16,
    y: u16,
    sprite_id: Option<u16>,
    level: Option<u16>,
    script: Option<String>,
    #[serde(default)]
    shop_items: Vec<ShopYamlItem>,
    #[serde(default)]
    skills: Vec<SkillYamlItem>,
    /// 传送目标地图（仅 Warp 类型）
    dest_map: Option<String>,
    /// 传送目标 X（仅 Warp 类型）
    dest_x: Option<u16>,
    /// 传送目标 Y（仅 Warp 类型）
    dest_y: Option<u16>,
    /// 事件触发方式（OnClick/OnTouch/OnInit）
    event: Option<String>,
    /// 触发半径（OnTouch 事件时使用）
    trigger_radius: Option<u16>,
}

#[derive(Deserialize, Debug)]
struct ShopYamlItem {
    item_id: u16,
    buy_price: u32,
    sell_price: u32,
}

#[derive(Deserialize, Debug)]
struct SkillYamlItem {
    skill_id: u16,
    sp_cost: u16,
    price: u32,
}

/// 解析 NPC 类型字符串
fn parse_npc_type(s: &str) -> NpcType {
    match s.to_lowercase().as_str() {
        "shop" => NpcType::Shop,
        "skill_trainer" | "skilltrainer" => NpcType::SkillTrainer,
        "quest" => NpcType::Quest,
        "warp" => NpcType::Warp,
        "cashshop" | "cash_shop" => NpcType::CashShop,
        _ => NpcType::Shop,
    }
}

/// 解析 NPC 事件触发方式
fn parse_npc_event(s: &str) -> NpcEvent {
    match s.to_lowercase().as_str() {
        "onclick" | "click" => NpcEvent::OnClick,
        "ontouch" | "touch" => NpcEvent::OnTouch,
        "oninit" | "init" => NpcEvent::OnInit,
        _ => NpcEvent::None,
    }
}

/// 从 YAML 文件加载 NPC 数据
///
/// 返回 (npc_id -> Npc) 映射
pub fn load_npc_db(path: &str) -> Result<HashMap<u32, Npc>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let yaml: NpcYamlFile = serde_yaml::from_str(&content)?;

    let mut npcs = HashMap::new();

    for entry in yaml.npcs {
        let npc_type = parse_npc_type(&entry.npc_type);

        let npc = Npc {
            id: entry.id,
            name: entry.name.clone(),
            display_name: entry.display_name.unwrap_or(entry.name),
            type_: npc_type,
            pos_x: entry.x,
            pos_y: entry.y,
            map_name: entry.map,
            sprite_id: entry.sprite_id.unwrap_or(100),
            level: entry.level.unwrap_or(1),
            flags: 0,
            shop_items: parking_lot::RwLock::new(Vec::new()),
            skills: parking_lot::RwLock::new(Vec::new()),
            script: entry.script,
            dest_map: entry.dest_map,
            dest_x: entry.dest_x.unwrap_or(0),
            dest_y: entry.dest_y.unwrap_or(0),
            event: entry
                .event
                .as_ref()
                .map(|e| parse_npc_event(e))
                .unwrap_or_default(),
            trigger_radius: entry.trigger_radius.unwrap_or(0),
        };

        // 加载商店物品
        for item in entry.shop_items {
            npc.add_shop_item(item.item_id, item.buy_price, item.sell_price);
        }

        // 加载技能
        for skill in entry.skills {
            npc.add_skill(skill.skill_id, skill.sp_cost, skill.price);
        }

        npcs.insert(npc.id, npc);
    }

    Ok(npcs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npc_type() {
        assert_eq!(parse_npc_type("shop"), NpcType::Shop);
        assert_eq!(parse_npc_type("Shop"), NpcType::Shop);
        assert_eq!(parse_npc_type("skill_trainer"), NpcType::SkillTrainer);
        assert_eq!(parse_npc_type("quest"), NpcType::Quest);
        assert_eq!(parse_npc_type("warp"), NpcType::Warp);
        assert_eq!(parse_npc_type("cashshop"), NpcType::CashShop);
        assert_eq!(parse_npc_type("unknown"), NpcType::Shop); // 默认 Shop
    }

    #[test]
    fn test_load_npc_from_yaml_string() {
        let yaml_str = r#"
npcs:
  - id: 1
    name: "Poring Merchant"
    display_name: "波利商人"
    type: shop
    map: new_1-1.gat
    x: 50
    y: 100
    sprite_id: 124
    script: |
      mes "[Poring Merchant]"
      mes "Welcome!"
    shop_items:
      - item_id: 501
        buy_price: 50
        sell_price: 25
      - item_id: 502
        buy_price: 40
        sell_price: 20
  - id: 3
    name: "To Prontera"
    display_name: "前往普隆德拉"
    type: warp
    map: new_1-1.gat
    x: 150
    y: 150
    sprite_id: 405
    dest_map: prontera.gat
    dest_x: 150
    dest_y: 100
  - id: 7
    name: "Nurse"
    type: skill_trainer
    map: prontera.gat
    x: 150
    y: 180
    skills:
      - skill_id: 28
        sp_cost: 6
        price: 0
"#;

        // 写入临时文件并加载
        let tmp_path = "/tmp/test_npc_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let npcs = load_npc_db(tmp_path).unwrap();
        assert_eq!(npcs.len(), 3);

        // 验证商店 NPC
        let merchant = npcs.get(&1).unwrap();
        assert_eq!(merchant.name, "Poring Merchant");
        assert_eq!(merchant.display_name, "波利商人");
        assert_eq!(merchant.type_, NpcType::Shop);
        assert_eq!(merchant.map_name, "new_1-1.gat");
        assert_eq!(merchant.pos_x, 50);
        assert_eq!(merchant.sprite_id, 124);
        assert!(merchant.script.is_some());
        assert_eq!(merchant.shop_items.read().len(), 2);

        // 验证传送 NPC
        let warp = npcs.get(&3).unwrap();
        assert_eq!(warp.type_, NpcType::Warp);
        assert_eq!(warp.dest_map, Some("prontera.gat".to_string()));
        assert_eq!(warp.dest_x, 150);
        assert_eq!(warp.dest_y, 100);

        // 验证技能训练师
        let nurse = npcs.get(&7).unwrap();
        assert_eq!(nurse.type_, NpcType::SkillTrainer);
        assert_eq!(nurse.skills.read().len(), 1);

        // 清理
        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_parse_npc_event() {
        assert_eq!(parse_npc_event("onclick"), NpcEvent::OnClick);
        assert_eq!(parse_npc_event("click"), NpcEvent::OnClick);
        assert_eq!(parse_npc_event("OnTouch"), NpcEvent::OnTouch);
        assert_eq!(parse_npc_event("touch"), NpcEvent::OnTouch);
        assert_eq!(parse_npc_event("OnInit"), NpcEvent::OnInit);
        assert_eq!(parse_npc_event("init"), NpcEvent::OnInit);
        assert_eq!(parse_npc_event("unknown"), NpcEvent::None);
    }

    #[test]
    fn test_npc_with_event_and_trigger_radius() {
        let yaml_str = r#"
npcs:
  - id: 10
    name: "Touch NPC"
    type: quest
    map: test.gat
    x: 100
    y: 100
    event: OnTouch
    trigger_radius: 3
  - id: 20
    name: "Init NPC"
    type: quest
    map: test.gat
    x: 200
    y: 200
    event: OnInit
"#;

        let tmp_path = "/tmp/test_npc_event.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let npcs = load_npc_db(tmp_path).unwrap();
        assert_eq!(npcs.len(), 2);

        let touch_npc = npcs.get(&10).unwrap();
        assert_eq!(touch_npc.event, NpcEvent::OnTouch);
        assert_eq!(touch_npc.trigger_radius, 3);

        let init_npc = npcs.get(&20).unwrap();
        assert_eq!(init_npc.event, NpcEvent::OnInit);
        assert_eq!(init_npc.trigger_radius, 0); // 默认值

        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_npc_database_with_yaml() {
        let yaml_str = r#"
npcs:
  - id: 100
    name: "Test NPC"
    type: shop
    map: test.gat
    x: 10
    y: 20
"#;

        let tmp_path = "/tmp/test_npc_db2.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let npcs = load_npc_db(tmp_path).unwrap();
        let npc = npcs.get(&100).unwrap();
        assert_eq!(npc.name, "Test NPC");
        assert_eq!(npc.map_name, "test.gat");
        assert_eq!(npc.level, 1); // 默认值
        assert_eq!(npc.sprite_id, 100); // 默认值

        std::fs::remove_file(tmp_path).ok();
    }
}
