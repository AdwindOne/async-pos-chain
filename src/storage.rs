use crate::block::Block;
use rusqlite::{Connection, Result};
use rusqlite::params;

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS blocks (
            id INTEGER PRIMARY KEY,
            idx INTEGER,
            hash TEXT,
            prev_hash TEXT,
            proposer TEXT,
            timestamp INTEGER,
            transactions TEXT
        );
        "
    )
}

pub fn save_block(conn: &Connection, block: &Block) -> Result<()> {
    let tx_json = serde_json::to_string(&block.transactions).unwrap();
    conn.execute(
        "INSERT INTO blocks (idx, hash, prev_hash, proposer, timestamp, transactions) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            &block.index,
            &block.hash,
            &block.previous_hash,
            &block.proposer,
            &block.timestamp,
            &tx_json,
        ),
    )?;
    Ok(())
}

pub fn get_block_by_index(conn: &Connection, idx: u64) -> Result<Option<Block>> {
    let mut stmt = conn.prepare("SELECT idx, hash, prev_hash, proposer, timestamp, transactions FROM blocks WHERE idx = ?1 LIMIT 1")?;
    let mut rows = stmt.query(params![idx])?;
    if let Some(row) = rows.next()? {
        let index: u64 = row.get(0)?;
        let hash: String = row.get(1)?;
        let previous_hash: String = row.get(2)?;
        let proposer: String = row.get(3)?;
        let timestamp: u64 = row.get(4)?;
        let tx_json: String = row.get(5)?;
        let transactions: Vec<crate::transaction::Transaction> = serde_json::from_str(&tx_json).unwrap_or_default();
        Ok(Some(Block {
            index,
            hash,
            previous_hash,
            proposer,
            timestamp,
            transactions,
        }))
    } else {
        Ok(None)
    }
}

pub fn init_account_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS accounts (
            address TEXT PRIMARY KEY,
            balance INTEGER
        );"
    )
}

pub fn add_account(conn: &Connection, address: &str, balance: u64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO accounts (address, balance) VALUES (?1, ?2)",
        (address, balance),
    )?;
    Ok(())
}

pub fn get_balance(conn: &Connection, address: &str) -> Result<u64> {
    let mut stmt = conn.prepare("SELECT balance FROM accounts WHERE address = ?1")?;
    let mut rows = stmt.query(params![address])?;
    if let Some(row) = rows.next()? {
        let balance: u64 = row.get(0)?;
        Ok(balance)
    } else {
        Ok(0)
    }
}

pub fn set_balance(conn: &Connection, address: &str, balance: u64) -> Result<()> {
    conn.execute(
        "UPDATE accounts SET balance = ?1 WHERE address = ?2",
        (balance, address),
    )?;
    Ok(())
}

pub fn get_transaction_by_hash(conn: &Connection, tx_hash: &str) -> Result<Option<(u64, crate::transaction::Transaction)>> {
    let mut stmt = conn.prepare("SELECT idx, transactions FROM blocks")?;
    let mut rows = stmt.query([])?;
    use sha2::{Sha256, Digest};
    while let Some(row) = rows.next()? {
        let idx: u64 = row.get(0)?;
        let tx_json: String = row.get(1)?;
        let txs: Vec<crate::transaction::Transaction> = serde_json::from_str(&tx_json).unwrap_or_default();
        for tx in txs {
            let tx_str = format!("{}{}{}", tx.from, tx.to, tx.amount);
            let mut hasher = Sha256::new();
            hasher.update(tx_str.as_bytes());
            let hash = format!("0x{:x}", hasher.finalize());
            if hash == tx_hash {
                return Ok(Some((idx, tx)));
            }
        }
    }
    Ok(None)
}

pub fn init_mempool_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mempool (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tx_json TEXT
        );"
    )
}

pub fn insert_mempool_tx(conn: &Connection, tx: &crate::transaction::Transaction) -> Result<()> {
    let tx_json = serde_json::to_string(tx).unwrap();
    conn.execute(
        "INSERT INTO mempool (tx_json) VALUES (?1)",
        (tx_json,),
    )?;
    Ok(())
}

pub fn remove_mempool_tx(conn: &Connection, tx: &crate::transaction::Transaction) -> Result<()> {
    let tx_json = serde_json::to_string(tx).unwrap();
    conn.execute(
        "DELETE FROM mempool WHERE tx_json = ?1",
        (tx_json,),
    )?;
    Ok(())
}

pub fn load_all_mempool_txs(conn: &Connection) -> Result<Vec<crate::transaction::Transaction>> {
    let mut stmt = conn.prepare("SELECT tx_json FROM mempool")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut txs = Vec::new();
    for row in rows {
        if let Ok(json) = row {
            if let Ok(tx) = serde_json::from_str(&json) {
                txs.push(tx);
            }
        }
    }
    Ok(txs)
}
