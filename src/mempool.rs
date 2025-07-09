use crate::transaction::Transaction;
use rusqlite::Connection;

#[derive(Default)]
pub struct Mempool {
    pub pool: Vec<Transaction>,
}

impl Mempool {
    pub fn add(&mut self, tx: Transaction, conn: Option<&Connection>) {
        self.pool.push(tx.clone());
        if let Some(conn) = conn {
            let _ = crate::storage::insert_mempool_tx(conn, &tx);
        }
    }

    pub fn collect_for_block(&mut self, max: usize, conn: Option<&Connection>) -> Vec<Transaction> {
        let txs: Vec<_> = self.pool.drain(..max.min(self.pool.len())).collect();
        if let Some(conn) = conn {
            for tx in &txs {
                let _ = crate::storage::remove_mempool_tx(conn, tx);
            }
        }
        txs
    }

    pub fn load_from_db(&mut self, conn: &Connection) {
        if let Ok(txs) = crate::storage::load_all_mempool_txs(conn) {
            self.pool = txs;
        }
    }
}
