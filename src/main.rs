use std::env;
use std::sync::Arc;
//use tokio::net::TcpListener;
//use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use blockchain::Blockchain;
use secp256k1::{Secp256k1, SecretKey, PublicKey, All};
use structopt::StructOpt;
use crate::cli::Cli;
use crate::network::synchronize_or_initialize;
use crate::smart_contract::ContractManager; 
use env_logger;

mod blockchain;
mod block;
mod cli;
mod node;
mod api;
mod gui;
mod network;
mod server;
mod smart_contract;
mod public_key_serde;

#[derive(StructOpt, Debug)]
enum AppMode {
    #[structopt(about = "Run in CLI mode")]
    Cli(Cli),
    #[structopt(about = "Run in GUI mode")]
    Gui,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let mode = AppMode::from_args();
    let secp = Secp256k1::new();
    let (node_ip, node_id, peer_addresses, secret_key, public_key) = load_environment_vars(&secp).await;

    let blockchain = Blockchain::new_empty();
    let blockchain_arc = Arc::new(Mutex::new(blockchain));
    blockchain_arc.lock().await.set_db(sled::open("blockchain_db").expect("Failed to open database"));

    // Initialize ContractManager
    let contract_manager = ContractManager::new(None); // Assuming no database connection for simplicity
    let contract_manager_arc = Arc::new(Mutex::new(contract_manager));

    // Attempt to synchronize with peers or initialize genesis block
    if let Err(e) = synchronize_or_initialize(&node_id, &blockchain_arc, &peer_addresses, &secret_key, public_key.clone()).await {
        println!("Failed to synchronize with peers: {}", e);
        let mut bc = blockchain_arc.lock().await;
        if bc.blocks.is_empty() {
            println!("No blocks present after failed synchronization; initializing genesis block.");
            Blockchain::initialize_genesis(&mut bc, &node_id, &secret_key, public_key).await;
        }
    }

    // Proceed based on the application mode
    match mode {
        AppMode::Cli(cli) => {
            cli.run(&blockchain_arc, &secret_key, peer_addresses.clone()).await;
        }
        AppMode::Gui => {
            println!("API launch reached");
            api::start_api(blockchain_arc.clone(), contract_manager_arc.clone()).await;  // Pass contract manager here
            println!("GUI launch reached");
            gui::launch_gui().await;
        }
    }
}

async fn load_environment_vars(secp: &Secp256k1<All>) -> (String, String, Vec<String>, SecretKey, PublicKey) {
    let node_ip = env::var("NODE_IP").unwrap_or_else(|_| "0.0.0.0:6397".to_string());
    let node_id = env::var("NODE_ID").unwrap_or_else(|_| "node1".to_string());
    let peer_addresses: Vec<String> = env::var("PEER_ADDRESSES")
        .unwrap_or_else(|_| "127.0.0.1:6398,127.0.0.1:6399".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect();
    let secret_key_bytes = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
    let secret_key = SecretKey::from_slice(&hex::decode(secret_key_bytes).expect("Secret key is not valid hex"))
        .expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(secp, &secret_key);

    (node_ip, node_id, peer_addresses, secret_key, public_key)
}
