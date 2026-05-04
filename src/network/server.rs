use crate::network::{PacketCodec, PacketHandler, Session, SessionManager};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};

pub struct GameServer {
    addr: String,
    session_manager: Arc<SessionManager>,
    packet_handler: Arc<PacketHandler>,
}

impl GameServer {
    pub fn new(
        addr: String,
        session_manager: Arc<SessionManager>,
        packet_handler: Arc<PacketHandler>,
    ) -> Self {
        Self {
            addr,
            session_manager,
            packet_handler,
        }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("Server listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let session_manager = self.session_manager.clone();
                    let packet_handler = self.packet_handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(stream, addr, session_manager, packet_handler)
                                .await
                        {
                            error!("Connection error: {}", e);
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
        session_manager: Arc<SessionManager>,
        packet_handler: Arc<PacketHandler>,
    ) -> anyhow::Result<()> {
        info!("New connection: {}", addr);

        let mut session = Session::new();
        let session_id = session.id;

        // Create channel for game events from ChannelBus to client
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        session.map_event_tx = Some(event_tx);

        session_manager.add(addr.to_string(), session.clone());

        let mut framed = Framed::new(stream, PacketCodec);

        loop {
            tokio::select! {
                // Handle incoming packets from client
                result = framed.next() => {
                    match result {
                        Some(Ok(packet)) => {
                            info!("Received packet: id=0x{:04X}, len={}", packet.header.packet_id, packet.header.length);

                            if let Some(response) = packet_handler.handle(&mut session, packet.header.packet_id, &packet.data) {
                                framed.send(response).await?;
                            }

                            session_manager.update(&session_id, session.clone());
                        }
                        Some(Err(e)) => {
                            warn!("Packet error: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
                // Handle game events from ChannelBus
                event_data = event_rx.recv() => {
                    if let Some(data) = event_data {
                        if !data.is_empty()
                            && let Err(e) = framed.send(data).await {
                                warn!("Failed to send event to client: {}", e);
                            }
                    } else {
                        // Channel closed - client disconnected from event bus
                        warn!("Event channel closed for session {}", session_id);
                        break;
                    }
                }
            }
        }

        session_manager.remove(&session_id);
        info!("Connection closed: {}", addr);

        Ok(())
    }
}
