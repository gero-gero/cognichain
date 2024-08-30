use crate::block::Block;
use crate::node::Node;
//use crate::network::synchronize_with_peers;
//use std::str::FromStr;
//use std::sync::Arc;
//use tokio::sync::Mutex;
use secp256k1::{Secp256k1, SecretKey, ecdsa::Signature, Message};
use sled::Db;
use serde::{Serialize};
use bincode;
//use std::collections::HashMap;
use hex::decode;
use log;
use secp256k1::PublicKey;
use crate::smart_contract::ContractManager;


#[derive(Serialize, Debug, Clone)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub authorities: Vec<Node>,
    #[serde(skip)]
    pub db: Option<Db>,
    #[serde(skip)]
    pub contract_manager: ContractManager,
    pub resource_manager: ResourceManager,
}

impl Blockchain {
    // Constructor for an entirely new blockchain without any database initialization
    pub fn new_empty() -> Self {
        Blockchain {
            blocks: vec![],
            authorities: vec![],
            db: None,
            contract_manager: ContractManager::new(None),  // Initialize ContractManager without DB
        }
    }

    // Set the database after creating a new empty blockchain
    pub fn set_db(&mut self, db: Db) {
        self.db = Some(db.clone());
        self.contract_manager = ContractManager::new(Some(db));  // Update ContractManager with DB
    }

    // Constructor for creating a new blockchain with an existing database
    pub fn new(db: Db) -> Self {
        let mut blockchain = Blockchain {
            blocks: vec![],
            authorities: vec![],
            db: Some(db.clone()),
            contract_manager: ContractManager::new(Some(db)),  // Initialize ContractManager with DB
        };

        // Load blocks from the database
        println!("Loading blockchain from database...");
        if let Ok(block_bytes) = blockchain.db.as_ref().unwrap().get("0".as_bytes()) {
            if let Some(bytes) = block_bytes {
                println!("Found genesis block in database, deserializing...");
                let genesis_block: Block = bincode::deserialize(&bytes).unwrap();
                blockchain.blocks.push(genesis_block);
            } else {
                println!("Genesis block not found in database, creating new genesis block...");
                let genesis_block = Block::new(0, String::from("0"), String::from("Genesis Block"), String::from("genesis"));
                blockchain.db.as_ref().unwrap().insert("0".as_bytes(), bincode::serialize(&genesis_block).unwrap()).unwrap();
                blockchain.blocks.push(genesis_block);
            }
        }

        // Load authorities from the database
        if let Ok(authorities_bytes) = blockchain.db.as_ref().unwrap().get("authorities".as_bytes()) {
            if let Some(bytes) = authorities_bytes {
                println!("Found authorities in database, deserializing...");
                blockchain.authorities = bincode::deserialize(&bytes).unwrap();
            } else {
                println!("Authorities not found in database, initializing empty list...");
            }
        }
        
        blockchain.load_data_from_db();
        
        blockchain
    }
    
    // Example function to load data from database, to be implemented as needed
    fn load_data_from_db(&mut self) {
    // Load blocks, authorities, etc.
    // This function would include code to load existing blocks and possibly contracts
    }

    pub fn add_block(&mut self, data: String, node_id: String, secret_key: &SecretKey) -> Result<Block, &'static str> {
        if self.is_authority(&node_id) {
            let previous_block = &self.blocks[self.blocks.len() - 1];
            let mut new_block = Block::new(previous_block.index + 1, previous_block.hash.clone(), data, node_id.clone());
            new_block.sign_block(secret_key);
            self.db.as_ref().unwrap().insert(new_block.index.to_string().as_bytes(), bincode::serialize(&new_block).unwrap()).unwrap();
            self.blocks.push(new_block.clone());
            println!("New block added and saved to database: {:?}", new_block);
            Ok(new_block)
        } else {
            Err("Node is not an authority")
        }
    }

    pub fn add_authority(&mut self, node: Node) {
        println!("Adding authority node: {:?}", node);
        self.authorities.push(node);
        self.db.as_ref().unwrap().insert("authorities", bincode::serialize(&self.authorities).unwrap()).unwrap();
    }

    pub fn is_authority(&self, node_id: &str) -> bool {
        let is_auth = self.authorities.iter().any(|node| node.id == node_id && node.is_authority);
        println!("Is node_id '{}' an authority? {}", node_id, is_auth);
        is_auth
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.blocks.len() {
            let current_block = &self.blocks[i];
            let previous_block = &self.blocks[i - 1];

            if current_block.hash != current_block.calculate_hash() {
                println!("Block {}: Invalid hash", current_block.index);
                return false;
            }

            if current_block.previous_hash != previous_block.hash {
                println!("Block {}: Previous hash does not match", current_block.index);
                return false;
            }

            if !self.validate_block(current_block) {
                println!("Block {}: Invalid block signature", current_block.index);
                return false;
            }
        }
        true
    }

    pub fn validate_block(&self, block: &Block) -> bool {
        log::info!("Validating block: {:?}", block);

        if let Some(authority) = self.authorities.iter().find(|node| node.id == block.node_id) {
            log::debug!("Found authority for block {}: {:?}", block.index, authority);

            let secp = Secp256k1::new();
            if let Ok(sig) = Signature::from_der(&decode(&block.signature).expect("Decoding hex should succeed")).map_err(|e| {
                log::error!("Block {}: Failed to parse signature: {}", block.index, e);
                e
            }) {
                let message = Message::from_slice(&decode(&block.hash).expect("Decoding hex should succeed")).expect("32 bytes");

                // Log signature and verification result:
                log::debug!("Signature for verification: {:?}", sig);
                match secp.verify_ecdsa(&message, &sig, &authority.public_key) {
                    Ok(_) => {
                        log::debug!("Block {}: Signature verified successfully", block.index);
                        return true;
                    }
                    Err(err) => {
                        log::error!("Block {}: Invalid signature: {:?}", block.index, err);
                    }
                }
            }
        } else {
            log::error!("Block {}: Authority not found", block.index);
        }
        false
    }

    pub fn synchronize_from_peer(&mut self, peer_blockchain: Blockchain) {
        for block in peer_blockchain.blocks {
            if !self.blocks.contains(&block) {
                self.blocks.push(block.clone());
                self.db.as_ref().unwrap().insert(block.index.to_string().as_bytes(), bincode::serialize(&block).unwrap()).unwrap();
            }
        }
        for authority in peer_blockchain.authorities {
            if !self.authorities.contains(&authority) {
                self.authorities.push(authority.clone());
                self.db.as_ref().unwrap().insert("authorities", bincode::serialize(&self.authorities).unwrap()).unwrap();
            }
        }
    }

    pub async fn initialize_genesis(&mut self, node_id: &str, secret_key: &SecretKey, public_key: PublicKey) {
        if self.blocks.is_empty() {
            println!("Creating genesis block...");
            let genesis_block = Block::new(0, "0".to_string(), "Genesis Block".to_string(), node_id.to_string());
            self.blocks.push(genesis_block.clone());
            self.db.as_ref().unwrap().insert("0".as_bytes(), bincode::serialize(&genesis_block).unwrap()).unwrap();
        }

        if !self.is_authority(node_id) {
            println!("Adding authority node: {}", node_id);
            let node = Node { id: node_id.to_string(), is_authority: true, public_key };
            self.add_authority(node);
        }
    }
}
