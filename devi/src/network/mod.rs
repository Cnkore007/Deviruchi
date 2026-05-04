use bevy::prelude::*;

pub mod client;
pub mod protocol;

pub use client::NetworkClient;
pub use protocol::Packet;

use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Resource)]
pub struct NetworkResource {
    pub client: Arc<Mutex<Option<NetworkClient>>>,
}

impl NetworkResource {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for NetworkResource {
    fn default() -> Self {
        Self::new()
    }
}
