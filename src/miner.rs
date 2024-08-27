use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Miner {
    id: Uuid,
    public_key: String,
    registration_time: DateTime<Utc>,
    gpu_model: String,
    ram_capacity_gb: u32, // RAM capacity in gigabytes
}

impl Miner {
    pub fn new(public_key: String, gpu_model: String, ram_capacity_gb: u32) -> Self {
        Miner {
            id: Uuid::new_v4(),
            public_key,
            registration_time: Utc::now(),
            gpu_model,
            ram_capacity_gb,
        }
    }
}
