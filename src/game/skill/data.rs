use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 技能类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType {
    Passive,      // 被动技能
    Active,        // 主动技能
    Attack,        // 攻击技能
    Healing,       // 治疗技能
    Support,       // 辅助技能
    Debuff,        // 减益技能
}

/// 技能目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTarget {
    Self_,         // 自身
    Enemy,         // 敌方
    Ally,          // 友方
    Ground,        // 地面
    Party,         // 队伍
}

/// 技能数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: u16,
    pub name: &'static str,
    pub type_: SkillType,
    pub target: SkillTarget,
    pub level: u8,
    pub sp_cost: u16,
    pub hp_cost: u32,
    pub cast_time: u32,        // 吟唱时间(ms)
    pub cooldown: u32,         // 冷却时间(ms)
    pub range: u16,            // 施法范围
    pub skill_time: u32,       // 持续时间(ms)
    pub damage: i32,           // 基础伤害
    pub hit: i16,             // 命中加成
    pub element: u8,           // 属性 (0=无,1=火,2=水,3=风,4=地)
    pub flags: u32,
}

impl Skill {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            name: "Unknown",
            type_: SkillType::Active,
            target: SkillTarget::Enemy,
            level: 1,
            sp_cost: 0,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 0,
            hit: 0,
            element: 0,
            flags: 0,
        }
    }
}

/// 技能数据库
pub struct SkillDatabase {
    skills: HashMap<u16, Skill>,
}

impl SkillDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            skills: HashMap::new(),
        };
        db.init_default_skills();
        db
    }

    fn init_default_skills(&mut self) {
        // 基础攻击技能 - Bash
        self.skills.insert(1, Skill {
            id: 1,
            name: "Bash",
            type_: SkillType::Attack,
            target: SkillTarget::Enemy,
            level: 1,
            sp_cost: 8,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 1,
            skill_time: 0,
            damage: 110,  // 110% ATK
            hit: 3,
            element: 0,
            flags: 0,
        });

        // 火球
        self.skills.insert(25, Skill {
            id: 25,
            name: "Fire Ball",
            type_: SkillType::Attack,
            target: SkillTarget::Enemy,
            level: 1,
            sp_cost: 9,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 80,
            hit: 5,
            element: 1,  // 火属性
            flags: 0,
        });

        // 治愈术
        self.skills.insert(28, Skill {
            id: 28,
            name: "Heal",
            type_: SkillType::Healing,
            target: SkillTarget::Ally,
            level: 1,
            sp_cost: 6,
            hp_cost: 0,
            cast_time: 0,
            cooldown: 0,
            range: 9,
            skill_time: 0,
            damage: 35,  // 恢复量百分比
            hit: 0,
            element: 0,
            flags: 0,
        });

        // 加速术
        self.skills.insert(29, Skill {
            id: 29,
            name: "Increase AGI",
            type_: SkillType::Support,
            target: SkillTarget::Ally,
            level: 1,
            sp_cost: 10,
            hp_cost: 0,
            cast_time: 2000,
            cooldown: 0,
            range: 9,
            skill_time: 30000,  // 持续30秒
            damage: 0,
            hit: 0,
            element: 0,
            flags: 0,
        });
    }

    pub fn get(&self, skill_id: u16) -> Option<&Skill> {
        self.skills.get(&skill_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &Skill> {
        self.skills.values()
    }
}

impl Default for SkillDatabase {
    fn default() -> Self {
        Self::new()
    }
}
