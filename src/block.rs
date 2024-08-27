use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use secp256k1::{Secp256k1, SecretKey, Message};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Block {
    pub index: u64,
    pub timestamp: u128,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub data: String,
    pub signature: String,
    pub node_id: String,
}

impl Block {
    pub fn new(index: u64, previous_hash: String, data: String, node_id: String) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let mut block = Block {
            index,
            timestamp,
            previous_hash,
            hash: String::new(),
            nonce: 0,
            data,
            signature: String::new(),
            node_id,
        };
        block.hash = block.calculate_hash();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let data = format!("{}{}{}{}{}{}", self.index, self.timestamp, self.previous_hash, self.nonce, self.data, self.node_id);
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub fn sign_block(&mut self, secret_key: &SecretKey) {
        let secp = Secp256k1::new();
        let message = self.calculate_hash_bytes();
        println!("Message for signing: {:?}", message);
        let sig = secp.sign_ecdsa(&message, secret_key);
        self.signature = sig.to_string();
    }

    fn calculate_hash_bytes(&self) -> Message {
        let data = format!("{}{}{}{}{}{}", self.index, self.timestamp, self.previous_hash, self.nonce, self.data, self.node_id);
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let hash_bytes = result.as_slice();
        println!("Hash bytes: {:?}", hash_bytes);
        Message::from_slice(hash_bytes).expect("Hash should be 32 bytes")
    }
}
