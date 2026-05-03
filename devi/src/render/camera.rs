use bevy::prelude::*;
use crate::game::player::{Position, LocalPlayer};

pub fn follow_camera(
    player_query: Query<&Position, With<LocalPlayer>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(pos) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };

    camera_transform.translation.x = pos.x;
    camera_transform.translation.y = pos.y;
}
