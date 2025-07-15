use crate::cli::{Cli, Command};
use crate::blockchain::Blockchain;
use crate::mempool::Mempool;
use crate::peers::PeerManager;
use crate::transaction::Transaction;
use crate::storage;
use crate::network;
use crate::rpc;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn dispatch(cli: Cli, chain_db: storage::RocksDB, peers_db: storage::RocksDB) {
    match cli.command {
        Command::Run { port } => run_node(port, chain_db.clone(), peers_db.clone()).await,
        Command::Submit { from, to, amount } => submit_tx(from, to, amount, chain_db.clone(), peers_db.clone()).await,
        Command::Query { index } => query_block(index, chain_db.clone()),
        Command::QueryBalance { address } => query_balance(address, chain_db.clone()),
        Command::AddPeer { addr } => add_peer(addr, peers_db.clone()),
        Command::QueryPeers => query_peers(peers_db.clone()),
        Command::JsonRpcServer { port } => spawn_jsonrpc_server(port, chain_db.clone()).await,
        Command::QueryTx { hash } => query_tx(hash, chain_db.clone()),
    }
}

pub async fn run_node(port: u16, chain_db: storage::RocksDB, peers_db: storage::RocksDB) {
    init_accounts(&chain_db);
    let peers_arc = Arc::new(Mutex::new(PeerManager::load_from_db(&peers_db)));
    let chain_arc = Arc::new(Mutex::new(recover_blockchain(&chain_db)));
    let mempool_arc = Arc::new(Mutex::new(init_mempool(&chain_db)));

    spawn_block_producer(chain_arc.clone(), mempool_arc.clone(), peers_arc.clone(), chain_db.clone()).await;
    spawn_jsonrpc(mempool_arc.clone(), chain_db.clone()).await;
    spawn_peer_discovery(peers_arc.clone(), peers_db.clone()).await;

    network::start_server(port, chain_arc, mempool_arc).await;
}

fn init_accounts(chain_db: &storage::RocksDB) {
    storage::add_account(chain_db, "admin", 1000000);
    storage::add_account(chain_db, "Alice", 100);
    storage::add_account(chain_db, "Bob", 100);
}

fn recover_blockchain(chain_db: &storage::RocksDB) -> Blockchain {
    let mut chain = Blockchain::new();
    let mut idx = 0u64;
    loop {
        if let Some(block) = storage::get_block_by_index(chain_db, idx) {
            chain.chain.push(block);
            idx += 1;
        } else {
            break;
        }
    }
    if chain.chain.is_empty() {
        chain.create_genesis_block();
        storage::save_block(chain_db, chain.chain.last().unwrap());
    }
    chain
}

fn init_mempool(chain_db: &storage::RocksDB) -> Mempool {
    let mut mempool = Mempool::new();
    mempool.load_from_db(chain_db);
    mempool.add(Transaction::new("Alice", "Bob", 10), Some(chain_db));
    mempool.add(Transaction::new("Bob", "Charlie", 5), Some(chain_db));
    mempool
}

async fn spawn_block_producer(
    chain_arc: Arc<Mutex<Blockchain>>,
    mempool_arc: Arc<Mutex<Mempool>>,
    peers_arc: Arc<Mutex<PeerManager>>,
    chain_db: storage::RocksDB,
) {
    let chain_for_task = Arc::clone(&chain_arc);
    let mempool_for_task = Arc::clone(&mempool_arc);
    let peers_for_task = Arc::clone(&peers_arc);
    let chain_db_for_task = chain_db.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let txs = {
                let mut mempool = mempool_for_task.lock().unwrap();
                mempool.collect_for_block(10, Some(&chain_db_for_task))
            };
            let (block, proposer) = {
                let mut chain = chain_for_task.lock().unwrap();
                chain.add_block(txs.clone());
                let block = chain.chain.last().unwrap().clone();
                let proposer = block.proposer.clone();
                (block, proposer)
            };
            {
                for tx in &block.transactions {
                    let from_balance = storage::get_balance(&chain_db_for_task, &tx.from);
                    if from_balance >= tx.amount {
                        storage::set_balance(&chain_db_for_task, &tx.from, from_balance - tx.amount);
                        let to_balance = storage::get_balance(&chain_db_for_task, &tx.to);
                        storage::set_balance(&chain_db_for_task, &tx.to, to_balance + tx.amount);
                    }
                }
                let reward = 50;
                let proposer_balance = storage::get_balance(&chain_db_for_task, &proposer);
                storage::set_balance(&chain_db_for_task, &proposer, proposer_balance + reward);
                storage::save_block(&chain_db_for_task, &block);
            }
            println!("[⛓️ 出块] 高度: {} | Hash: {} | 提议者: {} | 交易数: {}", block.index, block.hash, block.proposer, block.transactions.len());
            print_account_balances(&chain_db_for_task, &["admin", "Alice", "Bob", "Charlie"]);
            let peer_list = {
                let peers = peers_for_task.lock().unwrap();
                peers.list()
            };
            network::broadcast_block(&block, &PeerManager { peers: peer_list }).await;
        }
    });
}

async fn spawn_jsonrpc(mempool: Arc<Mutex<Mempool>>, chain_db: storage::RocksDB) {
    let mempool_for_rpc = Arc::clone(&mempool);
    let chain_db_for_rpc = chain_db.clone();
    tokio::spawn(async move {
        rpc::start_jsonrpc_server(8545, mempool_for_rpc, chain_db_for_rpc).await;
    });
}

async fn spawn_peer_discovery(peers_arc: Arc<Mutex<PeerManager>>, peers_db: storage::RocksDB) {
    let peers_for_discover = Arc::clone(&peers_arc);
    let peers_db_for_discover = peers_db.clone();
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
}


pub async fn submit_tx(from: String, to: String, amount: u64, chain_db: storage::RocksDB, peers_db: storage::RocksDB) {
    let tx = Transaction::new(&from, &to, amount);
    let tx_hash = tx.hash();
    storage::insert_mempool_tx(&chain_db, &tx_hash, &tx);
    println!("已提交交易: {} -> {} [{}]", from, to, amount);
    // 广播到网络
    let peers = PeerManager::load_from_db(&peers_db);
    network::broadcast_transaction(&tx, &peers).await;
}

pub fn query_block(index: u64, chain_db: storage::RocksDB) {
    match storage::get_block_by_index(&chain_db, index) {
        Some(block) => {
            print_block(&block);
        }
        None => println!("未找到该高度区块"),
    }
}

pub fn query_balance(address: String, chain_db: storage::RocksDB) {
    let balance = storage::get_balance(&chain_db, &address);
    println!("{} 余额: {}", address, balance);
}

pub fn add_peer(addr: String, peers_db: storage::RocksDB) {
    let mut peers = PeerManager::load_from_db(&peers_db);
    peers.add_peer(addr.clone());
    peers.save_to_db(&peers_db);
    println!("已添加节点: {}", addr);
}

pub fn query_peers(peers_db: storage::RocksDB) {
    let peers = PeerManager::load_from_db(&peers_db);
    peers.display_peers();
}

pub async fn spawn_jsonrpc_server(port: u16, chain_db: storage::RocksDB) {
    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let chain_db_for_rpc = Arc::clone(&chain_db);
    rpc::start_jsonrpc_server(port, mempool, chain_db_for_rpc).await;
}

pub fn query_tx(hash: String, chain_db: storage::RocksDB) {
    match storage::get_transaction_by_hash(&chain_db, &hash) {
        Some(tx) => {
            print_transaction(&hash, &tx);
        }
        None => println!("未找到该交易"),
    }
}

fn print_account_balances(chain_db: &storage::RocksDB, addrs: &[&str]) {
    println!("📊 账户余额：");
    for addr in addrs {
        let balance = storage::get_balance(chain_db, addr);
        println!(" - {}: {}", addr, balance);
    }
}

fn print_block(block: &crate::block::Block) {
    println!("区块高度: {}", block.index);
    println!("Hash: {}", block.hash);
    println!("前置Hash: {}", block.previous_hash);
    println!("提议者: {}", block.proposer);
    println!("时间戳: {}", block.timestamp);
    println!("交易: {:?}", block.transactions);
}

fn print_transaction(hash: &str, tx: &crate::transaction::Transaction) {
    println!("交易哈希: {}", hash);
    println!("交易详情: from: {} -> to: {} amount: {}", tx.from, tx.to, tx.amount);
} 