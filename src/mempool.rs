use crate::transaction::Transaction;

#[derive(Default)]
pub struct Mempool {
    pub pool: Vec<Transaction>,
}

impl Mempool {
    pub fn add(&mut self, tx: Transaction) {
        self.pool.push(tx);
    }

    pub fn collect_for_block(&mut self, max: usize) -> Vec<Transaction> {
        let txs = self.pool.drain(..max.min(self.pool.len())).collect();
        txs
    }
}
