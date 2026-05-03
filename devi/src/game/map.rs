use bevy::prelude::*;

#[derive(Component)]
pub struct MapTile {
    pub x: u32,
    pub y: u32,
}

#[derive(Resource)]
pub struct GameMap {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
}

impl GameMap {
    pub fn new(width: u32, height: u32, tile_size: f32) -> Self {
        Self { width, height, tile_size }
    }
}

pub fn generate_test_map() -> Vec<MapTile> {
    let mut tiles = Vec::new();
    for y in 0..20 {
        for x in 0..30 {
            tiles.push(MapTile { x, y });
        }
    }
    tiles
}
