//! 登录系统
//!
//! 管理登录阶段的完整流程：
//! 1. 进入 Login 状态时初始化网络连接
//! 2. 发送登录请求（用户名 + 密码）
//! 3. 处理登录响应，成功则切换到 CharSelect 状态
//! 4. 登录 UI 构建（用户名/密码输入框、服务器选择、连接按钮）

use bevy::prelude::*;
use crate::core::config::ClientConfig;
use crate::core::state::GameState;
use crate::net::session::{NetworkCommand, NetworkEvent, NetworkManager};
use crate::protocol::Packet;
use crate::protocol::login::LoginRequest;

// ============================================================================
// 组件定义
// ============================================================================

/// 登录 UI 根节点标记组件
#[derive(Component)]
pub struct LoginUi;

/// 用户名输入框标记组件
#[derive(Component)]
pub struct UsernameInput;

/// 密码输入框标记组件
#[derive(Component)]
pub struct PasswordInput;

/// 登录按钮标记组件
#[derive(Component)]
pub struct LoginButton;

/// 登录状态信息文本组件
#[derive(Component)]
pub struct LoginStatusText;

/// 登录状态
#[derive(Resource, Default)]
pub struct LoginState {
    /// 是否正在连接中
    pub connecting: bool,
    /// 是否已发送登录请求
    pub login_sent: bool,
    /// 错误信息
    pub error_message: Option<String>,
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 服务器端口（登录服务器默认 6900）
    pub server_port: u16,
    /// 登录验证 ID1（来自 LoginResponse，用于后续地图服务器认证）
    pub login_id1: u32,
    /// 登录验证 ID2（来自 LoginResponse，用于后续地图服务器认证）
    pub login_id2: u32,
    /// 性别（来自 LoginResponse）
    pub sex: u8,
}

// ============================================================================
// 系统函数
// ============================================================================

/// 进入登录状态时的初始化系统
pub fn setup_login(
    mut commands: Commands,
    config: Res<ClientConfig>,
) {
    tracing::info!("进入登录状态，连接服务器: {}", config.server_address);

    let network = NetworkManager::new(&config.protocol);
    commands.insert_resource(network);
    commands.insert_resource(LoginState::default());

    build_login_ui(&mut commands, &config);
}

/// 离开登录状态时的清理系统
pub fn cleanup_login(mut commands: Commands, query: Query<Entity, With<LoginUi>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

/// 登录网络事件处理系统
pub fn login_network_handler(
    mut state: ResMut<LoginState>,
    network: Res<NetworkManager>,
    mut next_state: ResMut<NextState<GameState>>,
    mut status_query: Query<&mut Text, With<LoginStatusText>>,
) {
    let events = network.poll_events();
    for event in events {
        match event {
            NetworkEvent::Connected => {
                tracing::info!("已连接到登录服务器，发送登录请求");
                state.connecting = false;
                state.login_sent = true;

                let req = LoginRequest {
                    version: 20,
                    username: state.username.clone(),
                    password: state.password.clone(),
                };
                network.send_command(NetworkCommand::Send(Packet::LoginRequest(req)));

                for mut text in status_query.iter_mut() {
                    **text = "已连接，等待响应...".to_string();
                }
            }
            NetworkEvent::PacketReceived(packet) => {
                if let Packet::LoginResponse(resp) = packet {
                    tracing::info!("收到登录响应: account_id={}", resp.account_id);
                    // 保存登录验证信息，用于后续地图服务器认证
                    state.login_id1 = resp.login_id1;
                    state.login_id2 = resp.login_id2;
                    state.sex = resp.sex;
                    next_state.set(GameState::CharSelect);
                }
            }
            NetworkEvent::ConnectFailed(err) => {
                tracing::error!("连接登录服务器失败: {}", err);
                state.connecting = false;
                state.error_message = Some(format!("连接失败: {}", err));
                for mut text in status_query.iter_mut() {
                    **text = format!("连接失败: {}", err);
                }
            }
            NetworkEvent::RecvError(err) => {
                tracing::error!("接收错误: {}", err);
                state.error_message = Some(err);
            }
            NetworkEvent::Disconnected => {
                tracing::warn!("与登录服务器断开连接");
                state.connecting = false;
                if state.login_sent {
                    state.error_message = Some("连接已断开".to_string());
                }
            }
        }
    }
}

/// 登录按钮点击处理系统
pub fn handle_login_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<LoginButton>),
    >,
    username_query: Query<&Text, With<UsernameInput>>,
    password_query: Query<&Text, With<PasswordInput>>,
    mut login_state: ResMut<LoginState>,
    network: Res<NetworkManager>,
    config: Res<ClientConfig>,
    mut status_query: Query<&mut Text, With<LoginStatusText>>,
) {
    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.15, 0.15, 0.5));

                let username = username_query.iter().next().map(|t| t.to_string()).unwrap_or_default();
                let password = password_query.iter().next().map(|t| t.to_string()).unwrap_or_default();

                if username.is_empty() || password.is_empty() {
                    for mut text in status_query.iter_mut() {
                        **text = "请输入用户名和密码".to_string();
                    }
                    return;
                }

                login_state.username = username;
                login_state.password = password;
                login_state.connecting = true;
                login_state.error_message = None;

                tracing::info!("正在连接到 {}:6900", config.server_address);
                network.send_command(NetworkCommand::Connect {
                    address: config.server_address.clone(),
                    port: login_state.server_port,
                });
                for mut text in status_query.iter_mut() {
                    **text = "正在连接服务器...".to_string();
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.25, 0.25, 0.6));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.45));
            }
        }
    }
}

// ============================================================================
// UI 构建
// ============================================================================

/// 构建登录界面 UI
fn build_login_ui(commands: &mut Commands, config: &ClientConfig) {
    commands
        .spawn((
            LoginUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn((
                Text::new("Devi - Ragnarok Online"),
                TextFont { font_size: 36.0, ..default() },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                Node { margin: UiRect::bottom(Val::Px(30.0)), ..default() },
            ));

            // 服务器信息
            parent.spawn((
                Text::new(format!("服务器: {}:{}", config.server_address, 6900)),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
            ));

            // 用户名行
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new("用户名: "),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        UsernameInput,
                        Text::new(""),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        Node { min_width: Val::Px(200.0), ..default() },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                    ));
                });

            // 密码行
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new("密  码: "),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        PasswordInput,
                        Text::new(""),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        Node { min_width: Val::Px(200.0), ..default() },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                    ));
                });

            // 登录按钮
            parent
                .spawn((
                    LoginButton,
                    Button,
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::bottom(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.45)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("登 录"),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

            // 状态文本
            parent.spawn((
                LoginStatusText,
                Text::new("等待连接"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });
}
