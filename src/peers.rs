use serde::{Serialize, Deserialize};
use crate::storage::RocksDB;

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct PeerManager {
    pub peers: Vec<String>,
}

impl PeerManager {
    /// 创建新的 PeerManager
    pub fn new() -> Self {
        PeerManager { peers: Vec::new() }
    }

    /// 添加节点地址
    pub fn add_peer(&mut self, addr: String) {
        if !self.peers.contains(&addr) {
            self.peers.push(addr);
        }
    }

    /// 移除节点地址
    #[allow(dead_code)]
    pub fn remove_peer(&mut self, addr: &str) {
        self.peers.retain(|p| p != addr);
    }

    /// 保存节点列表到 RocksDB
    pub fn save_to_db(&self, db: &RocksDB) {
        crate::storage::save_peers(db, &self.peers);
    }

    /// 从 RocksDB 加载节点列表
    pub fn load_from_db(db: &RocksDB) -> Self {
        let peers = crate::storage::load_peers(db);
        PeerManager { peers }
    }

    /// 打印节点列表
    pub fn display_peers(&self) {
        println!("已知节点列表:");
        for (i, addr) in self.peers.iter().enumerate() {
            println!("  [{}] {}", i, addr);
        }
    }

    /// 获取节点列表副本
    pub fn list(&self) -> Vec<String> {
        self.peers.clone()
    }
}
