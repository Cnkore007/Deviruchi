// 3D 地形网格构建器
// 将 GAT 地面高度数据转换为 Bevy 可渲染的三角形网格

use crate::asset::gat::{GatFile, GatCell};

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

/// 地形网格构建器
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
