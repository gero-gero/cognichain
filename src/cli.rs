use structopt::StructOpt;
use std::sync::Arc;
use tokio::sync::Mutex;
use secp256k1::SecretKey;
use crate::blockchain::Blockchain;
use crate::network::broadcast_new_block;
use crate::server;
use std::env;

#[derive(StructOpt, Debug)]
#[structopt(name = "cognichain")]
pub enum Cli {
    #[structopt(about = "Add a new block")]
    AddBlock {
        #[structopt(help = "Data to be stored in the block")]
        data: String,
    },
    #[structopt(about = "View the blockchain")]
    ViewChain,
    #[structopt(about = "Check the validity of the blockchain")]
    CheckValidity,
    #[structopt(about = "Start the node and keep it running")]
    StartNode,
}

impl Cli {
    pub async fn run(&self, blockchain: &Arc<Mutex<Blockchain>>, secret_key: &SecretKey, peer_addresses: Vec<String>) {
        match self {
            Cli::AddBlock { data } => {
                let new_block = {
                    let mut blockchain = blockchain.lock().await; // Using async lock
                    blockchain.add_block(data.clone(), env::var("NODE_ID").unwrap_or_else(|_| "node1".to_string()), secret_key).unwrap()
                };
                println!("New block added: {:?}", new_block);
                broadcast_new_block(&new_block, peer_addresses).await; // Correctly using .await on an async function
            },
            Cli::ViewChain => {
                let blockchain = blockchain.lock().await; // Using async lock
                for block in &blockchain.blocks {
                    println!("{:?}", block);
                }
            },
            Cli::CheckValidity => {
                let blockchain = blockchain.lock().await; // Using async lock
                println!("Blockchain valid: {}", blockchain.is_valid());
            },
            Cli::StartNode => {
                println!("Starting the node...");
                let node_ip = "0.0.0.0:6397".to_string(); // Example IP and port
                server::start_node_server(node_ip, blockchain.clone()).await;
            }
        }
    }
}
