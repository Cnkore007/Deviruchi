use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{info, error, warn};
use futures_util::{SinkExt, StreamExt};
use crate::network::{PacketCodec, Session, SessionManager, PacketHandler};

pub struct GameServer {
    addr: String,
    session_manager: Arc<SessionManager>,
    packet_handler: Arc<PacketHandler>,
}

impl GameServer {
    pub fn new(addr: String, session_manager: Arc<SessionManager>, packet_handler: Arc<PacketHandler>) -> Self {
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
                        if let Err(e) = Self::handle_connection(stream, addr, session_manager, packet_handler).await {
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

        session_manager.add(addr.to_string(), session.clone());

        let mut framed = Framed::new(stream, PacketCodec);

        while let Some(result) = framed.next().await {
            match result {
                Ok(packet) => {
                    info!("Received packet: id=0x{:04X}, len={}", packet.header.packet_id, packet.header.length);

                    // 处理数据包
                    if let Some(response) = packet_handler.handle(&mut session, packet.header.packet_id, &packet.data) {
                        framed.send(response.into()).await?;
                    }

                    // 更新 session
                    session_manager.update(&session_id, session.clone());
                }
                Err(e) => {
                    warn!("Packet error: {}", e);
                    break;
                }
            }
        }

        session_manager.remove(&session_id);
        info!("Connection closed: {}", addr);

        Ok(())
    }
}
