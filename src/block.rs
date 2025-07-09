use crate::transaction::Transaction;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub proposer: String,
    pub hash: String,
}

impl Block {
    pub fn new(index: u64, previous_hash: String, transactions: Vec<Transaction>, proposer: String) -> Self {
        let timestamp = chrono::Utc::now().timestamp() as u64;
        let mut block = Block {
            index,
            previous_hash,
            timestamp,
            transactions,
            proposer,
            hash: String::new(),
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let input = format!(
            "{}{}{}{:?}{}",
            self.index, self.previous_hash, self.timestamp, self.transactions, self.proposer
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
