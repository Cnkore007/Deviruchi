#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::prelude::*;
use devi::game::input::handle_input;
use devi::game::map::GameMap;
use devi::network::{NetworkClient, NetworkResource};
use devi::render::camera::follow_camera;
use devi::render::tile::{setup_local_player, setup_map};
use std::sync::Arc;

async fn init_network(network: Arc<tokio::sync::Mutex<Option<NetworkClient>>>) {
    let url = "ws://127.0.0.1:16121";
    match NetworkClient::connect(url).await {
        Ok(client) => {
            *network.lock().await = Some(client);
            eprintln!("[Network] Connected to server at {}", url);
        }
        Err(e) => {
            eprintln!("[Network] Failed to connect to server: {}", e);
        }
    }
}

fn main() {
    let network = Arc::new(tokio::sync::Mutex::new(None));

    // 异步初始化网络 - 在独立线程中创建 Tokio runtime
    std::thread::spawn({
        let network_clone = network.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                init_network(network_clone).await;
            });
        }
    });

    App::new()
        .insert_resource(GameMap::new(30, 20, 32.0))
        .insert_resource(NetworkResource { client: network })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_map, setup_local_player))
        .add_systems(Update, (handle_input, follow_camera))
        .run();
}
