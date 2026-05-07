// 移动系统
// 实现游戏实体的路径移动逻辑，支持多点路径和帧同步更新

use bevy::prelude::*;

/// 移动组件
/// 管理实体的移动目标、路径和速度
#[derive(Debug, Component)]
pub struct Movement {
    /// 移动速度（世界单位/秒）
    pub speed: f32,
    /// 最终目标位置（世界坐标 x, z 平面）
    pub destination: Option<(f32, f32)>,
    /// 路径点列表，按顺序移动到每个点
    pub path: Vec<(f32, f32)>,
}

impl Movement {
    /// 创建新的移动组件
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            destination: None,
            path: Vec::new(),
        }
    }

    /// 设置移动目标
    /// 清空当前路径并设置新的目标点
    pub fn set_destination(&mut self, x: f32, y: f32) {
        self.destination = Some((x, y));
        self.path = vec![(x, y)];
    }

    /// 是否正在移动
    pub fn is_moving(&self) -> bool {
        self.destination.is_some()
    }

    /// 停止移动
    /// 清空目标和路径
    pub fn stop(&mut self) {
        self.destination = None;
        self.path.clear();
    }
}

/// 移动系统
/// 每帧更新所有带 Movement 组件的实体位置
/// 在 x-z 平面上沿路径点移动，到达一个点后自动切换到下一个
pub fn movement_system(time: Res<Time>, mut query: Query<(&mut Transform, &mut Movement)>) {
    let delta = time.delta().as_secs_f32();
    for (mut transform, mut movement) in query.iter_mut() {
        if !movement.is_moving() {
            continue;
        }
        // 取路径中的第一个目标点
        if let Some(&(dest_x, dest_y)) = movement.path.first() {
            let dx = dest_x - transform.translation.x;
            let dy = dest_y - transform.translation.z;
            let distance = (dx * dx + dy * dy).sqrt();
            // 距离足够近时，视为到达该路径点
            if distance < 0.1 {
                movement.path.remove(0);
                if movement.path.is_empty() {
                    movement.destination = None;
                }
                continue;
            }
            // 按速度和帧时间计算本帧移动量
            let move_amount = movement.speed * delta;
            let ratio = (move_amount / distance).min(1.0);
            transform.translation.x += dx * ratio;
            transform.translation.z += dy * ratio;
        }
    }
}
