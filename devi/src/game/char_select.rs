//! 角色选择系统
//!
//! 管理角色选择阶段的完整流程：
//! 1. 进入 CharSelect 状态时请求角色列表
//! 2. 显示角色列表 UI
//! 3. 处理角色选择，进入游戏
//! 4. 连接地图服务器并发送进入地图请求

use bevy::prelude::*;
use crate::core::state::GameState;
use crate::game::login::LoginState;
use crate::net::session::{NetworkCommand, NetworkEvent, NetworkManager};
use crate::protocol::Packet;
use crate::protocol::char_mod::{CharInfo, CharEnterRequest, CharCreateRequest};

// ============================================================================
// 组件定义
// ============================================================================

/// 角色选择 UI 根节点标记组件
#[derive(Component)]
pub struct CharSelectUi;

/// 角色卡片标记组件（包含角色索引）
#[derive(Component)]
pub struct CharCard(pub usize);

/// 进入游戏按钮标记组件
#[derive(Component)]
pub struct EnterGameButton;

/// 创建角色按钮标记组件
#[derive(Component)]
pub struct CreateCharButton;

/// 角色选择状态文本组件
#[derive(Component)]
pub struct CharSelectStatusText;

// --- 角色创建 UI 组件 ---

/// 角色创建 UI 根节点标记组件
#[derive(Component)]
pub struct CharCreateUi;

/// 角色名输入框标记组件
#[derive(Component)]
pub struct CharNameInput;

/// 属性值显示组件（包含属性名称）
#[derive(Component)]
pub struct StatDisplay(pub String);

/// 属性增加按钮组件（包含属性名称）
#[derive(Component)]
pub struct StatIncreaseButton(pub String);

/// 属性减少按钮组件（包含属性名称）
#[derive(Component)]
pub struct StatDecreaseButton(pub String);

/// 确认创建按钮组件
#[derive(Component)]
pub struct ConfirmCreateButton;

/// 返回按钮组件（从创建界面返回选择界面）
#[derive(Component)]
pub struct BackToSelectButton;

/// 角色创建模式资源
#[derive(Resource)]
pub struct CharCreateMode {
    /// 角色名
    pub name: String,
    /// 力量
    pub str: u8,
    /// 敏捷
    pub agi: u8,
    /// 体力
    pub vit: u8,
    /// 智力
    pub int: u8,
    /// 灵巧
    pub dex: u8,
    /// 幸运
    pub luk: u8,
    /// 总可用点数
    pub total_points: u8,
    /// 是否正在创建中
    pub creating: bool,
    /// 错误信息
    pub error: Option<String>,
}

impl Default for CharCreateMode {
    fn default() -> Self {
        Self {
            name: String::new(),
            str: 5,
            agi: 5,
            vit: 5,
            int: 5,
            dex: 5,
            luk: 5,
            total_points: 30,
            creating: false,
            error: None,
        }
    }
}

impl CharCreateMode {
    /// 计算已使用的点数
    fn used_points(&self) -> u8 {
        self.str + self.agi + self.vit + self.int + self.dex + self.luk
    }

    /// 获取剩余可用点数
    fn remaining_points(&self) -> i16 {
        self.total_points as i16 - self.used_points() as i16
    }
}

/// 角色选择状态
#[derive(Resource, Default)]
pub struct CharSelectState {
    /// 角色列表
    pub characters: Vec<CharInfo>,
    /// 当前选中的角色索引
    pub selected_index: Option<usize>,
    /// 是否正在请求角色列表
    pub loading: bool,
    /// 错误信息
    pub error_message: Option<String>,
    /// 地图服务器连接信息
    pub map_server_ip: Option<String>,
    pub map_server_port: Option<u16>,
    pub map_token: Option<String>,
    /// 登录验证 ID1（来自 LoginResponse，用于 MapEnterRequest）
    pub login_id1: u32,
    /// 登录验证 ID2（来自 LoginResponse，用于 MapEnterRequest）
    pub login_id2: u32,
    /// 性别（来自 LoginResponse，用于 MapEnterRequest）
    pub sex: u8,
}

// ============================================================================
// 系统函数
// ============================================================================

/// 进入角色选择状态时的初始化系统
pub fn setup_char_select(
    mut commands: Commands,
    network: Res<NetworkManager>,
    login_state: Res<LoginState>,
) {
    tracing::info!("进入角色选择状态");

    commands.insert_resource(CharSelectState {
        loading: true,
        login_id1: login_state.login_id1,
        login_id2: login_state.login_id2,
        sex: login_state.sex,
        ..default()
    });
    commands.insert_resource(CharCreateMode::default());

    // 请求角色列表
    network.send_command(NetworkCommand::Send(Packet::CharListRequest));

    build_char_select_ui(&mut commands);
}

/// 离开角色选择状态时的清理系统
pub fn cleanup_char_select(
    mut commands: Commands,
    select_query: Query<Entity, With<CharSelectUi>>,
    create_query: Query<Entity, With<CharCreateUi>>,
) {
    for entity in &select_query {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &create_query {
        commands.entity(entity).despawn_recursive();
    }
}

/// 角色选择网络事件处理系统
pub fn char_select_network_handler(
    mut commands: Commands,
    mut state: ResMut<CharSelectState>,
    mut create_mode: ResMut<CharCreateMode>,
    network: Res<NetworkManager>,
    mut next_state: ResMut<NextState<GameState>>,
    mut status_query: Query<&mut Text, With<CharSelectStatusText>>,
) {
    let events = network.poll_events();
    for event in events {
        match event {
            NetworkEvent::PacketReceived(packet) => {
                match packet {
                    Packet::CharListResponse(resp) => {
                        tracing::info!("收到角色列表，共 {} 个角色", resp.chars.len());
                        state.loading = false;
                        state.characters = resp.chars;

                        for mut text in status_query.iter_mut() {
                            if state.characters.is_empty() {
                                **text = "没有角色，请先创建一个".to_string();
                            } else {
                                **text = "请选择一个角色".to_string();
                            }
                        }
                    }
                    Packet::CharEnterResponse(resp) => {
                        tracing::info!("收到地图服务器信息: {}:{}", resp.map_ip, resp.map_port);

                        state.map_server_ip = Some(resp.map_ip.clone());
                        state.map_server_port = Some(resp.map_port);
                        state.map_token = Some(resp.token.clone());

                        // 断开角色服务器连接
                        network.send_command(NetworkCommand::Disconnect);

                        // 创建地图服务器的 NetworkManager
                        let map_network = NetworkManager::new("legacy");
                        let map_ip = resp.map_ip.clone();
                        let map_port = resp.map_port;

                        // 连接到地图服务器
                        tracing::info!("连接地图服务器: {}:{}", map_ip, map_port);
                        map_network.send_command(NetworkCommand::Connect {
                            address: map_ip,
                            port: map_port,
                        });

                        // 插入地图网络管理器资源，供 InGame 状态使用
                        commands.insert_resource(MapNetworkManager(map_network));

                        // 切换到游戏状态
                        tracing::info!("切换到游戏状态");
                        next_state.set(GameState::InGame);
                    }
                    Packet::CharCreateResponse(resp) => {
                        create_mode.creating = false;
                        match resp {
                            crate::protocol::char_mod::CharCreateResponse::Success(char_info) => {
                                tracing::info!("角色创建成功: {} (ID: {})", char_info.name, char_info.char_id);
                                state.characters.push(char_info);
                                create_mode.error = None;
                                // 请求刷新角色列表
                                network.send_command(NetworkCommand::Send(Packet::CharListRequest));
                            }
                            crate::protocol::char_mod::CharCreateResponse::Failure { error_code } => {
                                let msg = match error_code {
                                    0 => "角色名已存在".to_string(),
                                    1 => "角色名长度不合法".to_string(),
                                    2 => "创建失败（未知错误）".to_string(),
                                    _ => format!("创建失败（错误码: {}）", error_code),
                                };
                                tracing::warn!("角色创建失败: {}", msg);
                                create_mode.error = Some(msg);
                            }
                        }
                    }
                    _ => {
                        tracing::warn!("角色选择阶段收到意外包: 0x{:04X}", packet.packet_id());
                    }
                }
            }
            NetworkEvent::RecvError(err) => {
                tracing::error!("角色选择阶段接收错误: {}", err);
                state.error_message = Some(err);
            }
            NetworkEvent::Disconnected => {
                tracing::warn!("角色选择阶段连接断开");
                // 断开连接是预期行为（收到 CharEnterResponse 后主动断开）
                // 只有在非主动断开时才记录错误
            }
            _ => {}
        }
    }
}

/// 地图服务器网络管理器（包装 NetworkManager，作为独立资源）
#[derive(Resource)]
pub struct MapNetworkManager(pub NetworkManager);

/// 角色卡片点击处理系统
pub fn handle_char_card_click(
    mut interaction_query: Query<
        (&Interaction, &CharCard, &mut BackgroundColor),
        (Changed<Interaction>, With<CharCard>),
    >,
    mut state: ResMut<CharSelectState>,
    mut status_query: Query<&mut Text, With<CharSelectStatusText>>,
) {
    for (interaction, card, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                state.selected_index = Some(card.0);
                let name = state.characters.get(card.0)
                    .map(|c| c.name.as_str())
                    .unwrap_or("未知");
                for mut text in status_query.iter_mut() {
                    **text = format!("已选择: {}", name);
                }
            }
            Interaction::Hovered => {
                if state.selected_index != Some(card.0) {
                    *bg_color = BackgroundColor(Color::srgb(0.25, 0.25, 0.35));
                }
            }
            Interaction::None => {
                if state.selected_index == Some(card.0) {
                    *bg_color = BackgroundColor(Color::srgb(0.2, 0.3, 0.5));
                } else {
                    *bg_color = BackgroundColor(Color::srgb(0.15, 0.15, 0.25));
                }
            }
        }
    }
}

/// 进入游戏按钮点击处理系统
pub fn handle_enter_game_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<EnterGameButton>),
    >,
    state: Res<CharSelectState>,
    network: Res<NetworkManager>,
    mut status_query: Query<&mut Text, With<CharSelectStatusText>>,
) {
    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.15, 0.5, 0.15));

                if let Some(index) = state.selected_index {
                    if let Some(ch) = state.characters.get(index) {
                        tracing::info!("请求进入游戏，角色: {} (ID: {})", ch.name, ch.char_id);
                        let req = CharEnterRequest { char_id: ch.char_id };
                        network.send_command(NetworkCommand::Send(Packet::CharEnterRequest(req)));
                        for mut text in status_query.iter_mut() {
                            **text = "正在进入游戏...".to_string();
                        }
                    }
                } else {
                    for mut text in status_query.iter_mut() {
                        **text = "请先选择一个角色".to_string();
                    }
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.15, 0.4, 0.15));
            }
        }
    }
}

/// 创建角色按钮点击处理系统（从选择界面切换到创建界面）
pub fn handle_create_char_button(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<CreateCharButton>),
    >,
    select_query: Query<Entity, With<CharSelectUi>>,
) {
    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.5, 0.3, 0.15));

                // 隐藏选择界面，显示创建界面
                for entity in &select_query {
                    commands.entity(entity).despawn_recursive();
                }
                build_char_create_ui(&mut commands);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.6, 0.4, 0.2));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.5, 0.35, 0.15));
            }
        }
    }
}

/// 属性增加按钮处理系统
pub fn handle_stat_increase_button(
    mut interaction_query: Query<
        (&Interaction, &StatIncreaseButton, &mut BackgroundColor),
        (Changed<Interaction>, With<StatIncreaseButton>),
    >,
    mut create_mode: ResMut<CharCreateMode>,
    mut stat_display: Query<(&mut Text, &StatDisplay)>,
) {
    for (interaction, button, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.6));
                if create_mode.remaining_points() > 0 {
                    match button.0.as_str() {
                        "str" => create_mode.str = create_mode.str.saturating_add(1),
                        "agi" => create_mode.agi = create_mode.agi.saturating_add(1),
                        "vit" => create_mode.vit = create_mode.vit.saturating_add(1),
                        "int" => create_mode.int = create_mode.int.saturating_add(1),
                        "dex" => create_mode.dex = create_mode.dex.saturating_add(1),
                        "luk" => create_mode.luk = create_mode.luk.saturating_add(1),
                        _ => {}
                    }
                    update_stat_displays(&create_mode, &mut stat_display);
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.35, 0.35, 0.65));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.25, 0.25, 0.5));
            }
        }
    }
}

/// 属性减少按钮处理系统
pub fn handle_stat_decrease_button(
    mut interaction_query: Query<
        (&Interaction, &StatDecreaseButton, &mut BackgroundColor),
        (Changed<Interaction>, With<StatDecreaseButton>),
    >,
    mut create_mode: ResMut<CharCreateMode>,
    mut stat_display: Query<(&mut Text, &StatDisplay)>,
) {
    for (interaction, button, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.6, 0.3, 0.3));
                let current = match button.0.as_str() {
                    "str" => create_mode.str,
                    "agi" => create_mode.agi,
                    "vit" => create_mode.vit,
                    "int" => create_mode.int,
                    "dex" => create_mode.dex,
                    "luk" => create_mode.luk,
                    _ => 0,
                };
                if current > 1 {
                    match button.0.as_str() {
                        "str" => create_mode.str -= 1,
                        "agi" => create_mode.agi -= 1,
                        "vit" => create_mode.vit -= 1,
                        "int" => create_mode.int -= 1,
                        "dex" => create_mode.dex -= 1,
                        "luk" => create_mode.luk -= 1,
                        _ => {}
                    }
                    update_stat_displays(&create_mode, &mut stat_display);
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.65, 0.35, 0.35));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.5, 0.25, 0.25));
            }
        }
    }
}

/// 更新属性值显示
fn update_stat_displays(
    create_mode: &CharCreateMode,
    stat_display: &mut Query<(&mut Text, &StatDisplay)>,
) {
    for (mut text, display) in stat_display.iter_mut() {
        let value = match display.0.as_str() {
            "str" => create_mode.str,
            "agi" => create_mode.agi,
            "vit" => create_mode.vit,
            "int" => create_mode.int,
            "dex" => create_mode.dex,
            "luk" => create_mode.luk,
            _ => 0,
        };
        **text = format!("{}", value);
    }
}

/// 确认创建按钮处理系统
pub fn handle_confirm_create_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ConfirmCreateButton>),
    >,
    mut create_mode: ResMut<CharCreateMode>,
    network: Res<NetworkManager>,
    name_query: Query<&Text, With<CharNameInput>>,
) {
    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.15, 0.5, 0.15));

                let name = name_query.iter().next().map(|t| t.to_string()).unwrap_or_default();
                if name.is_empty() {
                    create_mode.error = Some("请输入角色名".to_string());
                    return;
                }

                if create_mode.remaining_points() < 0 {
                    create_mode.error = Some("属性点数超出限制".to_string());
                    return;
                }

                create_mode.creating = true;
                create_mode.error = None;

                let req = CharCreateRequest {
                    name,
                    str: create_mode.str,
                    agi: create_mode.agi,
                    vit: create_mode.vit,
                    int: create_mode.int,
                    dex: create_mode.dex,
                    luk: create_mode.luk,
                    hair_color: 0,
                    hair: 1,
                };
                tracing::info!("请求创建角色: {:?}", req);
                network.send_command(NetworkCommand::Send(Packet::CharCreateRequest(req)));
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.2, 0.6, 0.2));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.15, 0.4, 0.15));
            }
        }
    }
}

/// 返回角色选择界面按钮处理系统
pub fn handle_back_to_select_button(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<BackToSelectButton>),
    >,
    create_query: Query<Entity, With<CharCreateUi>>,
    network: Res<NetworkManager>,
) {
    for (interaction, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(Color::srgb(0.4, 0.4, 0.4));

                // 销毁创建界面
                for entity in &create_query {
                    commands.entity(entity).despawn_recursive();
                }
                // 重新请求角色列表并显示选择界面
                network.send_command(NetworkCommand::Send(Packet::CharListRequest));
                build_char_select_ui(&mut commands);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.5, 0.5, 0.5));
            }
            Interaction::None => {
                *bg_color = BackgroundColor(Color::srgb(0.35, 0.35, 0.35));
            }
        }
    }
}

// ============================================================================
// UI 构建
// ============================================================================

/// 构建角色选择界面 UI
fn build_char_select_ui(commands: &mut Commands) {
    commands
        .spawn((
            CharSelectUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn((
                Text::new("选择角色"),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                Node { margin: UiRect::bottom(Val::Px(30.0)), ..default() },
            ));

            // 角色卡片容器
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                })
                .with_children(|container| {
                    for i in 0..3 {
                        container
                            .spawn((
                                CharCard(i),
                                Button,
                                Node {
                                    width: Val::Px(150.0),
                                    height: Val::Px(200.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    margin: UiRect::horizontal(Val::Px(10.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.15, 0.15, 0.25)),
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    Text::new(format!("角色 {}", i + 1)),
                                    TextFont { font_size: 16.0, ..default() },
                                    TextColor(Color::WHITE),
                                ));
                                card.spawn((
                                    Text::new("加载中..."),
                                    TextFont { font_size: 14.0, ..default() },
                                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                                ));
                            });
                    }
                });

            // 按钮容器
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|button_row| {
                    // 进入游戏按钮
                    button_row
                        .spawn((
                            EnterGameButton,
                            Button,
                            Node {
                                width: Val::Px(150.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::right(Val::Px(20.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.4, 0.15)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("进入游戏"),
                                TextFont { font_size: 18.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // 创建角色按钮
                    button_row
                        .spawn((
                            CreateCharButton,
                            Button,
                            Node {
                                width: Val::Px(150.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.5, 0.35, 0.15)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("创建角色"),
                                TextFont { font_size: 18.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                });

            // 状态文本
            parent.spawn((
                CharSelectStatusText,
                Text::new("正在加载角色列表..."),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });
}

/// 构建角色创建界面 UI
fn build_char_create_ui(commands: &mut Commands) {
    let stats = ["str", "agi", "vit", "int", "dex", "luk"];
    let stat_names = ["力量 STR", "敏捷 AGI", "体力 VIT", "智力 INT", "灵巧 DEX", "幸运 LUK"];

    commands
        .spawn((
            CharCreateUi,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.12)),
        ))
        .with_children(|parent| {
            // 标题
            parent.spawn((
                Text::new("创建角色"),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
            ));

            // 角色名输入行
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new("角色名: "),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        CharNameInput,
                        Text::new(""),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        Node { min_width: Val::Px(200.0), ..default() },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
                    ));
                });

            // 属性分配区域
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|stats_container| {
                    for (i, stat) in stats.iter().enumerate() {
                        stats_container
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                margin: UiRect::vertical(Val::Px(4.0)),
                                ..default()
                            })
                            .with_children(|row| {
                                // 属性名
                                row.spawn((
                                    Text::new(format!("{:>10}", stat_names[i])),
                                    TextFont { font_size: 16.0, ..default() },
                                    TextColor(Color::WHITE),
                                    Node { min_width: Val::Px(120.0), ..default() },
                                ));

                                // 减少按钮
                                row.spawn((
                                    StatDecreaseButton(stat.to_string()),
                                    Button,
                                    Node {
                                        width: Val::Px(30.0),
                                        height: Val::Px(30.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::right(Val::Px(10.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.5, 0.25, 0.25)),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new("-"),
                                        TextFont { font_size: 18.0, ..default() },
                                        TextColor(Color::WHITE),
                                    ));
                                });

                                // 属性值显示
                                row.spawn((
                                    StatDisplay(stat.to_string()),
                                    Text::new("5"),
                                    TextFont { font_size: 18.0, ..default() },
                                    TextColor(Color::srgb(0.9, 0.8, 0.3)),
                                    Node { min_width: Val::Px(40.0), justify_content: JustifyContent::Center, ..default() },
                                ));

                                // 增加按钮
                                row.spawn((
                                    StatIncreaseButton(stat.to_string()),
                                    Button,
                                    Node {
                                        width: Val::Px(30.0),
                                        height: Val::Px(30.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::left(Val::Px(10.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.25, 0.25, 0.5)),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new("+"),
                                        TextFont { font_size: 18.0, ..default() },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            });
                    }
                });

            // 剩余点数提示
            parent.spawn((
                CharSelectStatusText,
                Text::new("剩余点数: 0"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
            ));

            // 按钮容器
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|button_row| {
                    // 确认创建按钮
                    button_row
                        .spawn((
                            ConfirmCreateButton,
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::right(Val::Px(20.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.4, 0.15)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("确认创建"),
                                TextFont { font_size: 18.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // 返回按钮
                    button_row
                        .spawn((
                            BackToSelectButton,
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.35, 0.35, 0.35)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("返回"),
                                TextFont { font_size: 18.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}
