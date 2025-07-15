use crate::block::Block;
use crate::transaction::Transaction;
use crate::account::AccountState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rand::seq::IteratorRandom;

#[derive(Clone)]
/// 区块链主结构，包含链、验证人、账户状态
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub validators: HashMap<String, u64>,
    pub state: AccountState,
}

impl Blockchain {
    /// 创建新的区块链实例，初始化验证人和账户状态
    pub fn new() -> Self {
        let mut bc = Blockchain {
            chain: vec![],
            validators: HashMap::new(),
            state: AccountState::new(),
        };
        bc.validators.insert("Alice".into(), 100);
        bc.validators.insert("Bob".into(), 50);
        bc
    }

    /// 创建创世区块
    pub fn create_genesis_block(&mut self) {
        let genesis = Block::new(0, "0".into(), vec![], "genesis".into());
        self.chain.push(genesis);
    }

    /// 获取最新区块的哈希
    pub fn get_last_hash(&self) -> String {
        self.chain.last().map(|b| b.hash.clone()).unwrap_or_else(|| "0".to_string())
    }

    /// 随机选择一个提议者（PoS 权重）
    pub fn select_proposer(&self) -> String {
        let mut rng = rand::thread_rng();
        self.validators
            .iter()
            .flat_map(|(addr, stake)| std::iter::repeat(addr).take(*stake as usize))
            .choose(&mut rng)
            .unwrap_or(&"fallback".into())
            .clone()
    }

    /// 添加新区块，应用所有交易，奖励提议者
    pub fn add_block(&mut self, txs: Vec<Transaction>) {
        let proposer = self.select_proposer();

        for tx in &txs {
            self.state.apply_transaction(&tx.from, &tx.to, tx.amount);
        }

        let block = Block::new(self.chain.len() as u64, self.get_last_hash(), txs, proposer.clone());

        *self.validators.entry(proposer.clone()).or_insert(0) += 10;

        self.chain.push(block);
    }

    /// 打印区块链结构
    #[allow(dead_code)]
    pub fn print_chain(&self) {
        println!("📦 区块链结构：");
        for block in &self.chain {
            println!(
                " - 区块 {} | Hash: {} | 提议者: {}",
                block.index, block.hash, block.proposer
            );
        }
    }

    /// 转为 Arc<Mutex<Self>>
    #[allow(dead_code)]
    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }
}
