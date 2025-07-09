use crate::block::Block;
use crate::blockchain::Blockchain;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::{Arc, Mutex};
use serde_json;
use crate::peers::PeerManager;
use crate::transaction::Transaction;
use crate::mempool::Mempool;

pub async fn broadcast_transaction(tx: &Transaction, peers: &PeerManager) {
    let data = serde_json::to_string(tx).unwrap();
    for addr in peers.list() {
        let addr = format!("{}", addr);
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let _ = stream.write_all(data.as_bytes()).await;
        }
    }
}

pub async fn broadcast_block(block: &Block, peers: &PeerManager) {
    let data = serde_json::to_string(block).unwrap();
    for addr in peers.list() {
        let addr = format!("{}", addr);
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let _ = stream.write_all(data.as_bytes()).await;
        }
    }
}

pub async fn start_server(port: u16, chain: Arc<Mutex<Blockchain>>, mempool: Arc<Mutex<Mempool>>) {
    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("🌐 监听地址: 0.0.0.0:{}", port);

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let chain = Arc::clone(&chain);
        let mempool = Arc::clone(&mempool);

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    // 先尝试解析为 Transaction
                    if let Ok(tx) = serde_json::from_str::<Transaction>(text) {
                        println!("📥 接收到交易: {} -> {} [{}]", tx.from, tx.to, tx.amount);
                        mempool.lock().unwrap().add(tx);
                        return;
                    }
                    // 再尝试解析为 Block
                    if let Ok(block) = serde_json::from_str::<Block>(text) {
                        println!("📥 接收到区块: {} from {}", block.index, block.proposer);
                        chain.lock().unwrap().chain.push(block);
                    }
                }
            }
        });
    }
}
