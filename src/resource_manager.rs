pub struct ResourceManager {
    gpu_resources: HashMap<String, GPUResourceContract>,
}

impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {
            gpu_resources: HashMap::new(),
        }
    }

    pub fn register_gpu(&mut self, node_id: String, contract: GPUResourceContract) {
        self.gpu_resources.insert(node_id, contract);
    }

    pub fn allocate_gpu(&mut self, task_requirements: GPURequirements) -> Result<String, String> {
        for (node_id, gpu) in &mut self.gpu_resources {
            if gpu.available && gpu.meets_requirements(&task_requirements) {
                gpu.reserve(task_requirements.task_id.clone())?;
                return Ok(node_id.clone());
            }
        }
        Err("No suitable GPU available".to_string())
    }

    pub fn release_gpu(&mut self, node_id: &str) -> Result<(), String> {
        if let Some(gpu) = self.gpu_resources.get_mut(node_id) {
            gpu.release()
        } else {
            Err("GPU not found".to_string())
        }
    }
}