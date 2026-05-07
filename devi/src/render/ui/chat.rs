// 聊天窗口系统
// 实现 RO 风格的左下角聊天窗口，支持消息显示和输入框
// 使用 Bevy 0.15 的 required components API

use bevy::prelude::*;

/// 聊天消息组件
/// 存储单条聊天消息的发送者、内容和时间戳
#[derive(Debug, Component)]
pub struct ChatMessage {
    /// 发送者名称
    pub sender: String,
    /// 消息内容
    pub content: String,
    /// 时间戳（秒）
    pub timestamp: f64,
}

/// 聊天窗口组件标记
#[derive(Debug, Component)]
pub struct ChatWindow;

/// 构建聊天窗口
/// 创建位于屏幕左下角的聊天窗口，包含消息显示区域和输入框
pub fn build_chat_window(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(40.0),
                width: Val::Px(300.0),
                height: Val::Px(200.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ChatWindow,
        ))
        .with_children(|parent| {
            // 消息显示区域（从底部向上排列，溢出裁剪）
            parent.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::ColumnReverse,
                overflow: Overflow::clip_y(),
                ..default()
            });
            // 输入框区域
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(24.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
            ));
        });
}
