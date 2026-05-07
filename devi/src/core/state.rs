// 游戏状态机模块
// 控制客户端在不同阶段（登录、选角色、游戏中）的行为切换

use bevy::prelude::States;

/// 游戏状态枚举
///
/// 使用 Bevy 的 States trait 实现状态机，
/// 用于控制客户端在不同游戏阶段的系统激活与界面切换。
///
/// 状态流转: Login -> CharSelect -> InGame
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
pub enum GameState {
    /// 登录界面 - 客户端启动后的默认状态
    #[default]
    Login,
    /// 选角色界面 - 登录成功后进入角色选择
    CharSelect,
    /// 游戏中 - 角色选择完成后进入游戏世界
    InGame,
}
