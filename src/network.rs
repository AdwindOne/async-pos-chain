use crate::block::block::Block;
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::peers::PeerManager;
use crate::transaction::Transaction;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn broadcast_transaction(tx: &Transaction, peers: &PeerManager) {
    broadcast_to_peers(&serde_json::to_string(tx).unwrap(), peers).await;
}

pub async fn broadcast_block(block: &Block, peers: &PeerManager) {
    broadcast_to_peers(&serde_json::to_string(block).unwrap(), peers).await;
}

async fn broadcast_to_peers(data: &str, peers: &PeerManager) {
    for addr in peers.list() {
        if let Ok(mut stream) = TcpStream::connect(&addr).await {
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
            handle_incoming_connection(&mut socket, chain, mempool).await;
        });
    }
}

async fn handle_incoming_connection(
    socket: &mut TcpStream,
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
) {
    let mut buf = [0; 1024];
    if let Ok(n) = socket.read(&mut buf).await {
        if let Ok(text) = std::str::from_utf8(&buf[..n]) {
            match parse_network_message(text) {
                NetworkMessage::PeersRequest => {
                    send_peers_response(socket).await;
                }
                NetworkMessage::PeersResponse(arr) => {
                    update_peers_from_response(&arr).await;
                }
                NetworkMessage::Transaction(tx) => {
                    println!("📥 接收到交易: {} -> {} [{}]", tx.from, tx.to, tx.amount);
                    mempool.lock().unwrap().add(tx, None);
                }
                NetworkMessage::Block(block) => {
                    println!("📥 接收到区块: {} from {}", block.index, block.proposer);
                    chain.lock().unwrap().chain.push(block);
                }
                NetworkMessage::Unknown => {}
            }
        }
    }
}

enum NetworkMessage {
    PeersRequest,
    PeersResponse(Vec<String>),
    Transaction(Transaction),
    Block(Block),
    Unknown,
}

fn parse_network_message(text: &str) -> NetworkMessage {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
        match val.get("type").and_then(|v| v.as_str()) {
            Some("peers_request") => NetworkMessage::PeersRequest,
            Some("peers_response") => {
                let arr = val
                    .get("peers")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|p| p.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                NetworkMessage::PeersResponse(arr)
            }
            _ => NetworkMessage::Unknown,
        }
    } else if let Ok(tx) = serde_json::from_str::<Transaction>(text) {
        NetworkMessage::Transaction(tx)
    } else if let Ok(block) = serde_json::from_str::<Block>(text) {
        NetworkMessage::Block(block)
    } else {
        NetworkMessage::Unknown
    }
}

async fn send_peers_response(socket: &mut TcpStream) {
    let peer_conn = Connection::open("peers.db").unwrap();
    let peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
    let resp = serde_json::json!({"type": "peers_response", "peers": peers.list()});
    let _ = socket
        .write_all(serde_json::to_string(&resp).unwrap().as_bytes())
        .await;
}

async fn update_peers_from_response(arr: &Vec<String>) {
    let peer_conn = Connection::open("peers.db").unwrap();
    let mut peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
    for addr in arr {
        peers.add_peer(addr.to_string());
    }
    let _ = peers.save_to_db(&peer_conn);
}

pub async fn discover_peers(peers: &mut PeerManager) {
    let peer_list = peers.list();
    for addr in peer_list {
        if let Ok(mut stream) = TcpStream::connect(&addr).await {
            let req = serde_json::json!({"type": "peers_request"});
            let _ = stream
                .write_all(serde_json::to_string(&req).unwrap().as_bytes())
                .await;
            if let Err(e) = handle_stream(stream, peers).await {
                eprintln!("Error handling stream: {}", e);
            }
        }
    }
}

async fn handle_stream(
    stream: TcpStream,
    peers: &mut PeerManager,
) -> Result<(), Box<dyn std::error::Error>> {
    read_and_handle_json(stream, peers).await
}

async fn read_and_handle_json(
    mut stream: TcpStream,
    peers: &mut PeerManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0; 2048];
    if let Ok(n) = stream.read(&mut buf).await {
        if let Ok(text) = std::str::from_utf8(&buf[..n]) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                handle_json_value(val, peers).await;
            }
        }
    }
    Ok(())
}

pub async fn handle_json_value(val: serde_json::Value, peers: &mut PeerManager) {
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
