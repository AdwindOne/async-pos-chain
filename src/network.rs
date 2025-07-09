use crate::block::Block;
use crate::blockchain::Blockchain;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::{Arc, Mutex};
use serde_json;

pub async fn start_server(port: u16, chain: Arc<Mutex<Blockchain>>) {
    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("🌐 监听地址: 0.0.0.0:{}", port);

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let chain = Arc::clone(&chain);

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    if let Ok(block) = serde_json::from_str::<Block>(text) {
                        println!("📥 接收到区块: {} from {}", block.index, block.proposer);
                        chain.lock().unwrap().chain.push(block);
                    }
                }
            }
        });
    }
}
