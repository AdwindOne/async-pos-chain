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
        from: String,
        to: String,
        amount: u64,
    },
    Run {
        #[arg(default_value = "8000", value_parser)]
        port: u16,
    },
    Query {
        index: u64,
    },
    QueryBalance {
        address: String,
    },
    AddPeer {
        addr: String,
    },
    QueryPeers,
    JsonRpcServer { port: u16 },
    QueryTx {
        hash: String,
    },
}

pub fn parse_cli() -> Cli {
    Cli::parse()
} 