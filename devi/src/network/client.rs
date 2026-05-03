use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct NetworkClient {
    sender: mpsc::Sender<String>,
}

impl NetworkClient {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = mpsc::channel::<String>(100);

        // 发送任务
        let _send_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                write.send(Message::Text(msg)).await.ok();
            }
        });

        // 接收任务 (这里只是消费，不处理)
        let _recv_handle = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    eprintln!("[Network] Received: {}", text);
                }
            }
        });

        Ok(Self { sender: tx })
    }

    pub async fn send(&self, packet: &str) -> Result<()> {
        self.sender.send(packet.to_string()).await?;
        Ok(())
    }
}
