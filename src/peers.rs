use serde::{Serialize, Deserialize};
use crate::storage::RocksDB;

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct PeerManager {
    pub peers: Vec<String>,
}

impl PeerManager {
    pub fn new() -> Self {
        PeerManager { peers: Vec::new() }
    }

    pub fn add_peer(&mut self, addr: String) {
        if !self.peers.contains(&addr) {
            self.peers.push(addr);
        }
    }

    #[allow(dead_code)]
    pub fn remove_peer(&mut self, addr: &str) {
        self.peers.retain(|p| p != addr);
    }

    pub fn save_to_db(&self, db: &RocksDB) {
        crate::storage::save_peers(db, &self.peers);
    }

    pub fn load_from_db(db: &RocksDB) -> Self {
        let peers = crate::storage::load_peers(db);
        PeerManager { peers }
    }

    pub fn display_peers(&self) {
        println!("已知节点列表:");
        for (i, addr) in self.peers.iter().enumerate() {
            println!("  [{}] {}", i, addr);
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.peers.clone()
    }
}
