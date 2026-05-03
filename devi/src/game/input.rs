use bevy::prelude::*;
use crate::game::player::{Position, LocalPlayer};
use crate::game::map::GameMap;

const MOVE_SPEED: f32 = 5.0;

pub fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Position, &mut Transform), With<LocalPlayer>>,
    map: Res<GameMap>,
) {
    let Ok((mut pos, mut transform)) = query.get_single_mut() else {
        return;
    };

    let mut dx: f32 = 0.0;
    let mut dy: f32 = 0.0;

    if keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW) {
        dy = 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
        dy = -1.0;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
        dx = -1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
        dx = 1.0;
    }

    if dx != 0.0 || dy != 0.0 {
        let delta = time.delta_seconds() * MOVE_SPEED * 32.0;
        pos.x += dx * delta;
        pos.y += dy * delta;

        // 边界检查
        pos.x = pos.x.clamp(0.0, (map.width - 1) as f32 * map.tile_size);
        pos.y = pos.y.clamp(0.0, (map.height - 1) as f32 * map.tile_size);

        transform.translation.x = pos.x;
        transform.translation.y = pos.y;
    }
}
