use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::blockchain::Blockchain;
use serde_json;

pub async fn start_node_server(node_ip: String, blockchain: Arc<Mutex<Blockchain>>) {
    let listener = TcpListener::bind(&node_ip).await.unwrap();
    println!("Node server running on {}", node_ip);

    while let Ok((mut socket, _)) = listener.accept().await {
        let blockchain = blockchain.clone();
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            while let Ok(n) = socket.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                let received_data = String::from_utf8_lossy(&buffer[..n]);
                if received_data == "request_blockchain" {
                    let blockchain_data = {
                        let blockchain = blockchain.lock().await;
                        serde_json::to_string(&*blockchain).unwrap()
                    };
                    socket.write_all(blockchain_data.as_bytes()).await.unwrap();
                }
            }
        });
    }
}
