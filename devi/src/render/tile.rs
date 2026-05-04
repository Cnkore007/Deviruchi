use crate::game::map::{generate_test_map, GameMap, MapTile};
use crate::game::player::{LocalPlayer, Player, Position};
use bevy::prelude::*;

pub fn setup_map(mut commands: Commands) {
    commands.insert_resource(GameMap::new(30, 20, 32.0));

    let tiles = generate_test_map();
    for tile in tiles {
        commands.spawn((
            MapTile {
                x: tile.x,
                y: tile.y,
            },
            Transform::from_xyz(tile.x as f32 * 32.0, tile.y as f32 * 32.0, 0.0),
            Sprite {
                color: Color::rgb(0.3, 0.3, 0.35),
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
        ));
    }
}

pub fn setup_local_player(mut commands: Commands) {
    commands.spawn((
        Player {
            id: 0,
            name: "LocalPlayer".to_string(),
        },
        Position { x: 5.0, y: 5.0 },
        LocalPlayer,
        Transform::from_xyz(5.0 * 32.0, 5.0 * 32.0, 1.0),
        Sprite {
            color: Color::rgb(0.2, 0.8, 0.2), // 绿色
            custom_size: Some(Vec2::new(28.0, 28.0)),
            ..default()
        },
    ));
}
