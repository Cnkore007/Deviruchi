// GND 地面渲染模块
// 将 GND 网格数据转换为 Bevy 实体，包含 Mesh、Material 和纹理加载
// 纹理从 GRF 归档中加载 BMP 格式图片并转换为 Bevy Image

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use std::path::Path;

use crate::asset::gnd::GndFile;
use crate::asset::grf::GrfArchive;
use crate::render::terrain::{GndMeshBuilder, GndMeshGroup, TerrainMeshBuilder};

/// 地面渲染配置资源
/// 存储 GRF 归档路径，供地面纹理加载使用
#[derive(Resource)]
pub struct GroundConfig {
    /// GRF 文件路径
    pub grf_path: String,
    /// 是否渲染前表面墙壁
    pub render_walls: bool,
}

impl Default for GroundConfig {
    fn default() -> Self {
        Self {
            grf_path: String::new(),
            render_walls: false,
        }
    }
}

/// 地面网格实体标记组件
/// 用于标识地面渲染实体，便于后续查询和清理
#[derive(Component)]
pub struct GroundTile {
    /// 该网格对应的纹理索引
    pub texture_index: u16,
}

/// 从 GND 文件加载并渲染地面
/// 完整管线：解析 GND → 构建网格分组 → 加载纹理 → 创建 Bevy 实体
pub fn spawn_ground(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    gnd: &GndFile,
    config: &GroundConfig,
) {
    // 构建上表面网格分组（按纹理索引分组）
    let surface_groups = GndMeshBuilder::build_groups(gnd);

    // 可选：构建前表面墙壁
    let wall_groups = if config.render_walls {
        GndMeshBuilder::build_front_walls(gnd)
    } else {
        Vec::new()
    };

    // 合并所有网格分组
    let all_groups: Vec<GndMeshGroup> = surface_groups
        .into_iter()
        .chain(wall_groups.into_iter())
        .collect();

    if all_groups.is_empty() {
        tracing::warn!("GND 文件未包含任何可渲染的地面网格");
        return;
    }

    // 加载纹理（去重：同一纹理路径只加载一次）
    let mut texture_cache: HashMap<String, Handle<Image>> = HashMap::new();
    let grf_path = Path::new(&config.grf_path);

    // 尝试打开 GRF 归档用于纹理加载
    let mut grf = if !config.grf_path.is_empty() {
        GrfArchive::open(grf_path).ok()
    } else {
        None
    };

    // 为每个网格分组创建 Bevy 实体
    for group in all_groups {
        // 确定纹理路径（标准化路径分隔符）
        let tex_path = group.texture_path.replace('\\', "/");

        // 加载或复用纹理
        let texture_handle = texture_cache.entry(tex_path.clone()).or_insert_with(|| {
            load_texture_from_grf(&mut grf, &tex_path, images)
                .unwrap_or_else(|| {
                    // 纹理加载失败时使用默认颜色纹理
                    tracing::warn!("无法加载纹理 '{}'，使用默认灰色纹理", tex_path);
                    create_fallback_texture(images)
                })
        });

        // 将 TerrainMesh 转换为 Bevy Mesh
        let bevy_mesh = TerrainMeshBuilder::to_bevy_mesh(&group.mesh);

        // 创建材质（使用标准材质 + 纹理）
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle.clone()),
            // 地面不需要金属感
            metallic: 0.0,
            perceptual_roughness: 1.0,
            // 双面渲染（前表面墙壁可能需要）
            cull_mode: None,
            ..default()
        });

        // 生成实体
        commands.spawn((
            Mesh3d(meshes.add(bevy_mesh)),
            MeshMaterial3d(material),
            GroundTile {
                texture_index: group.texture_index,
            },
        ));
    }

    tracing::info!(
        "GND 地面渲染完成：加载了 {} 个纹理",
        texture_cache.len()
    );
}

/// 从 GRF 归档加载纹理并转换为 Bevy Image
/// 尝试加载 BMP 格式的纹理文件，解码后创建 Bevy 可用的 Image 对象
fn load_texture_from_grf(
    grf: &mut Option<GrfArchive>,
    path: &str,
    images: &mut ResMut<Assets<Image>>,
) -> Option<Handle<Image>> {
    let archive = grf.as_mut()?;

    // 标准化路径：确保使用 data/texture/ 前缀
    let full_path = if path.starts_with("data/") || path.starts_with("Data/") {
        path.to_string()
    } else {
        format!("data/texture/{}", path)
    };

    let entry = archive.find_entry(&full_path)?;
    let entry = entry.clone();
    let file_data = archive.read_file(&entry).ok()?;

    // 使用 image crate 解码纹理（支持 BMP、PNG 等格式）
    let img = image::load_from_memory(&file_data).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // 创建 Bevy Image 对象
    let bevy_image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    Some(images.add(bevy_image))
}

/// 创建回退纹理（4x4 灰色棋盘格）
/// 纹理加载失败时使用，确保地面始终可见
fn create_fallback_texture(images: &mut ResMut<Assets<Image>>) -> Handle<Image> {
    let size = 4;
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            // 棋盘格：浅灰和深灰交替
            let gray = if (x + y) % 2 == 0 { 180u8 } else { 120u8 };
            data.extend_from_slice(&[gray, gray, gray, 255]);
        }
    }

    let image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    images.add(image)
}

/// 清理所有地面渲染实体
/// 切换地图时调用，移除旧的地面网格
pub fn cleanup_ground(
    commands: &mut Commands,
    query: &Query<Entity, With<GroundTile>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
