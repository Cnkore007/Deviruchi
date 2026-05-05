//! Homunculus 数据模块
//!
//! 参考 rAthena homunculus_db.yml 格式，包含完整的
//! 属性/技能/进化/种族/元素等字段。

use serde::{Deserialize, Serialize};

/// 生命体类型（基础形态）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomunculusType {
    Lif,
    Amistr,
    Filir,
    Vanilmirth,
    // Renewal 生命体
    Eira,
    Bayeri,
    Sera,
    Dieter,
    Eleanor,
}

impl HomunculusType {
    /// 从字符串解析（用于数据库加载）
    pub fn from_str(s: &str) -> Self {
        match s {
            "Lif" => HomunculusType::Lif,
            "Amistr" => HomunculusType::Amistr,
            "Filir" => HomunculusType::Filir,
            "Vanilmirth" => HomunculusType::Vanilmirth,
            "Eira" => HomunculusType::Eira,
            "Bayeri" => HomunculusType::Bayeri,
            "Sera" => HomunculusType::Sera,
            "Dieter" => HomunculusType::Dieter,
            "Eleanor" => HomunculusType::Eleanor,
            _ => HomunculusType::Lif,
        }
    }
}

/// 生命体种族
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomunculusRace {
    Demihuman,
    Brute,
    Formless,
    Angel,
    Insect,
}

impl HomunculusRace {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            HomunculusRace::Demihuman => "Demihuman",
            HomunculusRace::Brute => "Brute",
            HomunculusRace::Formless => "Formless",
            HomunculusRace::Angel => "Angel",
            HomunculusRace::Insect => "Insect",
        }
    }

    /// 从字符串解析（用于数据库加载）
    pub fn from_str(s: &str) -> Self {
        match s {
            "Demihuman" => HomunculusRace::Demihuman,
            "Brute" => HomunculusRace::Brute,
            "Formless" => HomunculusRace::Formless,
            "Angel" => HomunculusRace::Angel,
            "Insect" => HomunculusRace::Insect,
            _ => HomunculusRace::Formless,
        }
    }
}

/// 生命体进化阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionStage {
    /// 基础形态
    Base,
    /// 进化形态 (H)
    Evolved,
    /// S 级进化形态 (H2)
    SuperEvolved,
}

impl EvolutionStage {
    /// 转换为字符串（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            EvolutionStage::Base => "Base",
            EvolutionStage::Evolved => "Evolved",
            EvolutionStage::SuperEvolved => "SuperEvolved",
        }
    }

    /// 从字符串解析（用于数据库加载）
    pub fn from_str(s: &str) -> Self {
        match s {
            "Evolved" => EvolutionStage::Evolved,
            "SuperEvolved" => EvolutionStage::SuperEvolved,
            _ => EvolutionStage::Base,
        }
    }
}

/// 生命体技能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomunculusSkill {
    pub skill_id: u16,
    pub skill_name: String,
    pub level: u8,
    pub max_level: u8,
    /// 需要的基础等级
    pub required_level: u16,
    /// 需要的亲密度
    pub required_intimacy: u32,
    /// 是否需要进化
    pub require_evolution: bool,
    /// 前置技能: (skill_name, min_level)
    pub prerequisites: Vec<(String, u8)>,
}

/// 生命体属性成长数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatGrowth {
    pub base: u32,
    pub growth_min: u32,
    pub growth_max: u32,
    pub evolution_min: u32,
    pub evolution_max: u32,
}

/// 生命体模板数据（从 YAML 或硬编码加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomunculusTemplate {
    pub class_name: String,
    pub name: String,
    pub evolution_class: Option<String>,
    pub food_item: Option<String>,
    pub hungry_delay: u32,
    pub race: HomunculusRace,
    pub element: String,
    pub size: String,
    pub evolution_size: Option<String>,
    pub attack_delay: u32,
    /// 属性成长数据
    pub hp_growth: StatGrowth,
    pub sp_growth: StatGrowth,
    pub str_growth: StatGrowth,
    pub agi_growth: StatGrowth,
    pub vit_growth: StatGrowth,
    pub int_growth: StatGrowth,
    pub dex_growth: StatGrowth,
    pub luk_growth: StatGrowth,
    /// 技能树
    pub skill_tree: Vec<HomunculusSkill>,
}

/// 生命体实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Homunculus {
    pub homun_id: u32,
    pub owner_id: u32,
    pub homunculus_type: HomunculusType,
    pub name: String,
    pub level: u16,
    pub exp: u64,
    pub hunger: u32,
    pub intimacy: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub alive: bool,
    // 六属性
    pub str: u16,
    pub agi: u16,
    pub vit: u16,
    pub int: u16,
    pub dex: u16,
    pub luk: u16,
    // 战斗属性
    pub atk: u16,
    pub matk: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub hit: i16,
    pub flee: i16,
    pub walk_speed: u16,
    pub attack_delay: u32,
    // 进化
    pub evolution_stage: EvolutionStage,
    pub evolved: bool,
    // 技能
    pub skills: Vec<HomunculusSkill>,
    pub skill_points: u16,
    // 种族/元素
    pub race: HomunculusRace,
    pub element: String,
}

impl Homunculus {
    /// 从模板创建新生命体
    pub fn from_template(
        homun_id: u32,
        owner_id: u32,
        template: &HomunculusTemplate,
        name: String,
    ) -> Self {
        Self {
            homun_id,
            owner_id,
            homunculus_type: Self::type_from_class(&template.class_name),
            name,
            level: 1,
            exp: 0,
            hunger: 100,
            intimacy: 100,
            hp: template.hp_growth.base,
            max_hp: template.hp_growth.base,
            sp: template.sp_growth.base,
            max_sp: template.sp_growth.base,
            alive: true,
            str: template.str_growth.base as u16,
            agi: template.agi_growth.base as u16,
            vit: template.vit_growth.base as u16,
            int: template.int_growth.base as u16,
            dex: template.dex_growth.base as u16,
            luk: template.luk_growth.base as u16,
            atk: 0,
            matk: 0,
            defense: 0,
            magic_defense: 0,
            hit: 0,
            flee: 0,
            walk_speed: 200,
            attack_delay: template.attack_delay,
            evolution_stage: EvolutionStage::Base,
            evolved: false,
            skills: Vec::new(),
            skill_points: 0,
            race: template.race,
            element: template.element.clone(),
        }
    }

    fn type_from_class(class_name: &str) -> HomunculusType {
        match class_name {
            "Lif" | "Lif2" => HomunculusType::Lif,
            "Amistr" | "Amistr2" => HomunculusType::Amistr,
            "Filir" | "Filir2" => HomunculusType::Filir,
            "Vanilmirth" | "Vanilmirth2" => HomunculusType::Vanilmirth,
            "Eira" => HomunculusType::Eira,
            "Bayeri" => HomunculusType::Bayeri,
            "Sera" => HomunculusType::Sera,
            "Dieter" => HomunculusType::Dieter,
            "Eleanor" => HomunculusType::Eleanor,
            _ => HomunculusType::Lif,
        }
    }

    /// 喂食：恢复饥饿度
    pub fn feed(&mut self, hunger_restore: u32) {
        self.hunger = (self.hunger + hunger_restore).min(100);
    }

    /// 增加亲密度
    pub fn increase_intimacy(&mut self, amount: u32) {
        self.intimacy = (self.intimacy + amount).min(100000);
    }

    /// 降低亲密度
    pub fn decrease_intimacy(&mut self, amount: u32) {
        self.intimacy = self.intimacy.saturating_sub(amount);
    }

    /// 检查是否饥饿
    pub fn is_hungry(&self) -> bool {
        self.hunger < 20
    }

    /// 检查是否死亡
    pub fn is_dead(&self) -> bool {
        !self.alive || self.hp == 0
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

    /// 复活
    pub fn revive(&mut self, hp_percent: u32) {
        self.alive = true;
        self.hp = (self.max_hp * hp_percent / 100).max(1);
    }
}

/// 生命体数据库（从 rAthena homunculus_db.yml 加载或硬编码）
pub struct HomunculusDatabase {
    templates: std::collections::HashMap<String, HomunculusTemplate>,
}

impl HomunculusDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            templates: std::collections::HashMap::new(),
        };

        // 尝试从 YAML 加载
        let yaml_paths = ["rathena/db/re/homunculus_db.yml", "db/homunculus_db.yml"];

        for path in &yaml_paths {
            if std::path::Path::new(path).exists() {
                match Self::load_from_yaml(path) {
                    Ok(templates) if !templates.is_empty() => {
                        let count = templates.len();
                        db.templates = templates;
                        tracing::info!("从 {} 加载了 {} 个生命体模板", path, count);
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
        tracing::info!("使用硬编码生命体数据");
        db.init_hardcoded();
        db
    }

    fn load_from_yaml(
        _path: &str,
    ) -> Result<std::collections::HashMap<String, HomunculusTemplate>, Box<dyn std::error::Error>>
    {
        // TODO: 实现完整解析 rAthena homunculus_db.yml 格式
        Ok(std::collections::HashMap::new())
    }

    fn init_hardcoded(&mut self) {
        // Lif - 治疗型生命体
        self.templates.insert(
            "Lif".to_string(),
            HomunculusTemplate {
                class_name: "Lif".to_string(),
                name: "Lif".to_string(),
                evolution_class: Some("Lif_H".to_string()),
                food_item: None,
                hungry_delay: 60000,
                race: HomunculusRace::Demihuman,
                element: "Neutral".to_string(),
                size: "Small".to_string(),
                evolution_size: Some("Medium".to_string()),
                attack_delay: 700,
                hp_growth: StatGrowth {
                    base: 150,
                    growth_min: 60,
                    growth_max: 100,
                    evolution_min: 800,
                    evolution_max: 2400,
                },
                sp_growth: StatGrowth {
                    base: 40,
                    growth_min: 4,
                    growth_max: 9,
                    evolution_min: 220,
                    evolution_max: 480,
                },
                str_growth: StatGrowth {
                    base: 17,
                    growth_min: 5,
                    growth_max: 19,
                    evolution_min: 10,
                    evolution_max: 30,
                },
                agi_growth: StatGrowth {
                    base: 20,
                    growth_min: 5,
                    growth_max: 19,
                    evolution_min: 10,
                    evolution_max: 30,
                },
                vit_growth: StatGrowth {
                    base: 15,
                    growth_min: 5,
                    growth_max: 19,
                    evolution_min: 20,
                    evolution_max: 40,
                },
                int_growth: StatGrowth {
                    base: 35,
                    growth_min: 4,
                    growth_max: 20,
                    evolution_min: 30,
                    evolution_max: 50,
                },
                dex_growth: StatGrowth {
                    base: 24,
                    growth_min: 6,
                    growth_max: 20,
                    evolution_min: 20,
                    evolution_max: 50,
                },
                luk_growth: StatGrowth {
                    base: 12,
                    growth_min: 6,
                    growth_max: 20,
                    evolution_min: 10,
                    evolution_max: 30,
                },
                skill_tree: vec![HomunculusSkill {
                    skill_id: 8001,
                    skill_name: "HLIF_HEAL".to_string(),
                    level: 0,
                    max_level: 5,
                    required_level: 0,
                    required_intimacy: 0,
                    require_evolution: false,
                    prerequisites: Vec::new(),
                }],
            },
        );

        // Amistr - 坦克型生命体
        self.templates.insert(
            "Amistr".to_string(),
            HomunculusTemplate {
                class_name: "Amistr".to_string(),
                name: "Amistr".to_string(),
                evolution_class: Some("Amistr_H".to_string()),
                food_item: None,
                hungry_delay: 60000,
                race: HomunculusRace::Brute,
                element: "Neutral".to_string(),
                size: "Medium".to_string(),
                evolution_size: Some("Large".to_string()),
                attack_delay: 800,
                hp_growth: StatGrowth {
                    base: 200,
                    growth_min: 80,
                    growth_max: 120,
                    evolution_min: 1000,
                    evolution_max: 3000,
                },
                sp_growth: StatGrowth {
                    base: 30,
                    growth_min: 3,
                    growth_max: 7,
                    evolution_min: 150,
                    evolution_max: 350,
                },
                str_growth: StatGrowth {
                    base: 20,
                    growth_min: 6,
                    growth_max: 20,
                    evolution_min: 15,
                    evolution_max: 35,
                },
                agi_growth: StatGrowth {
                    base: 12,
                    growth_min: 4,
                    growth_max: 15,
                    evolution_min: 8,
                    evolution_max: 25,
                },
                vit_growth: StatGrowth {
                    base: 25,
                    growth_min: 7,
                    growth_max: 22,
                    evolution_min: 25,
                    evolution_max: 50,
                },
                int_growth: StatGrowth {
                    base: 10,
                    growth_min: 3,
                    growth_max: 12,
                    evolution_min: 10,
                    evolution_max: 25,
                },
                dex_growth: StatGrowth {
                    base: 15,
                    growth_min: 5,
                    growth_max: 18,
                    evolution_min: 15,
                    evolution_max: 35,
                },
                luk_growth: StatGrowth {
                    base: 8,
                    growth_min: 4,
                    growth_max: 15,
                    evolution_min: 8,
                    evolution_max: 25,
                },
                skill_tree: vec![HomunculusSkill {
                    skill_id: 8005,
                    skill_name: "HAMI_DEFENCE".to_string(),
                    level: 0,
                    max_level: 5,
                    required_level: 0,
                    required_intimacy: 0,
                    require_evolution: false,
                    prerequisites: Vec::new(),
                }],
            },
        );

        // Filir - 攻击型生命体
        self.templates.insert(
            "Filir".to_string(),
            HomunculusTemplate {
                class_name: "Filir".to_string(),
                name: "Filir".to_string(),
                evolution_class: Some("Filir_H".to_string()),
                food_item: None,
                hungry_delay: 60000,
                race: HomunculusRace::Brute,
                element: "Wind".to_string(),
                size: "Small".to_string(),
                evolution_size: Some("Medium".to_string()),
                attack_delay: 600,
                hp_growth: StatGrowth {
                    base: 100,
                    growth_min: 40,
                    growth_max: 80,
                    evolution_min: 600,
                    evolution_max: 1800,
                },
                sp_growth: StatGrowth {
                    base: 35,
                    growth_min: 3,
                    growth_max: 8,
                    evolution_min: 180,
                    evolution_max: 400,
                },
                str_growth: StatGrowth {
                    base: 22,
                    growth_min: 7,
                    growth_max: 22,
                    evolution_min: 15,
                    evolution_max: 35,
                },
                agi_growth: StatGrowth {
                    base: 25,
                    growth_min: 7,
                    growth_max: 22,
                    evolution_min: 15,
                    evolution_max: 35,
                },
                vit_growth: StatGrowth {
                    base: 10,
                    growth_min: 3,
                    growth_max: 12,
                    evolution_min: 10,
                    evolution_max: 25,
                },
                int_growth: StatGrowth {
                    base: 15,
                    growth_min: 4,
                    growth_max: 15,
                    evolution_min: 15,
                    evolution_max: 30,
                },
                dex_growth: StatGrowth {
                    base: 20,
                    growth_min: 6,
                    growth_max: 20,
                    evolution_min: 15,
                    evolution_max: 35,
                },
                luk_growth: StatGrowth {
                    base: 10,
                    growth_min: 5,
                    growth_max: 18,
                    evolution_min: 10,
                    evolution_max: 25,
                },
                skill_tree: vec![HomunculusSkill {
                    skill_id: 8009,
                    skill_name: "HFLI_SBR44".to_string(),
                    level: 0,
                    max_level: 3,
                    required_level: 0,
                    required_intimacy: 0,
                    require_evolution: false,
                    prerequisites: Vec::new(),
                }],
            },
        );

        // Vanilmirth - 魔法型生命体
        self.templates.insert(
            "Vanilmirth".to_string(),
            HomunculusTemplate {
                class_name: "Vanilmirth".to_string(),
                name: "Vanilmirth".to_string(),
                evolution_class: Some("Vanilmirth_H".to_string()),
                food_item: None,
                hungry_delay: 60000,
                race: HomunculusRace::Formless,
                element: "Neutral".to_string(),
                size: "Medium".to_string(),
                evolution_size: Some("Large".to_string()),
                attack_delay: 750,
                hp_growth: StatGrowth {
                    base: 120,
                    growth_min: 50,
                    growth_max: 90,
                    evolution_min: 700,
                    evolution_max: 2100,
                },
                sp_growth: StatGrowth {
                    base: 45,
                    growth_min: 5,
                    growth_max: 10,
                    evolution_min: 250,
                    evolution_max: 500,
                },
                str_growth: StatGrowth {
                    base: 15,
                    growth_min: 5,
                    growth_max: 18,
                    evolution_min: 12,
                    evolution_max: 28,
                },
                agi_growth: StatGrowth {
                    base: 15,
                    growth_min: 5,
                    growth_max: 18,
                    evolution_min: 12,
                    evolution_max: 28,
                },
                vit_growth: StatGrowth {
                    base: 15,
                    growth_min: 5,
                    growth_max: 18,
                    evolution_min: 15,
                    evolution_max: 30,
                },
                int_growth: StatGrowth {
                    base: 25,
                    growth_min: 6,
                    growth_max: 22,
                    evolution_min: 25,
                    evolution_max: 45,
                },
                dex_growth: StatGrowth {
                    base: 20,
                    growth_min: 6,
                    growth_max: 20,
                    evolution_min: 18,
                    evolution_max: 38,
                },
                luk_growth: StatGrowth {
                    base: 12,
                    growth_min: 5,
                    growth_max: 18,
                    evolution_min: 10,
                    evolution_max: 28,
                },
                skill_tree: vec![HomunculusSkill {
                    skill_id: 8013,
                    skill_name: "HVAN_CAPRICE".to_string(),
                    level: 0,
                    max_level: 5,
                    required_level: 0,
                    required_intimacy: 0,
                    require_evolution: false,
                    prerequisites: Vec::new(),
                }],
            },
        );
    }

    /// 根据类名获取模板
    pub fn get(&self, class_name: &str) -> Option<&HomunculusTemplate> {
        self.templates.get(class_name)
    }

    /// 根据生命体类型获取模板
    pub fn get_by_type(&self, htype: HomunculusType) -> Option<&HomunculusTemplate> {
        let class_name = match htype {
            HomunculusType::Lif => "Lif",
            HomunculusType::Amistr => "Amistr",
            HomunculusType::Filir => "Filir",
            HomunculusType::Vanilmirth => "Vanilmirth",
            HomunculusType::Eira => "Eira",
            HomunculusType::Bayeri => "Bayeri",
            HomunculusType::Sera => "Sera",
            HomunculusType::Dieter => "Dieter",
            HomunculusType::Eleanor => "Eleanor",
        };
        self.templates.get(class_name)
    }

    /// 获取所有模板
    pub fn all(&self) -> impl Iterator<Item = (&String, &HomunculusTemplate)> {
        self.templates.iter()
    }

    /// 获取模板数量
    pub fn count(&self) -> usize {
        self.templates.len()
    }
}

impl Default for HomunculusDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homunculus_from_template() {
        let db = HomunculusDatabase::new();
        let template = db.get_by_type(HomunculusType::Lif).unwrap();

        let homun = Homunculus::from_template(1, 100, template, "MyLif".to_string());

        assert_eq!(homun.homun_id, 1);
        assert_eq!(homun.owner_id, 100);
        assert_eq!(homun.name, "MyLif");
        assert_eq!(homun.homunculus_type, HomunculusType::Lif);
        assert_eq!(homun.level, 1);
        assert_eq!(homun.hp, template.hp_growth.base);
        assert_eq!(homun.max_hp, template.hp_growth.base);
        assert_eq!(homun.sp, template.sp_growth.base);
        assert_eq!(homun.race, HomunculusRace::Demihuman);
        assert_eq!(homun.element, "Neutral");
        assert!(homun.alive);
        assert!(!homun.evolved);
    }

    #[test]
    fn test_homunculus_feed() {
        let db = HomunculusDatabase::new();
        let template = db.get_by_type(HomunculusType::Lif).unwrap();
        let mut homun = Homunculus::from_template(1, 100, template, "Test".to_string());

        homun.hunger = 50;
        homun.feed(20);
        assert_eq!(homun.hunger, 70);

        // 不超过 100
        homun.feed(50);
        assert_eq!(homun.hunger, 100);
    }

    #[test]
    fn test_homunculus_intimacy() {
        let db = HomunculusDatabase::new();
        let template = db.get_by_type(HomunculusType::Lif).unwrap();
        let mut homun = Homunculus::from_template(1, 100, template, "Test".to_string());

        homun.increase_intimacy(50);
        assert_eq!(homun.intimacy, 150);

        homun.decrease_intimacy(30);
        assert_eq!(homun.intimacy, 120);

        // 不会下溢
        homun.decrease_intimacy(200);
        assert_eq!(homun.intimacy, 0);
    }

    #[test]
    fn test_homunculus_damage_and_death() {
        let db = HomunculusDatabase::new();
        let template = db.get_by_type(HomunculusType::Amistr).unwrap();
        let mut homun = Homunculus::from_template(1, 100, template, "Tank".to_string());

        assert!(!homun.is_dead());

        homun.take_damage(100);
        assert_eq!(homun.hp, template.hp_growth.base - 100);
        assert!(homun.alive);

        // 致死伤害
        homun.take_damage(9999);
        assert_eq!(homun.hp, 0);
        assert!(!homun.alive);
        assert!(homun.is_dead());
    }

    #[test]
    fn test_homunculus_revive() {
        let db = HomunculusDatabase::new();
        let template = db.get_by_type(HomunculusType::Filir).unwrap();
        let mut homun = Homunculus::from_template(1, 100, template, "Test".to_string());

        homun.take_damage(9999);
        assert!(homun.is_dead());

        homun.revive(50); // 复活 50% HP
        assert!(homun.alive);
        assert_eq!(homun.hp, template.hp_growth.base / 2);
    }

    #[test]
    fn test_homunculus_database_types() {
        let db = HomunculusDatabase::new();

        // 验证 4 种基础生命体都存在
        assert!(db.get_by_type(HomunculusType::Lif).is_some());
        assert!(db.get_by_type(HomunculusType::Amistr).is_some());
        assert!(db.get_by_type(HomunculusType::Filir).is_some());
        assert!(db.get_by_type(HomunculusType::Vanilmirth).is_some());

        // Renewal 生命体在硬编码中不存在
        assert!(db.get_by_type(HomunculusType::Eira).is_none());

        assert_eq!(db.count(), 4);
    }

    #[test]
    fn test_type_from_class() {
        assert_eq!(
            Homunculus::type_from_class("Lif"),
            HomunculusType::Lif
        );
        assert_eq!(
            Homunculus::type_from_class("Lif2"),
            HomunculusType::Lif
        );
        assert_eq!(
            Homunculus::type_from_class("Amistr"),
            HomunculusType::Amistr
        );
        assert_eq!(
            Homunculus::type_from_class("unknown"),
            HomunculusType::Lif
        );
    }
}
