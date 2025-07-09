mod account;
mod block;
mod blockchain;
mod mempool;
mod network;
mod peers;
mod storage;
mod transaction;
use network::{broadcast_block, broadcast_transaction};

use blockchain::Blockchain;
use clap::{Parser, Subcommand};
use mempool::Mempool;
use peers::PeerManager;
use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use storage::{
    add_account, get_balance, get_block_by_index, init_account_table, init_db, save_block,
    set_balance,
};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use transaction::Transaction;

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

async fn handle_peer_connection(addr: &String, discovered: &mut PeerManager) {
    if let Ok(mut stream) = tokio::net::TcpStream::connect(&addr).await {
        let req = serde_json::json!({"type": "peers_request"});
        let _ = stream
            .write_all(serde_json::to_string(&req).unwrap().as_bytes())
            .await;
        let mut buf = [0; 2048];
        if let Ok(n) = stream.read(&mut buf).await {
            if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                    #[warn(unused_must_use)]
                    network::handle_json_value(val, discovered).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Submit { from, to, amount } => {
            let tx = Transaction::new(&from, &to, amount);
            println!("💸 交易提交: {} -> {} [{}]", from, to, amount);
            // 读取 peers 列表，广播交易
            let peer_conn = Connection::open("peers.db").unwrap();
            let peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
            broadcast_transaction(&tx, &peers).await;
            // 假设本地节点监听 8000
            let addr = "127.0.0.1:8000";
            let data = serde_json::to_string(&tx).unwrap();
            if let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await {
                let _ = stream.write_all(data.as_bytes()).await;
            }
            // 本地 mempool 持久化
            let conn = Connection::open("chain.db").unwrap();
            let mut mempool = Mempool::default();
            mempool.load_from_db(&conn);
            mempool.add(tx, Some(&conn));
        }

        Commands::Run { port } => {
            println!("🚀 启动 PoS 节点，监听端口 {}", port);

            let conn = Connection::open("chain.db").unwrap();
            init_db(&conn).unwrap();
            init_account_table(&conn).unwrap();
            // 初始化 admin 账户
            add_account(&conn, "admin", 1000000).unwrap();
            add_account(&conn, "Alice", 100).unwrap();
            add_account(&conn, "Bob", 100).unwrap();
            let conn_arc = Arc::new(Mutex::new(conn));

            // 加载 peers
            let peer_conn = Connection::open("peers.db").unwrap();
            let peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
            let _peers_arc = Arc::new(Mutex::new(peers));

            // 从数据库恢复区块链
            let mut chain = Blockchain::new();
            let conn = conn_arc.lock().unwrap();
            let mut idx = 0u64;
            loop {
                let block_opt = get_block_by_index(&conn, idx).unwrap();
                if let Some(block) = block_opt {
                    chain.chain.push(block);
                    idx += 1;
                } else {
                    break;
                }
            }
            if chain.chain.is_empty() {
                chain.create_genesis_block();
                save_block(&conn, chain.chain.last().unwrap()).unwrap();
            }
            drop(conn);

            let mut mempool = Mempool::default();
            {
                let conn = conn_arc.lock().unwrap();
                mempool.load_from_db(&conn);
            }
            mempool.add(
                Transaction::new("Alice", "Bob", 10),
                Some(&conn_arc.lock().unwrap()),
            );
            mempool.add(
                Transaction::new("Bob", "Charlie", 5),
                Some(&conn_arc.lock().unwrap()),
            );

            let peers = PeerManager::default();
            let _peers_arc = Arc::new(Mutex::new(peers));

            // 启动持续出块任务
            let chain_arc = Arc::new(Mutex::new(chain));
            let mempool_arc = Arc::new(Mutex::new(mempool));
            let peers_for_task = Arc::clone(&_peers_arc);
            let chain_for_task = Arc::clone(&chain_arc);
            let mempool_for_task = Arc::clone(&mempool_arc);
            let conn_for_task = Arc::clone(&conn_arc);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    // 每3秒都出块（即使没有交易）
                    let txs = {
                        let mut mempool = mempool_for_task.lock().unwrap();
                        let conn = conn_for_task.lock().unwrap();
                        mempool.collect_for_block(10, Some(&conn))
                    };
                    let (block, proposer) = {
                        let mut chain = chain_for_task.lock().unwrap();
                        chain.add_block(txs.clone());
                        let block = chain.chain.last().unwrap().clone();
                        let proposer = block.proposer.clone();
                        (block, proposer)
                    };
                    // 处理区块内所有交易的余额
                    {
                        let conn = conn_for_task.lock().unwrap();
                        for tx in &block.transactions {
                            let from_balance = get_balance(&conn, &tx.from).unwrap_or(0);
                            if from_balance >= tx.amount {
                                set_balance(&conn, &tx.from, from_balance - tx.amount).unwrap();
                                let to_balance = get_balance(&conn, &tx.to).unwrap_or(0);
                                set_balance(&conn, &tx.to, to_balance + tx.amount).unwrap();
                            }
                        }
                        // 出块奖励给 proposer
                        let reward = 50;
                        let proposer_balance = get_balance(&conn, &proposer).unwrap_or(0);
                        set_balance(&conn, &proposer, proposer_balance + reward).unwrap();
                        save_block(&conn, &block).unwrap();
                    }
                    println!(
                        "[⛓️ 出块] 高度: {} | Hash: {} | 提议者: {} | 交易数: {}",
                        block.index,
                        block.hash,
                        block.proposer,
                        block.transactions.len()
                    );
                    println!("📊 账户余额：");
                    {
                        let conn = conn_for_task.lock().unwrap();
                        let mut stmt = conn
                            .prepare("SELECT address, balance FROM accounts")
                            .unwrap();
                        let mut rows = stmt.query([]).unwrap();
                        while let Some(row) = rows.next().unwrap() {
                            let address: String = row.get(0).unwrap();
                            let balance: u64 = row.get(1).unwrap();
                            println!(" - {}: {}", address, balance);
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
            tokio::spawn(async move {
                use serde_json::json;
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                use tokio::net::TcpListener;
                println!("🚀 启动 JSON-RPC 服务，监听端口 8545");
                let listener = TcpListener::bind(("0.0.0.0", 8545)).await.unwrap();
                loop {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let mempool_for_rpc = Arc::clone(&mempool_for_rpc);
                    tokio::spawn(async move {
                        let mut buf = [0; 4096];
                        if let Ok(n) = socket.read(&mut buf).await {
                            if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                                if let Some(body_start) = text.find("\r\n\r\n") {
                                    let body = &text[body_start + 4..];
                                    if let Ok(req) = serde_json::from_str::<serde_json::Value>(body)
                                    {
                                        let method = req
                                            .get("method")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("");
                                        if method == "send_transaction" {
                                            if let Some(params) =
                                                req.get("params").and_then(|p| p.as_array())
                                            {
                                                if params.len() == 3 {
                                                    let from = params[0].as_str().unwrap_or("");
                                                    let to = params[1].as_str().unwrap_or("");
                                                    let amount = params[2].as_u64().unwrap_or(0);
                                                    let _tx = Transaction::new(from, to, amount);
                                                    println!(
                                                        "[JSON-RPC] 交易提交: {} -> {} [{}]",
                                                        from, to, amount
                                                    );
                                                    // 计算交易hash
                                                    use sha2::{Digest, Sha256};
                                                    let tx_str =
                                                        format!("{}{}{}", from, to, amount);
                                                    let mut hasher = Sha256::new();
                                                    hasher.update(tx_str.as_bytes());
                                                    let tx_hash =
                                                        format!("0x{:x}", hasher.finalize());
                                                    // 直接插入 mempool
                                                    mempool_for_rpc.lock().unwrap().add(
                                                        _tx,
                                                        Some(
                                                            &Connection::open("chain.db").unwrap(),
                                                        ),
                                                    );
                                                    let resp = json!({
                                                        "jsonrpc": "2.0",
                                                        "result": {"status": "ok", "tx_hash": tx_hash},
                                                        "id": req.get("id").cloned().unwrap_or(json!(1))
                                                    });
                                                    let resp_str = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                                    let _ =
                                                        socket.write_all(resp_str.as_bytes()).await;
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                                let resp =
                                    json!({"jsonrpc":"2.0","error":"invalid request","id":null});
                                let resp_str = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                let _ = socket.write_all(resp_str.as_bytes()).await;
                            }
                        }
                    });
                }
            });

            let peers_for_discover = Arc::clone(&_peers_arc);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    // 先 clone peers 列表
                    let peer_list = {
                        let peers = peers_for_discover.lock().unwrap();
                        peers.list()
                    };
                    // 异步发现
                    let mut discovered = PeerManager::default();
                    for addr in peer_list {
                        handle_peer_connection(&addr, &mut discovered).await;
                    }
                    // 合并新 peers
                    let mut peers = peers_for_discover.lock().unwrap();
                    let before = peers.list().len();
                    for addr in discovered.list() {
                        peers.add_peer(addr);
                    }
                    let after = peers.list().len();
                    if after > before {
                        println!(
                            "[发现节点] 新增 {} 个节点，当前已知节点总数: {}",
                            after - before,
                            after
                        );
                    } else {
                        println!("[发现节点] 未发现新节点，当前已知节点总数: {}", after);
                    }
                    // 保存到 peers.db
                    let peer_conn = Connection::open("peers.db").unwrap();
                    let _ = peers.save_to_db(&peer_conn);
                }
            });

            network::start_server(port, chain_arc, mempool_arc).await;
        }

        Commands::Query { index } => {
            let conn = Connection::open("chain.db").unwrap();
            match get_block_by_index(&conn, index) {
                Ok(Some(block)) => {
                    println!("区块高度: {}", block.index);
                    println!("Hash: {}", block.hash);
                    println!("前置Hash: {}", block.previous_hash);
                    println!("提议者: {}", block.proposer);
                    println!("时间戳: {}", block.timestamp);
                    println!("交易: {:?}", block.transactions);
                }
                Ok(None) => println!("未找到该高度区块"),
                Err(e) => println!("查询出错: {}", e),
            }
        }
        Commands::QueryBalance { address } => {
            let conn = Connection::open("chain.db").unwrap();
            match get_balance(&conn, &address) {
                Ok(balance) => println!("{} 余额: {}", address, balance),
                Err(e) => println!("查询出错: {}", e),
            }
        }
        Commands::AddPeer { addr } => {
            let peer_conn = Connection::open("peers.db").unwrap();
            let mut peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
            peers.add_peer(addr.clone());
            peers.save_to_db(&peer_conn).unwrap();
            println!("已添加节点: {}", addr);
        }
        Commands::QueryPeers => {
            let peer_conn = Connection::open("peers.db").unwrap();
            let peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
            peers.display_peers();
        }
        Commands::JsonRpcServer { port } => {
            use serde_json::json;
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use tokio::net::TcpListener;
            println!("🚀 启动 JSON-RPC 服务，监听端口 {}", port);
            let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut buf = [0; 4096];
                    if let Ok(n) = socket.read(&mut buf).await {
                        if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                            // 解析 HTTP POST
                            if let Some(body_start) = text.find("\r\n\r\n") {
                                let body = &text[body_start + 4..];
                                if let Ok(req) = serde_json::from_str::<serde_json::Value>(body) {
                                    let method =
                                        req.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                    if method == "send_transaction" {
                                        if let Some(params) =
                                            req.get("params").and_then(|p| p.as_array())
                                        {
                                            if params.len() == 3 {
                                                let from = params[0].as_str().unwrap_or("");
                                                let to = params[1].as_str().unwrap_or("");
                                                let amount = params[2].as_u64().unwrap_or(0);
                                                let _tx = Transaction::new(from, to, amount);
                                                // 这里直接打印，实际可插入 mempool 或广播
                                                println!(
                                                    "[JSON-RPC] 交易提交: {} -> {} [{}]",
                                                    from, to, amount
                                                );
                                                let resp = json!({"jsonrpc":"2.0","result":"ok","id":req.get("id").cloned().unwrap_or(json!(1))});
                                                let resp_str = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                                                let _ = socket.write_all(resp_str.as_bytes()).await;
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                            // 非法请求
                            let resp = json!({"jsonrpc":"2.0","error":"invalid request","id":null});
                            let resp_str = format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", resp.to_string().len(), resp.to_string());
                            let _ = socket.write_all(resp_str.as_bytes()).await;
                        }
                    }
                });
            }
        }
        Commands::QueryTx { hash } => {
            let conn = Connection::open("chain.db").unwrap();
            match storage::get_transaction_by_hash(&conn, &hash) {
                Ok(Some((block_idx, tx))) => {
                    println!("交易哈希: {}", hash);
                    println!("区块高度: {}", block_idx);
                    println!(
                        "交易详情: from: {} -> to: {} amount: {}",
                        tx.from, tx.to, tx.amount
                    );
                }
                Ok(None) => println!("未找到该交易"),
                Err(e) => println!("查询出错: {}", e),
            }
        }
    }
}
