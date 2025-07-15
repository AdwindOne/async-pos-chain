use crate::account;
use crate::block;
use crate::transaction;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Blockchain {
    pub chain: Vec<block::Block>,
    pub validators: HashMap<String, u64>,
    pub state: account::AccountState,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut bc = Blockchain {
            chain: vec![],
            validators: HashMap::new(),
            state: account::AccountState::new(),
        };
        bc.validators.insert("Alice".into(), 100);
        bc.validators.insert("Bob".into(), 50);
        bc
    }

    pub fn create_genesis_block(&mut self) {
        let genesis = block::Block::new(0, "0".into(), vec![], "genesis".into());
        self.chain.push(genesis);
    }

    pub fn get_last_hash(&self) -> String {
        self.chain
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn select_proposer(&self) -> String {
        let mut rng = rand::thread_rng();
        self.validators
            .iter()
            .flat_map(|(addr, stake)| std::iter::repeat(addr).take(*stake as usize))
            .choose(&mut rng)
            .unwrap_or(&"fallback".into())
            .clone()
    }

    pub fn add_block(&mut self, txs: Vec<transaction::Transaction>) {
        let proposer = self.select_proposer();

        for tx in &txs {
            self.state.apply_transaction(&tx.from, &tx.to, tx.amount);
        }

        let block = block::Block::new(
            self.chain.len() as u64,
            self.get_last_hash(),
            txs,
            proposer.clone(),
        );

        *self.validators.entry(proposer.clone()).or_insert(0) += 10;

        self.chain.push(block);
    }

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
    #[allow(dead_code)]
    pub fn into_arc(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }
}
