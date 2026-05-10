//! 地图服务器连接管理
//!
//! 处理与地图服务器的连接生命周期：
//! 1. 连接成功后发送 MapEnterRequest
//! 2. 处理 MapEnteredResponse，获取初始位置
//! 3. 处理地图阶段的其他网络事件（实体同步、聊天等）

use std::collections::HashMap;
use bevy::prelude::*;
use crate::game::attack::{DamageDisplayEvent, AttackAnimation};
use crate::game::char_select::{CharSelectState, MapNetworkManager};
use crate::game::mob::Mob;
use crate::game::movement::Movement;
use crate::game::player::Player;
use crate::net::session::{NetworkCommand, NetworkEvent};
use crate::protocol::Packet;
use crate::protocol::map::MapEnterRequest;
use crate::render::ui::chat::{ChatWindow, ChatMessage as UiChatMessage, ChatMessageType, ChatHistory, ChatSendEvent};
use crate::protocol::map::ChatSendRequest;

/// RO cell 坐标到 Bevy 世界坐标的缩放系数
///
/// RO 使用 cell 网格坐标系，Bevy 使用浮点世界坐标系。
/// 此值将 RO 的整数 cell 坐标转换为 Bevy 的浮点世界单位。
/// 后续需要根据实际地图尺寸和相机参数调整此值。
const CELL_TO_WORLD_SCALE: f32 = 0.15;

/// RO 移动速度基准值（正常玩家速度为 150）
///
/// RO 的 speed 值越小移动越快，150 为默认玩家速度。
const RO_SPEED_BASE: f32 = 150.0;

/// 基准速度对应的世界单位/秒
///
/// 当 RO speed 为 150 时，对应此世界移动速度。
const WORLD_SPEED_BASE: f32 = 5.0;

/// 将 RO cell 坐标转换为 Bevy 世界坐标（x-z 平面）
///
/// 坐标系映射关系：
/// - RO X（向右/东）→ Bevy X
/// - RO Y（向下/南）→ Bevy Z（Bevy 的 Z 轴正方向对应屏幕下方）
/// - Bevy Y 固定为 0.0（地面高度）
fn ro_cell_to_world(cell_x: u16, cell_y: u16) -> Vec3 {
    Vec3::new(
        cell_x as f32 * CELL_TO_WORLD_SCALE,
        0.0,
        cell_y as f32 * CELL_TO_WORLD_SCALE,
    )
}

/// 将 RO 移动速度转换为 Bevy 世界单位/秒
///
/// RO 的 speed 值越小移动越快（150 为正常玩家速度）。
/// 转换公式：world_speed = WORLD_SPEED_BASE * RO_SPEED_BASE / ro_speed
fn ro_speed_to_world(ro_speed: u16) -> f32 {
    if ro_speed == 0 {
        // speed 为 0 时使用默认速度，避免除零
        WORLD_SPEED_BASE
    } else {
        WORLD_SPEED_BASE * RO_SPEED_BASE / ro_speed as f32
    }
}

/// 地图连接状态
///
/// 维护与地图服务器的连接状态，包括：
/// - 连接和进入状态标志
/// - 玩家初始位置信息
/// - 服务器实体 ID 到本地 Bevy Entity 的映射
#[derive(Resource, Default)]
pub struct MapConnectionState {
    /// 是否已连接到地图服务器
    pub connected: bool,
    /// 是否已发送进入地图请求
    pub enter_sent: bool,
    /// 是否已收到地图进入响应
    pub entered: bool,
    /// 初始 X 坐标
    pub pos_x: u16,
    /// 初始 Y 坐标
    pub pos_y: u16,
    /// 朝向
    pub direction: u16,
    /// 错误信息
    pub error_message: Option<String>,
    /// 服务器实体 ID → Bevy Entity 的映射表
    /// 用于将服务器下发的 entity_id 关联到本地 ECS 实体，
    /// 实体出现时插入，实体消失时移除。
    pub entity_map: HashMap<u32, Entity>,
}

/// 进入地图状态时的初始化系统
pub fn setup_map_connection(mut commands: Commands) {
    tracing::info!("初始化地图连接");
    commands.insert_resource(MapConnectionState::default());
}

/// 地图服务器网络事件处理系统
///
/// 处理所有地图阶段的网络事件，包括：
/// - 连接建立和地图进入请求
/// - 实体出现/消失/移动同步
/// - 聊天消息接收与显示
pub fn map_network_handler(
    mut commands: Commands,
    time: Res<Time>,
    mut conn_state: ResMut<MapConnectionState>,
    char_state: Res<CharSelectState>,
    map_network: Res<MapNetworkManager>,
    mut movement_query: Query<&mut Movement>,
    chat_children_query: Query<&Children, With<ChatWindow>>,
    mut chat_history: ResMut<ChatHistory>,
    mut damage_events: EventWriter<DamageDisplayEvent>,
) {
    let events = map_network.0.poll_events();
    for event in events {
        match event {
            NetworkEvent::Connected => {
                tracing::info!("已连接到地图服务器，发送进入地图请求");
                conn_state.connected = true;
                conn_state.enter_sent = true;

                // 从角色选择状态获取选中的角色信息
                let char_id = char_state
                    .selected_index
                    .and_then(|i| char_state.characters.get(i))
                    .map(|c| c.char_id)
                    .unwrap_or(0);

                let req = MapEnterRequest {
                    char_id,
                    login_id: char_state.login_id1,
                    // 使用客户端已运行的毫秒数作为 tick
                    client_tick: time.elapsed().as_millis() as u32,
                    gender: char_state.sex,
                };

                tracing::info!(
                    "发送 MapEnterRequest: char_id={}, login_id={}, client_tick={}",
                    req.char_id,
                    req.login_id,
                    req.client_tick
                );
                map_network
                    .0
                    .send_command(NetworkCommand::Send(Packet::MapEnter(req)));
            }
            NetworkEvent::PacketReceived(packet) => {
                match packet {
                    Packet::MapEntered(resp) => {
                        tracing::info!(
                            "地图进入成功: 位置 ({}, {}), 朝向 {}",
                            resp.pos_x,
                            resp.pos_y,
                            resp.direction
                        );
                        conn_state.entered = true;
                        conn_state.pos_x = resp.pos_x;
                        conn_state.pos_y = resp.pos_y;
                        conn_state.direction = resp.direction;
                    }
                    Packet::EntityAppear(notify) => {
                        tracing::debug!(
                            "实体出现: id={}, type={}, pos=({}, {})",
                            notify.entity_id,
                            notify.entity_type,
                            notify.pos_x,
                            notify.pos_y
                        );

                        // 如果该实体已存在于映射表中（重复通知），先移除旧实体
                        if let Some(old_entity) = conn_state.entity_map.remove(&notify.entity_id) {
                            commands.entity(old_entity).despawn();
                        }

                        // 将 RO cell 坐标转换为 Bevy 世界坐标
                        let world_pos = ro_cell_to_world(notify.pos_x, notify.pos_y);
                        // 使用默认玩家速度初始化移动组件
                        let speed = ro_speed_to_world(150);

                        // 根据实体类型创建不同的 ECS 实体
                        let entity = match notify.entity_type {
                            0 => {
                                // 类型 0 = 玩家：创建带 Player 组件的实体
                                tracing::debug!("创建玩家实体: id={}", notify.entity_id);
                                commands.spawn((
                                    Player::new(
                                        notify.entity_id,
                                        format!("Player_{}", notify.entity_id),
                                    ),
                                    Transform::from_translation(world_pos),
                                    Movement::new(speed),
                                )).id()
                            }
                            6 => {
                                // 类型 6 = 怪物：创建带 Mob 组件的实体
                                // EntityAppearNotify 的 look 字段作为怪物数据库 ID
                                tracing::debug!(
                                    "创建怪物实体: id={}, mob_id={}",
                                    notify.entity_id,
                                    notify.look
                                );
                                commands.spawn((
                                    Mob::new(
                                        notify.entity_id,
                                        notify.look as u32,
                                        format!("Mob_{}", notify.look),
                                    ),
                                    Transform::from_translation(world_pos),
                                    Movement::new(speed),
                                )).id()
                            }
                            _ => {
                                // 其他类型（NPC=5 等）：创建基础实体，仅带位置和移动组件
                                tracing::debug!(
                                    "创建其他实体: id={}, type={}",
                                    notify.entity_id,
                                    notify.entity_type
                                );
                                commands.spawn((
                                    Transform::from_translation(world_pos),
                                    Movement::new(speed),
                                )).id()
                            }
                        };

                        // 将服务器实体 ID 映射到本地 Bevy Entity
                        conn_state.entity_map.insert(notify.entity_id, entity);
                    }
                    Packet::EntityDisappear(notify) => {
                        tracing::debug!(
                            "实体消失: id={}, reason={}",
                            notify.entity_id,
                            notify.reason
                        );

                        // 从映射表中查找并移除实体
                        if let Some(entity) = conn_state.entity_map.remove(&notify.entity_id) {
                            commands.entity(entity).despawn();
                            tracing::debug!("已从场景中移除实体: id={}", notify.entity_id);
                        } else {
                            tracing::warn!(
                                "尝试移除不存在的实体: id={}",
                                notify.entity_id
                            );
                        }
                    }
                    Packet::EntityMove(notify) => {
                        tracing::debug!(
                            "实体移动: id={}, ({},{}) -> ({},{})",
                            notify.entity_id,
                            notify.from_x,
                            notify.from_y,
                            notify.dest_x,
                            notify.dest_y
                        );

                        // 从映射表中查找对应的 Bevy Entity
                        if let Some(&entity) = conn_state.entity_map.get(&notify.entity_id) {
                            // 查询该实体的 Movement 组件，设置目标位置
                            if let Ok(mut movement) = movement_query.get_mut(entity) {
                                let dest = ro_cell_to_world(notify.dest_x, notify.dest_y);
                                movement.set_destination(dest.x, dest.z);
                                // 如果服务器提供了速度信息，同步更新移动速度
                                if notify.speed > 0 {
                                    movement.speed = ro_speed_to_world(notify.speed);
                                }
                            } else {
                                tracing::warn!(
                                    "实体 {} 没有 Movement 组件，无法更新移动目标",
                                    notify.entity_id
                                );
                            }
                        } else {
                            // 收到未知实体的移动通知，可能是实体出现包丢失
                            tracing::warn!(
                                "收到未知实体的移动通知: id={}",
                                notify.entity_id
                            );
                        }
                    }
                    Packet::ChatMessage(msg) => {
                        tracing::info!(
                            "[{}] {}: {}",
                            msg.sender_id,
                            msg.sender_name,
                            msg.message
                        );

                        // 确定消息类型（后续可根据包类型区分系统/私聊/队伍/公会消息）
                        let msg_type = ChatMessageType::Normal;
                        let timestamp = time.elapsed_secs_f64();

                        // 添加到消息历史
                        chat_history.push(&msg.sender_name, &msg.message, msg_type, timestamp);

                        // 将聊天消息添加到聊天窗口 UI
                        // ChatWindow 的第一个子节点是消息显示区域
                        if let Ok(children) = chat_children_query.get_single() {
                            if let Some(&msg_area) = children.first() {
                                commands.entity(msg_area).with_children(|parent| {
                                    parent.spawn((
                                        UiChatMessage {
                                            sender: msg.sender_name.clone(),
                                            content: msg.message.clone(),
                                            msg_type,
                                            timestamp,
                                        },
                                        Text::new(format!(
                                            "[{}] {}",
                                            msg.sender_name, msg.message
                                        )),
                                        TextFont {
                                            font_size: 12.0,
                                            ..default()
                                        },
                                        TextColor(msg_type.color()),
                                    ));
                                });
                            }
                        }
                    }
                    Packet::AttackNotify(notify) => {
                        tracing::info!(
                            "攻击通知: src={}, dst={}, damage={}, type={}",
                            notify.src_id,
                            notify.dst_id,
                            notify.damage,
                            notify.damage_type
                        );

                        // 为目标实体添加攻击动画（如果攻击者在视野内）
                        if let Some(&attacker_entity) = conn_state.entity_map.get(&notify.src_id) {
                            commands.entity(attacker_entity).insert(AttackAnimation::default());
                        }

                        // 更新目标实体的 HP（如果目标是 Mob）
                        if let Some(&_target_entity) = conn_state.entity_map.get(&notify.dst_id) {
                            // 尝试减少 Mob 的 HP
                            // 注意：这里通过 commands 延迟执行，实际更新在下一帧
                            // 后续可以通过专门的 HP 更新系统来处理
                        }

                        // 触发伤害数字显示事件
                        damage_events.send(DamageDisplayEvent {
                            target_entity_id: notify.dst_id,
                            damage: notify.damage,
                            damage_type: notify.damage_type,
                        });
                    }
                    _ => {
                        tracing::warn!(
                            "地图阶段收到未处理包: 0x{:04X}",
                            packet.packet_id()
                        );
                    }
                }
            }
            NetworkEvent::ConnectFailed(err) => {
                tracing::error!("连接地图服务器失败: {}", err);
                conn_state.error_message = Some(format!("连接地图服务器失败: {}", err));
            }
            NetworkEvent::RecvError(err) => {
                tracing::error!("地图服务器接收错误: {}", err);
                conn_state.error_message = Some(err);
            }
            NetworkEvent::Disconnected => {
                tracing::warn!("与地图服务器断开连接");
                conn_state.connected = false;
                conn_state.entered = false;
            }
        }
    }
}

/// 聊天消息发送处理系统
///
/// 监听 ChatSendEvent 事件，将消息通过网络发送到服务器。
/// 同时在本地聊天窗口显示自己发送的消息。
pub fn chat_send_handler(
    mut commands: Commands,
    mut chat_send_events: EventReader<ChatSendEvent>,
    map_network: Res<MapNetworkManager>,
    conn_state: Res<MapConnectionState>,
    time: Res<Time>,
    chat_children_query: Query<&Children, With<ChatWindow>>,
    mut chat_history: ResMut<ChatHistory>,
) {
    for event in chat_send_events.read() {
        if !conn_state.connected {
            tracing::warn!("未连接到地图服务器，无法发送聊天消息");
            continue;
        }

        // 通过网络发送聊天消息到服务器
        let packet = Packet::ChatSendRequest(ChatSendRequest {
            message: event.message.clone(),
        });
        map_network.0.send_command(NetworkCommand::Send(packet));
        tracing::info!("发送聊天消息到服务器: {}", event.message);

        // 在本地聊天窗口显示自己发送的消息
        let msg_type = ChatMessageType::Normal;
        let timestamp = time.elapsed_secs_f64();
        chat_history.push("我", &event.message, msg_type, timestamp);

        // 将消息添加到 UI
        if let Ok(children) = chat_children_query.get_single() {
            if let Some(&msg_area) = children.first() {
                commands.entity(msg_area).with_children(|parent| {
                    parent.spawn((
                        UiChatMessage {
                            sender: "我".to_string(),
                            content: event.message.clone(),
                            msg_type,
                            timestamp,
                        },
                        Text::new(format!("[我] {}", event.message)),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(msg_type.color()),
                    ));
                });
            }
        }
    }
}
