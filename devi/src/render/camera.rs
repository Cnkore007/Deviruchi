// RO 风格 45° 等距相机系统
// 模拟 Ragnarok Online 的经典等距视角，支持旋转和缩放

use bevy::prelude::*;

/// 相机配置参数
/// 定义了 RO 风格相机的基础参数，包括俯仰角、视野和缩放级别
#[derive(Debug, Clone)]
pub struct CameraConfig {
    /// 俯仰角（度），RO 默认约 30° 等距视角
    pub pitch: f32,
    /// 视野角度（度）
    pub fov: f32,
    /// 可用的缩放距离级别列表，从近到远排列
    pub zoom_levels: Vec<f32>,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            pitch: 30.0,
            fov: 45.0,
            zoom_levels: vec![15.0, 25.0, 40.0],
        }
    }
}

/// RO 风格相机状态
/// 管理相机的旋转角度、缩放级别和跟随目标
#[derive(Debug, Resource)]
pub struct RoCamera {
    /// 相机配置
    pub config: CameraConfig,
    /// 水平旋转角度（度），0° 为正北方向
    pub yaw: f32,
    /// 当前缩放级别索引，对应 config.zoom_levels
    pub zoom_index: usize,
    /// 相机观察的目标点（世界坐标）
    pub target: Vec3,
}

impl RoCamera {
    /// 创建新的 RO 相机实例
    /// 默认缩放级别为中间值（index=1）
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            yaw: 0.0,
            zoom_index: 1,
            target: Vec3::ZERO,
        }
    }

    /// 旋转相机
    /// delta 为旋转增量（度），正值向右旋转，负值向左旋转
    /// 角度自动归一化到 [0, 360) 范围
    pub fn rotate(&mut self, delta: f32) {
        self.yaw = (self.yaw + delta) % 360.0;
        if self.yaw < 0.0 {
            self.yaw += 360.0;
        }
    }

    /// 放大（拉近相机）
    /// 缩放级别索引减小，对应更近的 zoom_level
    pub fn zoom_in(&mut self) {
        if self.zoom_index > 0 {
            self.zoom_index -= 1;
        }
    }

    /// 缩小（拉远相机）
    /// 缩放级别索引增大，对应更远的 zoom_level
    pub fn zoom_out(&mut self) {
        if self.zoom_index < self.config.zoom_levels.len() - 1 {
            self.zoom_index += 1;
        }
    }

    /// 获取当前缩放距离
    pub fn distance(&self) -> f32 {
        self.config.zoom_levels[self.zoom_index]
    }

    /// 计算相机在世界空间中的位置
    /// 基于目标点、俯仰角、偏航角和缩放距离，使用球坐标计算
    pub fn compute_position(&self) -> Vec3 {
        let distance = self.distance();
        let pitch_rad = self.config.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();
        Vec3::new(
            self.target.x + distance * pitch_rad.cos() * yaw_rad.sin(),
            self.target.y + distance * pitch_rad.sin(),
            self.target.z + distance * pitch_rad.cos() * yaw_rad.cos(),
        )
    }

    /// 计算相机的 Transform
    /// 返回包含位置和朝向的完整变换矩阵，相机始终朝向 target
    pub fn compute_transform(&self) -> Transform {
        Transform::from_translation(self.compute_position()).looking_at(self.target, Vec3::Y)
    }
}

/// 相机控制系统
/// 处理键盘输入来旋转（Q/E）和缩放（+/-）相机
/// 每帧更新相机的 Transform 组件
pub fn camera_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera: ResMut<RoCamera>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    // Q 键向左旋转
    if keyboard.pressed(KeyCode::KeyQ) {
        camera.rotate(-2.0);
    }
    // E 键向右旋转
    if keyboard.pressed(KeyCode::KeyE) {
        camera.rotate(2.0);
    }
    // = 键放大
    if keyboard.just_pressed(KeyCode::Equal) {
        camera.zoom_in();
    }
    // - 键缩小
    if keyboard.just_pressed(KeyCode::Minus) {
        camera.zoom_out();
    }

    // 更新所有 3D 相机的变换
    for mut transform in camera_query.iter_mut() {
        *transform = camera.compute_transform();
    }
}
