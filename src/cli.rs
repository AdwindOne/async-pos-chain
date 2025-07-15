use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "PoS Chain")]
#[command(about = "A minimal async PoS blockchain", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Submit {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
    },
    Run {
        #[arg(long, default_value = "8000")]
        port: u16,
    },
    Query {
        #[arg(long)]
        index: u64,
    },
    QueryBalance {
        #[arg(long)]
        address: String,
    },
    AddPeer {
        #[arg(long)]
        addr: String,
    },
    QueryPeers,
    JsonRpcServer {
        #[arg(long, default_value = "8545")]
        port: u16,
    },
    QueryTx {
        #[arg(long)]
        hash: String,
    },
}

pub fn parse_cli() -> Cli {
    Cli::parse()
} 