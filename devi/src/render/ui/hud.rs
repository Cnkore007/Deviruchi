// HUD 布局系统
// 实现 RO 风格的底部状态栏，包含 HP/SP 条和等级信息
// 使用 Bevy 0.15 的 required components API

use bevy::prelude::*;

/// HUD 布局参数
/// 定义屏幕尺寸和各 UI 元素的大小
#[derive(Debug, Clone)]
pub struct HudLayout {
    /// 屏幕宽度
    pub screen_width: f32,
    /// 屏幕高度
    pub screen_height: f32,
    /// 底部状态栏高度
    pub status_bar_height: f32,
    /// 聊天窗口宽度
    pub chat_window_width: f32,
    /// 小地图尺寸
    pub minimap_size: f32,
}

impl Default for HudLayout {
    fn default() -> Self {
        Self {
            screen_width: 1024.0,
            screen_height: 768.0,
            status_bar_height: 40.0,
            chat_window_width: 300.0,
            minimap_size: 150.0,
        }
    }
}

/// 状态栏组件标记
#[derive(Debug, Component)]
pub struct StatusBar;

/// HP 条组件
/// 记录当前和最大生命值，用于 UI 更新
#[derive(Debug, Component)]
pub struct HpBar {
    /// 当前 HP
    pub current: u32,
    /// 最大 HP
    pub max: u32,
}

/// SP 条组件
/// 记录当前和最大魔法值，用于 UI 更新
#[derive(Debug, Component)]
pub struct SpBar {
    /// 当前 SP
    pub current: u32,
    /// 最大 SP
    pub max: u32,
}

/// 构建 HUD 界面
/// 创建 RO 风格的底部状态栏，包含 HP 条（红色）、SP 条（蓝色）和等级信息文本
pub fn build_hud(commands: &mut Commands, layout: &HudLayout) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            // 游戏视口（占据剩余空间）
            parent.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                ..default()
            });
            // 底部面板
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(layout.status_bar_height),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                ))
                .with_children(|parent| {
                    // HP/SP 条容器
                    parent
                        .spawn(Node {
                            width: Val::Px(200.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(4.0)),
                            ..default()
                        })
                        .with_children(|parent| {
                            // HP 条（红色）
                            parent.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(14.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.8, 0.2, 0.2)),
                                HpBar {
                                    current: 100,
                                    max: 100,
                                },
                            ));
                            // SP 条（蓝色）
                            parent.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(14.0),
                                    margin: UiRect::top(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.2, 0.8)),
                                SpBar {
                                    current: 50,
                                    max: 50,
                                },
                            ));
                        });
                    // 等级和数值信息文本
                    parent.spawn((
                        Text::new("Lv.1 | HP: 100/100 | SP: 50/50"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}
