use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::UdpSocket;
use std::thread::JoinHandle;

use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};
use crate::tools::io_layout;

// ── Messages between handler thread and UI ───────────────────────────

enum UdpCmd {
    Send(String),
    Stop,
}

enum UdpEvent {
    Recv(String, String),   // (source addr, data)
    Sent(String),
    Status(String),
    Error(String),
}

// ── Network interface helper ──────────────────────────────────────────

struct LocalInterface {
    name: String,
    ip: String,
    mac: String,
}

impl LocalInterface {
    fn display(&self) -> String {
        format!("{} | {} | {}", self.name, self.ip, self.mac)
    }
}

fn list_local_interfaces() -> Vec<LocalInterface> {
    let mut ifaces = Vec::new();

    if let Ok(addrs) = if_addrs::get_if_addrs() {
        // Collect unique interface names for MAC lookup
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for addr in &addrs {
            if !addr.is_loopback() {
                seen_names.insert(addr.name.clone());
            }
        }

        // Look up MAC for each unique interface name
        let mac_map: std::collections::HashMap<String, String> = seen_names
            .into_iter()
            .filter_map(|name| {
                mac_address::mac_address_by_name(&name)
                    .ok()
                    .flatten()
                    .map(|mac| (name, mac.to_string()))
            })
            .collect();

        for addr in addrs {
            if addr.is_loopback() {
                continue;
            }
            let ip = addr.ip().to_string();
            let mac = mac_map.get(&addr.name).cloned().unwrap_or_else(|| "N/A".to_string());
            ifaces.push(LocalInterface {
                name: addr.name,
                ip,
                mac,
            });
        }
    }

    // Deduplicate by IP, sort by name
    ifaces.sort_by(|a, b| a.name.cmp(&b.name).then(a.ip.cmp(&b.ip)));
    ifaces.dedup_by(|a, b| a.ip == b.ip && a.name == b.name);

    // Always include "Any" option at the front
    ifaces.insert(0, LocalInterface {
        name: "*".to_string(),
        ip: "0.0.0.0".to_string(),
        mac: "-".to_string(),
    });
    ifaces
}

// ── Tool struct ───────────────────────────────────────────────────────

pub struct UdpTool {
    addr: String,
    local_ifaces: Vec<LocalInterface>,
    local_if_idx: usize,
    local_port: String,
    send_input: String,
    output: String,
    error: String,
    bound: Arc<AtomicBool>,
    heartbeat: bool,
    heartbeat_sec: f64,
    heartbeat_msg: String,
    cmd_tx: Option<mpsc::Sender<UdpCmd>>,
    event_rx: Option<mpsc::Receiver<UdpEvent>>,
    handler: Option<JoinHandle<()>>,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for UdpTool {
    fn default() -> Self {
        let ifaces = list_local_interfaces();
        Self {
            addr: String::new(),
            local_ifaces: ifaces,
            local_if_idx: 0,
            local_port: String::new(),
            send_input: String::new(),
            output: String::new(),
            error: String::new(),
            bound: Arc::new(AtomicBool::new(false)),
            heartbeat: false,
            heartbeat_sec: 1.0,
            heartbeat_msg: String::new(),
            cmd_tx: None,
            event_rx: None,
            handler: None,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Drop for UdpTool {
    fn drop(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(UdpCmd::Stop);
        }
    }
}

// ── Tool trait ────────────────────────────────────────────────────────

impl Tool for UdpTool {
    fn name(&self) -> String { tr!("udp_name") }
    fn description(&self) -> String { tr!("udp_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Network }
    fn is_busy(&self) -> bool { self.bound.load(Ordering::Relaxed) }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.send_input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        self.poll_events();

        let is_bound = self.bound.load(Ordering::Relaxed);

        ui.group(|ui| {
            ui.set_width(ui.available_width());

        // ── Row 1: Local Interface + Port ─────────────────────────────
        ui.horizontal(|ui| {
            ui.label(tr!("udp_local_ip"));
            let if_label = if self.local_ifaces.is_empty() {
                "0.0.0.0".to_string()
            } else {
                self.local_ifaces[self.local_if_idx].display()
            };
            egui::ComboBox::from_id_salt("udp_local_ip_combo")
                .selected_text(&if_label)
                .width(360.0)
                .show_ui(ui, |ui| {
                    for (i, iface) in self.local_ifaces.iter().enumerate() {
                        ui.selectable_value(&mut self.local_if_idx, i, iface.display());
                    }
                });

            ui.add_space(8.0);
            ui.label(tr!("udp_local_port"));
            ui.add(
                egui::TextEdit::singleline(&mut self.local_port)
                    .desired_width(80.0)
                    .hint_text(tr!("udp_port_hint")),
            );
        });
        ui.add_space(2.0);

        // ── Row 2: Target Address + Bind/Unbind ───────────────────────
        ui.horizontal(|ui| {
            ui.label(tr!("udp_target"));
            ui.add(
                egui::TextEdit::singleline(&mut self.addr)
                    .desired_width(260.0)
                    .hint_text("host:port"),
            );
            ui.add_space(8.0);
            if !is_bound {
                if ui.button(tr!("udp_btn_bind")).clicked() {
                    self.bind();
                }
            } else {
                if ui.button(tr!("udp_btn_unbind")).clicked() {
                    self.unbind();
                }
                ui.label(format!("  ● {}", tr!("udp_bound")));
            }
        });
        ui.add_space(2.0);

        // ── Row 3: Heartbeat options ─────────────────────────────────
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.heartbeat, tr!("ws_heartbeat"));
            ui.add(egui::DragValue::new(&mut self.heartbeat_sec).range(0.1..=3600.0).speed(0.1).suffix(format!(" {}", tr!("ws_heartbeat_sec"))));
            ui.add_space(4.0);
            ui.add_enabled_ui(self.heartbeat, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.heartbeat_msg)
                        .desired_width(160.0)
                        .hint_text(tr!("ws_heartbeat_hint")),
                );
            });
        });
        }); // config group

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let lbl_input = tr!("label_input");
        let lbl_output = tr!("label_output");
        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_send = tr!("udp_btn_send");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");

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
                                Ok(text) => self.send_input = text,
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
                            self.send_input.clear();
                            self.output.clear();
                            self.error.clear();
                        }
                        ui.add_enabled_ui(is_bound, |ui| {
                            if ui.button(&lbl_send).clicked() && !self.send_input.is_empty() {
                                self.send_msg();
                            }
                        });
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field_at(ui, w, field_h, "udp_input_scroll", &mut self.send_input);
            }
            io_layout::IoColumn::Right => {
                io_layout::column_header_row(ui, w, opt_h, |ui| {
                    ui.label(egui::RichText::new(&lbl_output).strong());
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
                                &tr!("udp_save_default"),
                                self.output.clone(),
                            );
                        }
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field_at(ui, w, field_h, "udp_output_scroll", &mut self.output);
            }
        });
    }
}

// ── Implementation ────────────────────────────────────────────────────

impl UdpTool {
    fn bind(&mut self) {
        self.error.clear();

        let addr = self.addr.trim().to_string();
        if addr.is_empty() || !addr.contains(':') {
            self.error = tr!("udp_invalid_addr");
            return;
        }

        // Parse server port from target host:port
        let server_port = match addr.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()) {
            Some(p) => p,
            None => {
                self.error = tr!("udp_invalid_addr");
                return;
            }
        };

        // Determine local bind port: use user input if provided, otherwise same as server port
        let local_port = if self.local_port.trim().is_empty() {
            server_port
        } else {
            match self.local_port.trim().parse::<u16>() {
                Ok(p) => p,
                Err(_) => {
                    self.error = tr!("udp_invalid_port");
                    return;
                }
            }
        };

        // Determine local bind IP
        let local_ip = if self.local_ifaces.is_empty() {
            "0.0.0.0".to_string()
        } else {
            self.local_ifaces[self.local_if_idx].ip.clone()
        };

        // Strip IPv6 zone id if present (e.g. fe80::1%eth0)
        let bind_ip = if let Some(percent) = local_ip.find('%') {
            &local_ip[..percent]
        } else {
            &local_ip
        };

        let local_bind = format!("{}:{}", bind_ip, local_port);
        let socket = match UdpSocket::bind(&local_bind) {
            Ok(s) => s,
            Err(e) => {
                self.error = format!("Bind failed: {}", e);
                return;
            }
        };

        if let Err(e) = socket.set_nonblocking(true) {
            self.error = format!("{}", e);
            return;
        }

        let (cmd_tx, cmd_rx) = mpsc::channel::<UdpCmd>();
        let (ev_tx, ev_rx) = mpsc::channel::<UdpEvent>();
        let bound = Arc::clone(&self.bound);
        let heartbeat = self.heartbeat;
        let heartbeat_sec = self.heartbeat_sec;
        let heartbeat_msg = self.heartbeat_msg.clone();

        self.cmd_tx = Some(cmd_tx);
        self.event_rx = Some(ev_rx);
        self.bound.store(true, Ordering::Relaxed);

        let local = socket.local_addr().unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
        self.output.push_str(&format!("[{}] {} {} -> {}\n", now_str(), tr!("udp_msg_sys"), local, addr));

        let handler = std::thread::spawn(move || {
            udp_handler(socket, &addr, &cmd_rx, &ev_tx, &bound, heartbeat, heartbeat_sec, &heartbeat_msg);
        });
        self.handler = Some(handler);
    }

    fn unbind(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(UdpCmd::Stop);
        }
        self.bound.store(false, Ordering::Relaxed);
        self.cmd_tx = None;
        self.handler = None;
    }

    fn send_msg(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(UdpCmd::Send(self.send_input.clone()));
        }
    }

    fn poll_events(&mut self) {
        if let Some(rx) = &self.event_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    UdpEvent::Status(s) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_sys"), s));
                    }
                    UdpEvent::Recv(src, data) => {
                        self.output.push_str(&format!("[{}] {} [{}] {}\n", now_str(), tr!("udp_msg_recv"), src, data));
                    }
                    UdpEvent::Sent(data) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("udp_msg_sent"), data));
                    }
                    UdpEvent::Error(s) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_error"), s));
                    }
                }
            }
        }
    }
}

// ── Handler thread ────────────────────────────────────────────────────

fn udp_handler(
    socket: UdpSocket,
    target: &str,
    cmd_rx: &mpsc::Receiver<UdpCmd>,
    ev_tx: &mpsc::Sender<UdpEvent>,
    bound: &AtomicBool,
    heartbeat: bool,
    heartbeat_sec: f64,
    heartbeat_msg: &str,
) {
    let mut buf = [0u8; 65535];
    let mut last_hb = std::time::Instant::now();
    let hb_interval = std::time::Duration::from_secs_f64(heartbeat_sec.max(0.01));

    loop {
        // ── Check commands ──
        match cmd_rx.try_recv() {
            Ok(UdpCmd::Stop) => break,
            Ok(UdpCmd::Send(text)) => {
                match socket.send_to(text.as_bytes(), target) {
                    Ok(_) => { let _ = ev_tx.send(UdpEvent::Sent(text)); }
                    Err(e) => { let _ = ev_tx.send(UdpEvent::Error(format!("Send failed: {}", e))); }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        // ── Heartbeat ──
        if heartbeat && last_hb.elapsed() >= hb_interval {
            if !heartbeat_msg.is_empty() {
                let _ = socket.send_to(heartbeat_msg.as_bytes(), target);
            }
            last_hb = std::time::Instant::now();
        }

        // ── Receive ──
        match socket.recv_from(&mut buf) {
            Ok((n, src)) => {
                let data = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = ev_tx.send(UdpEvent::Recv(src.to_string(), data));
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => {
                let _ = ev_tx.send(UdpEvent::Error(format!("Recv error: {}", e)));
                break;
            }
        }
    }

    bound.store(false, Ordering::Relaxed);
    let _ = ev_tx.send(UdpEvent::Status(tr!("udp_unbound")));
}

fn now_str() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}
