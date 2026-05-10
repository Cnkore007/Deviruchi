// 3D 地形网格构建器
// 支持两种数据源：
// - GAT：纯高度网格，用于碰撞检测和基础地形
// - GND：带纹理和 UV 的地面网格，用于渲染带贴图的地面

use std::collections::HashMap;

use crate::asset::gat::{GatCell, GatFile};
use crate::asset::gnd::GndFile;

/// 地形顶点数据
/// 包含位置、法线和 UV 坐标，用于渲染地形表面
#[derive(Debug, Clone)]
pub struct TerrainVertex {
    /// 顶点世界坐标 [x, y, z]
    pub position: [f32; 3],
    /// 顶点法线方向 [nx, ny, nz]，默认朝上
    pub normal: [f32; 3],
    /// 纹理坐标 [u, v]
    pub uv: [f32; 2],
}

/// 地形网格数据
/// 包含顶点列表和三角形索引列表，可直接转换为 Bevy Mesh
#[derive(Debug)]
pub struct TerrainMesh {
    /// 所有顶点数据
    pub vertices: Vec<TerrainVertex>,
    /// 三角形索引列表（每三个索引构成一个三角形）
    pub indices: Vec<u32>,
}

/// GND 网格分组
/// 按纹理索引分组的网格数据，每个分组对应一个独立的 Bevy 实体（材质不同）
#[derive(Debug)]
pub struct GndMeshGroup {
    /// 纹理索引（对应 GndFile::textures 的下标）
    pub texture_index: u16,
    /// 纹理路径（如 "ground/brock01.bmp"）
    pub texture_path: String,
    /// 该纹理对应的网格数据
    pub mesh: TerrainMesh,
}

/// 地形网格构建器（GAT 数据源）
/// 从 GAT 文件数据生成可渲染的地形网格
pub struct TerrainMeshBuilder;

impl TerrainMeshBuilder {
    /// 从 GAT 文件构建地形网格
    /// 遍历所有单元格，为每个单元格生成 4 个顶点和 2 个三角形
    pub fn build(gat: &GatFile) -> TerrainMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let cell_size = 1.0;

        for y in 0..gat.height {
            for x in 0..gat.width {
                if let Some(cell) = gat.get_cell(x, y) {
                    let base_x = x as f32 * cell_size;
                    let base_y = y as f32 * cell_size;
                    Self::build_cell(&mut vertices, &mut indices, base_x, base_y, cell_size, cell);
                }
            }
        }

        TerrainMesh { vertices, indices }
    }

    /// 为单个 GAT 单元格构建几何体
    /// 生成 4 个角顶点和 2 个三角形（共 6 个索引）
    /// 顶点顺序：左下、右下、左上、右上
    fn build_cell(
        vertices: &mut Vec<TerrainVertex>,
        indices: &mut Vec<u32>,
        base_x: f32,
        base_y: f32,
        size: f32,
        cell: &GatCell,
    ) {
        let base_index = vertices.len() as u32;

        // 四个角的顶点，高度取自 GAT 单元格
        let corners = [
            // 左下角
            TerrainVertex {
                position: [base_x, cell.heights[0], base_y + size],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
            // 右下角
            TerrainVertex {
                position: [base_x + size, cell.heights[1], base_y + size],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
            },
            // 左上角
            TerrainVertex {
                position: [base_x, cell.heights[2], base_y],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
            },
            // 右上角
            TerrainVertex {
                position: [base_x + size, cell.heights[3], base_y],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
            },
        ];

        vertices.extend_from_slice(&corners);

        // 第一个三角形：左下 -> 右下 -> 左上
        indices.extend_from_slice(&[base_index, base_index + 1, base_index + 2]);
        // 第二个三角形：右下 -> 右上 -> 左上
        indices.extend_from_slice(&[base_index + 1, base_index + 3, base_index + 2]);
    }

    /// 将 TerrainMesh 转换为 Bevy 可渲染的 Mesh
    /// 设置顶点位置、法线、UV 坐标和三角形索引
    pub fn to_bevy_mesh(mesh: &TerrainMesh) -> bevy::render::mesh::Mesh {
        use bevy::render::mesh::{Indices, Mesh};
        use bevy::render::render_resource::PrimitiveTopology;

        let mut bevy_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );

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

/// GND 地面网格构建器
/// 从 GND 文件生成按纹理分组的网格数据，用于带贴图的地面渲染
pub struct GndMeshBuilder;

/// GND 单元格高度索引常量
/// RO 的 GND 单元格高度顺序：[左下, 右下, 左上, 右上]
const IDX_LEFT_BOTTOM: usize = 0;
const IDX_RIGHT_BOTTOM: usize = 1;
const IDX_LEFT_TOP: usize = 2;
const IDX_RIGHT_TOP: usize = 3;

impl GndMeshBuilder {
    /// 从 GND 文件构建按纹理分组的网格列表
    /// 遍历所有单元格的上表面，按纹理索引分组，为每组生成独立的 TerrainMesh
    pub fn build_groups(gnd: &GndFile) -> Vec<GndMeshGroup> {
        // 临时存储：纹理索引 -> (顶点列表, 索引列表)
        let mut groups: HashMap<u16, (Vec<TerrainVertex>, Vec<u32>)> = HashMap::new();

        // GND 单元格尺寸为 2 个世界单位
        let cell_size = 2.0;

        for y in 0..gnd.height {
            for x in 0..gnd.width {
                if let Some(cell) = gnd.get_cell(x, y) {
                    // 跳过没有上表面贴图的单元格
                    if cell.tile_up == u16::MAX {
                        continue;
                    }

                    // 获取瓦片数据以确定纹理
                    let tile = match gnd.tiles.get(cell.tile_up as usize) {
                        Some(t) => t,
                        None => continue,
                    };

                    // 跳过无纹理的瓦片
                    if tile.tex_id == u16::MAX {
                        continue;
                    }

                    // 计算世界坐标（RO 坐标系：x 东，z 南，y 上）
                    let base_x = x as f32 * cell_size;
                    let base_z = y as f32 * cell_size;

                    // 从瓦片计算 UV 坐标（0-63 范围映射到 0.0-1.0）
                    let u1 = tile.x1 as f32 / 64.0;
                    let v1 = tile.y1 as f32 / 64.0;
                    let u2 = tile.x2 as f32 / 64.0;
                    let v2 = tile.y2 as f32 / 64.0;

                    let entry = groups.entry(tile.tex_id).or_insert_with(|| (Vec::new(), Vec::new()));
                    let (ref mut verts, ref mut idxs) = *entry;
                    let base_index = verts.len() as u32;

                    // 计算法线：由两条边叉积得出
                    // 使用对角线向量来得到平面法线
                    let normal = Self::compute_normal(&cell.heights, cell_size);

                    // 四个角顶点
                    // 左下
                    verts.push(TerrainVertex {
                        position: [base_x, cell.heights[IDX_LEFT_BOTTOM], base_z + cell_size],
                        normal,
                        uv: [u1, v2],
                    });
                    // 右下
                    verts.push(TerrainVertex {
                        position: [base_x + cell_size, cell.heights[IDX_RIGHT_BOTTOM], base_z + cell_size],
                        normal,
                        uv: [u2, v2],
                    });
                    // 左上
                    verts.push(TerrainVertex {
                        position: [base_x, cell.heights[IDX_LEFT_TOP], base_z],
                        normal,
                        uv: [u1, v1],
                    });
                    // 右上
                    verts.push(TerrainVertex {
                        position: [base_x + cell_size, cell.heights[IDX_RIGHT_TOP], base_z],
                        normal,
                        uv: [u2, v1],
                    });

                    // 两个三角形
                    idxs.extend_from_slice(&[base_index, base_index + 1, base_index + 2]);
                    idxs.extend_from_slice(&[base_index + 1, base_index + 3, base_index + 2]);
                }
            }
        }

        // 转换为 GndMeshGroup 列表
        groups
            .into_iter()
            .filter_map(|(tex_id, (vertices, indices))| {
                if vertices.is_empty() {
                    return None;
                }
                let texture_path = gnd
                    .textures
                    .get(tex_id as usize)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                Some(GndMeshGroup {
                    texture_index: tex_id,
                    texture_path,
                    mesh: TerrainMesh { vertices, indices },
                })
            })
            .collect()
    }

    /// 从 GND 文件构建前表面（南侧墙壁）网格
    /// 当相邻行存在高度差时，生成墙壁面以填补缝隙
    pub fn build_front_walls(gnd: &GndFile) -> Vec<GndMeshGroup> {
        let mut groups: HashMap<u16, (Vec<TerrainVertex>, Vec<u32>)> = HashMap::new();
        let cell_size = 2.0;

        for y in 0..gnd.height.saturating_sub(1) {
            for x in 0..gnd.width {
                if let Some(cell) = gnd.get_cell(x, y) {
                    if cell.tile_front == u16::MAX {
                        continue;
                    }
                    let tile = match gnd.tiles.get(cell.tile_front as usize) {
                        Some(t) => t,
                        None => continue,
                    };
                    if tile.tex_id == u16::MAX {
                        continue;
                    }

                    let base_x = x as f32 * cell_size;
                    let base_z = y as f32 * cell_size;

                    let u1 = tile.x1 as f32 / 64.0;
                    let v1 = tile.y1 as f32 / 64.0;
                    let u2 = tile.x2 as f32 / 64.0;
                    let v2 = tile.y2 as f32 / 64.0;

                    let entry = groups.entry(tile.tex_id).or_insert_with(|| (Vec::new(), Vec::new()));
                    let (ref mut verts, ref mut idxs) = *entry;
                    let base_index = verts.len() as u32;

                    // 前墙连接当前行底边（z + cell_size）到下一行顶边
                    // 当前行底边的两个高度
                    let h_bl = cell.heights[IDX_LEFT_BOTTOM];
                    let h_br = cell.heights[IDX_RIGHT_BOTTOM];

                    // 墙壁面：当前行底边两个角 + 下沉到 z+cell_size 的底部
                    // 顶边（当前行底边高度）
                    verts.push(TerrainVertex {
                        position: [base_x, h_bl, base_z + cell_size],
                        normal: [0.0, 0.0, -1.0],
                        uv: [u1, v1],
                    });
                    verts.push(TerrainVertex {
                        position: [base_x + cell_size, h_br, base_z + cell_size],
                        normal: [0.0, 0.0, -1.0],
                        uv: [u2, v1],
                    });
                    // 底边（向下延伸）
                    verts.push(TerrainVertex {
                        position: [base_x, h_bl - cell_size, base_z + cell_size],
                        normal: [0.0, 0.0, -1.0],
                        uv: [u1, v2],
                    });
                    verts.push(TerrainVertex {
                        position: [base_x + cell_size, h_br - cell_size, base_z + cell_size],
                        normal: [0.0, 0.0, -1.0],
                        uv: [u2, v2],
                    });

                    idxs.extend_from_slice(&[base_index, base_index + 1, base_index + 2]);
                    idxs.extend_from_slice(&[base_index + 1, base_index + 3, base_index + 2]);
                }
            }
        }

        groups
            .into_iter()
            .filter_map(|(tex_id, (vertices, indices))| {
                if vertices.is_empty() {
                    return None;
                }
                let texture_path = gnd
                    .textures
                    .get(tex_id as usize)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                Some(GndMeshGroup {
                    texture_index: tex_id,
                    texture_path,
                    mesh: TerrainMesh { vertices, indices },
                })
            })
            .collect()
    }

    /// 计算单元格上表面法线
    /// 使用两条对角线的叉积得到平面法线，确保朝上
    fn compute_normal(heights: &[f32; 4], size: f32) -> [f32; 3] {
        // 使用左下→右上 和 左下→左上 两条边
        let edge1 = [size, heights[IDX_RIGHT_TOP] - heights[IDX_LEFT_BOTTOM], 0.0];
        let edge2 = [0.0, heights[IDX_LEFT_TOP] - heights[IDX_LEFT_BOTTOM], -size];

        // 叉积 edge1 × edge2
        let nx = edge1[1] * edge2[2] - edge1[2] * edge2[1];
        let ny = edge1[2] * edge2[0] - edge1[0] * edge2[2];
        let nz = edge1[0] * edge2[1] - edge1[1] * edge2[0];

        // 归一化
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        if len < 1e-8 {
            return [0.0, 1.0, 0.0];
        }
        [nx / len, ny / len, nz / len]
    }
}
