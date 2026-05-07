// 怪物实体组件
// 定义怪物（Mob）的属性和 AI 状态

use bevy::prelude::*;

/// 怪物组件
/// 存储怪物的核心属性，包括怪物 ID、生命值和 AI 状态
#[derive(Debug, Component)]
pub struct Mob {
    /// 服务器分配的实体 ID
    pub entity_id: u32,
    /// 怪物数据库 ID（对应 mob_db 中的编号）
    pub mob_id: u32,
    /// 怪物名称
    pub name: String,
    /// 当前生命值
    pub hp: u32,
    /// 最大生命值
    pub max_hp: u32,
    /// 移动速度
    pub speed: u16,
    /// 当前 AI 状态
    pub ai_state: MobAiState,
}

/// 怪物 AI 状态枚举
/// 对应 RO 中怪物的 AI 行为模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobAiState {
    /// 空闲状态，原地待机
    Idle,
    /// 追踪状态，追逐目标
    Chase,
    /// 攻击状态，对目标发起攻击
    Attack,
    /// 回归状态，超出范围后返回出生点
    Return,
}

impl Default for MobAiState {
    fn default() -> Self {
        Self::Idle
    }
}

impl Mob {
    /// 创建新怪物实例
    /// 默认生命值 100，速度 200，AI 状态为空闲
    pub fn new(entity_id: u32, mob_id: u32, name: String) -> Self {
        Self {
            entity_id,
            mob_id,
            name,
            hp: 100,
            max_hp: 100,
            speed: 200,
            ai_state: MobAiState::Idle,
        }
    }
}
