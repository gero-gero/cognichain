use secp256k1::{Secp256k1, Message, Signature, PublicKey, SecretKey, ecdsa};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use sled::Db;
use bincode::{self, serialize};
use std::fmt;
use crate::public_key_serde::SerializablePublicKey;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SmartContract {
    pub owner: SerializablePublicKey,
    pub code: Vec<u8>,
    pub state: HashMap<String, String>,
    pub contract_type: ContractType,  // Add contract_type field
}

impl SmartContract {
    pub fn new(owner: PublicKey, code: Vec<u8>, contract_type: ContractType) -> Self {
        SmartContract {
            owner: SerializablePublicKey(owner),
            code,
            state: HashMap::new(),
            contract_type,  // Initialize contract_type
        }
    }

    pub fn execute(&mut self, input: &str) -> String {
        let parts: Vec<&str> = input.split_whitespace().collect();
        match parts.as_slice() {
            ["set", key, value] => {
                self.state.insert(key.to_string(), value.to_string());
                "Set operation completed".to_string()
            },
            ["get", key] => {
                self.state.get(*key).cloned().unwrap_or_else(|| "Key not found".to_string())
            },
            _ => "Invalid operation".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ContractType {
    MinerRegistration {
        gpu_type: String,
        ram_capacity: f64,
    },
    // Add other contract types here as needed
}

#[derive(Clone)]
pub struct ContractManager {
    pub contracts: HashMap<String, SmartContract>,
    pub db: Option<Db>,
}

impl ContractManager {
    pub fn new(db: Option<Db>) -> Self {
        ContractManager {
            contracts: HashMap::new(),
            db,
        }
    }

    pub fn deploy_contract(&mut self, id: String, owner: PublicKey, code: Vec<u8>, contract_type: ContractType) -> Result<SmartContract, String> {
        if self.contracts.contains_key(&id) {
            return Err("Contract with this ID already exists".to_string());
        }

        let contract = SmartContract::new(owner, code, contract_type);
        self.contracts.insert(id.clone(), contract.clone());

        if let Some(db) = &self.db {
            let serialized_contract = serialize(&contract).map_err(|e| e.to_string())?;
            db.insert(id.as_bytes(), serialized_contract).map_err(|e| e.to_string())?;
        }

        Ok(contract)  // Return the contract for details extraction
    }

    pub fn execute_contract(&mut self, id: &str, input: &str) -> String {
        if let Some(contract) = self.contracts.get_mut(id) {
            contract.execute(input)
        } else {
            "Contract not found".to_string()
        }
    }

    pub fn verify_signature(&self, message: &[u8], sig: &[u8], pubkey: &PublicKey) -> Result<(), String> {
        let secp = Secp256k1::new();
        let message = Message::from_slice(message).map_err(|_| "Invalid message".to_string())?;
        let sig = Signature::from_der(sig).map_err(|_| "Invalid signature format".to_string())?;

        secp.verify_ecdsa(&message, &sig, pubkey)
            .map_err(|_| "Verification failed".to_string())
    }

    pub fn check_contract_exists(&self, id: &str) -> bool {
        if self.contracts.contains_key(id) {
            true
        } else if let Some(db) = &self.db {
            db.contains_key(id.as_bytes()).unwrap_or(false)
        } else {
            false
        }
    }
}

impl fmt::Debug for ContractManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContractManager")
            .field("contracts", &self.contracts)
            .finish_non_exhaustive()  // Use non_exhaustive to indicate not all fields are displayed
    }
}
