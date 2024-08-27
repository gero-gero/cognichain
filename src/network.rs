use crate::block::Block;
use crate::Blockchain;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use serde_json::to_string;
use tokio::sync::Mutex;
use std::sync::Arc;
use secp256k1::{Secp256k1, SecretKey, PublicKey, All};

pub async fn broadcast_new_block(block: &Block, peer_addresses: Vec<String>) {
    for peer_address in peer_addresses {
        if let Ok(mut stream) = TcpStream::connect(peer_address).await {
            if let Ok(block_data) = to_string(block) {
                let _ = stream.write_all(block_data.as_bytes()).await;
                // You might want to handle errors here
            }
        }
    }
}

pub async fn try_connect_and_sync(peer_address: &str, blockchain: &Arc<Mutex<Blockchain>>) -> Result<Blockchain, String> {
    let retry_limit = 3;
    for attempt in 1..=retry_limit {
        log::info!("Attempting to connect to peer at {}, attempt {}", peer_address, attempt);
        match TcpStream::connect(peer_address).await {
            Ok(mut stream) => {
                log::info!("Connected to peer at {}", peer_address);
                // Simulate sending a blockchain request
                if let Err(e) = stream.write_all(b"request_blockchain").await {
                    log::warn!("Failed to send request to {}: {}", peer_address, e);
                    continue; // Try connecting again
                }
                // Assume receiving and processing blockchain data
                return Ok(Blockchain::new_empty()); // Placeholder for actual received blockchain
            },
            Err(e) => {
                log::error!("Connection attempt {} failed: {}", attempt, e);
                if attempt == retry_limit {
                    return Err(format!("Failed to connect to peer at {} after {} attempts", peer_address, retry_limit));
                }
            }
        }
    }
    Err(format!("Failed to connect to peer at {}", peer_address))
}

pub async fn synchronize_with_peers(
    node_id: &str,
    blockchain: &Arc<Mutex<Blockchain>>,
    peer_addresses: &[String],
) -> Result<(), String> {
    for peer_address in peer_addresses {
        match try_connect_and_sync(peer_address, blockchain).await {
            Ok(peer_blockchain) => {
                let mut blockchain = blockchain.lock().await;
                blockchain.synchronize_from_peer(peer_blockchain);
                log::info!("Synchronized with peer at {}", peer_address);
            },
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub async fn synchronize_or_initialize(
    node_id: &str, 
    blockchain: &Arc<Mutex<Blockchain>>, 
    peer_addresses: &[String], 
    secret_key: &SecretKey, 
    public_key: PublicKey
) -> Result<(), String> {
    // Attempt to synchronize with each peer address provided.
    let mut any_successful = false; // Flag to track if any synchronization was successful

    for address in peer_addresses {
        match try_connect_and_sync(address, blockchain).await {
            Ok(_) => {
                println!("Synchronized with node at {}", address);
                any_successful = true; // Mark as successful if any synchronization succeeds
            },
            Err(e) => {
                println!("Failed to connect or sync with node at {}: {}", address, e);
            },
        }
    }

    // Check if any synchronization was successful
    if !any_successful {
        // If no synchronization was successful and the blockchain is still empty, log the condition
        if blockchain.lock().await.blocks.is_empty() {
            log::info!("No successful synchronization and blockchain is empty.");
        }
        return Err("Failed to synchronize with any peers.".to_string());
    }

    Ok(())
}
