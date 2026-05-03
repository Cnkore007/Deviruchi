use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub id: u32,
    pub name: String,
}

#[derive(Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct LocalPlayer;
