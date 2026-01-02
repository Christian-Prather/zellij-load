use std::process::Command;
use std::sync::{Arc, mpsc};
use std::thread;

use gfxinfo::active_gpu;
use glob::glob;
use single_instance::SingleInstance;

use tokio::sync::mpsc as async_mpsc;
use tokio::time::{Duration, MissedTickBehavior, interval};

use zellij_load::system_info::{CpuUsage, GPU, GpuUsage, MemUsage, SystemMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if another instance is already running
    let instance = SingleInstance::new("zellij-system-monitor").unwrap();
    if !instance.is_single() {
        eprintln!("Another instance of zellij-system-monitor is already running");
        std::process::exit(1);
    }

    println!("System Daemon started!");

    // Create channel for sending system updates
    let (tx, mut rx) = async_mpsc::channel(100);
    let tx = Arc::new(tx);

    // Create a shared system data holder
    let mut cpu_usage = CpuUsage::default();
    let mut mem_usage = MemUsage::default();
    let mut gpu_usage = GpuUsage::default();

    // Create a channel for GPU data communication
    let (gpu_tx, gpu_rx) = mpsc::channel();

    // Spawn a separate thread for GPU data collection
    thread::spawn(move || {
        loop {
            if let Ok(sys_gpu) = active_gpu() {
                let info = sys_gpu.info();
                let gpu_data = (
                    format!("{} {}", sys_gpu.vendor(), sys_gpu.model()),
                    info.used_vram(),
                    info.total_vram(),
                    info.load_pct(),
                );
                if let Err(_) = gpu_tx.send(gpu_data) {
                    break; // Channel was closed, exit thread
                }
            }
            // Sleep for 2 seconds before next update
            std::thread::sleep(Duration::from_secs(2));
        }
    });

    // Spawn a task to continuously collect system information
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut sys = sysinfo::System::new_all();

        loop {
            interval.tick().await;

            let pattern = "/tmp/zellij-*/system-monitor-lock";
            let lock_exists = match glob(pattern) {
                Ok(paths) => paths.count() > 0,
                Err(err) => {
                    eprintln!("Error checking lock file pattern: {}", err);
                    false
                }
            };

            if !lock_exists {
                std::process::exit(0);
            }

            // Update CPU and memory usage
            cpu_usage.update(&mut sys);
            mem_usage.update(&mut sys);

            // Update GPU usage from the channel
            if let Ok((name, used, total, util)) = gpu_rx.try_recv() {
                gpu_usage.name = name;
                gpu_usage.memory_used = used;
                gpu_usage.memory_total = total;
                gpu_usage.gpu_utilization = util;
            }

            // Send the updated data to all clients
            let cpu_val = cpu_usage.total;
            let mem_used = mem_usage.total - mem_usage.idle;
            let mem_total = mem_usage.total;

            let gpu_info = if let (Some(name), Some(mem_used), Some(mem_total), Some(gpu_util)) = (
                Some(&gpu_usage.name),
                Some(gpu_usage.memory_used),
                Some(gpu_usage.memory_total),
                Some(gpu_usage.gpu_utilization),
            ) {
                Some(GPU {
                    name: name.to_string(),
                    memory_used: mem_used,
                    memory_total: mem_total,
                    gpu_utilization: gpu_util,
                })
            } else {
                None
            };

            let msg = SystemMessage {
                cpu_usage: cpu_val,
                mem_used,
                mem_total,
                gpu_info,
            };

            // Serialize and send the message
            if let Ok(serialized) = serde_json::to_string(&msg) {
                if let Err(e) = tx_clone.send(serialized).await {
                    eprintln!("Failed to send system update: {}", e);
                }
            }
        }
    });

    // Send system updates to this client
    loop {
        while let Some(msg) = rx.recv().await {
            let _output = Command::new("zellij")
                .arg("pipe")
                .arg("--name")
                .arg("zellij-system-monitor")
                .arg("--")
                .arg(msg)
                .output()
                .expect("failed to execute process");
        }
    }
}
