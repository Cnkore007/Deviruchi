//! Map Server - 地图服务器核心

pub mod cell;
#[cfg(test)]
mod collision_tests;
pub mod channel;
pub mod data;
pub mod drop_item;
pub mod gat;
pub mod map_server;
pub mod map_state;
pub mod player;
pub mod respawn;
pub mod teleport;

pub use cell::{Cell, CellType};
pub use channel::{ChannelBus, ChatType, GameEvent, guild_channel_name, map_channel_name, party_channel_name};
pub use data::{CharacterData, MapData, MapDatabase};
pub use gat::{GatError, GatParser};
pub use drop_item::{DropItem, DropManager};
pub use map_server::MapServer;
pub use map_state::MapState;
pub use player::{Player, PlayerSaveData, PlayerState};
pub use respawn::{RespawnPoint, RespawnService, RespawnType};
pub use teleport::{
    MapAdjacency, MapEdge, SavePoint, SavePointManager, TeleportAction, TeleportManager, WarpError,
    WarpService,
};
