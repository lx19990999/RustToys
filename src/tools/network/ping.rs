use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::process::Command;

use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};
use crate::tools::io_layout;

struct PingResult {
    host: String,
    time_ms: Option<f64>,
    detail: String,
    success: bool,
}

pub struct PingSpeedTest {
    input: String,
    output: String,
    error: String,
    running: bool,
    total: usize,
    sort_by_latency: bool,
    results: Arc<Mutex<Vec<Option<PingResult>>>>,
    remaining: Arc<AtomicU32>,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for PingSpeedTest {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            running: false,
            total: 0,
            sort_by_latency: false,
            results: Arc::new(Mutex::new(Vec::new())),
            remaining: Arc::new(AtomicU32::new(0)),
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Tool for PingSpeedTest {
    fn name(&self) -> String { tr!("ping_name") }
    fn description(&self) -> String { tr!("ping_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Network }
    fn is_busy(&self) -> bool { self.running }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }


        // Poll results
        if self.running {
            if let Ok(results) = self.results.lock() {
                if !results.is_empty() {
                    let mut lines = vec![tr!("ping_header")];
                    lines.push("-".repeat(72));

                    if self.sort_by_latency {
                        // Latency sort: completed first (sorted by time), then pending
                        let mut completed: Vec<&PingResult> = results.iter().filter_map(|s| s.as_ref()).collect();
                        completed.sort_by(|a, b| {
                            let ta = a.time_ms.unwrap_or(f64::MAX);
                            let tb = b.time_ms.unwrap_or(f64::MAX);
                            ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        for r in &completed {
                            let status = if r.success { tr!("ping_ok") } else { tr!("ping_fail") };
                            let time_str = r.time_ms.map(|t| format!("{:.1}", t)).unwrap_or_default();
                            lines.push(format!("{:<32}{:<12}{:<12}{}", r.host, status, time_str, r.detail));
                        }
                        let pending = results.iter().filter(|s| s.is_none()).count();
                        for _ in 0..pending {
                            lines.push("...".to_string());
                        }
                    } else {
                        // Input order
                        for slot in results.iter() {
                            match slot {
                                Some(r) => {
                                    let status = if r.success { tr!("ping_ok") } else { tr!("ping_fail") };
                                    let time_str = r.time_ms.map(|t| format!("{:.1}", t)).unwrap_or_default();
                                    lines.push(format!("{:<32}{:<12}{:<12}{}", r.host, status, time_str, r.detail));
                                }
                                None => {
                                    lines.push("...".to_string());
                                }
                            }
                        }
                    }

                    let rem = self.remaining.load(Ordering::Relaxed);
                    if rem > 0 {
                        lines.push(String::new());
                        lines.push(tr!("ping_testing"));
                    }
                    self.output = lines.join("\n");
                }
            }
            if self.remaining.load(Ordering::Relaxed) == 0 && !self.results.lock().map_or(true, |r| r.is_empty()) {
                self.running = false;
                self.output.push_str(&format!("\n\n{}", tr!("ping_done")));
            }
        }

        let lbl_input = tr!("label_input");
        let lbl_output = tr!("label_output");
        let lbl_input_order = tr!("ping_sort_input");
        let lbl_latency = tr!("ping_sort_latency");
        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_start = tr!("ping_btn_start");
        let lbl_stop = tr!("ping_btn_stop");

        io_layout::show_error(ui, &self.error);
        let (opt_h, body_h, field_h) = io_layout::aligned_io_heights(ui);
        io_layout::two_column_io_with_height(ui, body_h, |ui, w, col| match col {
            io_layout::IoColumn::Left => {
                io_layout::column_header_row(ui, w, opt_h, |ui| {
                    ui.label(egui::RichText::new(&lbl_input).strong());
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::toolbar_row(ui, w, opt_h, |ui| {
                    ui.horizontal_wrapped(|ui| {
                    if ui.button(&lbl_paste).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(&lbl_open).clicked() {
                        open_file_async(
                            &mut self.pending_file,
                            &tr!("save_as_title"),
                            &tr!("save_filter_text"),
                            &["txt"],
                        );
                    }
                    if ui.button(&lbl_clear).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                    if self.running {
                        if ui.button(&lbl_stop).clicked() {
                            self.running = false;
                        }
                    } else if ui.button(&lbl_start).clicked() {
                        self.start_test();
                    }
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field_at(ui, w, field_h, "ping_input_scroll", &mut self.input);
            }
            io_layout::IoColumn::Right => {
                io_layout::column_header_row(ui, w, opt_h, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&lbl_output).strong());
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.radio_value(&mut self.sort_by_latency, true, &lbl_latency);
                                ui.radio_value(&mut self.sort_by_latency, false, &lbl_input_order);
                            },
                        );
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::toolbar_row(ui, w, opt_h, |ui| {
                    ui.horizontal_wrapped(|ui| {
                    if ui.button(&lbl_copy).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() && !self.output.is_empty() {
                        save_file_async(
                            &mut self.save_pending,
                            &tr!("save_as_title"),
                            &tr!("save_filter_text"),
                            &["txt"],
                            &tr!("ping_save_default"),
                            self.output.clone(),
                        );
                    }
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                ping_scroll_output(ui, w, field_h, self);
            }
        });
    }
}

fn ping_scroll_output(ui: &mut egui::Ui, col_w: f32, h: f32, tool: &PingSpeedTest) {
    ui.allocate_ui_with_layout(
        egui::vec2(col_w, h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.set_width(col_w);
            ui.set_max_width(col_w);
            ui.set_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .id_salt("ping_output_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(col_w);
                    if let Ok(results) = tool.results.lock() {
                        if !results.is_empty() {
                            render_ping_results(ui, col_w, &results, tool.sort_by_latency, tool.running);
                            return;
                        }
                    }
                    if !tool.output.is_empty() {
                        ui.label(
                            egui::RichText::new(&tool.output)
                                .monospace()
                                .color(ui.visuals().text_color()),
                        );
                    }
                });
        },
    );
}

fn render_ping_results(
    ui: &mut egui::Ui,
    col_w: f32,
    results: &[Option<PingResult>],
    sort_by_latency: bool,
    running: bool,
) {
    let lbl_host = tr!("ping_host");
    let lbl_status = tr!("ping_status");
    let lbl_time = tr!("ping_time");
    let lbl_detail = tr!("ping_detail");
    let lbl_ok = tr!("ping_ok");
    let lbl_fail = tr!("ping_fail");
    let lbl_testing = tr!("ping_testing");

    ui.set_min_width(col_w);
    ui.set_max_width(col_w);

    let ordered: Vec<Option<&PingResult>> = if sort_by_latency {
        let mut completed: Vec<&PingResult> = results.iter().filter_map(|s| s.as_ref()).collect();
        completed.sort_by(|a, b| {
            let ta = a.time_ms.unwrap_or(f64::MAX);
            let tb = b.time_ms.unwrap_or(f64::MAX);
            ta.partial_cmp(&tb).unwrap_or(std::cmp::Ordering::Equal)
        });
        completed
            .into_iter()
            .map(Some)
            .chain(results.iter().filter(|s| s.is_none()).map(|_| None))
            .collect()
    } else {
        results.iter().map(|s| s.as_ref()).collect()
    };

    egui::Grid::new("ping_results_table")
        .num_columns(4)
        .spacing([10.0, 6.0])
        .striped(true)
        .min_col_width(52.0)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(&lbl_host).strong());
            ui.label(egui::RichText::new(&lbl_status).strong());
            ui.label(egui::RichText::new(&lbl_time).strong());
            ui.label(egui::RichText::new(&lbl_detail).strong());
            ui.end_row();

            for slot in &ordered {
                match slot {
                    Some(r) => ping_grid_row(ui, r, &lbl_ok, &lbl_fail),
                    None => {
                        ui.label(egui::RichText::new("…").weak());
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    }
                }
            }
        });

    if running && results.iter().any(|s| s.is_none()) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(&lbl_testing).italics().weak());
    } else if !running && results.iter().all(|s| s.is_some()) && !results.is_empty() {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(tr!("ping_done")).strong());
    }
}

fn ping_grid_row(ui: &mut egui::Ui, r: &PingResult, lbl_ok: &str, lbl_fail: &str) {
    let status = if r.success { lbl_ok } else { lbl_fail };
    let status_color = if r.success {
        egui::Color32::from_rgb(0, 140, 0)
    } else {
        ui.visuals().error_fg_color
    };
    let time_str = r
        .time_ms
        .map(|t| format!("{:.1}", t))
        .unwrap_or_else(|| "-".to_string());

    ui.label(egui::RichText::new(&r.host).monospace());
    ui.colored_label(status_color, status);
    ui.label(egui::RichText::new(time_str).monospace());
    ui.label(
        egui::RichText::new(&r.detail)
            .small()
            .weak()
            .monospace(),
    );
    ui.end_row();
}

impl PingSpeedTest {
    fn start_test(&mut self) {
        self.error.clear();
        self.output.clear();

        let mut hosts = Vec::new();
        for line in self.input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(host) = extract_host(trimmed) {
                hosts.push(host);
            }
        }

        if hosts.is_empty() {
            self.error = tr!("ping_no_input");
            return;
        }

        let count = hosts.len() as u32;
        self.total = hosts.len();
        *self.results.lock().unwrap() = (0..hosts.len()).map(|_| None).collect();
        self.remaining.store(count, Ordering::Relaxed);
        self.running = true;

        let results = Arc::clone(&self.results);
        let remaining = Arc::clone(&self.remaining);

        for (i, host) in hosts.into_iter().enumerate() {
            let results = Arc::clone(&results);
            let remaining = Arc::clone(&remaining);
            std::thread::spawn(move || {
                let result = ping_host(&host);
                if let Ok(mut r) = results.lock() {
                    r[i] = Some(result);
                }
                remaining.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }
}

/// Extract hostname from input. Handles URLs like `https://host/path`,
/// plain hostnames, and IP addresses. Returns None for invalid entries.
fn extract_host(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() || s.contains('<') || s.contains('>') {
        return None;
    }

    // Strip scheme: http:// or https://
    let after_scheme = if let Some(rest) = s.strip_prefix("https://") {
        rest
    } else if let Some(rest) = s.strip_prefix("http://") {
        rest
    } else {
        s
    };

    if after_scheme.is_empty() {
        return None;
    }

    // Take everything before the first '/' (path), '?' (query), '#' (fragment)
    let host = after_scheme.split(|c| c == '/' || c == '?' || c == '#').next().unwrap_or(after_scheme);

    // Strip port: host:port
    let host = if host.starts_with('[') {
        // IPv6 like [::1]:8080
        if let Some(end) = host.find(']') {
            &host[1..end]
        } else {
            host
        }
    } else {
        host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host)
    };

    let host = host.trim();
    if host.is_empty() {
        return None;
    }

    // Must contain only valid hostname chars
    if host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':') {
        Some(host.to_string())
    } else {
        None
    }
}

fn ping_host(host: &str) -> PingResult {
    let mut cmd = Command::new("ping");
    #[cfg(target_os = "windows")]
    {
        cmd.args(["-n", "1", "-w", "5000", host]);
    }
    #[cfg(not(target_os = "windows"))]
    {
        cmd.args(["-c", "1", "-W", "5", host]);
    }

    match cmd.output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}\n{}", stdout, stderr);
            parse_ping_output(host, &combined)
        }
        Err(e) => PingResult {
            host: host.to_string(),
            time_ms: None,
            detail: format!("Error: {}", e),
            success: false,
        },
    }
}

fn parse_ping_output(host: &str, output: &str) -> PingResult {
    // Linux/macOS: "time=1.23 ms" or "time<1ms"
    for line in output.lines() {
        if let Some(pos) = line.find("time=") {
            let rest = &line[pos + 5..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(ms) = num_str.parse::<f64>() {
                return PingResult {
                    host: host.to_string(),
                    time_ms: Some(ms),
                    detail: line.trim().to_string(),
                    success: true,
                };
            }
        }
        if let Some(pos) = line.find("time<") {
            let rest = &line[pos + 5..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(ms) = num_str.parse::<f64>() {
                return PingResult {
                    host: host.to_string(),
                    time_ms: Some(ms),
                    detail: line.trim().to_string(),
                    success: true,
                };
            }
        }
    }

    // Windows: "time=1ms" or "time<1ms"
    for line in output.lines() {
        if line.contains("time=") || line.contains("time<") {
            return PingResult {
                host: host.to_string(),
                time_ms: None,
                detail: line.trim().to_string(),
                success: line.contains("TTL") || line.contains("ttl"),
            };
        }
    }

    // Failure cases
    let detail = output.lines()
        .find(|l| l.contains("unreachable") || l.contains("timeout") || l.contains("100% packet loss") || l.contains("failure"))
        .unwrap_or("Host unreachable")
        .trim();

    PingResult {
        host: host.to_string(),
        time_ms: None,
        detail: detail.to_string(),
        success: false,
    }
}
