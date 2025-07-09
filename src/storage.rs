// 依赖：请在 Cargo.toml 添加 rocksdb = "0.21"
use crate::block::Block;
use crate::transaction::Transaction;
use rocksdb::{DB, Options, IteratorMode};
use std::sync::{Arc, Mutex};
use bincode;

pub type RocksDB = Arc<Mutex<DB>>;

pub fn open_db(path: &str) -> RocksDB {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, path).expect("Failed to open RocksDB");
    Arc::new(Mutex::new(db))
}

// Block
pub fn save_block(db: &RocksDB, block: &Block) {
    let key = format!("block:{}", block.index);
    let value = bincode::serialize(block).unwrap();
    db.lock().unwrap().put(key.as_bytes(), &value).unwrap();
}

pub fn get_block_by_index(db: &RocksDB, idx: u64) -> Option<Block> {
    let key = format!("block:{}", idx);
    if let Ok(Some(val)) = db.lock().unwrap().get(key.as_bytes()) {
        bincode::deserialize(&val).ok()
    } else {
        None
    }
}

// Account
pub fn add_account(db: &RocksDB, address: &str, balance: u64) {
    let key = format!("account:{}", address);
    let value = bincode::serialize(&balance).unwrap();
    db.lock().unwrap().put(key.as_bytes(), &value).unwrap();
}

pub fn get_balance(db: &RocksDB, address: &str) -> u64 {
    let key = format!("account:{}", address);
    if let Ok(Some(val)) = db.lock().unwrap().get(key.as_bytes()) {
        bincode::deserialize(&val).unwrap_or(0)
    } else {
        0
    }
}

pub fn set_balance(db: &RocksDB, address: &str, balance: u64) {
    add_account(db, address, balance);
}

// Transaction (for query by hash)
#[allow(dead_code)]
pub fn save_transaction(db: &RocksDB, tx_hash: &str, tx: &Transaction) {
    let key = format!("tx:{}", tx_hash);
    let value = bincode::serialize(tx).unwrap();
    db.lock().unwrap().put(key.as_bytes(), &value).unwrap();
}

pub fn get_transaction_by_hash(db: &RocksDB, tx_hash: &str) -> Option<Transaction> {
    let key = format!("tx:{}", tx_hash);
    if let Ok(Some(val)) = db.lock().unwrap().get(key.as_bytes()) {
        bincode::deserialize(&val).ok()
    } else {
        None
    }
}

// Mempool
pub fn insert_mempool_tx(db: &RocksDB, tx_hash: &str, tx: &Transaction) {
    let key = format!("mempool:{}", tx_hash);
    let value = bincode::serialize(tx).unwrap();
    db.lock().unwrap().put(key.as_bytes(), &value).unwrap();
}

#[allow(dead_code)]
pub fn remove_mempool_tx(db: &RocksDB, tx_hash: &str) {
    let key = format!("mempool:{}", tx_hash);
    db.lock().unwrap().delete(key.as_bytes()).unwrap();
}

pub fn load_all_mempool_txs(db: &RocksDB) -> Vec<Transaction> {
    let db = db.lock().unwrap();
    let mut txs = Vec::new();
    let iter = db.iterator(IteratorMode::Start);
    for item in iter {
        if let Ok((key, value)) = item {
            if let Ok(k) = std::str::from_utf8(&key) {
                if k.starts_with("mempool:") {
                    if let Ok(tx) = bincode::deserialize::<crate::transaction::Transaction>(&value) {
                        txs.push(tx);
                    }
                }
            }
        }
    }
    txs
}

// Peers (存储为 peers:Vec<String>)
pub fn save_peers(db: &RocksDB, peers: &Vec<String>) {
    let key = b"peers";
    let value = bincode::serialize(peers).unwrap();
    db.lock().unwrap().put(key, &value).unwrap();
}

pub fn load_peers(db: &RocksDB) -> Vec<String> {
    let key = b"peers";
    if let Ok(Some(val)) = db.lock().unwrap().get(key) {
        bincode::deserialize(&val).unwrap_or_default()
    } else {
        Vec::new()
    }
}
