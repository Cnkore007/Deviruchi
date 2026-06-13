pub mod routes;

use axum::{Router, routing::{get, post, delete}};
use std::sync::Arc;
use crate::game::map::MapState;
use routes::AppState;

pub struct WebServer {
    addr: String,
    state: Arc<AppState>,
}

impl WebServer {
    pub fn new(addr: String, map_state: Arc<MapState>) -> Self {
        Self {
            addr,
            state: Arc::new(AppState::new(map_state)),
        }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let app = Router::new()
            .route("/api/v1/status", get(routes::server_status))
            .route("/api/v1/players", get(routes::player_list))
            .route("/api/v1/party/list", get(routes::party_list))
            .route("/api/v1/party/add", post(routes::party_add))
            .route("/api/v1/party/del", delete(routes::party_del))
            .with_state(self.state.clone());

        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        tracing::info!("Web API 监听: {}", self.addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}
