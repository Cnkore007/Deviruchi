//! Modern Protocol WebSocket Server
//!
//! 使用 WebSocket + JSON 格式的协议，与 Devi 客户端通信

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::network::SessionManager;

/// Modern Protocol Packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    #[serde(rename = "type")]
    pub packet_type: String,
    pub payload: serde_json::Value,
}

/// Modern Session
pub struct ModernSession {
    pub id: Uuid,
    pub player_id: Option<u32>,
    pub map_id: Option<String>,
    pub x: f32,
    pub y: f32,
    pub name: String,
}

impl Default for ModernSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ModernSession {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            player_id: None,
            map_id: None,
            x: 0.0,
            y: 0.0,
            name: format!("Player_{}", &Uuid::new_v4().to_string()[..8]),
        }
    }
}

/// Modern Server - WebSocket Server for Devi Client
pub struct ModernServer {
    addr: String,
    session_manager: Arc<SessionManager>,
    /// Broadcast channel for game events to all clients
    tx: broadcast::Sender<String>,
}

impl ModernServer {
    pub fn new(addr: String, session_manager: Arc<SessionManager>) -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            addr,
            session_manager,
            tx,
        }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("Modern Server listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let tx = self.tx.clone();
                    let session_manager = self.session_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(stream, addr, tx, session_manager).await
                        {
                            error!("WebSocket connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: std::net::SocketAddr,
        tx: broadcast::Sender<String>,
        _session_manager: Arc<SessionManager>,
    ) -> anyhow::Result<()> {
        info!("New Modern connection: {}", addr);

        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        let mut session = ModernSession::new();
        let _player_id = session.id.to_string();

        // Subscribe to broadcasts
        let mut rx = tx.subscribe();

        // Send welcome message
        let welcome = Packet {
            packet_type: "WELCOME".to_string(),
            payload: serde_json::json!({
                "session_id": session.id.to_string(),
                "name": session.name,
                "x": 160.0,
                "y": 160.0,
            }),
        };
        let welcome_json = serde_json::to_string(&welcome)?;
        write.send(Message::Text(welcome_json)).await?;

        // Send spawn for this player to everyone
        let spawn = Packet {
            packet_type: "ACTOR_SPAWN".to_string(),
            payload: serde_json::json!({
                "id": session.id.to_string(),
                "name": session.name,
                "x": 160.0,
                "y": 160.0,
            }),
        };
        let spawn_json = serde_json::to_string(&spawn)?;
        tx.send(spawn_json.clone())?;

        info!("Player {} joined at (160, 160)", session.name);

        loop {
            tokio::select! {
                // Handle incoming messages from client
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            Self::handle_message(&text, &mut session, &tx).await;
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        Some(Err(e)) => {
                            warn!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                // Handle broadcasts from server
                broadcast_msg = rx.recv() => {
                    match broadcast_msg {
                        Ok(msg) => {
                            if write.send(Message::Text(msg)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(_) => break,
                    }
                }
            }
        }

        // Remove player from game
        let despawn = Packet {
            packet_type: "ACTOR_DESPAWN".to_string(),
            payload: serde_json::json!({
                "id": session.id.to_string(),
            }),
        };
        let despawn_json = serde_json::to_string(&despawn)?;
        let _ = tx.send(despawn_json);

        info!("Player {} disconnected", session.name);
        Ok(())
    }

    async fn handle_message(
        text: &str,
        session: &mut ModernSession,
        tx: &broadcast::Sender<String>,
    ) {
        // Parse the incoming packet
        let Ok(packet) = serde_json::from_str::<Packet>(text) else {
            warn!("Invalid JSON: {}", text);
            return;
        };

        info!("Received packet type: {}", packet.packet_type);

        match packet.packet_type.as_str() {
            "MOVE" => {
                // Handle MOVE packet: {"type":"MOVE","payload":{"x":160.0,"y":200.0}}
                if let (Some(x), Some(y)) = (
                    packet.payload.get("x").and_then(|v| v.as_f64()),
                    packet.payload.get("y").and_then(|v| v.as_f64()),
                ) {
                    session.x = x as f32;
                    session.y = y as f32;

                    // Broadcast move to all players
                    let move_packet = Packet {
                        packet_type: "ACTOR_MOVE".to_string(),
                        payload: serde_json::json!({
                            "id": session.id.to_string(),
                            "x": session.x,
                            "y": session.y,
                        }),
                    };
                    if let Ok(json) = serde_json::to_string(&move_packet) {
                        let _ = tx.send(json);
                    }
                }
            }
            "CHAT" => {
                // Handle CHAT packet
                let message = packet
                    .payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let chat_packet = Packet {
                    packet_type: "CHAT".to_string(),
                    payload: serde_json::json!({
                        "id": session.id.to_string(),
                        "name": session.name,
                        "message": message,
                    }),
                };
                if let Ok(json) = serde_json::to_string(&chat_packet) {
                    let _ = tx.send(json);
                }
            }
            _ => {
                info!("Unknown packet type: {}", packet.packet_type);
            }
        }
    }
}
