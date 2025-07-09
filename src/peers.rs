#[derive(Debug, Default)]
pub struct PeerManager {
    pub peers: Vec<String>,
}

impl PeerManager {
    pub fn add_peer(&mut self, addr: String) {
        if !self.peers.contains(&addr) {
            self.peers.push(addr);
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.peers.clone()
    }
}
