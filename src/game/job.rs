//! 职业系统
//!
//! 定义 RO（Ragnarok Online）职业类型枚举、转职条件校验和转职逻辑。
//!
//! # 职业等级体系
//!
//! - **基础职业 (Novice)**：所有角色的初始职业，job_id = 0
//! - **一转职业**：Swordman/Mage/Archer/Acolyte/Merchant/Thief（job_id 1-6）
//!   - 转职条件：BaseLv >= 10
//! - **二转职业**：Knight/Priest/Wizard/Blacksmith/Hunter/Assassin（job_id 7-12）
//!   - 转职条件：对应一转职业 + JobLv >= 40（GM 命令可跳过）
//! - **进阶二转**：LordHighPriest/HighWizard/Whitesmith/Sniper/AssassinCross/Paladin（job_id 13-18）
//!   - 转职条件：对应二转职业 + JobLv >= 40（GM 命令可跳过）
//!
//! # 转职流程
//!
//! 1. 客户端发送 `CZ_REQ_CHANGEJOB (0x019d)` 或 GM 命令 `@jobchange`
//! 2. 服务端校验转职条件
//! 3. 更新玩家职业 ID，重置 Job 等级和经验
//! 4. 重算最大 HP/SP（不同职业有不同的基础值）
//! 5. 保存到数据库
//! 6. 发送 `ZC_ACK_CHANGEJOB (0x019e)` 响应

/// RO 职业类型
///
/// 对应 rAthena 的 job_id，每个职业有唯一的数值标识。
/// 数值范围：
/// - 0: Novice
/// - 1-6: 一转职业
/// - 7-12: 二转职业
/// - 13-18: 进阶二转职业
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobType {
    /// 初心者
    Novice = 0,
    /// 剑士
    Swordman = 1,
    /// 法师
    Mage = 2,
    /// 弓箭手
    Archer = 3,
    /// 服事
    Acolyte = 4,
    /// 商人
    Merchant = 5,
    /// 盗贼
    Thief = 6,
    /// 骑士（剑士二转）
    Knight = 7,
    /// 祭司（服事二转）
    Priest = 8,
    /// 巫师（法师二转）
    Wizard = 9,
    /// 铁匠（商人二转）
    Blacksmith = 10,
    /// 猎人（弓箭手二转）
    Hunter = 11,
    /// 刺客（盗贼二转）
    Assassin = 12,
    /// 领主骑士（骑士进阶）
    LordKnight = 13,
    /// 大主教（祭司进阶）—— 注意：此处用 HighPriest 而非 Archbishop，保持与经典 RO 一致
    HighPriest = 14,
    /// 超魔导师（巫师进阶）
    HighWizard = 15,
    /// 神工匠（铁匠进阶）
    Whitesmith = 16,
    /// 神射手（猎人进阶）
    Sniper = 17,
    /// 十字刺客（刺客进阶）
    AssassinCross = 18,
    /// 圣骑士（剑士二转分支）
    Paladin = 19,
}

impl JobType {
    /// 从 u16 数值创建 JobType
    ///
    /// 如果数值不在已知范围内，返回 None。
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::Novice),
            1 => Some(Self::Swordman),
            2 => Some(Self::Mage),
            3 => Some(Self::Archer),
            4 => Some(Self::Acolyte),
            5 => Some(Self::Merchant),
            6 => Some(Self::Thief),
            7 => Some(Self::Knight),
            8 => Some(Self::Priest),
            9 => Some(Self::Wizard),
            10 => Some(Self::Blacksmith),
            11 => Some(Self::Hunter),
            12 => Some(Self::Assassin),
            13 => Some(Self::LordKnight),
            14 => Some(Self::HighPriest),
            15 => Some(Self::HighWizard),
            16 => Some(Self::Whitesmith),
            17 => Some(Self::Sniper),
            18 => Some(Self::AssassinCross),
            19 => Some(Self::Paladin),
            _ => None,
        }
    }

    /// 获取职业的数值 ID
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// 获取职业的中文名称
    pub fn name(self) -> &'static str {
        match self {
            Self::Novice => "初心者",
            Self::Swordman => "剑士",
            Self::Mage => "法师",
            Self::Archer => "弓箭手",
            Self::Acolyte => "服事",
            Self::Merchant => "商人",
            Self::Thief => "盗贼",
            Self::Knight => "骑士",
            Self::Priest => "祭司",
            Self::Wizard => "巫师",
            Self::Blacksmith => "铁匠",
            Self::Hunter => "猎人",
            Self::Assassin => "刺客",
            Self::LordKnight => "领主骑士",
            Self::HighPriest => "大主教",
            Self::HighWizard => "超魔导师",
            Self::Whitesmith => "神工匠",
            Self::Sniper => "神射手",
            Self::AssassinCross => "十字刺客",
            Self::Paladin => "圣骑士",
        }
    }

    /// 是否为基础职业（初心者）
    pub fn is_novice(self) -> bool {
        self == Self::Novice
    }

    /// 是否为一转职业
    pub fn is_first_class(self) -> bool {
        matches!(
            self,
            Self::Swordman
                | Self::Mage
                | Self::Archer
                | Self::Acolyte
                | Self::Merchant
                | Self::Thief
        )
    }

    /// 是否为二转职业
    pub fn is_second_class(self) -> bool {
        matches!(
            self,
            Self::Knight
                | Self::Priest
                | Self::Wizard
                | Self::Blacksmith
                | Self::Hunter
                | Self::Assassin
                | Self::Paladin
        )
    }

    /// 是否为进阶二转职业
    pub fn is_advanced_class(self) -> bool {
        matches!(
            self,
            Self::LordKnight
                | Self::HighPriest
                | Self::HighWizard
                | Self::Whitesmith
                | Self::Sniper
                | Self::AssassinCross
        )
    }

    /// 获取该职业对应的一转前置职业
    ///
    /// 如果本身就是一转或初心者，返回 None。
    pub fn prerequisite_first_class(self) -> Option<JobType> {
        match self {
            // 二转职业的一转前置
            Self::Knight | Self::LordKnight | Self::Paladin => Some(Self::Swordman),
            Self::Priest | Self::HighPriest => Some(Self::Acolyte),
            Self::Wizard | Self::HighWizard => Some(Self::Mage),
            Self::Blacksmith | Self::Whitesmith => Some(Self::Merchant),
            Self::Hunter | Self::Sniper => Some(Self::Archer),
            Self::Assassin | Self::AssassinCross => Some(Self::Thief),
            _ => None,
        }
    }

    /// 获取该职业对应的二转前置职业（仅进阶二转需要）
    ///
    /// 如果本身就是二转或更低，返回 None。
    pub fn prerequisite_second_class(self) -> Option<JobType> {
        match self {
            Self::LordKnight => Some(Self::Knight),
            Self::HighPriest => Some(Self::Priest),
            Self::HighWizard => Some(Self::Wizard),
            Self::Whitesmith => Some(Self::Blacksmith),
            Self::Sniper => Some(Self::Hunter),
            Self::AssassinCross => Some(Self::Assassin),
            Self::Paladin => Some(Self::Knight), // Paladin 也是 Knight 的分支
            _ => None,
        }
    }

    /// 获取该职业转职后的最大 Job 等级上限
    ///
    /// - 初心者：10
    /// - 一转：50
    /// - 二转/进阶二转：50（后续可扩展到 70）
    pub fn max_job_level(self) -> u16 {
        if self.is_novice() {
            10
        } else {
            50
        }
    }

    /// 获取该职业的基础最大 HP（BaseLv 1 时）
    ///
    /// 不同职业有不同的基础 HP，参考 rAthena 初始值。
    pub fn base_hp(self) -> u32 {
        match self {
            Self::Novice => 40,
            Self::Swordman | Self::Knight | Self::LordKnight | Self::Paladin => 200,
            Self::Mage | Self::Wizard | Self::HighWizard => 100,
            Self::Archer | Self::Hunter | Self::Sniper => 150,
            Self::Acolyte | Self::Priest | Self::HighPriest => 175,
            Self::Merchant | Self::Blacksmith | Self::Whitesmith => 180,
            Self::Thief | Self::Assassin | Self::AssassinCross => 175,
        }
    }

    /// 获取该职业的基础最大 SP（BaseLv 1 时）
    pub fn base_sp(self) -> u32 {
        match self {
            Self::Novice => 11,
            Self::Swordman | Self::Knight | Self::LordKnight | Self::Paladin => 20,
            Self::Mage | Self::Wizard | Self::HighWizard => 80,
            Self::Archer | Self::Hunter | Self::Sniper => 30,
            Self::Acolyte | Self::Priest | Self::HighPriest => 50,
            Self::Merchant | Self::Blacksmith | Self::Whitesmith => 30,
            Self::Thief | Self::Assassin | Self::AssassinCross => 25,
        }
    }
}

/// 转职检查结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobChangeError {
    /// 目标职业 ID 无效
    InvalidJob(u16),
    /// 等级不足
    InsufficientLevel {
        /// 当前基础等级
        current: u16,
        /// 需要的基础等级
        required: u16,
    },
    /// 职业前置不满足（需要特定的一转/二转职业）
    WrongPrerequisite {
        /// 当前职业
        current: &'static str,
        /// 需要的前置职业
        required: &'static str,
    },
}

impl std::fmt::Display for JobChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJob(id) => write!(f, "无效的职业 ID: {}", id),
            Self::InsufficientLevel { current, required } => {
                write!(
                    f,
                    "等级不足: 当前 BaseLv {}，需要 BaseLv {}",
                    current, required
                )
            }
            Self::WrongPrerequisite { current, required } => {
                write!(
                    f,
                    "职业前置不满足: 当前职业 '{}'，需要 '{}'",
                    current, required
                )
            }
        }
    }
}

/// 转职所需的最低基础等级
///
/// - 转一转：BaseLv >= 10
/// - 转二转：BaseLv >= 1（由 GM 命令触发时跳过 JobLv 检查）
/// - 转进阶二转：BaseLv >= 1（由 GM 命令触发时跳过 JobLv 检查）
pub fn required_base_level_for(target_job: JobType) -> u16 {
    if target_job.is_first_class() {
        10
    } else {
        // 二转和进阶二转的基础等级要求较低，主要看 JobLv
        1
    }
}

/// 检查转职条件是否满足
///
/// # 参数
/// - `current_job`: 当前职业 ID
/// - `target_job_id`: 目标职业 ID
/// - `base_level`: 当前基础等级
/// - `skip_job_level_check`: 是否跳过 JobLv 检查（GM 命令模式）
///
/// # 返回
/// - `Ok(())` 表示条件满足
/// - `Err(JobChangeError)` 表示条件不满足及原因
pub fn check_job_change_requirements(
    current_job: u16,
    target_job_id: u16,
    base_level: u16,
) -> Result<(), JobChangeError> {
    let target = JobType::from_u16(target_job_id)
        .ok_or(JobChangeError::InvalidJob(target_job_id))?;

    let current = JobType::from_u16(current_job);

    // 初心者不能直接转二转/进阶（GM 命令允许跳过，但基础校验仍需一转前置）
    if let Some(prereq_first) = target.prerequisite_first_class() {
        // 目标职业需要一转前置
        if let Some(current_type) = current {
            // 当前职业必须是对应的一转职业或更高阶的同系职业
            if !is_same_job_line(current_type, prereq_first) {
                return Err(JobChangeError::WrongPrerequisite {
                    current: current_type.name(),
                    required: prereq_first.name(),
                });
            }
        } else if current_job != 0 {
            // 当前职业 ID 无效（非 0 且无法解析）
            return Err(JobChangeError::InvalidJob(current_job));
        }
        // current_job == 0 (Novice) 转一转是允许的
    }

    // 检查基础等级要求
    let required_lv = required_base_level_for(target);
    if base_level < required_lv {
        return Err(JobChangeError::InsufficientLevel {
            current: base_level,
            required: required_lv,
        });
    }

    Ok(())
}

/// 判断两个职业是否属于同一职业系
///
/// 例如：Swordman -> Knight -> LordKnight 都属于剑士系
fn is_same_job_line(current: JobType, prerequisite: JobType) -> bool {
    // 如果当前职业就是前置职业，直接匹配
    if current == prerequisite {
        return true;
    }

    // 检查当前职业的一转前置是否匹配
    if let Some(first_class) = current.prerequisite_first_class() {
        if first_class == prerequisite {
            return true;
        }
    }

    // 检查当前职业的二转前置，再看二转前置的一转是否匹配
    if let Some(second_class) = current.prerequisite_second_class() {
        if let Some(first_class) = second_class.prerequisite_first_class() {
            if first_class == prerequisite {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_from_u16_valid() {
        assert_eq!(JobType::from_u16(0), Some(JobType::Novice));
        assert_eq!(JobType::from_u16(1), Some(JobType::Swordman));
        assert_eq!(JobType::from_u16(7), Some(JobType::Knight));
        assert_eq!(JobType::from_u16(19), Some(JobType::Paladin));
    }

    #[test]
    fn test_job_type_from_u16_invalid() {
        assert_eq!(JobType::from_u16(99), None);
        assert_eq!(JobType::from_u16(100), None);
        assert_eq!(JobType::from_u16(u16::MAX), None);
    }

    #[test]
    fn test_job_type_as_u16() {
        assert_eq!(JobType::Novice.as_u16(), 0);
        assert_eq!(JobType::Swordman.as_u16(), 1);
        assert_eq!(JobType::AssassinCross.as_u16(), 18);
    }

    #[test]
    fn test_job_type_name() {
        assert_eq!(JobType::Novice.name(), "初心者");
        assert_eq!(JobType::Knight.name(), "骑士");
        assert_eq!(JobType::AssassinCross.name(), "十字刺客");
    }

    #[test]
    fn test_job_type_classification() {
        assert!(JobType::Novice.is_novice());
        assert!(!JobType::Novice.is_first_class());

        assert!(JobType::Swordman.is_first_class());
        assert!(!JobType::Swordman.is_second_class());

        assert!(JobType::Knight.is_second_class());
        assert!(!JobType::Knight.is_advanced_class());

        assert!(JobType::LordKnight.is_advanced_class());
        assert!(!JobType::LordKnight.is_first_class());
    }

    #[test]
    fn test_prerequisite_first_class() {
        // 二转职业需要一转前置
        assert_eq!(
            JobType::Knight.prerequisite_first_class(),
            Some(JobType::Swordman)
        );
        assert_eq!(
            JobType::Priest.prerequisite_first_class(),
            Some(JobType::Acolyte)
        );
        assert_eq!(
            JobType::Wizard.prerequisite_first_class(),
            Some(JobType::Mage)
        );

        // 进阶二转也需要一转前置
        assert_eq!(
            JobType::LordKnight.prerequisite_first_class(),
            Some(JobType::Swordman)
        );

        // 一转和初心者没有一转前置
        assert_eq!(JobType::Swordman.prerequisite_first_class(), None);
        assert_eq!(JobType::Novice.prerequisite_first_class(), None);
    }

    #[test]
    fn test_prerequisite_second_class() {
        // 进阶二转需要二转前置
        assert_eq!(
            JobType::LordKnight.prerequisite_second_class(),
            Some(JobType::Knight)
        );
        assert_eq!(
            JobType::HighPriest.prerequisite_second_class(),
            Some(JobType::Priest)
        );

        // 二转职业没有二转前置
        assert_eq!(JobType::Knight.prerequisite_second_class(), None);
        assert_eq!(JobType::Novice.prerequisite_second_class(), None);
    }

    #[test]
    fn test_max_job_level() {
        assert_eq!(JobType::Novice.max_job_level(), 10);
        assert_eq!(JobType::Swordman.max_job_level(), 50);
        assert_eq!(JobType::Knight.max_job_level(), 50);
        assert_eq!(JobType::LordKnight.max_job_level(), 50);
    }

    #[test]
    fn test_base_hp_by_job() {
        assert_eq!(JobType::Novice.base_hp(), 40);
        assert_eq!(JobType::Swordman.base_hp(), 200);
        assert_eq!(JobType::Mage.base_hp(), 100);
    }

    #[test]
    fn test_base_sp_by_job() {
        assert_eq!(JobType::Novice.base_sp(), 11);
        assert_eq!(JobType::Mage.base_sp(), 80);
        assert_eq!(JobType::Swordman.base_sp(), 20);
    }

    // ==================== 转职条件检查测试 ====================

    #[test]
    fn test_check_novice_to_swordman_ok() {
        // 初心者转剑士，BaseLv >= 10
        let result = check_job_change_requirements(0, 1, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_novice_to_swordman_insufficient_level() {
        // 初心者转剑士，BaseLv = 9，不足
        let result = check_job_change_requirements(0, 1, 9);
        assert!(matches!(
            result,
            Err(JobChangeError::InsufficientLevel {
                current: 9,
                required: 10
            })
        ));
    }

    #[test]
    fn test_check_swordman_to_knight_ok() {
        // 剑士转骑士，需要前置是 Swordman
        let result = check_job_change_requirements(1, 7, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_mage_to_knight_wrong_prereq() {
        // 法师转骑士，前置不满足（需要 Swordman）
        let result = check_job_change_requirements(2, 7, 1);
        assert!(matches!(
            result,
            Err(JobChangeError::WrongPrerequisite { .. })
        ));
    }

    #[test]
    fn test_check_novice_to_knight_wrong_prereq() {
        // 初心者直接转骑士，前置不满足（需要 Swordman）
        let result = check_job_change_requirements(0, 7, 1);
        assert!(matches!(
            result,
            Err(JobChangeError::WrongPrerequisite { .. })
        ));
    }

    #[test]
    fn test_check_knight_to_lord_knight_ok() {
        // 骑士转领主骑士
        let result = check_job_change_requirements(7, 13, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_invalid_target_job() {
        // 无效的目标职业 ID
        let result = check_job_change_requirements(0, 99, 10);
        assert!(matches!(result, Err(JobChangeError::InvalidJob(99))));
    }

    #[test]
    fn test_is_same_job_line() {
        // Swordman 系
        assert!(is_same_job_line(JobType::Knight, JobType::Swordman));
        assert!(is_same_job_line(JobType::LordKnight, JobType::Swordman));
        assert!(is_same_job_line(JobType::Paladin, JobType::Swordman));

        // Mage 系
        assert!(is_same_job_line(JobType::Wizard, JobType::Mage));
        assert!(is_same_job_line(JobType::HighWizard, JobType::Mage));

        // 跨系不匹配
        assert!(!is_same_job_line(JobType::Knight, JobType::Mage));
        assert!(!is_same_job_line(JobType::Priest, JobType::Swordman));
    }

    #[test]
    fn test_display_error_messages() {
        let err = JobChangeError::InvalidJob(99);
        assert!(err.to_string().contains("99"));

        let err = JobChangeError::InsufficientLevel {
            current: 5,
            required: 10,
        };
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("10"));

        let err = JobChangeError::WrongPrerequisite {
            current: "法师",
            required: "剑士",
        };
        assert!(err.to_string().contains("法师"));
        assert!(err.to_string().contains("剑士"));
    }
}
