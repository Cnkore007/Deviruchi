#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::*;
use devi::game::map::GameMap;
use devi::render::tile::{setup_map, setup_local_player};
use devi::render::camera::follow_camera;
use devi::game::input::handle_input;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameMap::new(30, 20, 32.0))
        .add_systems(Startup, (setup_map, setup_local_player))
        .add_systems(Update, (handle_input, follow_camera))
        .run();
}
