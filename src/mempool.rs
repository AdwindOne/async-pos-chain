use crate::transaction::Transaction;
use crate::storage::RocksDB;

#[derive(Default)]
pub struct Mempool {
    pub txs: Vec<Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        Mempool { txs: Vec::new() }
    }

    pub fn add(&mut self, tx: Transaction, db: Option<&RocksDB>) {
        if let Some(db) = db {
            let tx_hash = tx.hash();
            let _ = crate::storage::insert_mempool_tx(db, &tx_hash, &tx);
        }
        self.txs.push(tx);
    }

    pub fn remove(&mut self, tx: &Transaction, db: Option<&RocksDB>) {
        if let Some(db) = db {
            let tx_hash = tx.hash();
            let _ = crate::storage::remove_mempool_tx(db, &tx_hash);
        }
        self.txs.retain(|t| t != tx);
    }

    pub fn load_from_db(&mut self, db: &RocksDB) {
        self.txs = crate::storage::load_all_mempool_txs(db);
    }

    pub fn collect_for_block(&mut self, max: usize, _db: Option<&crate::storage::RocksDB>) -> Vec<crate::transaction::Transaction> {
        let n = max.min(self.txs.len());
        self.txs.drain(0..n).collect()
    }
}
