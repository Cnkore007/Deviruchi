use crate::network::{PacketCodec, PacketHandler, Session, SessionManager};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};

pub struct GameServer {
    addr: String,
    session_manager: Arc<SessionManager>,
    packet_handler: Arc<PacketHandler>,
    initial_stage: crate::network::session::SessionStage,
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
            initial_stage: crate::network::session::SessionStage::Login,
        }
    }

    pub fn with_initial_stage(mut self, stage: crate::network::session::SessionStage) -> Self {
        self.initial_stage = stage;
        self
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        // 使用 std::net 绑定端口，绕过 tokio reactor 在 macOS 26.5 上的兼容问题
        let std_listener = std::net::TcpListener::bind(&self.addr)?;
        std_listener.set_nonblocking(false)?;
        info!("Server listening on {}", self.addr);

        let session_manager = self.session_manager.clone();
        let packet_handler = self.packet_handler.clone();
        let initial_stage = self.initial_stage.clone();

        // 在阻塞线程中运行 accept 循环，通过 channel 将连接传给 tokio 任务
        let (tx, mut rx) = mpsc::unbounded_channel::<(std::net::TcpStream, std::net::SocketAddr)>();

        std::thread::spawn(move || {
            loop {
                match std_listener.accept() {
                    Ok((stream, addr)) => {
                        info!("[ACCEPT] 接受到新连接: {}", addr);
                        if tx.send((stream, addr)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("[ACCEPT] 错误: {}", e);
                    }
                }
            }
        });

        // 在 tokio 任务中处理连接
        while let Some((std_stream, addr)) = rx.recv().await {
            let stream = TcpStream::from_std(std_stream)?;
            let sm = session_manager.clone();
            let ph = packet_handler.clone();
            let stage = initial_stage.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, sm, ph, stage).await {
                    error!("Connection error: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: std::net::SocketAddr,
        session_manager: Arc<SessionManager>,
        packet_handler: Arc<PacketHandler>,
        initial_stage: crate::network::session::SessionStage,
    ) -> anyhow::Result<()> {
        info!("New connection: {}", addr);

        let mut session = Session::new();
        session.stage = initial_stage;
        let session_id = session.id;

        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        session.map_event_tx = Some(event_tx);

        session_manager.add(addr.to_string(), session.clone());

        let mut framed = Framed::new(stream, PacketCodec);

        loop {
            tokio::select! {
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
                event_data = event_rx.recv() => {
                    if let Some(data) = event_data {
                        if !data.is_empty()
                            && let Err(e) = framed.send(data).await {
                                warn!("Failed to send event to client: {}", e);
                            }
                    } else {
                        warn!("Event channel closed for session {}", session_id);
                        break;
                    }
                }
            }
        }

        packet_handler.handle_disconnect(&session);

        session_manager.remove(&session_id);
        info!("Connection closed: {}", addr);

        Ok(())
    }
}
