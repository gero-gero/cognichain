use warp::{http::StatusCode, Filter, Rejection, Reply, reply::Json};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use secp256k1::PublicKey;
use crate::blockchain::Blockchain;
use crate::block::Block;
use crate::server::start_node_server;
use crate::smart_contract::{ContractManager, SmartContract, ContractType};  // Import ContractType
use crate::public_key_serde::SerializablePublicKey;
use std::collections::HashMap;

#[derive(Serialize)]
struct BlockchainStatusResponse {
    block_height: usize,
    current_hash: String,
}

#[derive(Serialize)]
struct BlockResponse {
    index: u64,
    timestamp: u128,
    previous_hash: String,
    hash: String,
    data: String,
    node_id: String,
}

#[derive(Serialize, Deserialize)]
struct ContractOperationRequest {
    id: String,
    owner: String,
    code: Vec<u8>,
    contract_type: ContractType,  // Add contract_type field
}

#[derive(Serialize, Deserialize)]
struct ContractExecutionRequest {
    id: String,
    input: String,
}

#[derive(Serialize, Deserialize)]
struct ContractCheckRequest {
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractDetails {
    id: String,
    owner: SerializablePublicKey,
    code: Vec<u8>,
    state: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct OperationResponse {
    pub success: bool,
    pub message: String,
    pub details: Option<ContractDetails>,
}

impl From<&Block> for BlockResponse {
    fn from(block: &Block) -> Self {
        BlockResponse {
            index: block.index,
            timestamp: block.timestamp,
            previous_hash: block.previous_hash.clone(),
            hash: block.hash.clone(),
            data: block.data.clone(),
            node_id: block.node_id.clone(),
        }
    }
}

fn with_contract_manager(contract_manager: Arc<Mutex<ContractManager>>) -> impl Filter<Extract = (Arc<Mutex<ContractManager>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || contract_manager.clone())
}

fn contract_routes(contract_manager: Arc<Mutex<ContractManager>>) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let deploy_contract = warp::path("contract")
        .and(warp::path("deploy"))
        .and(warp::path::end())
        .and(warp::post())
        .and(json_body::<ContractOperationRequest>())
        .and(with_contract_manager(contract_manager.clone()))
        .and_then(deploy_contract_handler);

    let execute_contract = warp::path("contract")
        .and(warp::path("execute"))
        .and(warp::path::end())
        .and(warp::post())
        .and(json_body::<ContractExecutionRequest>())
        .and(with_contract_manager(contract_manager.clone()))
        .and_then(execute_contract_handler);

    let check_contract = warp::path("contract")
        .and(warp::path("check"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<ContractCheckRequest>())
        .and(with_contract_manager(contract_manager.clone()))
        .and_then(check_contract_exists_handler);

    deploy_contract.or(execute_contract).or(check_contract)
}

pub async fn start_api(blockchain: Arc<Mutex<Blockchain>>, contract_manager: Arc<Mutex<ContractManager>>) {
    let blockchain_filter = warp::any().map(move || blockchain.clone());

    let start_node_route = warp::path("start_node")
        .and(warp::post())
        .and(blockchain_filter.clone())
        .and_then(move |blockchain: Arc<Mutex<Blockchain>>| async move {
            let ip = "0.0.0.0:6397".to_string();
            tokio::spawn(async move {
                start_node_server(ip, blockchain).await;
            });
            Ok::<_, Rejection>(warp::reply::json(&OperationResponse { 
                success: true, 
                message: "Node started successfully".to_string(),
                details: None 
            }))
        });

    let status_route = warp::path("status")
        .and(warp::get())
        .and(blockchain_filter.clone())
        .and_then(|blockchain: Arc<Mutex<Blockchain>>| async move {
            let blockchain = blockchain.lock().await;
            let status = BlockchainStatusResponse {
                block_height: blockchain.blocks.len(),
                current_hash: blockchain.blocks.last().map_or(String::new(), |b| b.hash.clone()),
            };
            Ok::<_, Rejection>(warp::reply::json(&status))
        });

    let blocks_route = warp::path("blocks")
        .and(warp::get())
        .and(blockchain_filter)
        .and_then(|blockchain: Arc<Mutex<Blockchain>>| async move {
            let blockchain = blockchain.lock().await;
            let blocks: Vec<BlockResponse> = blockchain.blocks.iter().map(BlockResponse::from).collect();
            Ok::<_, Rejection>(warp::reply::json(&blocks))
        });

    let contract_mgmt_routes = contract_routes(contract_manager);

    let routes = start_node_route.or(status_route).or(blocks_route).or(contract_mgmt_routes)
        .recover(handle_rejection);

    tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    });
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found";
    } else {
        eprintln!("Unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    let json = warp::reply::json(&{
        let mut map = std::collections::HashMap::new();
        map.insert("message", message);
        map
    });

    Ok(warp::reply::with_status(json, code))
}

fn json_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
where
    T: for<'a> Deserialize<'a> + Send + 'static {
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

async fn deploy_contract_handler(body: ContractOperationRequest, contract_manager: Arc<Mutex<ContractManager>>) -> Result<Json, Rejection> {
    let mut manager = contract_manager.lock().await;
    let public_key = PublicKey::from_slice(&hex::decode(&body.owner).unwrap()).unwrap(); // Consider proper error handling
    let contract_id = body.id.clone();  // Clone id to avoid move
    match manager.deploy_contract(contract_id, public_key, body.code, body.contract_type) {
        Ok(contract) => Ok(warp::reply::json(&OperationResponse { 
            success: true, 
            message: "Contract deployed successfully".to_string(),
            details: Some(ContractDetails {
                id: body.id,  // Use the original body.id
                owner: contract.owner,
                code: contract.code,
                state: contract.state,
            })
        })),
        Err(e) => Ok(warp::reply::json(&OperationResponse { 
            success: false, 
            message: e, 
            details: None 
        })),
    }
}

async fn execute_contract_handler(body: ContractExecutionRequest, contract_manager: Arc<Mutex<ContractManager>>) -> Result<Json, Rejection> {
    let mut manager = contract_manager.lock().await;
    let output = manager.execute_contract(&body.id, &body.input);
    Ok(warp::reply::json(&OperationResponse { 
        success: true, 
        message: output, 
        details: None 
    }))
}

async fn check_contract_exists_handler(query: ContractCheckRequest, contract_manager: Arc<Mutex<ContractManager>>) -> Result<Json, Rejection> {
    let manager = contract_manager.lock().await;
    let exists = manager.check_contract_exists(&query.id);
    let response = if exists {
        let contract = manager.contracts.get(&query.id).unwrap();  // Assuming contract is in the HashMap
        let details = ContractDetails {
            id: query.id.clone(),
            owner: contract.owner.clone(),
            code: contract.code.clone(),
            state: contract.state.clone(),
        };
        OperationResponse {
            success: true,
            message: "Contract exists".to_string(),
            details: Some(details),
        }
    } else {
        OperationResponse {
            success: false,
            message: "Contract does not exist".to_string(),
            details: None,
        }
    };
    Ok(warp::reply::json(&response))
}
