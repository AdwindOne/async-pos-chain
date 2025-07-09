use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

impl Transaction {
    pub fn new(from: &str, to: &str, amount: u64) -> Self {
        Transaction {
            from: from.to_string(),
            to: to.to_string(),
            amount,
        }
    }
    pub fn hash(&self) -> String {
        let tx_str = format!("{}{}{}", self.from, self.to, self.amount);
        let mut hasher = Sha256::new();
        hasher.update(tx_str.as_bytes());
        format!("0x{:x}", hasher.finalize())
    }
}
