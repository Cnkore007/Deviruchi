// 攻击系统
// 实现玩家攻击请求发送、攻击动画状态和伤害数字显示
// 遵循 RO 的攻击流程：点击目标 -> 发送攻击请求 -> 播放动画 -> 显示伤害

use bevy::prelude::*;
use crate::game::char_select::MapNetworkManager;
use crate::game::map_connection::MapConnectionState;
use crate::game::player::Player;
use crate::net::session::NetworkCommand;
use crate::protocol::Packet;
use crate::protocol::map::AttackRequest;

// ============================================================================
// 组件定义
// ============================================================================

/// 攻击目标组件
/// 标记当前实体正在攻击的目标，存储目标的服务器实体 ID
#[derive(Debug, Component)]
pub struct AttackTarget {
    /// 目标服务器实体 ID
    pub target_entity_id: u32,
}

/// 攻击动画状态
/// 追踪攻击动画的当前阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackPhase {
    /// 准备阶段（举起武器）
    Prepare,
    /// 挥砍阶段（攻击判定）
    Strike,
    /// 结束阶段（收招恢复）
    Recovery,
}

/// 攻击动画组件
/// 管理攻击动画的播放状态和计时
#[derive(Debug, Component)]
pub struct AttackAnimation {
    /// 当前动画阶段
    pub phase: AttackPhase,
    /// 当前阶段已过时间（秒）
    pub elapsed: f32,
    /// 准备阶段持续时间（秒）
    pub prepare_duration: f32,
    /// 挥砍阶段持续时间（秒）
    pub strike_duration: f32,
    /// 结束阶段持续时间（秒）
    pub recovery_duration: f32,
}

impl Default for AttackAnimation {
    fn default() -> Self {
        Self {
            phase: AttackPhase::Prepare,
            elapsed: 0.0,
            // RO 中普通攻击动画约 400-600ms，这里分三段
            prepare_duration: 0.1,
            strike_duration: 0.2,
            recovery_duration: 0.2,
        }
    }
}

impl AttackAnimation {
    /// 动画是否已播放完毕
    pub fn is_finished(&self) -> bool {
        self.phase == AttackPhase::Recovery
            && self.elapsed >= self.recovery_duration
    }

    /// 获取动画总持续时间（秒）
    pub fn total_duration(&self) -> f32 {
        self.prepare_duration + self.strike_duration + self.recovery_duration
    }
}

/// 伤害数字组件
/// 控制伤害数字的显示、浮动和消失动画
#[derive(Debug, Component)]
pub struct DamageNumber {
    /// 已显示时间（秒）
    pub elapsed: f32,
    /// 总显示持续时间（秒），超时后销毁实体
    pub duration: f32,
    /// 垂直浮动速度（世界单位/秒）
    pub float_speed: f32,
    /// 初始位置（用于浮动偏移计算）
    pub origin_y: f32,
}

impl Default for DamageNumber {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            duration: 1.5,
            float_speed: 2.0,
            origin_y: 0.0,
        }
    }
}

// ============================================================================
// 事件定义
// ============================================================================

/// 伤害显示事件
/// 当收到服务器攻击通知时触发，由 damage_display_system 消费
#[derive(Event)]
pub struct DamageDisplayEvent {
    /// 目标服务器实体 ID（伤害数字显示位置）
    pub target_entity_id: u32,
    /// 伤害值
    pub damage: u32,
    /// 伤害类型（0=普通, 3=暴击等）
    pub damage_type: u8,
}

// ============================================================================
// 系统实现
// ============================================================================

/// 攻击输入系统
/// 检测玩家点击实体（右键），发送攻击请求到服务器
///
/// RO 攻击流程：
/// 1. 玩家右键点击目标实体
/// 2. 客户端发送 CZ_REQUEST_ACT (0x0089) 到服务器
/// 3. 服务器返回 ZC_NOTIFY_ACT (0x008a) 广播攻击结果
pub fn attack_input_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    // TODO: 后续接入射线检测，从鼠标位置拾取 3D 实体
    // 目前先用查询找到最近的怪物作为攻击目标
    player_query: Query<&Player>,
    _commands: Commands,
    map_network: Option<Res<MapNetworkManager>>,
    time: Res<Time>,
) {
    // 仅处理右键点击
    if !mouse_button.just_pressed(MouseButton::Right) {
        return;
    }

    let Some(_map_network) = map_network else {
        return;
    };

    // 获取本地玩家（目前场景中只有一个玩家）
    let Ok(_player) = player_query.get_single() else {
        return;
    };

    // TODO: 射线检测拾取点击的实体
    // 当前简化实现：暂时不发送攻击请求，等待射线检测模块接入
    // 实际使用时需要：
    // 1. 从鼠标位置发射射线
    // 2. 检测射线命中的实体
    // 3. 判断实体类型（怪物/NPC/玩家）
    // 4. 发送攻击请求

    tracing::debug!("右键点击检测到，等待射线检测模块接入");
    let _ = time; // 消除未使用警告，后续射线检测会用到
}

/// 发送攻击请求到服务器
/// 当确定攻击目标后调用此函数
///
/// # 参数
/// * `map_network` - 地图网络管理器
/// * `target_entity_id` - 目标服务器实体 ID
pub fn send_attack_request(map_network: &MapNetworkManager, target_entity_id: u32) {
    let req = AttackRequest {
        target_id: target_entity_id,
        action: 0, // 0 = 普通攻击
    };
    tracing::info!("发送攻击请求: target_id={}", target_entity_id);
    map_network
        .0
        .send_command(NetworkCommand::Send(Packet::AttackRequest(req)));
}

/// 攻击动画系统
/// 更新攻击动画状态，在动画结束后移除 AttackAnimation 组件
pub fn attack_animation_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut AttackAnimation)>,
) {
    let delta = time.delta().as_secs_f32();

    for (entity, mut anim) in query.iter_mut() {
        anim.elapsed += delta;

        // 根据已过时间判断当前阶段
        let total_prepare = anim.prepare_duration;
        let total_strike = total_prepare + anim.strike_duration;

        if anim.elapsed < total_prepare {
            if anim.phase != AttackPhase::Prepare {
                anim.phase = AttackPhase::Prepare;
                tracing::debug!("攻击动画: 准备阶段");
            }
        } else if anim.elapsed < total_strike {
            if anim.phase != AttackPhase::Strike {
                anim.phase = AttackPhase::Strike;
                tracing::debug!("攻击动画: 挥砍阶段");
            }
        } else if anim.elapsed < total_strike + anim.recovery_duration {
            if anim.phase != AttackPhase::Recovery {
                anim.phase = AttackPhase::Recovery;
                tracing::debug!("攻击动画: 结束阶段");
            }
        } else {
            // 动画播放完毕，移除组件
            tracing::debug!("攻击动画: 播放完毕");
            commands.entity(entity).remove::<AttackAnimation>();
        }
    }
}

/// 伤害数字显示系统
/// 监听伤害事件，在目标位置创建浮动伤害数字，并在超时后销毁
pub fn damage_display_system(
    mut commands: Commands,
    time: Res<Time>,
    mut damage_events: EventReader<DamageDisplayEvent>,
    mut damage_query: Query<(Entity, &mut DamageNumber, &mut Transform)>,
    conn_state: Res<MapConnectionState>,
) {
    // 处理新的伤害事件，创建伤害数字实体
    for event in damage_events.read() {
        // 从实体映射表中查找目标实体的位置
        if let Some(&target_entity) = conn_state.entity_map.get(&event.target_entity_id) {
            // 需要通过命令获取目标实体的位置（使用 observe 或下一帧）
            // 简化处理：使用默认位置创建伤害数字
            let damage_text = format!("{}", event.damage);
            let text_color = if event.damage_type == 3 {
                // 暴击：黄色
                Color::srgb(1.0, 1.0, 0.0)
            } else {
                // 普通伤害：白色
                Color::srgb(1.0, 1.0, 1.0)
            };

            // 在目标实体上方创建伤害数字
            // 使用 3D Text 显示，位置在目标头顶
            let initial_y = 1.5; // 头顶偏移

            commands.spawn((
                Text2d::new(damage_text),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(text_color),
                // 伤害数字初始位置（使用目标实体的位置 + 头顶偏移）
                // 由于无法直接读取目标位置，使用 Transform 在下一帧同步
                Transform::from_xyz(0.0, initial_y, 0.0),
                DamageNumber {
                    elapsed: 0.0,
                    duration: 1.5,
                    float_speed: 2.0,
                    origin_y: initial_y,
                },
                // 标记需要与目标实体同步位置
                DamageNumberTarget(target_entity),
            ));

            tracing::debug!(
                "创建伤害数字: target={}, damage={}, type={}",
                event.target_entity_id,
                event.damage,
                event.damage_type
            );
        }
    }

    // 更新现有伤害数字：浮动和消失
    for (entity, mut dmg, mut transform) in damage_query.iter_mut() {
        dmg.elapsed += time.delta().as_secs_f32();

        // 向上浮动
        transform.translation.y += dmg.float_speed * time.delta().as_secs_f32();

        // 超时后销毁
        if dmg.elapsed >= dmg.duration {
            commands.entity(entity).despawn();
        }
    }
}

/// 伤害数字目标实体标记组件
/// 用于将伤害数字与目标实体的位置同步
#[derive(Debug, Component)]
pub struct DamageNumberTarget(pub Entity);

/// 伤害数字位置同步系统
/// 将伤害数字的位置与目标实体同步（每帧更新 X/Z 坐标）
/// TODO: 后续实现伤害数字跟随目标实体移动
pub fn damage_position_sync_system(
    _transform_query: Query<&Transform>,
    _damage_query: Query<(&DamageNumberTarget, &mut Transform), Without<DamageNumberTarget>>,
) {
    // 此系统需要在 damage_display_system 之后运行
    // 简化实现：伤害数字使用世界坐标，在创建时设置正确位置
    // 后续可以通过事件传递位置信息
}
