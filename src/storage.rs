use crate::block::Block;
use rusqlite::{Connection, Result};

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS blocks (
            id INTEGER PRIMARY KEY,
            idx INTEGER,
            hash TEXT,
            prev_hash TEXT,
            proposer TEXT,
            timestamp INTEGER
        );
        "
    )
}

pub fn save_block(conn: &Connection, block: &Block) -> Result<()> {
    conn.execute(
        "INSERT INTO blocks (idx, hash, prev_hash, proposer, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            &block.index,
            &block.hash,
            &block.previous_hash,
            &block.proposer,
            &block.timestamp,
        ),
    )?;
    Ok(())
}
