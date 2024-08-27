use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::ser::SerializeStruct;
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use secp256k1::{PublicKey};
use std::str::FromStr;
use crate::block::Block;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub is_authority: bool,
    pub public_key: PublicKey,
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let public_key_str = self.public_key.to_string();
        let mut state = serializer.serialize_struct("Node", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("is_authority", &self.is_authority)?;
        state.serialize_field("public_key", &public_key_str)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct NodeHelper {
            id: String,
            is_authority: bool,
            public_key: String,
        }

        let helper = NodeHelper::deserialize(deserializer)?;
        let public_key = PublicKey::from_str(&helper.public_key).map_err(serde::de::Error::custom)?;
        Ok(Node {
            id: helper.id,
            is_authority: helper.is_authority,
            public_key,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    RequestBlockchain,
    BlockchainBlocks(Vec<Block>),
}


impl Node {
    pub async fn synchronize(&self, blockchain: &Arc<Mutex<Blockchain>>, peer_addresses: Vec<String>) {
        for peer in peer_addresses {
            if let Ok(mut stream) = TcpStream::connect(peer).await {
                // Request blockchain data from the peer
                let request = Message::RequestBlockchain;
                let request_data = serde_json::to_vec(&request).unwrap();
                stream.write_all(&request_data).await.unwrap();

                // Read the response
                let mut buffer = vec![0; 1024];
                let n = stream.read(&mut buffer).await.unwrap();
                let response: Message = serde_json::from_slice(&buffer[..n]).unwrap();

                if let Message::BlockchainBlocks(peer_blocks) = response {
                    let mut local_blockchain = blockchain.lock().unwrap();
                    // Logic to integrate peer blocks into local blockchain
                    // This can be replacing the local blocks or merging them
                    if peer_blocks.len() > local_blockchain.blocks.len() {
                        local_blockchain.blocks = peer_blocks;
                    }
                }
            }
        }
    }

    pub async fn start_server(&self, blockchain: Arc<Mutex<Blockchain>>) {
        let listener = TcpListener::bind("127.0.0.1:6397").await.unwrap();

        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let blockchain = blockchain.clone();

            tokio::spawn(async move {
                let mut buffer = vec![0; 1024];
                let n = socket.read(&mut buffer).await.unwrap();
                let request: Message = serde_json::from_slice(&buffer[..n]).unwrap();

                if let Message::RequestBlockchain = request {
                    let response_data;
                    {
                        let local_blockchain = blockchain.lock().unwrap();
                        let blocks = local_blockchain.blocks.clone();
                        let response = Message::BlockchainBlocks(blocks);
                        response_data = serde_json::to_vec(&response).unwrap();
                    }
                    socket.write_all(&response_data).await.unwrap();
                }
            });
        }
    }
}

