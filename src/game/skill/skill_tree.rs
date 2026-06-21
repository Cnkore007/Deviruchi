//! 技能树系统
//!
//! 定义技能树结构和技能学习条件检查。
//! 支持从 YAML 文件加载技能树数据，或使用硬编码的默认技能树。
//!
//! # 技能树结构
//! - 每个职业（job）有独立的技能树
//! - 每个技能节点包含：技能ID、最大等级、前置技能要求
//! - 学习技能前需要检查：职业匹配、前置技能等级满足

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;

/// 技能树节点
///
/// 定义单个技能在技能树中的位置和学习条件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTreeNode {
    /// 技能 ID（对应 SkillDatabase 中的技能）
    pub skill_id: u16,
    /// 该技能最大可学习等级
    pub max_level: u8,
    /// 前置技能要求列表：(skill_id, required_level)
    /// 学习此技能前，需要所有前置技能达到指定等级
    pub prerequisite_skills: Vec<(u16, u8)>,
    /// 要求的职业 ID（0 = 所有职业可学习）
    pub required_job: u16,
}

impl SkillTreeNode {
    /// 创建新的技能树节点
    pub fn new(skill_id: u16, max_level: u8) -> Self {
        Self {
            skill_id,
            max_level,
            prerequisite_skills: Vec::new(),
            required_job: 0,
        }
    }

    /// 设置前置技能要求
    pub fn with_prerequisites(mut self, prerequisites: Vec<(u16, u8)>) -> Self {
        self.prerequisite_skills = prerequisites;
        self
    }

    /// 设置职业要求
    pub fn with_job(mut self, job_id: u16) -> Self {
        self.required_job = job_id;
        self
    }
}

/// 技能树数据库
///
/// 管理所有职业的技能树数据，提供技能学习条件检查。
pub struct SkillTree {
    /// job_id -> 该职业的技能树节点列表
    trees: HashMap<u16, Vec<SkillTreeNode>>,
}

impl SkillTree {
    /// 创建空的技能树数据库
    pub fn new() -> Self {
        Self {
            trees: HashMap::new(),
        }
    }

    /// 从 YAML 文件加载技能树数据
    ///
    /// # 参数
    /// - `path`: YAML 文件路径
    ///
    /// # 返回
    /// 加载成功返回 SkillTree 实例，失败返回错误
    pub fn from_yaml(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let yaml_trees: HashMap<u16, Vec<SkillTreeNode>> = serde_yaml::from_str(&content)?;
        Ok(Self { trees: yaml_trees })
    }

    /// 使用硬编码的默认技能树（一转职业示例）
    ///
    /// 包含以下职业的技能树：
    /// - 初学者 (Job 0)
    /// - 剑士 (Job 1)
    /// - 法师 (Job 2)
    /// - 弓箭手 (Job 3)
    /// - 盗贼 (Job 4)
    /// - 服事 (Job 5)
    pub fn with_default_trees() -> Self {
        let mut trees = HashMap::new();

        // 初学者技能树（Job 0）
        trees.insert(
            0,
            vec![
                SkillTreeNode::new(1, 9), // 基础技能
                SkillTreeNode::new(2, 1), // 战斗呐喊
            ],
        );

        // 剑士技能树（Job 1）
        trees.insert(
            1,
            vec![
                SkillTreeNode::new(1, 9), // 基础技能
                SkillTreeNode::new(5, 10) // Bash（猛击）
                    .with_prerequisites(vec![(1, 1)]), // 需要基础技能 Lv1
                SkillTreeNode::new(6, 10) // Provoke（挑衅）
                    .with_prerequisites(vec![(1, 5)]), // 需要基础技能 Lv5
                SkillTreeNode::new(7, 10) // Magnum Break（狂击）
                    .with_prerequisites(vec![(5, 5)]), // 需要猛击 Lv5
                SkillTreeNode::new(8, 1) // Endure（霸体）
                    .with_prerequisites(vec![(6, 5)]), // 需要挑衅 Lv5
            ],
        );

        // 法师技能树（Job 2）
        trees.insert(
            2,
            vec![
                SkillTreeNode::new(1, 9), // 基础技能
                SkillTreeNode::new(25, 10) // Fire Ball（火球术）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(26, 10) // Fire Wall（火墙术）
                    .with_prerequisites(vec![(25, 5)]),
                SkillTreeNode::new(27, 10) // Cold Bolt（冰箭术）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(28, 10) // Lightning Bolt（雷击术）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(29, 10) // Increase AGI（加速术）
                    .with_prerequisites(vec![(25, 3), (27, 3)]), // 需要火球和冰箭各 Lv3
            ],
        );

        // 弓箭手技能树（Job 3）
        trees.insert(
            3,
            vec![
                SkillTreeNode::new(1, 9), // 基础技能
                SkillTreeNode::new(35, 10) // Double Strafe（二连矢）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(36, 10) // Arrow Shower（箭雨）
                    .with_prerequisites(vec![(35, 5)]),
                SkillTreeNode::new(37, 10) // Concentration（专注）
                    .with_prerequisites(vec![(1, 5)]),
            ],
        );

        // 盗贼技能树（Job 4）
        trees.insert(
            4,
            vec![
                SkillTreeNode::new(1, 9), // 基础技能
                SkillTreeNode::new(45, 10) // Double Attack（二连击）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(46, 10) // Steal（偷窃）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(47, 10) // Hiding（隐匿）
                    .with_prerequisites(vec![(46, 5)]),
                SkillTreeNode::new(48, 10) // Envenom（涂毒）
                    .with_prerequisites(vec![(45, 3)]),
            ],
        );

        // 服事技能树（Job 5）
        trees.insert(
            5,
            vec![
                SkillTreeNode::new(1, 9), // 基础技能
                SkillTreeNode::new(28, 10) // Heal（治愈术）
                    .with_prerequisites(vec![(1, 1)]),
                SkillTreeNode::new(29, 10) // Increase AGI（加速术）
                    .with_prerequisites(vec![(28, 3)]),
                SkillTreeNode::new(55, 10) // Angelus（天使之护）
                    .with_prerequisites(vec![(1, 5)]),
                SkillTreeNode::new(56, 10) // Blessing（祝福）
                    .with_prerequisites(vec![(28, 5)]),
            ],
        );

        Self { trees }
    }

    /// 获取指定职业的技能树节点列表
    pub fn get_tree(&self, job_id: u16) -> Option<&Vec<SkillTreeNode>> {
        self.trees.get(&job_id)
    }

    /// 获取指定职业的技能树节点列表（可变引用）
    pub fn get_tree_mut(&mut self, job_id: u16) -> Option<&mut Vec<SkillTreeNode>> {
        self.trees.get_mut(&job_id)
    }

    /// 添加技能树节点到指定职业
    pub fn add_node(&mut self, job_id: u16, node: SkillTreeNode) {
        self.trees.entry(job_id).or_default().push(node);
    }

    /// 检查是否可以学习指定技能
    ///
    /// # 检查条件
    /// 1. 职业匹配（required_job == 0 表示所有职业可学习）
    /// 2. 所有前置技能等级满足要求
    /// 3. 技能当前等级未超过最大等级
    ///
    /// # 参数
    /// - `job`: 玩家当前职业 ID
    /// - `skill_id`: 要学习的技能 ID
    /// - `current_skills`: 玩家当前已学习的技能 {(skill_id, current_level)}
    ///
    /// # 返回
    /// `true` 表示可以学习，`false` 表示不满足条件
    pub fn can_learn_skill(
        &self,
        job: u16,
        skill_id: u16,
        current_skills: &HashMap<u16, u8>,
    ) -> bool {
        // 查找技能节点
        let node = match self.find_node(job, skill_id) {
            Some(node) => node,
            None => return false, // 技能不存在或不属于该职业
        };

        // 检查职业要求
        if node.required_job != 0 && node.required_job != job {
            return false;
        }

        // 检查当前技能等级是否已达到最大等级
        let current_level = current_skills.get(&skill_id).copied().unwrap_or(0);
        if current_level >= node.max_level {
            return false; // 已达到最大等级
        }

        // 检查所有前置技能是否满足要求
        for &(prereq_id, required_level) in &node.prerequisite_skills {
            let prereq_level = current_skills.get(&prereq_id).copied().unwrap_or(0);
            if prereq_level < required_level {
                return false; // 前置技能等级不足
            }
        }

        true
    }

    /// 获取指定技能的最大等级
    pub fn get_max_level(&self, job: u16, skill_id: u16) -> Option<u8> {
        self.find_node(job, skill_id).map(|node| node.max_level)
    }

    /// 获取指定技能的前置技能要求
    pub fn get_prerequisites(&self, job: u16, skill_id: u16) -> Option<&Vec<(u16, u8)>> {
        self.find_node(job, skill_id)
            .map(|node| &node.prerequisite_skills)
    }

    /// 在指定职业的技能树中查找技能节点
    fn find_node(&self, job: u16, skill_id: u16) -> Option<&SkillTreeNode> {
        // 先在指定职业的技能树中查找
        if let Some(tree) = self.trees.get(&job)
            && let Some(node) = tree.iter().find(|n| n.skill_id == skill_id)
        {
            return Some(node);
        }

        // 如果指定职业没有该技能，在通用技能树（job=0）中查找
        if job != 0
            && let Some(tree) = self.trees.get(&0)
            && let Some(node) = tree.iter().find(|n| n.skill_id == skill_id)
        {
            // 只有 required_job == 0 的技能才对所有职业开放
            if node.required_job == 0 {
                return Some(node);
            }
        }

        None
    }

    /// 获取所有职业 ID 列表
    pub fn get_job_ids(&self) -> Vec<u16> {
        self.trees.keys().copied().collect()
    }

    /// 获取指定职业的所有技能节点
    pub fn get_skills_for_job(&self, job: u16) -> Vec<&SkillTreeNode> {
        let mut skills = Vec::new();

        // 添加职业技能
        if let Some(tree) = self.trees.get(&job) {
            skills.extend(tree.iter());
        }

        // 添加通用技能（job=0 且 required_job=0）
        if job != 0
            && let Some(tree) = self.trees.get(&0)
        {
            skills.extend(tree.iter().filter(|n| n.required_job == 0));
        }

        skills
    }
}

impl Default for SkillTree {
    fn default() -> Self {
        Self::new()
    }
}

/// YAML 文件格式的技能树数据结构
///
/// 用于从 YAML 文件反序列化技能树数据。
#[derive(Debug, Deserialize)]
struct SkillTreeYaml {
    #[serde(rename = "JobId")]
    job_id: u16,
    #[serde(rename = "Skills")]
    skills: Vec<SkillTreeNodeYaml>,
}

#[derive(Debug, Deserialize)]
struct SkillTreeNodeYaml {
    #[serde(rename = "SkillId")]
    skill_id: u16,
    #[serde(rename = "MaxLevel")]
    max_level: u8,
    #[serde(rename = "Prerequisites", default)]
    prerequisites: Vec<PrerequisiteYaml>,
}

#[derive(Debug, Deserialize)]
struct PrerequisiteYaml {
    #[serde(rename = "SkillId")]
    skill_id: u16,
    #[serde(rename = "Level")]
    level: u8,
}

/// 从 rAthena 风格的 skill_tree.yml 加载技能树
///
/// YAML 格式示例：
/// ```yaml
/// - JobId: 1
///   Skills:
///     - SkillId: 5
///       MaxLevel: 10
///       Prerequisites:
///         - SkillId: 1
///           Level: 1
/// ```
pub fn load_skill_tree_from_yaml(path: &str) -> Result<SkillTree, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let yaml_data: Vec<SkillTreeYaml> = serde_yaml::from_str(&content)?;

    let mut skill_tree = SkillTree::new();

    for job_data in yaml_data {
        for skill_data in job_data.skills {
            let prerequisites: Vec<(u16, u8)> = skill_data
                .prerequisites
                .iter()
                .map(|p| (p.skill_id, p.level))
                .collect();

            let node = SkillTreeNode {
                skill_id: skill_data.skill_id,
                max_level: skill_data.max_level,
                prerequisite_skills: prerequisites,
                required_job: job_data.job_id,
            };

            skill_tree.add_node(job_data.job_id, node);
        }
    }

    Ok(skill_tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的技能树
    fn create_test_skill_tree() -> SkillTree {
        let mut tree = SkillTree::new();

        // 基础技能（所有职业可用）
        tree.add_node(0, SkillTreeNode::new(1, 9));

        // 剑士技能
        tree.add_node(
            1,
            SkillTreeNode::new(5, 10).with_prerequisites(vec![(1, 1)]),
        );
        tree.add_node(
            1,
            SkillTreeNode::new(6, 10).with_prerequisites(vec![(1, 5)]),
        );
        tree.add_node(
            1,
            SkillTreeNode::new(7, 10).with_prerequisites(vec![(5, 5)]),
        );

        // 法师技能
        tree.add_node(
            2,
            SkillTreeNode::new(25, 10).with_prerequisites(vec![(1, 1)]),
        );
        tree.add_node(
            2,
            SkillTreeNode::new(26, 10).with_prerequisites(vec![(25, 5)]),
        );

        tree
    }

    #[test]
    fn test_can_learn_basic_skill() {
        let tree = create_test_skill_tree();
        let skills = HashMap::new(); // 没有学习任何技能

        // 基础技能没有前置要求，应该可以学习
        assert!(tree.can_learn_skill(0, 1, &skills));
        assert!(tree.can_learn_skill(1, 1, &skills));
        assert!(tree.can_learn_skill(2, 1, &skills));
    }

    #[test]
    fn test_cannot_learn_without_prerequisite() {
        let tree = create_test_skill_tree();
        let skills = HashMap::new(); // 没有学习任何技能

        // 技能 5 需要基础技能 Lv1，没有前置技能时不能学习
        assert!(!tree.can_learn_skill(1, 5, &skills));
    }

    #[test]
    fn test_can_learn_with_prerequisite() {
        let tree = create_test_skill_tree();
        let mut skills = HashMap::new();
        skills.insert(1, 1); // 基础技能 Lv1

        // 现在可以学习技能 5 了
        assert!(tree.can_learn_skill(1, 5, &skills));
    }

    #[test]
    fn test_cannot_learn_with_insufficient_prerequisite() {
        let tree = create_test_skill_tree();
        let mut skills = HashMap::new();
        skills.insert(1, 4); // 基础技能 Lv4（不足 Lv5）

        // 技能 6 需要基础技能 Lv5，当前只有 Lv4
        assert!(!tree.can_learn_skill(1, 6, &skills));
    }

    #[test]
    fn test_can_learn_with_exact_prerequisite() {
        let tree = create_test_skill_tree();
        let mut skills = HashMap::new();
        skills.insert(1, 5); // 基础技能 Lv5（刚好满足）

        // 现在可以学习技能 6 了
        assert!(tree.can_learn_skill(1, 6, &skills));
    }

    #[test]
    fn test_cannot_exceed_max_level() {
        let tree = create_test_skill_tree();
        let mut skills = HashMap::new();
        skills.insert(1, 9); // 基础技能已达最大等级

        // 已达到最大等级，不能再学习
        assert!(!tree.can_learn_skill(0, 1, &skills));
    }

    #[test]
    fn test_cannot_learn_skill_from_wrong_job() {
        let tree = create_test_skill_tree();
        let mut skills = HashMap::new();
        skills.insert(1, 1);

        // 技能 5 是剑士技能（Job 1），法师（Job 2）不能学习
        assert!(!tree.can_learn_skill(2, 5, &skills));
    }

    #[test]
    fn test_chain_prerequisites() {
        let tree = create_test_skill_tree();
        let mut skills = HashMap::new();
        skills.insert(1, 1); // 基础技能 Lv1

        // 可以学习技能 5
        assert!(tree.can_learn_skill(1, 5, &skills));

        // 学习技能 5 并升级到 Lv5
        skills.insert(5, 5);

        // 现在可以学习技能 7（需要技能 5 Lv5）
        assert!(tree.can_learn_skill(1, 7, &skills));
    }

    #[test]
    fn test_get_max_level() {
        let tree = create_test_skill_tree();

        assert_eq!(tree.get_max_level(0, 1), Some(9));
        assert_eq!(tree.get_max_level(1, 5), Some(10));
        assert_eq!(tree.get_max_level(1, 999), None); // 不存在的技能
    }

    #[test]
    fn test_get_prerequisites() {
        let tree = create_test_skill_tree();

        let prereqs = tree.get_prerequisites(1, 5).unwrap();
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0], (1, 1));

        // 基础技能没有前置要求
        let prereqs = tree.get_prerequisites(0, 1).unwrap();
        assert_eq!(prereqs.len(), 0);
    }

    #[test]
    fn test_get_skills_for_job() {
        let tree = create_test_skill_tree();

        // 剑士应该有 4 个技能：1（通用）、5、6、7
        let skills = tree.get_skills_for_job(1);
        assert_eq!(skills.len(), 4);

        // 初学者只有 1 个技能
        let skills = tree.get_skills_for_job(0);
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn test_default_skill_tree() {
        let tree = SkillTree::with_default_trees();

        // 验证默认技能树包含所有职业
        assert!(tree.get_tree(0).is_some()); // 初学者
        assert!(tree.get_tree(1).is_some()); // 剑士
        assert!(tree.get_tree(2).is_some()); // 法师
        assert!(tree.get_tree(3).is_some()); // 弓箭手
        assert!(tree.get_tree(4).is_some()); // 盗贼
        assert!(tree.get_tree(5).is_some()); // 服事

        // 验证剑士可以学习猛击（需要基础技能 Lv1）
        let mut skills = HashMap::new();
        skills.insert(1, 1);
        assert!(tree.can_learn_skill(1, 5, &skills));

        // 验证法师不能学习猛击
        assert!(!tree.can_learn_skill(2, 5, &skills));
    }

    #[test]
    fn test_skill_tree_node_builder() {
        let node = SkillTreeNode::new(100, 5)
            .with_prerequisites(vec![(1, 3), (2, 2)])
            .with_job(1);

        assert_eq!(node.skill_id, 100);
        assert_eq!(node.max_level, 5);
        assert_eq!(node.prerequisite_skills.len(), 2);
        assert_eq!(node.required_job, 1);
    }

    #[test]
    fn test_load_skill_tree_from_yaml() {
        let yaml_str = r#"
- JobId: 1
  Skills:
    - SkillId: 5
      MaxLevel: 10
      Prerequisites:
        - SkillId: 1
          Level: 1
    - SkillId: 6
      MaxLevel: 10
      Prerequisites:
        - SkillId: 1
          Level: 5
- JobId: 2
  Skills:
    - SkillId: 25
      MaxLevel: 10
      Prerequisites:
        - SkillId: 1
          Level: 1
"#;

        let tmp_path = "/tmp/test_skill_tree.yml";
        std::fs::write(tmp_path, yaml_str).unwrap();

        let tree = load_skill_tree_from_yaml(tmp_path).unwrap();

        // 验证加载的数据
        assert!(tree.get_tree(1).is_some());
        assert!(tree.get_tree(2).is_some());

        let swordman_skills = tree.get_tree(1).unwrap();
        assert_eq!(swordman_skills.len(), 2);

        let mage_skills = tree.get_tree(2).unwrap();
        assert_eq!(mage_skills.len(), 1);

        // 验证前置技能
        let mut skills = HashMap::new();
        skills.insert(1, 1);
        assert!(tree.can_learn_skill(1, 5, &skills));
        assert!(!tree.can_learn_skill(1, 6, &skills)); // 需要 Lv5

        skills.insert(1, 5);
        assert!(tree.can_learn_skill(1, 6, &skills));

        std::fs::remove_file(tmp_path).ok();
    }

    #[test]
    fn test_nonexistent_skill() {
        let tree = create_test_skill_tree();
        let skills = HashMap::new();

        // 不存在的技能应该返回 false
        assert!(!tree.can_learn_skill(1, 999, &skills));
    }

    #[test]
    fn test_job_ids() {
        let tree = create_test_skill_tree();
        let mut job_ids = tree.get_job_ids();
        job_ids.sort();

        assert_eq!(job_ids, vec![0, 1, 2]);
    }
}
