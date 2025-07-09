use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AccountState {
    pub balances: HashMap<String, u64>,
}

impl AccountState {
    pub fn new() -> Self {
        AccountState {
            balances: HashMap::new(),
        }
    }

    pub fn apply_transaction(&mut self, from: &str, to: &str, amount: u64) -> bool {
        let from_balance = self.balances.entry(from.to_string()).or_insert(100); // 初始100
        if *from_balance < amount {
            return false;
        }
        *from_balance -= amount;
        let to_balance = self.balances.entry(to.to_string()).or_insert(0);
        *to_balance += amount;
        true
    }

    #[allow(dead_code)]
    pub fn show(&self) {
        println!("📊 账户余额：");
        for (k, v) in &self.balances {
            println!(" - {}: {}", k, v);
        }
    }
}
