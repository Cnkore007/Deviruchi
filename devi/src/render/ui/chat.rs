// 聊天窗口系统
// 实现 RO 风格的左下角聊天窗口，支持中文 IME 输入、消息显示和发送
// 使用 Bevy 0.15 的 required components API
//
// 功能概览：
// - 聊天输入框：支持 IME 输入法（中文/日文/韩文等）合成
// - 消息发送：Enter 发送，Escape 关闭输入，Backspace 删除
// - 消息显示：支持不同颜色（系统消息、玩家消息、私聊）
// - 消息缓冲：保留最近 N 条历史消息，自动滚动

use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::window::Ime;

/// 聊天消息最大历史条数
/// 超过此数量时，最早的消息将被移除
const MAX_CHAT_HISTORY: usize = 100;

/// 聊天消息类型
/// 用于区分不同来源的消息，决定显示颜色
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChatMessageType {
    /// 普通玩家消息（白色）
    Normal,
    /// 系统消息（黄色）
    System,
    /// 私聊消息（绿色）
    Whisper,
    /// 队伍消息（蓝色）
    Party,
    /// 公会消息（青色）
    Guild,
}

impl ChatMessageType {
    /// 获取消息类型对应的显示颜色
    pub fn color(&self) -> Color {
        match self {
            ChatMessageType::Normal => Color::WHITE,
            ChatMessageType::System => Color::srgb(1.0, 1.0, 0.3), // 黄色
            ChatMessageType::Whisper => Color::srgb(0.3, 1.0, 0.3), // 绿色
            ChatMessageType::Party => Color::srgb(0.5, 0.7, 1.0),  // 蓝色
            ChatMessageType::Guild => Color::srgb(0.3, 1.0, 1.0),  // 青色
        }
    }
}

/// 聊天消息组件
/// 存储单条聊天消息的发送者、内容、类型和时间戳
#[derive(Debug, Component)]
pub struct ChatMessage {
    /// 发送者名称
    pub sender: String,
    /// 消息内容
    pub content: String,
    /// 消息类型
    pub msg_type: ChatMessageType,
    /// 时间戳（秒）
    pub timestamp: f64,
}

/// 聊天窗口组件标记
#[derive(Debug, Component)]
pub struct ChatWindow;

/// 聊天输入框组件标记
#[derive(Debug, Component)]
pub struct ChatInputBox;

/// 聊天输入框光标组件标记
#[derive(Debug, Component)]
pub struct ChatInputCursor;

/// 聊天输入状态资源
/// 管理聊天输入框的状态，包括文本内容、光标位置、IME 合成状态
#[derive(Resource)]
pub struct ChatInputState {
    /// 输入框文本内容
    pub text: String,
    /// 光标位置（字节偏移）
    pub cursor_pos: usize,
    /// 输入框是否激活（获得焦点）
    pub is_active: bool,
    /// IME 合成中的预编辑文本
    pub ime_buffer: String,
    /// 是否需要刷新输入框显示
    pub dirty: bool,
}

impl Default for ChatInputState {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_pos: 0,
            is_active: false,
            ime_buffer: String::new(),
            dirty: true,
        }
    }
}

impl ChatInputState {
    /// 检查是否需要刷新显示
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 清除脏标记
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// 在光标位置插入文本
    /// 支持中文等多字节字符，使用 char_indices 进行安全的字符边界定位
    pub fn insert_text(&mut self, text: &str) {
        // 使用 char_indices 找到光标位置对应的字节偏移
        let byte_pos = self.cursor_byte_pos();
        self.text.insert_str(byte_pos, text);
        // 光标移动到插入文本之后（按字符数移动）
        self.cursor_pos += text.chars().count();
        self.dirty = true;
    }

    /// 删除光标前的一个字符（Backspace）
    /// 正确处理多字节 UTF-8 字符
    pub fn delete_char_before(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let byte_pos = self.cursor_byte_pos();
        // 找到前一个字符的起始位置
        let prev_char_len = self.text[..byte_pos]
            .chars()
            .last()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        if byte_pos >= prev_char_len {
            self.text.drain((byte_pos - prev_char_len)..byte_pos);
            self.cursor_pos -= 1;
            self.dirty = true;
        }
    }

    /// 清空输入框
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_pos = 0;
        self.ime_buffer.clear();
        self.dirty = true;
    }

    /// 将字符偏移的光标位置转换为字节偏移
    /// 用于安全地操作 UTF-8 字符串
    fn cursor_byte_pos(&self) -> usize {
        self.text
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(pos, _)| pos)
            .unwrap_or(self.text.len())
    }

    /// 获取输入框的显示文本（包含 IME 预编辑文本和光标）
    pub fn display_text(&self) -> String {
        let mut display = String::new();
        let byte_pos = self.cursor_byte_pos();

        // 光标前的文本
        display.push_str(&self.text[..byte_pos]);

        // IME 预编辑文本（带下划线标记）
        if !self.ime_buffer.is_empty() {
            display.push_str(&self.ime_buffer);
        }

        // 光标符号
        display.push('|');

        // 光标后的文本
        display.push_str(&self.text[byte_pos..]);

        display
    }
}

/// 消息历史资源
/// 缓存最近的聊天消息，用于在 UI 刷新时重建消息列表
#[derive(Resource)]
pub struct ChatHistory {
    /// 消息缓冲区，最新的消息在末尾
    pub messages: Vec<(String, ChatMessageType, f64)>,
}

impl Default for ChatHistory {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl ChatHistory {
    /// 添加一条消息到历史记录
    /// 超过 MAX_CHAT_HISTORY 时移除最早的消息
    pub fn push(&mut self, sender: &str, content: &str, msg_type: ChatMessageType, timestamp: f64) {
        let formatted = if sender.is_empty() {
            content.to_string()
        } else {
            format!("[{}] {}", sender, content)
        };
        self.messages.push((formatted, msg_type, timestamp));

        // 超过上限时移除最早的消息
        if self.messages.len() > MAX_CHAT_HISTORY {
            self.messages.remove(0);
        }
    }
}

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
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(24.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        padding: UiRect::horizontal(Val::Px(4.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                    ChatInputBox,
                ))
                .with_children(|parent| {
                    // 输入框文本（包含光标和 IME 预编辑）
                    parent.spawn((
                        Text::new("|"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        ChatInputCursor,
                    ));
                });
        });
}

/// IME 事件处理系统
///
/// 处理输入法合成事件，支持中文、日文、韩文等语言的输入。
/// - Ime::Preedit: 显示输入法候选文本（预编辑状态）
/// - Ime::Commit: 确认最终输入的字符
/// - Ime::Enabled / Ime::Disabled: 跟踪 IME 状态
fn handle_ime_events(
    mut ime_events: EventReader<Ime>,
    mut input_state: ResMut<ChatInputState>,
) {
    for event in ime_events.read() {
        if !input_state.is_active {
            continue;
        }

        match event {
            Ime::Preedit { value, .. } => {
                // 预编辑文本：输入法正在合成中，显示候选文本
                input_state.ime_buffer = value.clone();
                input_state.dirty = true;
            }
            Ime::Commit { value, .. } => {
                // 提交文本：用户确认了输入法候选，将文本插入到输入框
                if !value.is_empty() {
                    input_state.insert_text(value);
                }
                input_state.ime_buffer.clear();
                input_state.dirty = true;
            }
            Ime::Enabled { .. } => {
                tracing::debug!("IME 已启用");
            }
            Ime::Disabled { .. } => {
                tracing::debug!("IME 已禁用");
                input_state.ime_buffer.clear();
                input_state.dirty = true;
            }
        }
    }
}

/// 键盘输入处理系统
///
/// 处理聊天输入框的键盘事件：
/// - Enter: 发送消息（通过 ChatSendEvent 事件）
/// - Escape: 关闭聊天输入
/// - Backspace: 删除光标前的字符
/// - 字符输入：直接输入的 ASCII 字符（IME 未激活时）
fn handle_keyboard_input(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut input_state: ResMut<ChatInputState>,
    mut chat_send_events: EventWriter<ChatSendEvent>,
    mut window: Single<&mut Window>,
) {
    for event in keyboard_events.read() {
        // 只处理刚按下的按键
        if event.state != ButtonState::Pressed {
            continue;
        }

        // 如果输入框未激活，检查是否按下 Enter 激活
        if !input_state.is_active {
            if event.key_code == KeyCode::Enter {
                input_state.is_active = true;
                input_state.dirty = true;
                // 启用 IME 输入
                window.ime_enabled = true;
                tracing::debug!("聊天输入框已激活，IME 已启用");
            }
            continue;
        }

        match event.key_code {
            KeyCode::Enter => {
                // 发送消息
                let message = input_state.text.trim().to_string();
                if !message.is_empty() {
                    chat_send_events.send(ChatSendEvent {
                        message: message.clone(),
                    });
                    tracing::info!("发送聊天消息: {}", message);
                }
                input_state.clear();
                // 发送后关闭输入框
                input_state.is_active = false;
                window.ime_enabled = false;
            }
            KeyCode::Escape => {
                // 关闭聊天输入
                input_state.clear();
                input_state.is_active = false;
                window.ime_enabled = false;
                tracing::debug!("聊天输入框已关闭");
            }
            KeyCode::Backspace => {
                // 删除光标前的字符（仅在 IME 未激活时）
                if input_state.ime_buffer.is_empty() {
                    input_state.delete_char_before();
                }
            }
            _ => {
                // 处理直接输入的字符（IME 未激活时）
                // 注意：中文等字符通过 Ime::Commit 事件处理，这里只处理 ASCII
                if input_state.ime_buffer.is_empty() {
                    if let Key::Character(ref ch) = event.logical_key {
                        // 只处理可打印的 ASCII 字符，过滤掉控制字符
                        let printable: String = ch
                            .chars()
                            .filter(|c| c.is_ascii_graphic() || *c == ' ')
                            .collect();
                        if !printable.is_empty() {
                            input_state.insert_text(&printable);
                        }
                    }
                }
            }
        }
    }
}

/// 聊天发送事件
/// 当用户在聊天输入框中按下 Enter 时触发
#[derive(Event)]
pub struct ChatSendEvent {
    /// 消息内容
    pub message: String,
}

/// 聊天输入框显示更新系统
///
/// 当输入状态发生变化时，更新输入框的显示文本。
/// 包括光标位置、IME 预编辑文本和已输入的文本。
fn update_chat_input_display(
    mut input_state: ResMut<ChatInputState>,
    mut cursor_query: Query<&mut Text, With<ChatInputCursor>>,
) {
    if !input_state.is_dirty() {
        return;
    }

    if let Ok(mut text) = cursor_query.get_single_mut() {
        **text = input_state.display_text();
    }

    input_state.clear_dirty();
}

/// 添加新消息到聊天窗口系统
///
/// 监听 ChatMessage 组件的变化，在消息显示区域添加文本节点。
/// 支持不同消息类型的颜色显示。
pub fn add_chat_message_system(
    mut commands: Commands,
    chat_children_query: Query<&Children, With<ChatWindow>>,
    new_messages_query: Query<&ChatMessage, Added<ChatMessage>>,
) {
    for msg in new_messages_query.iter() {
        if let Ok(children) = chat_children_query.get_single() {
            if let Some(&msg_area) = children.first() {
                commands.entity(msg_area).with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("[{}] {}", msg.sender, msg.content)),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(msg.msg_type.color()),
                    ));
                });
            }
        }
    }
}

/// 聊天窗口系统插件
/// 注册所有聊天相关的系统和资源
pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ChatInputState>()
            .init_resource::<ChatHistory>()
            .add_event::<ChatSendEvent>()
            .add_systems(Update, (
                handle_ime_events,
                handle_keyboard_input,
                update_chat_input_display,
                add_chat_message_system,
            ).chain());
    }
}
