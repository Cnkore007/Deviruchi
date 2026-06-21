#![allow(non_snake_case)]

//! rAthena quest_db.yml 格式加载器

use super::data::{ObjectiveType, Quest, QuestObjective, QuestType};
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

#[derive(Deserialize, Debug)]
struct QuestDbFile {
    #[allow(dead_code)] // rAthena YAML compat
    Header: QuestDbHeader,
    Body: Option<Vec<QuestDbEntry>>,
}

#[derive(Deserialize, Debug)]
struct QuestDbHeader {
    #[serde(rename = "Type")]
    #[allow(dead_code)] // rAthena YAML compat
    _type: String,
    #[allow(dead_code)] // rAthena YAML compat
    Version: u32,
}

#[derive(Deserialize, Debug)]
struct QuestDbEntry {
    Id: u32,
    Title: Option<String>,
    #[serde(default)]
    Targets: Vec<QuestTarget>,
    #[serde(default)]
    Drops: Vec<QuestDrop>,
}

#[derive(Deserialize, Debug)]
struct QuestTarget {
    Mob: String,
    Count: u32,
}

#[derive(Deserialize, Debug)]
struct QuestDrop {
    #[allow(dead_code)] // rAthena YAML compat
    Mob: Option<String>,
    Item: String,
    #[serde(default = "default_one")]
    Count: u32,
    #[serde(default)]
    #[allow(dead_code)] // rAthena YAML compat
    Rate: u32,
}

fn default_one() -> u32 {
    1
}

impl QuestDbEntry {
    fn to_quest(&self) -> Quest {
        let title = self
            .Title
            .clone()
            .unwrap_or_else(|| format!("Quest {}", self.Id));
        let mut objectives = Vec::new();
        let mut obj_id = 1u32;

        // 击杀目标
        for target in &self.Targets {
            objectives.push(QuestObjective::new(
                obj_id,
                ObjectiveType::Kill,
                0, // mob name -> ID 需要额外解析
                target.Count,
                &format!("Defeat {} x{}", target.Mob, target.Count),
            ));
            obj_id += 1;
        }

        // 掉落收集目标
        for drop in &self.Drops {
            objectives.push(QuestObjective::new(
                obj_id,
                ObjectiveType::Collect,
                0, // item name -> ID 需要额外解析
                drop.Count,
                &format!("Collect {} x{}", drop.Item, drop.Count),
            ));
            obj_id += 1;
        }

        let quest_type = if !self.Targets.is_empty() {
            QuestType::KillHunt
        } else if !self.Drops.is_empty() {
            QuestType::CollectItem
        } else {
            QuestType::Custom
        };

        Quest::new(self.Id, &title, &title, quest_type).with_objectives(objectives)
    }
}

/// 从 rAthena quest_db.yml 加载任务数据库
pub fn load_quest_db(path: &str) -> Result<HashMap<u32, Quest>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml: QuestDbFile = serde_yaml::from_str(&content)?;
    let mut db = HashMap::new();
    if let Some(body) = yaml.Body {
        for entry in body {
            db.insert(entry.Id, entry.to_quest());
        }
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_quest_db_from_string() {
        let yaml_str = r#"
Header:
  Type: QUEST_DB
  Version: 3
Body:
  - Id: 1100
    Title: Solo in the Sphinx Dungeon!
    Targets:
      - Mob: ZEROM
        Count: 20
  - Id: 60000
    Title: Collect Materials
    Drops:
      - Mob: PORING
        Item: Jellopy
        Count: 10
        Rate: 5000
  - Id: 1000
    Title: Transcend
"#;

        let tmp_path = "/tmp/test_quest_db.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();
        let quests = load_quest_db(tmp_path).unwrap();
        assert_eq!(quests.len(), 3);

        let hunt = quests.get(&1100).unwrap();
        assert_eq!(hunt.title, "Solo in the Sphinx Dungeon!");
        assert_eq!(hunt.quest_type, QuestType::KillHunt);
        assert_eq!(hunt.objectives.len(), 1);
        assert_eq!(hunt.objectives[0].target_count, 20);

        let collect = quests.get(&60000).unwrap();
        assert_eq!(collect.quest_type, QuestType::CollectItem);
        assert_eq!(collect.objectives.len(), 1);
        assert_eq!(collect.objectives[0].target_count, 10);

        let simple = quests.get(&1000).unwrap();
        assert_eq!(simple.title, "Transcend");
        assert_eq!(simple.objectives.len(), 0);

        std::fs::remove_file(tmp_path).ok();
    }
}
