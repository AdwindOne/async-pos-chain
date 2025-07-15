use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
/// 交易结构体，包含 from、to、amount 字段
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

impl Transaction {
    /// 创建新交易
    pub fn new(from: &str, to: &str, amount: u64) -> Self {
        Transaction {
            from: from.to_string(),
            to: to.to_string(),
            amount,
        }
    }
    /// 计算交易哈希
    pub fn hash(&self) -> String {
        let tx_str = format!("{}{}{}", self.from, self.to, self.amount);
        let mut hasher = Sha256::new();
        hasher.update(tx_str.as_bytes());
        format!("0x{:x}", hasher.finalize())
    }
}
