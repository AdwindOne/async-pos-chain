use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::network;
use crate::peers::PeerManager;
use crate::storage;
use crate::transaction::Transaction;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;

tokio::task_local! {
    static NODE_LOG: String;
}

pub async fn run_node(port: u16) {
    println!("🚀 启动 PoS 节点，监听端口 {}", port);
    let conn_arc = Arc::new(Mutex::new(init_db_and_accounts()));
    let _peers_arc = Arc::new(Mutex::new(load_peers()));
    let chain_arc = Arc::new(Mutex::new(load_blockchain(&conn_arc)));
    let mempool_arc = Arc::new(Mutex::new(load_mempool(&conn_arc)));

    spawn_block_producer(
        Arc::clone(&chain_arc),
        Arc::clone(&mempool_arc),
        Arc::clone(&_peers_arc),
        Arc::clone(&conn_arc),
    );
    spawn_jsonrpc_server();
    spawn_peer_discovery(Arc::clone(&_peers_arc));
    network::start_server(port, chain_arc, mempool_arc).await;
}

fn init_db_and_accounts() -> Connection {
    let conn = Connection::open("chain.db").unwrap();
    storage::init_db(&conn).unwrap();
    storage::init_account_table(&conn).unwrap();
    storage::add_account(&conn, "admin", 1000000).unwrap();
    storage::add_account(&conn, "Alice", 100).unwrap();
    storage::add_account(&conn, "Bob", 100).unwrap();
    conn
}

fn load_peers() -> PeerManager {
    let peer_conn = Connection::open("peers.db").unwrap();
    PeerManager::load_from_db(&peer_conn).unwrap_or_default()
}

fn load_blockchain(conn_arc: &Arc<Mutex<Connection>>) -> Blockchain {
    let mut chain = Blockchain::new();
    let conn = conn_arc.lock().unwrap();
    let mut idx = 0u64;
    loop {
        let block_opt = storage::get_block_by_index(&conn, idx).unwrap();
        if let Some(block) = block_opt {
            chain.chain.push(block);
            idx += 1;
        } else {
            break;
        }
    }
    if chain.chain.is_empty() {
        chain.create_genesis_block();
        storage::save_block(&conn, chain.chain.last().unwrap()).unwrap();
    }
    chain
}

fn load_mempool(conn_arc: &Arc<Mutex<Connection>>) -> Mempool {
    let mut mempool = Mempool::default();
    let conn = conn_arc.lock().unwrap();
    mempool.load_from_db(&conn);
    mempool.add(Transaction::new("Alice", "Bob", 10), Some(&conn));
    mempool.add(Transaction::new("Bob", "Charlie", 5), Some(&conn));
    mempool
}

fn spawn_block_producer(
    chain_arc: Arc<Mutex<Blockchain>>,
    mempool_arc: Arc<Mutex<Mempool>>,
    peers_arc: Arc<Mutex<PeerManager>>,
    conn_arc: Arc<Mutex<Connection>>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let txs = {
                let mut mempool = mempool_arc.lock().unwrap();
                let conn = conn_arc.lock().unwrap();
                mempool.collect_for_block(10, Some(&conn))
            };
            let (block, proposer) = {
                let mut chain = chain_arc.lock().unwrap();
                chain.add_block(txs.clone());
                let block = chain.chain.last().unwrap().clone();
                let proposer = block.proposer.clone();
                (block, proposer)
            };
            {
                let conn = conn_arc.lock().unwrap();
                for tx in &block.transactions {
                    let from_balance = storage::get_balance(&conn, &tx.from).unwrap_or(0);
                    if from_balance >= tx.amount {
                        storage::set_balance(&conn, &tx.from, from_balance - tx.amount).unwrap();
                        let to_balance = storage::get_balance(&conn, &tx.to).unwrap_or(0);
                        storage::set_balance(&conn, &tx.to, to_balance + tx.amount).unwrap();
                    }
                }
                let reward = 50;
                let proposer_balance = storage::get_balance(&conn, &proposer).unwrap_or(0);
                storage::set_balance(&conn, &proposer, proposer_balance + reward).unwrap();
                storage::save_block(&conn, &block).unwrap();
            }
            print_block_info(&block);
            print_account_balances(&conn_arc);
            let peer_list = {
                let peers = peers_arc.lock().unwrap();
                peers.list()
            };
            network::broadcast_block(&block, &PeerManager { peers: peer_list }).await;
        }
    });
}

fn print_block_info(block: &crate::block::Block) {
    println!(
        "[⛓️ 出块] 高度: {} | Hash: {} | 提议者: {} | 交易数: {}",
        block.index,
        block.hash,
        block.proposer,
        block.transactions.len()
    );
}

fn print_account_balances(conn_arc: &Arc<Mutex<Connection>>) {
    println!("📊 账户余额：");
    let conn = conn_arc.lock().unwrap();
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

fn spawn_jsonrpc_server() {
    tokio::spawn(async move {
        crate::rpc::start_jsonrpc_server(8545).await;
    });
}

fn spawn_peer_discovery(peers_arc: Arc<Mutex<PeerManager>>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let peer_list = {
                let peers = peers_arc.lock().unwrap();
                peers.list()
            };
            let mut discovered = PeerManager::default();
            for _ in peer_list {
                network::discover_peers(&mut discovered).await;
            }
            let mut peers = peers_arc.lock().unwrap();
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
            let peer_conn = Connection::open("peers.db").unwrap();
            let _ = peers.save_to_db(&peer_conn);
        }
    });
}

pub async fn submit_tx(from: String, to: String, amount: u64) {
    let tx = Transaction::new(&from, &to, amount);
    println!("💸 交易提交: {} -> {} [{}]", from, to, amount);
    let peer_conn = Connection::open("peers.db").unwrap();
    let peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
    network::broadcast_transaction(&tx, &peers).await;
    let data = serde_json::to_string(&tx).unwrap();
    if let Ok(mut stream) = tokio::net::TcpStream::connect("127.0.0.1:8000").await {
        let _ = stream.write_all(data.as_bytes()).await;
    }
    let conn = Connection::open("chain.db").unwrap();
    let mut mempool = Mempool::default();
    mempool.load_from_db(&conn);
    mempool.add(tx, Some(&conn));
}

pub fn query_block(index: u64) {
    let conn = Connection::open("chain.db").unwrap();
    match storage::get_block_by_index(&conn, index) {
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

pub fn query_balance(address: String) {
    let conn = Connection::open("chain.db").unwrap();
    match storage::get_balance(&conn, &address) {
        Ok(balance) => println!("{} 余额: {}", address, balance),
        Err(e) => println!("查询出错: {}", e),
    }
}

pub fn add_peer(addr: String) {
    let peer_conn = Connection::open("peers.db").unwrap();
    let mut peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
    peers.add_peer(addr.clone());
    peers.save_to_db(&peer_conn).unwrap();
    println!("已添加节点: {}", addr);
}

pub fn query_peers() {
    let peer_conn = Connection::open("peers.db").unwrap();
    let peers = PeerManager::load_from_db(&peer_conn).unwrap_or_default();
    peers.display_peers();
}

pub fn query_tx(hash: String) {
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
