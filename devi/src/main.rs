// Devi 客户端主程序
// 集成渲染管线（相机、精灵动画）、游戏逻辑（移动系统）和 UI（HUD、聊天窗口）

use bevy::prelude::*;
use devi::core::config::ClientConfig;
use devi::core::state::GameState;
use devi::game::movement::movement_system;
use devi::render::camera::{camera_system, CameraConfig, RoCamera};
use devi::render::sprite::{billboard_system, sprite_animation_system};
use devi::render::ui::chat::build_chat_window;
use devi::render::ui::hud::{build_hud, HudLayout};

fn main() {
    let config = ClientConfig::default();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Devi - Ragnarok Online".to_string(),
                resolution: (config.window_width as f32, config.window_height as f32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .insert_resource(config)
        .insert_resource(RoCamera::new(CameraConfig::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_system,
                sprite_animation_system,
                billboard_system,
                movement_system,
                log_state_changes,
            )
                .run_if(in_state(GameState::InGame)),
        )
        .run();
}

/// 初始化场景
/// 创建 3D 相机、方向光和 UI 界面（HUD + 聊天窗口）
fn setup(mut commands: Commands) {
    // 3D 相机
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // 方向光，模拟太阳光
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -45.0_f32.to_radians(),
            45.0_f32.to_radians(),
            0.0,
        )),
    ));
    // 构建 HUD 和聊天窗口
    let layout = HudLayout::default();
    build_hud(&mut commands, &layout);
    build_chat_window(&mut commands);
}

/// 状态变化日志系统
/// 当游戏状态发生变化时输出日志
fn log_state_changes(state: Res<State<GameState>>) {
    if state.is_changed() {
        tracing::info!("游戏状态切换: {:?}", state.get());
    }
}
