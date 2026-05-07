// 渲染模块单元测试
// 测试 RO 相机、地形网格构建器和精灵动画系统

use devi::render::camera::{RoCamera, CameraConfig};
use devi::render::terrain::TerrainMeshBuilder;
use devi::render::sprite::SpriteAnimation;
use devi::asset::gat::{GatFile, GatCell};

/// 测试相机默认配置
/// 验证俯仰角、视野和缩放级别的默认值
#[test]
fn test_camera_config_default() {
    let config = CameraConfig::default();
    assert!((config.pitch - 30.0_f32).abs() < f32::EPSILON);
    assert!((config.fov - 45.0_f32).abs() < f32::EPSILON);
    assert_eq!(config.zoom_levels.len(), 3);
}

/// 测试相机旋转功能
/// 验证旋转角度的累加和环绕（超过 360° 回绕）
#[test]
fn test_camera_rotation() {
    let mut camera = RoCamera::new(CameraConfig::default());
    assert!((camera.yaw - 0.0_f32).abs() < f32::EPSILON);

    // 旋转 90°
    camera.rotate(90.0);
    assert!((camera.yaw - 90.0_f32).abs() < f32::EPSILON);

    // 再旋转 300°，总计 390°，应环绕为 30°
    camera.rotate(300.0);
    assert!((camera.yaw - 30.0_f32).abs() < f32::EPSILON);
}

/// 测试相机缩放功能
/// 验证放大/缩小的边界限制
#[test]
fn test_camera_zoom() {
    let mut camera = RoCamera::new(CameraConfig::default());
    assert_eq!(camera.zoom_index, 1);

    // 放大到最近级别
    camera.zoom_in();
    assert_eq!(camera.zoom_index, 0);

    // 缩小到最远级别
    camera.zoom_out();
    camera.zoom_out();
    assert_eq!(camera.zoom_index, 2);

    // 尝试继续缩小，应保持在最大值
    camera.zoom_out();
    assert_eq!(camera.zoom_index, 2);
}

/// 测试 2x2 地形网格构建
/// 验证 4 个单元格生成正确数量的顶点和索引
#[test]
fn test_terrain_mesh_builder_2x2() {
    let cells = vec![
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
    ];
    let gat = GatFile { version: 0x0201, width: 2, height: 2, cells };
    let mesh = TerrainMeshBuilder::build(&gat);

    // 4 个单元格 * 4 个顶点 = 16 个顶点
    assert_eq!(mesh.vertices.len(), 16);
    // 4 个单元格 * 2 个三角形 * 3 个索引 = 24 个索引
    assert_eq!(mesh.indices.len(), 24);
}

/// 测试地形网格高度值
/// 验证 GAT 单元格的高度值正确映射到顶点 Y 坐标
#[test]
fn test_terrain_mesh_heights() {
    let cells = vec![GatCell { heights: [0.0, 1.0, 2.0, 3.0], cell_type: 0 }];
    let gat = GatFile { version: 0x0201, width: 1, height: 1, cells };
    let mesh = TerrainMeshBuilder::build(&gat);

    // 验证四个角的高度值
    assert!((mesh.vertices[0].position[1] - 0.0).abs() < f32::EPSILON);
    assert!((mesh.vertices[1].position[1] - 1.0).abs() < f32::EPSILON);
    assert!((mesh.vertices[2].position[1] - 2.0).abs() < f32::EPSILON);
    assert!((mesh.vertices[3].position[1] - 3.0).abs() < f32::EPSILON);
}

/// 测试精灵动画默认状态
/// 验证默认动画处于非播放状态
#[test]
fn test_sprite_animation_default() {
    let anim = SpriteAnimation::default();
    assert_eq!(anim.current_frame, 0);
    assert_eq!(anim.frame_count, 0);
    assert!(!anim.playing);
}

/// 测试精灵动画帧推进
/// 验证帧推进、循环播放和时间累积
#[test]
fn test_sprite_animation_advance() {
    let mut anim = SpriteAnimation {
        frame_count: 4,
        frame_duration_ms: 100,
        playing: true,
        looping: true,
        ..Default::default()
    };

    // 推进 150ms，应前进 1 帧（剩余 50ms 累积）
    anim.advance(150);
    assert_eq!(anim.current_frame, 1);

    // 推进 200ms，当前累积 50+200=250ms，应前进 2 帧到第 3 帧
    anim.advance(200);
    assert_eq!(anim.current_frame, 3);

    // 推进 100ms，应循环回到第 0 帧
    anim.advance(100);
    assert_eq!(anim.current_frame, 0);
}
