//! 最小 tokio TCP 服务器测试
//! 编译: rustc test_server.rs --edition 2021
//! 运行: ./test_server

#[tokio::main]
async fn main() {
    println!("Starting test server on 127.0.0.1:6999...");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:6999").await.unwrap();
    println!("Listening! Waiting for connections...");
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("ACCEPTED connection from {}", addr);
                tokio::spawn(async move {
                    use tokio::io::AsyncReadExt;
                    let mut stream = stream;
                    let mut buf = [0u8; 1024];
                    match stream.read(&mut buf).await {
                        Ok(n) => println!("Read {} bytes: {:?}", n, &buf[..n]),
                        Err(e) => println!("Read error: {}", e),
                    }
                });
            }
            Err(e) => println!("Accept error: {}", e),
        }
    }
}
