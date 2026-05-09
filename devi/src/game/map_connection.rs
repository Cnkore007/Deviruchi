//! 地图服务器连接管理
//!
//! 处理与地图服务器的连接生命周期：
//! 1. 连接成功后发送 MapEnterRequest
//! 2. 处理 MapEnteredResponse，获取初始位置
//! 3. 处理地图阶段的其他网络事件

use bevy::prelude::*;
use crate::game::char_select::{CharSelectState, MapNetworkManager};
use crate::net::session::{NetworkCommand, NetworkEvent};
use crate::protocol::Packet;
use crate::protocol::map::MapEnterRequest;

/// 地图连接状态
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
}

/// 进入地图状态时的初始化系统
pub fn setup_map_connection(mut commands: Commands) {
    tracing::info!("初始化地图连接");
    commands.insert_resource(MapConnectionState::default());
}

/// 地图服务器网络事件处理系统
pub fn map_network_handler(
    mut conn_state: ResMut<MapConnectionState>,
    char_state: Res<CharSelectState>,
    map_network: Res<MapNetworkManager>,
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
                    client_tick: 0, // TODO: 使用实际客户端 tick
                    gender: char_state.sex,
                };

                tracing::info!(
                    "发送 MapEnterRequest: char_id={}, login_id={}",
                    req.char_id,
                    req.login_id
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
                        // TODO: 在场景中创建实体
                    }
                    Packet::EntityDisappear(notify) => {
                        tracing::debug!(
                            "实体消失: id={}, reason={}",
                            notify.entity_id,
                            notify.reason
                        );
                        // TODO: 从场景中移除实体
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
                        // TODO: 更新实体位置
                    }
                    Packet::ChatMessage(msg) => {
                        tracing::info!("[{}] {}: {}", msg.sender_id, msg.sender_name, msg.message);
                        // TODO: 显示在聊天窗口
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
