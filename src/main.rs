mod cli;
mod node;
mod blockchain;
mod mempool;
mod peers;
mod storage;
mod transaction;
mod network;
mod account;
mod block;
mod rpc;

// use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let cli = cli::parse_cli();
    let chain_db = storage::open_db("chain.db");
    let peers_db = storage::open_db("peers.db");
    node::dispatch(cli, chain_db, peers_db).await;
}
