use druid::{AppLauncher, Widget, WindowDesc, Data, Lens, Env, WidgetExt, ExtEventSink, Target, Selector, Handled};
use druid::widget::{Flex, Button, Label, Scroll, List, TextBox, RadioGroup};
use im::Vector;
use serde::{Deserialize, Deserializer};
use reqwest::Client;
use std::sync::Arc;
use anyhow::Result;
use std::env;
use secp256k1::SecretKey;
use secp256k1::PublicKey;
use secp256k1::Secp256k1;
use crate::api::OperationResponse;
use crate::smart_contract::ContractType;

#[derive(Clone, Data, Lens, Deserialize)]
struct Block {
    index: usize,
    timestamp: u64,
    previous_hash: String,
    hash: String,
    data: String,
    node_id: String,
}

#[derive(Clone, Data, Lens, Deserialize)]
struct BlockchainStatus {
    block_height: usize,
    current_hash: String,
}

#[derive(Clone, Data, Lens)]
struct FullBlockchainStatus {
    status: BlockchainStatus,
    blocks: Arc<Vec<Block>>,
}

impl<'de> Deserialize<'de> for FullBlockchainStatus {
    fn deserialize<D>(deserializer: D) -> Result<FullBlockchainStatus, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = Helper::deserialize(deserializer)?;
        Ok(FullBlockchainStatus {
            status: helper.status,
            blocks: Arc::new(helper.blocks),
        })
    }
}

#[derive(Deserialize)]
struct Helper {
    status: BlockchainStatus,
    blocks: Vec<Block>,
}

#[derive(Clone, Data, Lens)]
struct AppState {
    status: BlockchainStatus,
    blocks: Arc<Vec<Block>>,
    is_node_running: bool,
    public_key: String,
    gpu_type: String,
    ram_capacity: f64,
    contract_exists: bool,
    contract_message: String,
}

const UPDATE_STATUS: Selector<FullBlockchainStatus> = Selector::new("update_status");
const UPDATE_NODE_RUNNING: Selector<bool> = Selector::new("update_node_running");
const UPDATE_CONTRACT_DETAILS: Selector<(bool, String)> = Selector::new("update_contract_details");

async fn fetch_blockchain_status() -> Result<FullBlockchainStatus> {
    let client = Client::new();

    let status_resp = client.get("http://127.0.0.1:3030/status")
        .send().await?;

    if !status_resp.status().is_success() {
        println!("Failed to fetch status: {}", status_resp.status());
        return Err(anyhow::anyhow!("Failed to fetch status"));
    }

    let status = status_resp.json::<BlockchainStatus>().await?;

    let blocks_resp = client.get("http://127.0.0.1:3030/blocks")
        .send().await?;

    if !blocks_resp.status().is_success() {
        println!("Failed to fetch blocks: {}", blocks_resp.status());
        return Err(anyhow::anyhow!("Failed to fetch blocks"));
    }

    let blocks = blocks_resp.json::<Vec<Block>>().await?;

    Ok(FullBlockchainStatus {
        status,
        blocks: Arc::new(blocks),
    })
}

async fn check_contract_exists(client: &Client, public_key: &str) -> Result<OperationResponse> {
    client.get(format!("http://127.0.0.1:3030/contract/check"))
        .query(&[("id", public_key)])
        .send().await?
        .json::<OperationResponse>().await
        .map_err(anyhow::Error::new)
}

async fn create_or_update_contract(client: &Client, public_key: &str, gpu_type: &str, ram_capacity: f64, exists: bool) -> Result<()> {
    let url = if exists {
        "http://127.0.0.1:3030/contract/update"
    } else {
        "http://127.0.0.1:3030/contract/deploy"
    };

    let contract_type = ContractType::MinerRegistration {
        gpu_type: gpu_type.to_string(),
        ram_capacity,
    };

    let body: serde_json::Value = serde_json::json!({
        "id": public_key,
        "owner": public_key,
        "code": vec![] as Vec<u8>,
        "contract_type": contract_type
    });

    let response = client.post(url)
        .json(&body)
        .send().await?
        .error_for_status()?;

    let _operation_response = response
        .json::<OperationResponse>().await?;

    Ok(())
}

fn build_ui() -> impl Widget<AppState> {
    Flex::column()
        .with_child(
            Label::new(|data: &AppState, _: &Env| {
                format!("Block Height: {}\nCurrent Hash: {}", data.status.block_height, data.status.current_hash)
            })
        )
        .with_child(
            Button::new("Start Node").on_click(|ctx, data: &mut AppState, _| {
                if !data.is_node_running {
                    let public_key = data.public_key.clone();
                    let gpu_type = data.gpu_type.clone();
                    let ram_capacity = data.ram_capacity;
                    let sink = ctx.get_external_handle();
                    let client = Client::new();

                    tokio::spawn(async move {
                        let gpu_type = gpu_type.clone();  // Clone the gpu_type for use inside the async block
                        match check_contract_exists(&client, &public_key).await {
                            Ok(response) => {
                                let exists = response.success;
                                if let Err(e) = create_or_update_contract(&client, &public_key, &gpu_type, ram_capacity, exists).await {
                                    println!("pubkey: {}", &public_key);
                                    println!("Failed to manage contract: {}", e);
                                }
                            },
                            Err(e) => println!("Failed to check contract existence: {}", e),
                        }

                        match client.post("http://127.0.0.1:3030/start_node").send().await {
                            Ok(_) => {
                                println!("Node start requested successfully");
                                sink.submit_command(UPDATE_NODE_RUNNING, true, Target::Auto)
                                    .expect("Failed to submit UPDATE_NODE_RUNNING");
                            },
                            Err(e) => println!("Failed to request node start: {}", e),
                        }
                    });
                    data.is_node_running = true;
                }
            })
        )
        .with_child(
            Button::new("Update Status").on_click(|ctx, _data: &mut AppState, _| {
                let sink = ctx.get_external_handle();
                tokio::spawn(update_status(sink));
            })
        )
        .with_child(
            Button::new("Check Miner Registration Smart Contract").on_click(|ctx, data: &mut AppState, _| {
                let client = Client::new();
                let public_key = data.public_key.clone();
                let sink = ctx.get_external_handle();
                tokio::spawn(async move {
                    match check_contract_exists(&client, &public_key).await {
                        Ok(response) => {
                            println!("Contract check: {}", response.success);
                            let message = format!("Contract exists: {}", response.success);
                            sink.submit_command(UPDATE_NODE_RUNNING, response.success, Target::Auto)
                                .expect("Failed to submit UPDATE_NODE_RUNNING");
                            sink.submit_command(UPDATE_CONTRACT_DETAILS, (response.success, message), Target::Auto)
                                .expect("Failed to submit UPDATE_CONTRACT_DETAILS");
                        },
                        Err(e) => {
                            println!("Failed to check contract: {}", e);
                            sink.submit_command(UPDATE_CONTRACT_DETAILS, (false, format!("Error: {}", e)), Target::Auto)
                                .expect("Failed to update AppState");
                        }
                    }
                });
            })
        )
        .with_child(
            Label::new(|data: &AppState, _: &Env| format!("Contract Status: {}\n{}", data.contract_exists, data.contract_message))
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Public Key:"))
                .with_flex_spacer(1.0)
                .with_flex_child(TextBox::new().lens(AppState::public_key), 2.0)
                .with_child(Button::new("Generate").on_click(|_ctx, data: &mut AppState, _env| {
                    let secret_key_hex = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
                    let secret_key_bytes = hex::decode(secret_key_hex).expect("Invalid hex in SECRET_KEY");
                    let secret_key = SecretKey::from_slice(&secret_key_bytes).expect("Invalid SECRET_KEY");

                    let secp = Secp256k1::new();
                    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
                    let public_key_serialized = public_key.serialize().to_vec();  // Compressed format
                    let public_key_hex = hex::encode(public_key_serialized);

                    data.public_key = public_key_hex;
                }))
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("GPU Type:"))
                .with_child(RadioGroup::new(vec![
                    ("3060 RTX".to_string(), "3060 RTX".to_string()),
                    ("3070 RTX".to_string(), "3070 RTX".to_string()),
                ]).lens(AppState::gpu_type))
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("RAM (GB):"))
                .with_child(TextBox::new().lens(FloatToString).fix_width(50.0))
        )
        .with_flex_child(
            Scroll::new(
                List::new(|| {
                    Label::new(|item: &Block, _: &Env| {
                        format!("Index: {}\nHash: {}\nData: {}", item.index, item.hash, item.data)
                    })
                }).lens(AppState::blocks)
            ),
            1.0
        )
}

async fn update_status(sink: ExtEventSink) {
    match fetch_blockchain_status().await {
        Ok(new_status) => {
            sink.submit_command(UPDATE_STATUS, new_status, Target::Auto).expect("Failed to submit command");
        },
        Err(e) => {
            println!("Failed to fetch blockchain status: {:?}", e);
        }
    }
}

pub async fn launch_gui() {
    let main_window = WindowDesc::new(build_ui)
        .title("Blockchain Status")
        .window_size((400.0, 400.0));

    let initial_state = AppState {
        status: BlockchainStatus {
            block_height: 0,
            current_hash: String::new(),
        },
        blocks: Arc::new(Vec::new()),
        is_node_running: false,
        public_key: "".to_string(),
        gpu_type: "3060 RTX".to_string(),
        ram_capacity: 32.0,
        contract_exists: false,
        contract_message: String::new(),
    };

    AppLauncher::with_window(main_window)
        .delegate(Delegate)
        .launch(initial_state)
        .expect("Failed to launch application");
}

struct Delegate;

impl druid::AppDelegate<AppState> for Delegate {
    fn command(&mut self, _ctx: &mut druid::DelegateCtx, _target: druid::Target, cmd: &druid::Command, data: &mut AppState, _env: &druid::Env) -> druid::Handled {
        if let Some(new_status) = cmd.get(UPDATE_STATUS) {
            data.status = new_status.status.clone();
            data.blocks = new_status.blocks.clone();
        }
        if let Some(is_running) = cmd.get(UPDATE_NODE_RUNNING) {
            data.is_node_running = *is_running;
        }
        if let Some((exists, message)) = cmd.get(UPDATE_CONTRACT_DETAILS) {
            data.contract_exists = *exists;
            data.contract_message = message.clone();
            return druid::Handled::Yes;
        }
        druid::Handled::No
    }
}

struct FloatToString;

impl Lens<AppState, String> for FloatToString {
    fn with<V, F: FnOnce(&String) -> V>(&self, data: &AppState, f: F) -> V {
        f(&data.ram_capacity.to_string())
    }

    fn with_mut<V, F: FnOnce(&mut String) -> V>(&self, data: &mut AppState, f: F) -> V {
        let mut s = data.ram_capacity.to_string();
        let result = f(&mut s);
        if let Ok(val) = s.parse::<f64>() {
            data.ram_capacity = val;
        }
        result
    }
}


