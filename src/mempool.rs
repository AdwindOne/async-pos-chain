use crate::transaction::Transaction;
use crate::storage::RocksDB;

#[derive(Default)]
pub struct Mempool {
    pub txs: Vec<Transaction>,
}

impl Mempool {
    /// 创建新的内存池
    pub fn new() -> Self {
        Mempool { txs: Vec::new() }
    }

    /// 向内存池添加交易，并可选地写入 RocksDB
    pub fn add(&mut self, tx: Transaction, db: Option<&RocksDB>) {
        if let Some(db) = db {
            let tx_hash = tx.hash();
            let _ = crate::storage::insert_mempool_tx(db, &tx_hash, &tx);
        }
        self.txs.push(tx);
    }

    /// 从内存池移除交易，并可选地从 RocksDB 删除
    #[allow(dead_code)]
    pub fn remove(&mut self, tx: &Transaction, db: Option<&RocksDB>) {
        if let Some(db) = db {
            let tx_hash = tx.hash();
            let _ = crate::storage::remove_mempool_tx(db, &tx_hash);
        }
        self.txs.retain(|t| t != tx);
    }

    /// 从 RocksDB 加载所有交易到内存池
    pub fn load_from_db(&mut self, db: &RocksDB) {
        self.txs = crate::storage::load_all_mempool_txs(db);
    }

    /// 从内存池收集最多 max 条交易用于出块，并可选地同步到 RocksDB
    pub fn collect_for_block(&mut self, max: usize, _db: Option<&crate::storage::RocksDB>) -> Vec<crate::transaction::Transaction> {
        let n = max.min(self.txs.len());
        self.txs.drain(0..n).collect()
    }
}
