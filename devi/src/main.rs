// Devi 客户端主程序
// 集成登录流程、角色选择、渲染管线和游戏逻辑

use bevy::prelude::*;
use devi::core::config::ClientConfig;
use devi::core::state::GameState;
use devi::game::char_select::{
    char_select_network_handler, cleanup_char_select, handle_back_to_select_button,
    handle_char_card_click, handle_confirm_create_button, handle_create_char_button,
    handle_enter_game_button, handle_stat_decrease_button, handle_stat_increase_button,
    setup_char_select,
};
use devi::game::login::{
    cleanup_login, handle_login_button, login_network_handler, setup_login,
};
use devi::game::map_connection::{map_network_handler, setup_map_connection};
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
        // ===== Startup =====
        .add_systems(Startup, setup)
        // ===== Login 状态 =====
        .add_systems(OnEnter(GameState::Login), setup_login)
        .add_systems(OnExit(GameState::Login), cleanup_login)
        .add_systems(
            Update,
            (
                login_network_handler,
                handle_login_button,
            )
                .run_if(in_state(GameState::Login)),
        )
        // ===== CharSelect 状态 =====
        .add_systems(OnEnter(GameState::CharSelect), setup_char_select)
        .add_systems(OnExit(GameState::CharSelect), cleanup_char_select)
        .add_systems(
            Update,
            (
                char_select_network_handler,
                handle_char_card_click,
                handle_enter_game_button,
                handle_create_char_button,
                handle_stat_increase_button,
                handle_stat_decrease_button,
                handle_confirm_create_button,
                handle_back_to_select_button,
            )
                .run_if(in_state(GameState::CharSelect)),
        )
        // ===== InGame 状态 =====
        .add_systems(OnEnter(GameState::InGame), (setup_ingame, setup_map_connection))
        .add_systems(
            Update,
            (
                camera_system,
                sprite_animation_system,
                billboard_system,
                movement_system,
                map_network_handler,
            )
                .run_if(in_state(GameState::InGame)),
        )
        // ===== 全局系统 =====
        .add_systems(Update, log_state_changes)
        .run();
}

/// 初始化场景（跨状态共享的资源）
fn setup(mut commands: Commands) {
    // 3D 相机
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // 方向光
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
}

/// 进入游戏状态时的初始化
fn setup_ingame(mut commands: Commands) {
    tracing::info!("进入游戏世界");
    let layout = HudLayout::default();
    build_hud(&mut commands, &layout);
    build_chat_window(&mut commands);
}

/// 状态变化日志系统
fn log_state_changes(state: Res<State<GameState>>) {
    if state.is_changed() {
        tracing::info!("游戏状态切换: {:?}", state.get());
    }
}
