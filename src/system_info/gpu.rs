use gfxinfo::Gpu;

#[derive(Default)]
pub struct GpuUsage {
    pub name: String,
    pub gpu_utilization: u32,
    pub memory_used: u64,
    pub memory_total: u64,
}

impl GpuUsage {
    pub fn update(&mut self, gpu: &dyn Gpu) {
        self.name = format!("{} {}", gpu.vendor(), gpu.model());
        let info = gpu.info();
        self.memory_used = info.used_vram();
        self.memory_total = info.total_vram();
        self.gpu_utilization = info.load_pct();
    }
}
