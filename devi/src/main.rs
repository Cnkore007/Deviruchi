#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::*;
use devi::game::map::GameMap;
use devi::render::tile::{setup_map, setup_local_player};
use devi::render::camera::follow_camera;
use devi::game::input::handle_input;
use devi::network::{NetworkClient, Packet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Resource)]
struct NetworkResource {
    client: Arc<Mutex<Option<NetworkClient>>>,
}

fn main() {
    App::new()
        .insert_resource(GameMap::new(30, 20, 32.0))
        .insert_resource(NetworkResource {
            client: Arc::new(Mutex::new(None)),
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_map, setup_local_player))
        .add_systems(Update, (handle_input, follow_camera))
        .run();
}
