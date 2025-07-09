mod block;
mod blockchain;
mod transaction;
mod mempool;
mod storage;
mod network;
mod account;
mod peers;

use clap::{Parser, Subcommand};
use blockchain::Blockchain;
use transaction::Transaction;
use mempool::Mempool;
use peers::PeerManager;

#[derive(Parser)]
#[command(name = "PoS Chain")]
#[command(about = "A minimal async PoS blockchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Submit { from, to, amount } => {
            let tx = Transaction::new(&from, &to, amount);
            println!("💸 交易提交: {} -> {} [{}]", from, to, amount);
            // TODO: 写入本地持久 mempool 或发送到节点网络（可后续扩展）
        }

        Commands::Run { port } => {
            println!("🚀 启动 PoS 节点，监听端口 {}", port);

            let mut chain = Blockchain::new();
            chain.create_genesis_block();

            let mut mempool = Mempool::default();
            mempool.add(Transaction::new("Alice", "Bob", 10));
            mempool.add(Transaction::new("Bob", "Charlie", 5));

            chain.add_block(mempool.collect_for_block(10));

            chain.print_chain();
            chain.state.show();

            let peers = PeerManager::default();
            network::start_server(port, chain.into_arc()).await;
        }
    }
}
