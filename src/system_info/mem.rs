use sysinfo::System;

#[derive(Default)]
pub struct MemUsage {
    pub total: u64,
    pub idle: u64,
}

impl MemUsage {
    pub fn update(&mut self, sys: &mut System) {
        sys.refresh_memory();
        self.total = sys.total_memory();
        self.idle = sys.available_memory();
    }
}
