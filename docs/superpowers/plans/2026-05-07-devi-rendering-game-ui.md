# Devi 客户端渲染管线与游戏逻辑实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现混合渲染管线（3D 地形 + 2D 精灵 billboard）、RO 风格相机、基础游戏逻辑（玩家/怪物实体）、原版布局 UI。

**Architecture:** 自定义 Bevy 渲染管线，地形用 wgpu mesh 渲染，精灵用 billboard quad 在 3D 空间中面向相机。UI 使用 Bevy UI 节点树，固定像素布局匹配原版 RO。

**Tech Stack:** Rust, Bevy 0.15+, wgpu, bevy_render, bevy_ui, bevy_ecs

---

## 文件结构

| 操作 | 文件路径 | 职责 |
|------|----------|------|
| Modify | `devi/src/render/mod.rs` | 渲染模块声明 |
| Create | `devi/src/render/terrain.rs` | 3D 地形渲染 |
| Create | `devi/src/render/sprite.rs` | 2D 精灵 billboard |
| Create | `devi/src/render/camera.rs` | RO 风格相机 |
| Create | `devi/src/render/ui/mod.rs` | UI 模块声明 |
| Create | `devi/src/render/ui/hud.rs` | 血条/SP条/快捷栏 |
| Create | `devi/src/render/ui/chat.rs` | 聊天窗口 |
| Modify | `devi/src/game/mod.rs` | 游戏逻辑模块声明 |
| Create | `devi/src/game/player.rs` | 玩家组件和系统 |
| Create | `devi/src/game/mob.rs` | 怪物组件和系统 |
| Create | `devi/src/game/movement.rs` | 移动和寻路 |
| Modify | `devi/src/main.rs` | 集成渲染和游戏逻辑 |
| Create | `devi/tests/render_test.rs` | 渲染系统测试 |
| Create | `devi/tests/game_test.rs` | 游戏逻辑测试 |

---

### Task 1: RO 风格相机

**Files:**
- Modify: `devi/src/render/mod.rs`
- Create: `devi/src/render/camera.rs`
- Create: `devi/tests/render_test.rs`

- [ ] **Step 1: 编写相机测试**

创建 `devi/tests/render_test.rs`:
```rust
use devi::render::camera::{RoCamera, CameraConfig};

#[test]
fn test_camera_config_default() {
    let config = CameraConfig::default();
    assert!((config.pitch - 30.0_f32).abs() < f32::EPSILON);
    assert!((config.fov - 45.0_f32).abs() < f32::EPSILON);
    assert_eq!(config.zoom_levels.len(), 3);
}

#[test]
fn test_camera_rotation() {
    let mut camera = RoCamera::new(CameraConfig::default());
    assert!((camera.yaw - 0.0_f32).abs() < f32::EPSILON);

    camera.rotate(90.0);
    assert!((camera.yaw - 90.0_f32).abs() < f32::EPSILON);

    camera.rotate(300.0);
    assert!((camera.yaw - 30.0_f32).abs() < f32::EPSILON); // 360 度回绕
}

#[test]
fn test_camera_zoom() {
    let mut camera = RoCamera::new(CameraConfig::default());
    assert_eq!(camera.zoom_index, 1); // 默认中等缩放

    camera.zoom_in();
    assert_eq!(camera.zoom_index, 0);

    camera.zoom_out();
    camera.zoom_out();
    assert_eq!(camera.zoom_index, 2); // 最远

    camera.zoom_out(); // 已经是最大，不变
    assert_eq!(camera.zoom_index, 2);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test render_test`
Expected: FAIL，编译错误

- [ ] **Step 3: 实现相机系统**

修改 `devi/src/render/mod.rs`:
```rust
// 渲染管线模块
pub mod camera;
pub mod terrain;
pub mod sprite;
pub mod ui;
```

创建 `devi/src/render/camera.rs`:
```rust
use bevy::prelude::*;

/// 相机配置
#[derive(Debug, Clone)]
pub struct CameraConfig {
    /// 俯仰角（度）
    pub pitch: f32,
    /// 视场角（度）
    pub fov: f32,
    /// 缩放级别列表（距离值）
    pub zoom_levels: Vec<f32>,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            pitch: 30.0,
            fov: 45.0,
            zoom_levels: vec![15.0, 25.0, 40.0], // 近、中、远
        }
    }
}

/// RO 风格相机
#[derive(Debug)]
pub struct RoCamera {
    /// 配置
    pub config: CameraConfig,
    /// 偏航角（度，0-360）
    pub yaw: f32,
    /// 当前缩放级别索引
    pub zoom_index: usize,
    /// 目标位置（跟随玩家）
    pub target: Vec3,
}

impl RoCamera {
    /// 创建新相机
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            yaw: 0.0,
            zoom_index: 1, // 默认中等缩放
            target: Vec3::ZERO,
        }
    }

    /// 旋转相机
    pub fn rotate(&mut self, delta: f32) {
        self.yaw = (self.yaw + delta) % 360.0;
        if self.yaw < 0.0 {
            self.yaw += 360.0;
        }
    }

    /// 放大（拉近）
    pub fn zoom_in(&mut self) {
        if self.zoom_index > 0 {
            self.zoom_index -= 1;
        }
    }

    /// 缩小（拉远）
    pub fn zoom_out(&mut self) {
        if self.zoom_index < self.config.zoom_levels.len() - 1 {
            self.zoom_index += 1;
        }
    }

    /// 获取当前距离
    pub fn distance(&self) -> f32 {
        self.config.zoom_levels[self.zoom_index]
    }

    /// 计算相机位置（基于目标、偏航角、俯仰角、距离）
    pub fn compute_position(&self) -> Vec3 {
        let distance = self.distance();
        let pitch_rad = self.config.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();

        let x = self.target.x + distance * pitch_rad.cos() * yaw_rad.sin();
        let y = self.target.y + distance * pitch_rad.sin();
        let z = self.target.z + distance * pitch_rad.cos() * yaw_rad.cos();

        Vec3::new(x, y, z)
    }

    /// 获取相机变换（用于 Bevy）
    pub fn compute_transform(&self) -> Transform {
        let position = self.compute_position();
        Transform::from_translation(position)
            .looking_at(self.target, Vec3::Y)
    }
}

/// 相机系统：根据输入更新相机
pub fn camera_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera: ResMut<RoCamera>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    // 旋转：Q/E 键
    if keyboard.pressed(KeyCode::KeyQ) {
        camera.rotate(-2.0);
    }
    if keyboard.pressed(KeyCode::KeyE) {
        camera.rotate(2.0);
    }

    // 缩放：+/- 键
    if keyboard.just_pressed(KeyCode::Equal) {
        camera.zoom_in();
    }
    if keyboard.just_pressed(KeyCode::Minus) {
        camera.zoom_out();
    }

    // 更新相机变换
    for mut transform in camera_query.iter_mut() {
        *transform = camera.compute_transform();
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test render_test`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/render/mod.rs devi/src/render/camera.rs devi/tests/render_test.rs
git commit -m "feat(devi/render): 实现 RO 风格 45° 等距相机，支持旋转和缩放"
```

---

### Task 2: 3D 地形渲染

**Files:**
- Create: `devi/src/render/terrain.rs`
- Modify: `devi/tests/render_test.rs`

- [ ] **Step 1: 编写地形 mesh 生成测试**

在 `devi/tests/render_test.rs` 中追加:
```rust
use devi::render::terrain::TerrainMeshBuilder;
use devi::asset::gat::{GatFile, GatCell};

#[test]
fn test_terrain_mesh_builder_2x2() {
    // 创建一个 2x2 的 GAT 数据
    let cells = vec![
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
        GatCell { heights: [0.0, 0.0, 0.0, 0.0], cell_type: 0 },
    ];
    let gat = GatFile {
        version: 0x0102,
        width: 2,
        height: 2,
        cells,
    };

    let mesh = TerrainMeshBuilder::build(&gat);
    // 2x2 网格 = 4 个 cell，每个 cell 2 个三角形 = 8 个三角形 = 24 个顶点
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.indices.len(), 24); // 8 三角形 * 3 顶点
}

#[test]
fn test_terrain_mesh_heights() {
    let cells = vec![
        GatCell { heights: [0.0, 1.0, 2.0, 3.0], cell_type: 0 },
    ];
    let gat = GatFile {
        version: 0x0102,
        width: 1,
        height: 1,
        cells,
    };

    let mesh = TerrainMeshBuilder::build(&gat);
    // 验证顶点高度值
    assert!((mesh.vertices[0].position[1] - 0.0).abs() < f32::EPSILON);
    assert!((mesh.vertices[1].position[1] - 1.0).abs() < f32::EPSILON);
    assert!((mesh.vertices[2].position[1] - 2.0).abs() < f32::EPSILON);
    assert!((mesh.vertices[3].position[1] - 3.0).abs() < f32::EPSILON);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test render_test`
Expected: FAIL，`unresolved import devi::render::terrain`

- [ ] **Step 3: 实现地形 mesh 构建器**

创建 `devi/src/render/terrain.rs`:
```rust
use devi::asset::gat::{GatFile, GatCell};

/// 地形顶点
#[derive(Debug, Clone)]
pub struct TerrainVertex {
    /// 位置 (x, y, z)
    pub position: [f32; 3],
    /// 法线 (x, y, z)
    pub normal: [f32; 3],
    /// UV 坐标 (u, v)
    pub uv: [f32; 2],
}

/// 地形网格数据
#[derive(Debug)]
pub struct TerrainMesh {
    /// 顶点列表
    pub vertices: Vec<TerrainVertex>,
    /// 索引列表（三角形）
    pub indices: Vec<u32>,
}

/// 地形网格构建器
pub struct TerrainMeshBuilder;

impl TerrainMeshBuilder {
    /// 从 GAT 数据构建地形网格
    pub fn build(gat: &GatFile) -> TerrainMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let cell_size = 1.0; // 每个 cell 的世界大小

        for y in 0..gat.height {
            for x in 0..gat.width {
                if let Some(cell) = gat.get_cell(x, y) {
                    let base_x = x as f32 * cell_size;
                    let base_y = y as f32 * cell_size;

                    Self::build_cell(
                        &mut vertices,
                        &mut indices,
                        base_x,
                        base_y,
                        cell_size,
                        cell,
                    );
                }
            }
        }

        TerrainMesh { vertices, indices }
    }

    /// 构建单个 cell 的两个三角形
    fn build_cell(
        vertices: &mut Vec<TerrainVertex>,
        indices: &mut Vec<u32>,
        base_x: f32,
        base_y: f32,
        size: f32,
        cell: &GatCell,
    ) {
        let base_index = vertices.len() as u32;

        // 四个角的顶点（RO GAT 高度顺序：左下、右下、左上、右上）
        // GAT 的 Y 轴是反的，需要翻转
        let corners = [
            // 左下 (0)
            TerrainVertex {
                position: [base_x, cell.heights[0], base_y + size],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
            // 右下 (1)
            TerrainVertex {
                position: [base_x + size, cell.heights[1], base_y + size],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
            },
            // 左上 (2)
            TerrainVertex {
                position: [base_x, cell.heights[2], base_y],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
            },
            // 右上 (3)
            TerrainVertex {
                position: [base_x + size, cell.heights[3], base_y],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
            },
        ];

        vertices.extend_from_slice(&corners);

        // 三角形 1: 左下 → 右下 → 左上
        indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
        ]);

        // 三角形 2: 右下 → 右上 → 左上
        indices.extend_from_slice(&[
            base_index + 1,
            base_index + 3,
            base_index + 2,
        ]);
    }

    /// 将 TerrainMesh 转换为 Bevy Mesh
    pub fn to_bevy_mesh(mesh: &TerrainMesh) -> bevy::render::mesh::Mesh {
        use bevy::render::mesh::{Indices, Mesh};
        use bevy::render::render_resource::PrimitiveTopology;

        let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList, bevy::render::render_asset::RenderAssetUsages::default());

        let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();
        let normals: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.normal).collect();
        let uvs: Vec<[f32; 2]> = mesh.vertices.iter().map(|v| v.uv).collect();

        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        bevy_mesh.insert_indices(Indices::U32(mesh.indices.clone()));

        bevy_mesh
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test render_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/render/terrain.rs devi/tests/render_test.rs
git commit -m "feat(devi/render): 实现地形网格构建器，GAT → Bevy Mesh 转换"
```

---

### Task 3: 2D 精灵 Billboard 渲染

**Files:**
- Create: `devi/src/render/sprite.rs`
- Modify: `devi/tests/render_test.rs`

- [ ] **Step 1: 编写精灵组件测试**

在 `devi/tests/render_test.rs` 中追加:
```rust
use devi::render::sprite::SpriteAnimation;
use devi::asset::action::FrameSprite;

#[test]
fn test_sprite_animation_default() {
    let anim = SpriteAnimation::default();
    assert_eq!(anim.current_frame, 0);
    assert_eq!(anim.frame_count, 0);
    assert!(!anim.playing);
}

#[test]
fn test_sprite_animation_advance() {
    let mut anim = SpriteAnimation {
        frame_count: 4,
        frame_duration_ms: 100,
        ..Default::default()
    };
    anim.playing = true;

    // 模拟时间推进
    anim.advance(150); // 1.5 帧时间
    assert_eq!(anim.current_frame, 1);

    anim.advance(200); // 再过 2 帧
    assert_eq!(anim.current_frame, 3);

    anim.advance(100); // 循环回到 0
    assert_eq!(anim.current_frame, 0);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test render_test`
Expected: FAIL，`unresolved import devi::render::sprite`

- [ ] **Step 3: 实现精灵系统**

创建 `devi/src/render/sprite.rs`:
```rust
use bevy::prelude::*;

/// 精灵动画组件
#[derive(Debug, Component, Default)]
pub struct SpriteAnimation {
    /// 当前帧索引
    pub current_frame: usize,
    /// 总帧数
    pub frame_count: usize,
    /// 每帧持续时间（毫秒）
    pub frame_duration_ms: u32,
    /// 累计时间（毫秒）
    pub elapsed_ms: u32,
    /// 是否正在播放
    pub playing: bool,
    /// 是否循环
    pub looping: bool,
}

impl SpriteAnimation {
    /// 创建新的动画
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

    /// 推进动画时间
    pub fn advance(&mut self, delta_ms: u32) {
        if !self.playing || self.frame_count == 0 {
            return;
        }

        self.elapsed_ms += delta_ms;

        while self.elapsed_ms >= self.frame_duration_ms {
            self.elapsed_ms -= self.frame_duration_ms;
            self.current_frame += 1;

            if self.current_frame >= self.frame_count {
                if self.looping {
                    self.current_frame = 0;
                } else {
                    self.current_frame = self.frame_count - 1;
                    self.playing = false;
                    break;
                }
            }
        }
    }

    /// 重置动画
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.elapsed_ms = 0;
        self.playing = true;
    }
}

/// 精灵朝向组件（8方向）
#[derive(Debug, Component, Clone, Copy, PartialEq, Eq)]
pub enum SpriteDirection {
    South = 0,
    SouthWest = 1,
    West = 2,
    NorthWest = 3,
    North = 4,
    NorthEast = 5,
    East = 6,
    SouthEast = 7,
}

impl Default for SpriteDirection {
    fn default() -> Self {
        Self::South
    }
}

/// Billboard 精灵组件
/// 标记需要始终面向相机的实体
#[derive(Debug, Component)]
pub struct BillboardSprite {
    /// 精灵宽度（世界单位）
    pub width: f32,
    /// 精灵高度（世界单位）
    pub height: f32,
    /// 纹理句柄
    pub texture: Handle<Image>,
}

/// 精灵动画系统
pub fn sprite_animation_system(
    time: Res<Time>,
    mut query: Query<&mut SpriteAnimation>,
) {
    let delta_ms = time.delta().as_millis() as u32;
    for mut anim in query.iter_mut() {
        anim.advance(delta_ms);
    }
}

/// Billboard 系统：让精灵始终面向相机
pub fn billboard_system(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut sprite_query: Query<&mut Transform, (With<BillboardSprite>, Without<Camera3d>)>,
) {
    let camera_transform = match camera_query.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };

    for mut transform in sprite_query.iter_mut() {
        // 让精灵面向相机方向（只绕 Y 轴旋转）
        let direction = camera_transform.translation - transform.translation;
        let angle = direction.z.atan2(direction.x);
        transform.rotation = Quat::from_rotation_y(-angle + std::f32::consts::FRAC_PI_2);
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test render_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/render/sprite.rs devi/tests/render_test.rs
git commit -m "feat(devi/render): 实现精灵动画系统和 Billboard 朝向系统"
```

---

### Task 4: 游戏实体系统

**Files:**
- Modify: `devi/src/game/mod.rs`
- Create: `devi/src/game/player.rs`
- Create: `devi/src/game/mob.rs`
- Create: `devi/src/game/movement.rs`
- Create: `devi/tests/game_test.rs`

- [ ] **Step 1: 编写游戏实体测试**

创建 `devi/tests/game_test.rs`:
```rust
use devi::game::player::Player;
use devi::game::mob::Mob;
use devi::game::movement::Movement;

#[test]
fn test_player_creation() {
    let player = Player::new(1, "TestPlayer".to_string());
    assert_eq!(player.entity_id, 1);
    assert_eq!(player.name, "TestPlayer");
    assert_eq!(player.base_level, 1);
}

#[test]
fn test_mob_creation() {
    let mob = Mob::new(100, 1001, "Poring".to_string());
    assert_eq!(mob.entity_id, 100);
    assert_eq!(mob.mob_id, 1001);
    assert_eq!(mob.name, "Poring");
}

#[test]
fn test_movement_pathfinding() {
    let mut movement = Movement::new(10.0);
    assert!(!movement.is_moving());

    movement.set_destination(5.0, 5.0);
    assert!(movement.is_moving());
    assert_eq!(movement.destination, Some((5.0, 5.0)));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test game_test`
Expected: FAIL，编译错误

- [ ] **Step 3: 实现游戏模块**

修改 `devi/src/game/mod.rs`:
```rust
// 游戏逻辑模块
pub mod player;
pub mod mob;
pub mod movement;
```

创建 `devi/src/game/player.rs`:
```rust
use bevy::prelude::*;

/// 玩家组件
#[derive(Debug, Component)]
pub struct Player {
    /// 实体 ID（服务端分配）
    pub entity_id: u32,
    /// 角色名称
    pub name: String,
    /// 基础等级
    pub base_level: u32,
    /// 职业等级
    pub job_level: u32,
    /// 当前 HP
    pub hp: u32,
    /// 最大 HP
    pub max_hp: u32,
    /// 当前 SP
    pub sp: u32,
    /// 最大 SP
    pub max_sp: u32,
    /// 移动速度
    pub speed: u16,
}

impl Player {
    /// 创建新玩家
    pub fn new(entity_id: u32, name: String) -> Self {
        Self {
            entity_id,
            name,
            base_level: 1,
            job_level: 1,
            hp: 100,
            max_hp: 100,
            sp: 50,
            max_sp: 50,
            speed: 150, // 默认移动速度
        }
    }
}
```

创建 `devi/src/game/mob.rs`:
```rust
use bevy::prelude::*;

/// 怪物组件
#[derive(Debug, Component)]
pub struct Mob {
    /// 实体 ID（服务端分配）
    pub entity_id: u32,
    /// 怪物 ID（mob_db 中的 ID）
    pub mob_id: u32,
    /// 怪物名称
    pub name: String,
    /// 当前 HP
    pub hp: u32,
    /// 最大 HP
    pub max_hp: u32,
    /// 移动速度
    pub speed: u16,
    /// AI 状态
    pub ai_state: MobAiState,
}

/// 怪物 AI 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobAiState {
    /// 空闲
    Idle,
    /// 追击
    Chase,
    /// 攻击
    Attack,
    /// 返回
    Return,
}

impl Default for MobAiState {
    fn default() -> Self {
        Self::Idle
    }
}

impl Mob {
    /// 创建新怪物
    pub fn new(entity_id: u32, mob_id: u32, name: String) -> Self {
        Self {
            entity_id,
            mob_id,
            name,
            hp: 100,
            max_hp: 100,
            speed: 200,
            ai_state: MobAiState::Idle,
        }
    }
}
```

创建 `devi/src/game/movement.rs`:
```rust
use bevy::prelude::*;

/// 移动组件
#[derive(Debug, Component)]
pub struct Movement {
    /// 移动速度（单位/秒）
    pub speed: f32,
    /// 目标位置
    pub destination: Option<(f32, f32)>,
    /// 当前路径点列表
    pub path: Vec<(f32, f32)>,
}

impl Movement {
    /// 创建新移动组件
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            destination: None,
            path: Vec::new(),
        }
    }

    /// 设置目标位置
    pub fn set_destination(&mut self, x: f32, y: f32) {
        self.destination = Some((x, y));
        // 简单直线移动，后续可接入 A* 寻路
        self.path = vec![(x, y)];
    }

    /// 是否正在移动
    pub fn is_moving(&self) -> bool {
        self.destination.is_some()
    }

    /// 停止移动
    pub fn stop(&mut self) {
        self.destination = None;
        self.path.clear();
    }
}

/// 移动系统：更新实体位置
pub fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Movement)>,
) {
    let delta = time.delta().as_secs_f32();

    for (mut transform, mut movement) in query.iter_mut() {
        if !movement.is_moving() {
            continue;
        }

        if let Some(&(dest_x, dest_y)) = movement.path.first() {
            let current_x = transform.translation.x;
            let current_y = transform.translation.z; // Bevy Z 对应 RO Y

            let dx = dest_x - current_x;
            let dy = dest_y - current_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < 0.1 {
                // 到达目标点
                movement.path.remove(0);
                if movement.path.is_empty() {
                    movement.destination = None;
                }
                continue;
            }

            let move_amount = movement.speed * delta;
            let ratio = (move_amount / distance).min(1.0);

            transform.translation.x += dx * ratio;
            transform.translation.z += dy * ratio;
        }
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test game_test`
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/game/ devi/tests/game_test.rs
git commit -m "feat(devi/game): 实现玩家/怪物实体组件和移动系统"
```

---

### Task 5: UI 布局（原版 RO 风格）

**Files:**
- Create: `devi/src/render/ui/mod.rs`
- Create: `devi/src/render/ui/hud.rs`
- Create: `devi/src/render/ui/chat.rs`

- [ ] **Step 1: 编写 UI 布局测试**

在 `devi/tests/render_test.rs` 中追加:
```rust
use devi::render::ui::hud::HudLayout;

#[test]
fn test_hud_layout_default() {
    let layout = HudLayout::default();
    assert_eq!(layout.screen_width, 1024);
    assert_eq!(layout.screen_height, 768);
    assert_eq!(layout.status_bar_height, 40);
    assert_eq!(layout.chat_window_width, 300);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd devi && cargo test --test render_test`
Expected: FAIL，`unresolved import devi::render::ui`

- [ ] **Step 3: 实现 UI 模块**

创建 `devi/src/render/ui/mod.rs`:
```rust
// UI 渲染模块
pub mod hud;
pub mod chat;
```

创建 `devi/src/render/ui/hud.rs`:
```rust
use bevy::prelude::*;

/// HUD 布局配置
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
    /// 迷你地图大小
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

/// 底部状态栏组件
#[derive(Debug, Component)]
pub struct StatusBar;

/// HP 条组件
#[derive(Debug, Component)]
pub struct HpBar {
    pub current: u32,
    pub max: u32,
}

/// SP 条组件
#[derive(Debug, Component)]
pub struct SpBar {
    pub current: u32,
    pub max: u32,
}

/// 构建 HUD 布局
pub fn build_hud(commands: &mut Commands, layout: &HudLayout) {
    // 根节点：全屏覆盖
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        ..default()
    })
    .with_children(|parent| {
        // 游戏视口（占据大部分屏幕）
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                ..default()
            },
            ..default()
        });

        // 底部面板
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(layout.status_bar_height),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.8).into(),
            ..default()
        })
        .with_children(|parent| {
            // 左侧：HP/SP 条
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                // HP 条
                parent.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(14.0),
                            ..default()
                        },
                        background_color: Color::srgb(0.8, 0.2, 0.2).into(),
                        ..default()
                    },
                    HpBar { current: 100, max: 100 },
                ));

                // SP 条
                parent.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(14.0),
                            margin: UiRect::top(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.2, 0.8).into(),
                        ..default()
                    },
                    SpBar { current: 50, max: 50 },
                ));
            });

            // 右侧：基础信息
            parent.spawn(TextBundle::from_section(
                "Lv.1 | HP: 100/100 | SP: 50/50",
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
    });
}
```

创建 `devi/src/render/ui/chat.rs`:
```rust
use bevy::prelude::*;

/// 聊天消息组件
#[derive(Debug, Component)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: f64,
}

/// 聊天窗口组件
#[derive(Debug, Component)]
pub struct ChatWindow;

/// 构建聊天窗口
pub fn build_chat_window(commands: &mut Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(40.0), // 在状态栏上方
                width: Val::Px(300.0),
                height: Val::Px(200.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.6).into(),
            ..default()
        },
        ChatWindow,
    ))
    .with_children(|parent| {
        // 聊天消息区域
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::ColumnReverse, // 新消息在底部
                overflow: Overflow::clip_y(),
                ..default()
            },
            ..default()
        });

        // 输入框
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(24.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
            background_color: Color::srgba(0.2, 0.2, 0.2, 0.8).into(),
            ..default()
        });
    });
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd devi && cargo test --test render_test`
Expected: 所有测试 PASS

- [ ] **Step 5: Commit**

```bash
git add devi/src/render/ui/ devi/tests/render_test.rs
git commit -m "feat(devi/render/ui): 实现原版 RO 布局 HUD 和聊天窗口"
```

---

### Task 6: 主程序集成

**Files:**
- Modify: `devi/src/main.rs`

- [ ] **Step 1: 集成所有系统到 main.rs**

修改 `devi/src/main.rs`:
```rust
use bevy::prelude::*;
use devi::core::state::GameState;
use devi::core::config::ClientConfig;
use devi::render::camera::{RoCamera, CameraConfig, camera_system};
use devi::render::terrain::TerrainMeshBuilder;
use devi::render::sprite::{sprite_animation_system, billboard_system};
use devi::game::movement::movement_system;
use devi::render::ui::hud::{build_hud, HudLayout};
use devi::render::ui::chat::build_chat_window;

fn main() {
    let config = ClientConfig::default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Devi - Ragnarok Online".to_string(),
                resolution: (config.window_width as f32, config.window_height as f32).into(),
                ..default()
            }),
            ..default()
        }))
        .init_state::<GameState>()
        .insert_resource(config)
        .insert_resource(RoCamera::new(CameraConfig::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            camera_system,
            sprite_animation_system,
            billboard_system,
            movement_system,
            log_state_changes,
        ).run_if(in_state(GameState::InGame)))
        .run();
}

fn setup(mut commands: Commands) {
    // 3D 相机
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 20.0, 20.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // 光照
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -45.0_f32.to_radians(),
            45.0_f32.to_radians(),
            0.0,
        )),
        ..default()
    });

    // UI
    let layout = HudLayout::default();
    build_hud(&mut commands, &layout);
    build_chat_window(&mut commands);
}

fn log_state_changes(state: Res<State<GameState>>) {
    if state.is_changed() {
        tracing::info!("游戏状态切换: {:?}", state.get());
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd devi && cargo check`
Expected: 编译通过

- [ ] **Step 3: Commit**

```bash
git add devi/src/main.rs
git commit -m "feat(devi): 集成渲染管线、游戏逻辑和 UI 到主程序"
```
