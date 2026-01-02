use std::collections::BTreeMap;
use std::fs::{self, File};

use colored::Colorize;
use serde_json;

use zellij_load::system_info::SystemMessage;
use zellij_tile::prelude::*;

#[derive(Default)]
struct State {
    stats: SystemMessage,
}

register_plugin!(State);

fn strip_ansi_codes(s: &str) -> String {
    // Remove ANSI escape sequences
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
    re.replace_all(s, "").to_string()
}

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        // Request the necessary permissions
        request_permission(&[PermissionType::RunCommands, PermissionType::OpenFiles]);

        subscribe(&[
            EventType::PermissionRequestResult,
            EventType::RunCommandResult,
            EventType::BeforeClose,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let should_render = false;

        match event {
            Event::PermissionRequestResult(status) => match status {
                PermissionStatus::Granted => {
                    eprintln!("Permission granted");
                    let _ = File::create("/tmp/system-monitor-lock").unwrap();
                    // Get current directory and run command there
                    let current_dir = std::path::PathBuf::from(".");

                    // Pass plugin PID to monitor using environment variable
                    run_command_with_env_variables_and_cwd(
                        &["zellij_system_monitor"],
                        BTreeMap::new(),
                        current_dir,
                        BTreeMap::new(),
                    );
                }
                PermissionStatus::Denied => {
                    eprintln!("Permission denied");
                }
            },
            Event::RunCommandResult(_exit_code, out, error, _context) => {
                eprintln!(
                    "Command Ran {} {:?}",
                    String::from_utf8_lossy(&out),
                    String::from_utf8_lossy(&error)
                );
            }
            Event::BeforeClose => {
                eprintln!("Before close event received");
                fs::remove_file("/tmp/system-monitor-lock").unwrap();
            }
            _ => {}
        }

        should_render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let mut should_render = false;
        match pipe_message.source {
            PipeSource::Cli(_input_pipe_id) => {
                if let Some(payload) = pipe_message.payload {
                    match serde_json::from_str(&payload) as Result<SystemMessage, _> {
                        // Deserialize the JSON message
                        Ok(system_msg) => {
                            self.stats = system_msg;
                            should_render = true;
                        }
                        Err(e) => {
                            eprintln!("Failed to parse message: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        let text = format!(
            "CPU: {} | Mem Used: {} | Mem Total: {} | {}",
            format!("{:.2}%", self.stats.cpu_usage as f64).magenta(),
            format!(
                "{:.2} GB",
                self.stats.mem_used as f64 / 1024.0 / 1024.0 / 1024.0
            )
            .cyan(),
            format!(
                "{:.2} GB",
                self.stats.mem_total as f64 / 1024.0 / 1024.0 / 1024.0
            )
            .cyan(),
            match self.stats.gpu_info.as_ref() {
                Some(gpu) => {
                    let gpu_load = format!("{:.2}%", gpu.gpu_utilization as f64).blue();
                    let gpu_mem_used =
                        format!("{:.2}", gpu.memory_used as f64 / 1024.0 / 1024.0 / 1024.0).blue();
                    let gpu_mem_total = format!(
                        "{:.2} GB",
                        gpu.memory_total as f64 / 1024.0 / 1024.0 / 1024.0
                    )
                    .blue();
                    format!(
                        "GPU Load: {} | GPU Mem Used: {} / {}",
                        gpu_load, gpu_mem_used, gpu_mem_total
                    )
                }
                None => "N/A | N/A | N/A".red().to_string(),
            }
        );

        // Calculate adjustment for ANSI codes
        let visible_chars = strip_ansi_codes(&text).chars().count();
        // The amount of extra characters due to ANSI codes
        let adjustment = text.len() - visible_chars;

        // Position from right edge with some margin
        let right_margin = 5;
        let target_pos = cols.saturating_sub(right_margin);

        print!("{:>width$}", text, width = target_pos + adjustment);
    }
}
