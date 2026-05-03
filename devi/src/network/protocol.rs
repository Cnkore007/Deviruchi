use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    #[serde(rename = "type")]
    pub packet_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPayload {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePayload {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapEnterPayload {
    pub character_id: u32,
}

impl Packet {
    pub fn map_enter(character_id: u32) -> Self {
        Self {
            packet_type: "MAP_ENTER".to_string(),
            payload: serde_json::to_value(MapEnterPayload { character_id }).unwrap(),
        }
    }
}
