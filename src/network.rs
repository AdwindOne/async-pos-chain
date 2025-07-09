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
    for addr in peers.peers.clone() {
        let addr = format!("{}", addr);
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let _ = stream.write_all(data.as_bytes()).await;
        }
    }
}

pub async fn broadcast_block(block: &Block, peers: &PeerManager) {
    let data = serde_json::to_string(block).unwrap();
    for addr in peers.peers.clone() {
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
        let peer_db = crate::storage::open_db("peers.db");

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            if let Ok(n) = socket.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    // 网络自动发现：peers_request/peers_response
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                        if val.get("type") == Some(&serde_json::Value::String("peers_request".to_string())) {
                            // 返回本地 peers
                            let peers = crate::peers::PeerManager::load_from_db(&peer_db);
                            let resp = serde_json::json!({"type": "peers_response", "peers": peers.peers.clone()});
                            let _ = socket.write_all(serde_json::to_string(&resp).unwrap().as_bytes()).await;
                            return;
                        }
                        if val.get("type") == Some(&serde_json::Value::String("peers_response".to_string())) {
                            if let Some(arr) = val.get("peers").and_then(|v| v.as_array()) {
                                let mut peers = crate::peers::PeerManager::load_from_db(&peer_db);
                                for p in arr {
                                    if let Some(addr) = p.as_str() {
                                        peers.add_peer(addr.to_string());
                                    }
                                }
                                peers.save_to_db(&peer_db);
                            }
                            return;
                        }
                    }
                    // 先尝试解析为 Transaction
                    if let Ok(tx) = serde_json::from_str::<Transaction>(text) {
                        println!("📥 接收到交易: {} -> {} [{}]", tx.from, tx.to, tx.amount);
                        mempool.lock().unwrap().add(tx, None);
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

#[allow(dead_code)]
pub async fn discover_peers(peers: &mut PeerManager) {
    let peer_list = peers.peers.clone();
    for addr in peer_list {
        if let Ok(mut stream) = TcpStream::connect(&addr).await {
            let req = serde_json::json!({"type": "peers_request"});
            let _ = stream.write_all(serde_json::to_string(&req).unwrap().as_bytes()).await;
            let mut buf = [0; 2048];
            if let Ok(n) = stream.read(&mut buf).await {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                        if val.get("type") == Some(&serde_json::Value::String("peers_response".to_string())) {
                            if let Some(arr) = val.get("peers").and_then(|v| v.as_array()) {
                                for p in arr {
                                    if let Some(addr) = p.as_str() {
                                        peers.add_peer(addr.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
