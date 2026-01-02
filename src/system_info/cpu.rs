use sysinfo::System;

#[derive(Default)]
pub struct CpuUsage {
    pub total: f32,
}

impl CpuUsage {
    pub fn update(&mut self, sys: &mut System) {
        sys.refresh_cpu_all();
        std::thread::sleep(std::time::Duration::from_millis(100));

        sys.refresh_cpu_all();
        std::thread::sleep(std::time::Duration::from_millis(100));

        sys.refresh_cpu_all();
        std::thread::sleep(std::time::Duration::from_millis(100));

        sys.refresh_cpu_all();
        std::thread::sleep(std::time::Duration::from_millis(100));
        self.total = sys.global_cpu_usage();
    }
}
