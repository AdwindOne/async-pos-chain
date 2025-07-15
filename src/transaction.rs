use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
