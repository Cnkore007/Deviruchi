//! 雇佣兵数据模块
//!
//! 参考 rAthena mercenary_db.yml 格式，包含完整的
//! 属性/技能/合同/忠诚度等字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 雇佣兵类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MercenaryClass {
    /// 弓手系
    Archer,
    /// 枪兵系
    Lancer,
    /// 剑士系
    Swordman,
}

/// 雇佣兵技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MercenarySkill {
    pub skill_name: String,
    pub max_level: u8,
    pub current_level: u8,
}

/// 雇佣兵实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mercenary {
    pub mercenary_id: u32,
    pub owner_id: u32,
    pub mercenary_class: u16,
    pub name: String,
    pub level: u16,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub atk: u32,
    pub defense: u32,
    pub magic_defense: u32,
    // 六属性
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    // 战斗属性
    pub hit: i16,
    pub flee: i16,
    pub walk_speed: u16,
    pub attack_range: u16,
    // 忠诚度
    pub loyalty: u32,
    // 合同
    pub contract_end: Option<DateTime<Utc>>,
    pub contract_cost: u32,
    // 状态
    pub alive: bool,
    // 技能
    pub skills: Vec<MercenarySkill>,
}

impl Mercenary {
    /// 检查合同是否到期
    pub fn is_contract_expired(&self) -> bool {
        if let Some(end) = self.contract_end {
            Utc::now() >= end
        } else {
            false
        }
    }

    /// 剩余合同时间（秒）
    pub fn contract_remaining_secs(&self) -> i64 {
        if let Some(end) = self.contract_end {
            (end - Utc::now()).num_seconds().max(0)
        } else {
            0
        }
    }

    /// 受到伤害
    pub fn take_damage(&mut self, damage: u32) {
        if damage >= self.hp {
            self.hp = 0;
            self.alive = false;
        } else {
            self.hp -= damage;
        }
    }

    /// 增加忠诚度（上限 1000）
    pub fn increase_loyalty(&mut self, amount: u32) {
        self.loyalty = (self.loyalty + amount).min(1000);
    }

    /// 降低忠诚度
    pub fn decrease_loyalty(&mut self, amount: u32) {
        self.loyalty = self.loyalty.saturating_sub(amount);
    }
}

/// 雇佣兵模板数据（运行时使用）
#[derive(Debug, Clone)]
pub struct MercenaryData {
    pub class_id: u16,
    pub name: String,
    pub class_type: MercenaryClass,
    pub level: u16,
    pub hp: u32,
    pub sp: u32,
    pub atk: u32,
    pub atk2: u32,
    pub defense: u32,
    pub magic_defense: u32,
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    pub attack_range: u16,
    pub walk_speed: u16,
    pub contract_cost: u32,
    pub skills: Vec<(String, u8)>,
}

/// 雇佣兵数据库（支持 YAML 加载 + 硬编码回退）
pub struct MercenaryDatabase {
    templates: HashMap<u16, MercenaryData>,
}

impl MercenaryDatabase {
    /// 仅使用硬编码数据创建（供测试使用）
    pub fn new_hardcoded() -> Self {
        let mut db = Self {
            templates: HashMap::new(),
        };
        db.init_hardcoded();
        db
    }

    pub fn new() -> Self {
        let mut db = Self {
            templates: HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = ["db/mercenary_db.yml"];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match Self::load_from_yaml(path) {
                    Ok(templates) if !templates.is_empty() => {
                        let count = templates.len();
                        db.templates = templates;
                        tracing::info!("从 {} 加载了 {} 个雇佣兵模板", path, count);
                        return db;
                    }
                    Ok(_) => {
                        tracing::warn!("{} 解析结果为空，跳过", path);
                    }
                    Err(e) => {
                        tracing::warn!("加载 {} 失败: {}", path, e);
                    }
                }
            }
        }

        // 回退到硬编码
        tracing::info!("使用硬编码雇佣兵数据");
        db.init_hardcoded();
        db
    }

    fn load_from_yaml(
        path: &str,
    ) -> Result<HashMap<u16, MercenaryData>, Box<dyn std::error::Error>> {
        let yaml_db = super::yaml_loader::load_mercenary_db(path)?;
        let mut templates = HashMap::new();

        for (id, yt) in yaml_db {
            // 根据 ID 范围推断职业类型
            let class_type = match id {
                6001..=6019 => MercenaryClass::Swordman,
                6020..=6039 => MercenaryClass::Lancer,
                _ => MercenaryClass::Archer,
            };

            templates.insert(
                id,
                MercenaryData {
                    class_id: yt.id,
                    name: yt.name,
                    class_type,
                    level: yt.level,
                    hp: yt.hp,
                    sp: yt.sp,
                    atk: yt.attack,
                    atk2: yt.attack2,
                    defense: yt.defense,
                    magic_defense: yt.magic_defense,
                    str: yt.str,
                    agi: yt.agi,
                    vit: yt.vit,
                    int: yt.int,
                    dex: yt.dex,
                    luk: yt.luk,
                    attack_range: yt.attack_range,
                    walk_speed: yt.walk_speed,
                    contract_cost: 0,
                    skills: yt
                        .skills
                        .iter()
                        .map(|s| (s.name.clone(), s.max_level))
                        .collect(),
                },
            );
        }

        Ok(templates)
    }

    fn init_hardcoded(&mut self) {
        // 弓手系 - 基础
        self.templates.insert(
            6017,
            MercenaryData {
                class_id: 6017,
                name: "Mina".to_string(),
                class_type: MercenaryClass::Archer,
                level: 20,
                hp: 256,
                sp: 200,
                atk: 170,
                atk2: 85,
                defense: 7,
                magic_defense: 5,
                str: 1,
                agi: 16,
                vit: 5,
                int: 1,
                dex: 28,
                luk: 8,
                attack_range: 10,
                walk_speed: 150,
                contract_cost: 5000,
                skills: vec![
                    ("MA_DOUBLE".to_string(), 2),
                    ("MER_AUTOBERSERK".to_string(), 1),
                ],
            },
        );

        // 弓手系 - 中级
        self.templates.insert(
            6018,
            MercenaryData {
                class_id: 6018,
                name: "Lenaire".to_string(),
                class_type: MercenaryClass::Archer,
                level: 40,
                hp: 450,
                sp: 300,
                atk: 280,
                atk2: 140,
                defense: 12,
                magic_defense: 8,
                str: 3,
                agi: 22,
                vit: 8,
                int: 3,
                dex: 40,
                luk: 12,
                attack_range: 10,
                walk_speed: 140,
                contract_cost: 10000,
                skills: vec![
                    ("MA_DOUBLE".to_string(), 4),
                    ("MA_SHOWER".to_string(), 3),
                    ("MER_AUTOBERSERK".to_string(), 1),
                ],
            },
        );

        // 枪兵系 - 基础
        self.templates.insert(
            6019,
            MercenaryData {
                class_id: 6019,
                name: "Lance".to_string(),
                class_type: MercenaryClass::Lancer,
                level: 20,
                hp: 400,
                sp: 100,
                atk: 150,
                atk2: 50,
                defense: 20,
                magic_defense: 5,
                str: 15,
                agi: 8,
                vit: 12,
                int: 1,
                dex: 18,
                luk: 5,
                attack_range: 3,
                walk_speed: 170,
                contract_cost: 5000,
                skills: vec![("ML_PIERCE".to_string(), 2), ("ML_BRANDISH".to_string(), 1)],
            },
        );

        // 剑士系 - 基础
        self.templates.insert(
            6020,
            MercenaryData {
                class_id: 6020,
                name: "Rodel".to_string(),
                class_type: MercenaryClass::Swordman,
                level: 20,
                hp: 500,
                sp: 80,
                atk: 130,
                atk2: 40,
                defense: 25,
                magic_defense: 3,
                str: 12,
                agi: 10,
                vit: 18,
                int: 1,
                dex: 14,
                luk: 5,
                attack_range: 1,
                walk_speed: 180,
                contract_cost: 5000,
                skills: vec![
                    ("MS_BASH".to_string(), 3),
                    ("MER_AUTOBERSERK".to_string(), 1),
                ],
            },
        );

        // 剑士系 - 中级
        self.templates.insert(
            6021,
            MercenaryData {
                class_id: 6021,
                name: "Erend".to_string(),
                class_type: MercenaryClass::Swordman,
                level: 40,
                hp: 800,
                sp: 120,
                atk: 220,
                atk2: 70,
                defense: 35,
                magic_defense: 8,
                str: 20,
                agi: 14,
                vit: 25,
                int: 3,
                dex: 20,
                luk: 8,
                attack_range: 1,
                walk_speed: 170,
                contract_cost: 10000,
                skills: vec![
                    ("MS_BASH".to_string(), 5),
                    ("MS_MAGNUM".to_string(), 3),
                    ("MER_AUTOBERSERK".to_string(), 1),
                ],
            },
        );
    }

    /// 根据 class_id 获取模板
    pub fn get(&self, class_id: u16) -> Option<&MercenaryData> {
        self.templates.get(&class_id)
    }

    /// 获取所有模板
    pub fn get_all(&self) -> &HashMap<u16, MercenaryData> {
        &self.templates
    }

    /// 获取模板数量
    pub fn count(&self) -> usize {
        self.templates.len()
    }
}

impl Default for MercenaryDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mercenary_database_hardcoded() {
        let db = MercenaryDatabase::new_hardcoded();
        assert!(db.count() >= 5);

        let mina = db.get(6017).unwrap();
        assert_eq!(mina.name, "Mina");
        assert_eq!(mina.class_type, MercenaryClass::Archer);
        assert_eq!(mina.level, 20);
    }

    #[test]
    fn test_mercenary_database_all_types() {
        let db = MercenaryDatabase::new_hardcoded();

        // 弓手
        let archer = db.get(6017).unwrap();
        assert_eq!(archer.class_type, MercenaryClass::Archer);

        // 枪兵
        let lancer = db.get(6019).unwrap();
        assert_eq!(lancer.class_type, MercenaryClass::Lancer);

        // 剑士
        let swordman = db.get(6020).unwrap();
        assert_eq!(swordman.class_type, MercenaryClass::Swordman);
    }

    #[test]
    fn test_mercenary_contract() {
        let merc = Mercenary {
            mercenary_id: 1,
            owner_id: 100,
            mercenary_class: 6017,
            name: "Test".to_string(),
            level: 20,
            hp: 256,
            max_hp: 256,
            sp: 200,
            max_sp: 200,
            atk: 170,
            defense: 7,
            magic_defense: 5,
            str: 1,
            agi: 16,
            vit: 5,
            int: 1,
            dex: 28,
            luk: 8,
            hit: 28,
            flee: 16,
            walk_speed: 150,
            attack_range: 10,
            loyalty: 100,
            contract_end: Some(Utc::now() + chrono::Duration::hours(1)),
            contract_cost: 5000,
            alive: true,
            skills: Vec::new(),
        };

        assert!(!merc.is_contract_expired());
        assert!(merc.contract_remaining_secs() > 0);
    }

    #[test]
    fn test_mercenary_loyalty() {
        let mut merc = Mercenary {
            mercenary_id: 1,
            owner_id: 100,
            mercenary_class: 6017,
            name: "Test".to_string(),
            level: 20,
            hp: 256,
            max_hp: 256,
            sp: 200,
            max_sp: 200,
            atk: 170,
            defense: 7,
            magic_defense: 5,
            str: 1,
            agi: 16,
            vit: 5,
            int: 1,
            dex: 28,
            luk: 8,
            hit: 28,
            flee: 16,
            walk_speed: 150,
            attack_range: 10,
            loyalty: 100,
            contract_end: None,
            contract_cost: 5000,
            alive: true,
            skills: Vec::new(),
        };

        merc.increase_loyalty(50);
        assert_eq!(merc.loyalty, 150);

        // 上限 1000
        merc.increase_loyalty(900);
        assert_eq!(merc.loyalty, 1000);

        merc.decrease_loyalty(200);
        assert_eq!(merc.loyalty, 800);

        // 不会下溢
        merc.decrease_loyalty(9999);
        assert_eq!(merc.loyalty, 0);
    }

    #[test]
    fn test_mercenary_damage() {
        let mut merc = Mercenary {
            mercenary_id: 1,
            owner_id: 100,
            mercenary_class: 6017,
            name: "Test".to_string(),
            level: 20,
            hp: 256,
            max_hp: 256,
            sp: 200,
            max_sp: 200,
            atk: 170,
            defense: 7,
            magic_defense: 5,
            str: 1,
            agi: 16,
            vit: 5,
            int: 1,
            dex: 28,
            luk: 8,
            hit: 28,
            flee: 16,
            walk_speed: 150,
            attack_range: 10,
            loyalty: 100,
            contract_end: None,
            contract_cost: 5000,
            alive: true,
            skills: Vec::new(),
        };

        merc.take_damage(100);
        assert_eq!(merc.hp, 156);
        assert!(merc.alive);

        merc.take_damage(9999);
        assert_eq!(merc.hp, 0);
        assert!(!merc.alive);
    }
}
