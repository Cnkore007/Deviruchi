// 玩家实体组件
// 定义玩家角色的属性和状态，对应 RO 中的角色数据

use bevy::prelude::*;

/// 玩家组件
/// 存储玩家角色的核心属性，包括等级、生命值、魔法值和移动速度
#[derive(Debug, Component)]
pub struct Player {
    /// 服务器分配的实体 ID
    pub entity_id: u32,
    /// 角色名称
    pub name: String,
    /// 基础等级（Base Level）
    pub base_level: u32,
    /// 职业等级（Job Level）
    pub job_level: u32,
    /// 当前生命值
    pub hp: u32,
    /// 最大生命值
    pub max_hp: u32,
    /// 当前魔法值
    pub sp: u32,
    /// 最大魔法值
    pub max_sp: u32,
    /// 移动速度（对应 RO 的 speed 值，值越小移动越快）
    pub speed: u16,
}

impl Player {
    /// 创建新玩家实例
    /// 使用 RO 新角色的默认属性值
    pub fn new(entity_id: u32, name: String) -> Self {
        Self {
            entity_id,
            name,
            base_level: 1,
            job_level: 1,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            speed: 150,
        }
    }
}
