mod block;
mod blockchain;
mod transaction;
mod mempool;
mod storage;
mod network;
mod account;
mod peers;
use peers::PeerManager;
use network::{broadcast_block, broadcast_transaction};
use peers::PeerManager as PeerManagerTrait;
use crate::storage::RocksDB;

use clap::{Parser, Subcommand};
use blockchain::Blockchain;
use transaction::Transaction;
use mempool::Mempool;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::AsyncReadExt;

#[derive(Parser)]
#[command(name = "PoS Chain")]
#[command(about = "A minimal async PoS blockchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Submit {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
    },
    Run {
        #[arg(long, default_value = "8000")]
        port: u16,
    },
    Query {
        #[arg(long)]
        index: u64,
    },
    QueryBalance {
        #[arg(long)]
        address: String,
    },
    AddPeer {
        #[arg(long)]
        addr: String,
    },
    QueryPeers,
    JsonRpcServer {
        #[arg(long, default_value = "8545")]
        port: u16,
    },
    QueryTx {
        #[arg(long)]
        hash: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // 全局只打开一次 RocksDB
    let chain_db = crate::storage::open_db("chain.db");
    let peers_db = crate::storage::open_db("peers.db");

    match cli.command {
        Commands::Submit { from, to, amount } => {
            let tx = Transaction::new(&from, &to, amount);
            println!("💸 交易提交: {} -> {} [{}]", from, to, amount);
            // 读取 peers 列表，广播交易
            let peers = PeerManager::load_from_db(&peers_db);
            broadcast_transaction(&tx, &peers).await;
            // 假设本地节点监听 8000
            let addr = "127.0.0.1:8000";
            let data = serde_json::to_string(&tx).unwrap();
            if let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await {
                let _ = stream.write_all(data.as_bytes()).await;
            }
            // 本地 mempool 持久化
            let mut mempool = Mempool::new();
            mempool.load_from_db(&chain_db);
            mempool.add(tx, Some(&chain_db));
        }
        Commands::Run { port } => {
            println!("🚀 启动 PoS 节点，监听端口 {}", port);

            // 初始化 admin 账户
            storage::add_account(&chain_db, "admin", 1000000);
            storage::add_account(&chain_db, "Alice", 100);
            storage::add_account(&chain_db, "Bob", 100);

            // 加载 peers
            let peers = PeerManager::load_from_db(&peers_db);
            let peers_arc = Arc::new(Mutex::new(peers));

            // 从数据库恢复区块链
            let mut chain = Blockchain::new();
            let mut idx = 0u64;
            loop {
                if let Some(block) = storage::get_block_by_index(&chain_db, idx) {
                    chain.chain.push(block);
                    idx += 1;
                } else {
                    break;
                }
            }
            if chain.chain.is_empty() {
                chain.create_genesis_block();
                storage::save_block(&chain_db, chain.chain.last().unwrap());
            }

            let mut mempool = Mempool::new();
            mempool.load_from_db(&chain_db);
            mempool.add(Transaction::new("Alice", "Bob", 10), Some(&chain_db));
            mempool.add(Transaction::new("Bob", "Charlie", 5), Some(&chain_db));

            // 启动持续出块任务
            let chain_arc = Arc::new(Mutex::new(chain));
            let mempool_arc = Arc::new(Mutex::new(mempool));
            let peers_for_task = Arc::clone(&peers_arc);
            let chain_for_task = Arc::clone(&chain_arc);
            let mempool_for_task = Arc::clone(&mempool_arc);
            let chain_db_for_task = Arc::clone(&chain_db);
            let peers_db_for_task = Arc::clone(&peers_db);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    // 每3秒都出块（即使没有交易）
                    let txs = {
                        let mut mempool = mempool_for_task.lock().unwrap();
                        mempool.collect_for_block(10, Some(&chain_db_for_task))
                    };
                    let (block, _chain_len, _chain_state, _chain_snapshot, proposer) = {
                        let mut chain = chain_for_task.lock().unwrap();
                        chain.add_block(txs.clone());
                        let block = chain.chain.last().unwrap().clone();
                        let chain_len = chain.chain.len();
                        let chain_state = chain.state.balances.clone();
                        let chain_snapshot = chain.chain.clone();
                        let proposer = block.proposer.clone();
                        (block, chain_len, chain_state, chain_snapshot, proposer)
                    };
                    // 处理区块内所有交易的余额
                    {
                        for tx in &block.transactions {
                            let from_balance = storage::get_balance(&chain_db_for_task, &tx.from);
                            if from_balance >= tx.amount {
                                storage::set_balance(&chain_db_for_task, &tx.from, from_balance - tx.amount);
                                let to_balance = storage::get_balance(&chain_db_for_task, &tx.to);
                                storage::set_balance(&chain_db_for_task, &tx.to, to_balance + tx.amount);
                            }
                        }
                        // 出块奖励给 proposer
                        let reward = 50;
                        let proposer_balance = storage::get_balance(&chain_db_for_task, &proposer);
                        storage::set_balance(&chain_db_for_task, &proposer, proposer_balance + reward);
                        storage::save_block(&chain_db_for_task, &block);
                    }
                    println!("[⛓️ 出块] 高度: {} | Hash: {} | 提议者: {} | 交易数: {}", block.index, block.hash, block.proposer, block.transactions.len());
                    println!("📊 账户余额：");
                    {
                        for addr in &["admin", "Alice", "Bob", "Charlie"] {
                            let balance = storage::get_balance(&chain_db_for_task, addr);
                            println!(" - {}: {}", addr, balance);
                        }
                    }
                    // 广播新出块
                    let peer_list = {
                        let peers = peers_for_task.lock().unwrap();
                        peers.list()
                    };
                    broadcast_block(&block, &PeerManager { peers: peer_list }).await;
                }
            });

            // 启动 JSON-RPC 服务（端口8545）
            let mempool_for_rpc = Arc::clone(&mempool_arc);
            let chain_db_for_rpc = Arc::clone(&chain_db);
            tokio::spawn(async move {
                use tokio::net::TcpListener;
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                use serde_json::json;
                println!("🚀 启动 JSON-RPC 服务，监听端口 8545");
                let listener = TcpListener::bind(("0.0.0.0", 8545)).await.unwrap();
                loop {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let mempool_for_rpc = Arc::clone(&mempool_for_rpc);
                    let chain_db_for_rpc = Arc::clone(&chain_db_for_rpc);
                    tokio::spawn(async move {
                        let mut buf = [0; 4096];
                        if let Ok(n) = socket.read(&mut buf).await {
                            if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                                if let Some(body_start) = text.find("\r\n\r\n") {
                                    let body = &text[body_start + 4..];
                                    if let Ok(req) = serde_json::from_str::<serde_json::Value>(body) {
                                        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                        if method == "send_transaction" {
                                            if let Some(params) = req.get("params").and_then(|p| p.as_array()) {
                                                if params.len() == 3 {
                                                    let from = params[0].as_str().unwrap_or("");
                                                    let to = params[1].as_str().unwrap_or("");
                                                    let amount = params[2].as_u64().unwrap_or(0);
                                                    let tx = Transaction::new(from, to, amount);
                                                    println!("[JSON-RPC] 交易提交: {} -> {} [{}]", from, to, amount);
                                                    let _ = mempool_for_rpc.lock().unwrap().add(tx, Some(&chain_db_for_rpc));
                                                    let resp = json!({
                                                        "jsonrpc": "2.0",
                                                        "result": {"status": "ok"},
                                                        "id": req.get("id").cloned().unwrap_or(json!(1))
                                                    });
                                                    let resp_str = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                                    let _ = socket.write_all(resp_str.as_bytes()).await;
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                                let resp = json!({"jsonrpc":"2.0","error":"invalid request","id":null});
                                let resp_str = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                let _ = socket.write_all(resp_str.as_bytes()).await;
                            }
                        }
                    });
                }
            });

            let peers_for_discover = Arc::clone(&peers_arc);
            let peers_db_for_discover = Arc::clone(&peers_db);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let peer_list = {
                        let peers = peers_for_discover.lock().unwrap();
                        peers.list()
                    };
                    let mut discovered = PeerManager::new();
                    for addr in peer_list {
                        if let Ok(mut stream) = tokio::net::TcpStream::connect(&addr).await {
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
                                                        discovered.add_peer(addr.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let mut peers = peers_for_discover.lock().unwrap();
                    let before = peers.list().len();
                    for addr in discovered.list() {
                        peers.add_peer(addr);
                    }
                    let after = peers.list().len();
                    if after > before {
                        println!("[发现节点] 新增 {} 个节点，当前已知节点总数: {}", after - before, after);
                    } else {
                        println!("[发现节点] 未发现新节点，当前已知节点总数: {}", after);
                    }
                    peers.save_to_db(&peers_db_for_discover);
                }
            });

            network::start_server(port, chain_arc, mempool_arc).await;
        }
        Commands::Query { index } => {
            match storage::get_block_by_index(&chain_db, index) {
                Some(block) => {
                    println!("区块高度: {}", block.index);
                    println!("Hash: {}", block.hash);
                    println!("前置Hash: {}", block.previous_hash);
                    println!("提议者: {}", block.proposer);
                    println!("时间戳: {}", block.timestamp);
                    println!("交易: {:?}", block.transactions);
                }
                None => println!("未找到该高度区块"),
            }
        }
        Commands::QueryBalance { address } => {
            let balance = storage::get_balance(&chain_db, &address);
            println!("{} 余额: {}", address, balance);
        }
        Commands::AddPeer { addr } => {
            let mut peers = PeerManager::load_from_db(&peers_db);
            peers.add_peer(addr.clone());
            peers.save_to_db(&peers_db);
            println!("已添加节点: {}", addr);
        }
        Commands::QueryPeers => {
            let peers = PeerManager::load_from_db(&peers_db);
            peers.display_peers();
        }
        Commands::JsonRpcServer { port } => {
            use tokio::net::TcpListener;
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use serde_json::json;
            use std::net::SocketAddr;
            println!("🚀 启动 JSON-RPC 服务，监听端口 {}", port);
            let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
            let chain_db = Arc::clone(&chain_db);
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let chain_db = Arc::clone(&chain_db);
                tokio::spawn(async move {
                    let mut buf = [0; 4096];
                    if let Ok(n) = socket.read(&mut buf).await {
                        if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                            if let Some(body_start) = text.find("\r\n\r\n") {
                                let body = &text[body_start + 4..];
                                if let Ok(req) = serde_json::from_str::<serde_json::Value>(body) {
                                    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                    if method == "send_transaction" {
                                        if let Some(params) = req.get("params").and_then(|p| p.as_array()) {
                                            if params.len() == 3 {
                                                let from = params[0].as_str().unwrap_or("");
                                                let to = params[1].as_str().unwrap_or("");
                                                let amount = params[2].as_u64().unwrap_or(0);
                                                let tx = Transaction::new(from, to, amount);
                                                println!("[JSON-RPC] 交易提交: {} -> {} [{}]", from, to, amount);
                                                let resp = json!({"jsonrpc":"2.0","result":"ok","id":req.get("id").cloned().unwrap_or(json!(1))});
                                                let resp_str = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                                let _ = socket.write_all(resp_str.as_bytes()).await;
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                            let resp = json!({"jsonrpc":"2.0","error":"invalid request","id":null});
                            let resp_str = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                            let _ = socket.write_all(resp_str.as_bytes()).await;
                        }
                    }
                });
            }
        }
        Commands::QueryTx { hash } => {
            match storage::get_transaction_by_hash(&chain_db, &hash) {
                Some(tx) => {
                    println!("交易哈希: {}", hash);
                    println!("交易详情: from: {} -> to: {} amount: {}", tx.from, tx.to, tx.amount);
                }
                None => println!("未找到该交易"),
            }
        }
    }
}
