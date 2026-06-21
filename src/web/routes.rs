use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::game::map::MapState;

pub struct AppState {
    pub map_state: Arc<MapState>,
    pub recruitment: RwLock<Vec<RecruitEntry>>,
    pub start_time: Instant,
}

impl AppState {
    pub fn new(map_state: Arc<MapState>) -> Self {
        Self {
            map_state,
            recruitment: RwLock::new(Vec::new()),
            start_time: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecruitEntry {
    pub id: Uuid,
    pub party_name: String,
    pub leader_name: String,
    pub description: String,
    pub created_at: u64,
}

#[derive(Deserialize)]
pub struct PlayerListQuery {
    pub map: Option<String>,
}

#[derive(Deserialize)]
pub struct PartyAddRequest {
    pub party_name: String,
    pub leader_name: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct PartyDelQuery {
    pub id: Uuid,
}

pub async fn server_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let player_count = state.map_state.player_count();
    Json(serde_json::json!({
        "uptime_seconds": uptime_secs,
        "online_players": player_count,
    }))
}

pub async fn player_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PlayerListQuery>,
) -> Json<serde_json::Value> {
    let players = if let Some(map_name) = &query.map {
        state.map_state.get_players_on_map(map_name)
    } else {
        state.map_state.get_all_players()
    };

    let list: Vec<serde_json::Value> = players
        .iter()
        .map(|p| {
            let pos = p.pos.read();
            let combat = p.combat.read();
            let level = p.level.read();
            serde_json::json!({
                "name": p.name,
                "map": p.map_name,
                "x": pos.x,
                "y": pos.y,
                "hp": combat.hp,
                "max_hp": combat.max_hp,
                "sp": combat.sp,
                "max_sp": combat.max_sp,
                "base_level": level.base_level,
                "job_level": level.job_level,
            })
        })
        .collect();

    Json(serde_json::json!({
        "count": list.len(),
        "players": list,
    }))
}

pub async fn party_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let entries = state.recruitment.read().clone();
    Json(serde_json::json!({
        "count": entries.len(),
        "entries": entries,
    }))
}

pub async fn party_add(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PartyAddRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let entry = RecruitEntry {
        id: Uuid::new_v4(),
        party_name: req.party_name,
        leader_name: req.leader_name,
        description: req.description,
        created_at: chrono::Utc::now().timestamp() as u64,
    };
    let id = entry.id;
    state.recruitment.write().push(entry);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "success": true })),
    )
}

pub async fn party_del(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PartyDelQuery>,
) -> Json<serde_json::Value> {
    let mut entries = state.recruitment.write();
    let before = entries.len();
    entries.retain(|e| e.id != query.id);
    let removed = entries.len() < before;
    Json(serde_json::json!({ "success": removed }))
}
