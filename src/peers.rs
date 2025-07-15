#[derive(Debug, Default)]
pub struct PeerManager {
    pub peers: Vec<String>,
}

use rusqlite::{Connection, Result};

impl PeerManager {
    pub fn add_peer(&mut self, addr: String) {
        if !self.peers.contains(&addr) {
            self.peers.push(addr);
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.peers.clone()
    }

    pub fn display_peers(&self) {
        println!("已知节点列表:");
        for (i, addr) in self.peers.iter().enumerate() {
            println!("  [{}] {}", i, addr);
        }
    }

    pub fn save_to_db(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS peers (addr TEXT PRIMARY KEY);")?;
        for addr in &self.peers {
            conn.execute("INSERT OR IGNORE INTO peers (addr) VALUES (?1)", (addr,))?;
        }
        Ok(())
    }

    pub fn load_from_db(conn: &Connection) -> Result<Self> {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS peers (addr TEXT PRIMARY KEY);")?;
        let mut stmt = conn.prepare("SELECT addr FROM peers")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut peers = Vec::new();
        for row in rows {
            peers.push(row?);
        }
        Ok(PeerManager { peers })
    }
}
