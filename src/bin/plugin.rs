use std::collections::BTreeMap;
use std::fs::{self, File};
use std::time::{SystemTime, UNIX_EPOCH};

use colored::Colorize;

use zellij_load::system_info::SystemMessage;
use zellij_tile::prelude::*;

#[derive(Default)]
struct State {
    stats: SystemMessage,
    lock_path: String,
}

register_plugin!(State);

fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
    re.replace_all(s, "").to_string()
}

fn usage_color(text: &str, pct: f64, warn: f64, crit: f64) -> colored::ColoredString {
    if pct >= crit {
        text.bright_red()
    } else if pct >= warn {
        text.yellow()
    } else {
        text.bright_green()
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        colored::control::set_override(true);
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        self.lock_path = format!("/tmp/system-monitor-lock-{}", id);

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
                    // Unselectable pane lets the tab close when the last shell pane exits
                    // but can't answer the permission prompt
                    set_selectable(false);
                    let _ = File::create(&self.lock_path);
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
                if let Err(err) = fs::remove_file(&self.lock_path) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        eprintln!("Failed to remove lock file: {}", err);
                    }
                }
            }
            _ => {}
        }

        should_render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let mut should_render = false;
        if let PipeSource::Cli(_input_pipe_id) = pipe_message.source
            && let Some(payload) = pipe_message.payload
        {
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
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        if cols == 0 {
            return;
        }

        let mut segments: Vec<String> = Vec::new();

        let cpu_pct = self.stats.cpu_usage as f64;
        segments.push(format!(
            "{} {}",
            "CPU:".bright_black().bold(),
            usage_color(&format!("{:.1}%", cpu_pct), cpu_pct, 50.0, 80.0)
        ));

        let mem_used_gb = self.stats.mem_used as f64 / 1024.0 / 1024.0 / 1024.0;
        let mem_total_gb = self.stats.mem_total as f64 / 1024.0 / 1024.0 / 1024.0;
        let mem_pct = if self.stats.mem_total > 0 {
            self.stats.mem_used as f64 / self.stats.mem_total as f64 * 100.0
        } else {
            0.0
        };
        segments.push(format!(
            "{} {}",
            "MEM:".bright_black().bold(),
            usage_color(&format!("{:.2}/{:.2}GB", mem_used_gb, mem_total_gb), mem_pct, 60.0, 80.0)
        ));

        if let Some(gpu) = &self.stats.gpu_info {
            let gpu_pct = gpu.gpu_utilization as f64;
            segments.push(format!(
                "{} {}",
                "GPU:".bright_black().bold(),
                usage_color(&format!("{:.1}%", gpu_pct), gpu_pct, 50.0, 80.0)
            ));

            let vram_used_gb = gpu.memory_used as f64 / 1024.0 / 1024.0 / 1024.0;
            let vram_total_gb = gpu.memory_total as f64 / 1024.0 / 1024.0 / 1024.0;
            let vram_pct = if gpu.memory_total > 0 {
                gpu.memory_used as f64 / gpu.memory_total as f64 * 100.0
            } else {
                0.0
            };
            segments.push(format!(
                "{} {}",
                "VRAM:".bright_black().bold(),
                usage_color(&format!("{:.2}/{:.2}GB", vram_used_gb, vram_total_gb), vram_pct, 60.0, 80.0)
            ));
        }

        let mut fitted: Vec<String> = Vec::new();
        let mut used_cols = 0;

        for segment in segments {
            let segment_visible = strip_ansi_codes(&segment).chars().count();
            let space_needed = if fitted.is_empty() {
                segment_visible
            } else {
                3 + segment_visible // 3 = length of separator " | "
            };

            if used_cols + space_needed <= cols {
                used_cols += space_needed;
                fitted.push(segment);
            } else {
                break;
            }
        }

        if fitted.is_empty() {
            return;
        }

        let sep = format!(" {} ", "|".bright_black());
        let text = fitted.join(&sep);
        let visible_length = strip_ansi_codes(&text).chars().count();
        let padding = cols.saturating_sub(visible_length);
        print!("{}{}", " ".repeat(padding), text);
    }
}
