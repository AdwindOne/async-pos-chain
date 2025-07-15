mod account;
mod block;
mod blockchain;
mod cli;
mod mempool;
mod network;
mod node;
mod peers;
mod rpc;
mod storage;
mod transaction;

#[tokio::main]
async fn main() {
    let cli = cli::parse_cli();
    match cli.command {
        cli::Command::Run { port } => node::run_node(port).await,
        cli::Command::Submit { from, to, amount } => node::submit_tx(from, to, amount).await,
        cli::Command::Query { index } => node::query_block(index),
        cli::Command::QueryBalance { address } => node::query_balance(address),
        cli::Command::AddPeer { addr } => node::add_peer(addr),
        cli::Command::QueryPeers => node::query_peers(),
        cli::Command::JsonRpcServer { port } => rpc::start_jsonrpc_server(port).await,
        cli::Command::QueryTx { hash } => node::query_tx(hash),
    }
}
