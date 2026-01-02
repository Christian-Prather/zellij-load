use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SystemMessage {
    pub cpu_usage: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub gpu_info: Option<GPU>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GPU {
    pub name: String,
    pub memory_used: u64,
    pub memory_total: u64,
    pub gpu_utilization: u32,
}
