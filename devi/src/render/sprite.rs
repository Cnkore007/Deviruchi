// 2D 精灵动画系统和 Billboard 朝向
// 实现精灵帧动画播放和始终面向相机的 Billboard 效果

use bevy::prelude::*;

/// 精灵动画状态组件
/// 管理帧动画的播放进度、帧率和循环设置
#[derive(Debug, Component, Default)]
pub struct SpriteAnimation {
    /// 当前播放的帧索引
    pub current_frame: usize,
    /// 动画总帧数
    pub frame_count: usize,
    /// 每帧持续时间（毫秒）
    pub frame_duration_ms: u32,
    /// 当前帧已过去的时间（毫秒）
    pub elapsed_ms: u32,
    /// 动画是否正在播放
    pub playing: bool,
    /// 动画是否循环播放
    pub looping: bool,
}

impl SpriteAnimation {
    /// 创建新的精灵动画
    /// 默认从第 0 帧开始播放，启用循环
    pub fn new(frame_count: usize, frame_duration_ms: u32) -> Self {
        Self {
            current_frame: 0,
            frame_count,
            frame_duration_ms,
            elapsed_ms: 0,
            playing: true,
            looping: true,
        }
    }

    /// 推进动画播放
    /// 根据经过的时间（毫秒）更新当前帧
    /// 支持循环和非循环模式
    pub fn advance(&mut self, delta_ms: u32) {
        if !self.playing || self.frame_count == 0 {
            return;
        }

        self.elapsed_ms += delta_ms;

        // 消耗已过去的时间，推进帧
        while self.elapsed_ms >= self.frame_duration_ms {
            self.elapsed_ms -= self.frame_duration_ms;
            self.current_frame += 1;

            if self.current_frame >= self.frame_count {
                if self.looping {
                    // 循环模式：回到第一帧
                    self.current_frame = 0;
                } else {
                    // 非循环模式：停在最后一帧
                    self.current_frame = self.frame_count - 1;
                    self.playing = false;
                    break;
                }
            }
        }
    }

    /// 重置动画到初始状态
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.elapsed_ms = 0;
        self.playing = true;
    }
}

/// 精灵朝向枚举
/// 对应 RO 的 8 方向，从南（0）开始顺时针排列
/// 用于确定精灵显示哪一方向的贴图
#[derive(Debug, Component, Clone, Copy, PartialEq, Eq)]
pub enum SpriteDirection {
    /// 南（面向屏幕）
    South = 0,
    /// 西南
    SouthWest = 1,
    /// 西
    West = 2,
    /// 西北
    NorthWest = 3,
    /// 北（背向屏幕）
    North = 4,
    /// 东北
    NorthEast = 5,
    /// 东
    East = 6,
    /// 东南
    SouthEast = 7,
}

impl Default for SpriteDirection {
    fn default() -> Self {
        Self::South
    }
}

/// Billboard 精灵组件
/// 标记该实体为始终面向相机的 2D 精灵
#[derive(Debug, Component)]
pub struct BillboardSprite {
    /// 精灵宽度（世界单位）
    pub width: f32,
    /// 精灵高度（世界单位）
    pub height: f32,
    /// 精灵纹理句柄
    pub texture: Handle<Image>,
}

/// 精灵动画更新系统
/// 每帧推进所有 SpriteAnimation 组件的动画进度
pub fn sprite_animation_system(time: Res<Time>, mut query: Query<&mut SpriteAnimation>) {
    let delta_ms = time.delta().as_millis() as u32;
    for mut anim in query.iter_mut() {
        anim.advance(delta_ms);
    }
}

/// Billboard 朝向系统
/// 每帧更新所有 BillboardSprite 的旋转，使其始终面向相机
/// 计算精灵到相机的方向向量，转换为 Y 轴旋转角度
pub fn billboard_system(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut sprite_query: Query<&mut Transform, (With<BillboardSprite>, Without<Camera3d>)>,
) {
    let camera_transform = match camera_query.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };

    for mut transform in sprite_query.iter_mut() {
        // 计算从精灵指向相机的方向向量
        let direction = camera_transform.translation - transform.translation;
        // 使用 atan2 计算水平面内的角度
        let angle = direction.z.atan2(direction.x);
        // 旋转精灵使其面向相机（补偿 90° 偏移）
        transform.rotation = Quat::from_rotation_y(-angle + std::f32::consts::FRAC_PI_2);
    }
}
