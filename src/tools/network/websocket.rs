use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use eframe::egui;
use tungstenite::Message;
use tungstenite::stream::MaybeTlsStream;

use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};
use crate::tools::io_layout;

// ── Messages between handler thread and UI ───────────────────────────

enum WsCmd {
    Send(String),
    Stop,
}

enum WsEvent {
    Status(String),
    Recv(String),
    Sent(String),
    Error(String),
}

// ── Tool struct ───────────────────────────────────────────────────────

pub struct WebSocketTool {
    url: String,
    send_input: String,
    output: String,
    error: String,
    auto_reconnect: bool,
    heartbeat: bool,
    heartbeat_sec: u64,
    heartbeat_msg: String,
    connected: Arc<AtomicBool>,
    cmd_tx: Option<mpsc::Sender<WsCmd>>,
    event_rx: Option<mpsc::Receiver<WsEvent>>,
    handler: Option<JoinHandle<()>>,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for WebSocketTool {
    fn default() -> Self {
        Self {
            url: String::new(),
            send_input: String::new(),
            output: String::new(),
            error: String::new(),
            auto_reconnect: false,
            heartbeat: true,
            heartbeat_sec: 30,
            heartbeat_msg: "ping".to_string(),
            connected: Arc::new(AtomicBool::new(false)),
            cmd_tx: None,
            event_rx: None,
            handler: None,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Drop for WebSocketTool {
    fn drop(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(WsCmd::Stop);
        }
    }
}

// ── Tool trait ────────────────────────────────────────────────────────

impl Tool for WebSocketTool {
    fn name(&self) -> String { tr!("ws_name") }
    fn description(&self) -> String { tr!("ws_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Network }
    fn is_busy(&self) -> bool { self.connected.load(Ordering::Relaxed) }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            crate::tools::async_utils::open_dropped_text_async(&mut self.pending_file, path);
        }
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.send_input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        self.poll_events();

        let is_connected = self.connected.load(Ordering::Relaxed);

        ui.group(|ui| {
            ui.set_width(ui.available_width());

        // ── Row 1: URL + Connect/Disconnect ──────────────────────────
        ui.horizontal(|ui| {
            ui.label("URL:");
            let _resp = ui.add(
                egui::TextEdit::singleline(&mut self.url)
                    .desired_width(320.0)
                    .hint_text("ws:// or wss://"),
            );
            if is_connected {
                ui.label(format!("  {}  ●", tr!("ws_connected")));
            } else {
                ui.label(format!("  ○  {}", tr!("ws_disconnected")));
            }
        });
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if !is_connected {
                if ui.button(tr!("ws_btn_connect")).clicked() {
                    self.connect();
                }
            } else {
                if ui.button(tr!("ws_btn_disconnect")).clicked() {
                    self.disconnect();
                }
            }
            ui.add_space(16.0);
            ui.checkbox(&mut self.auto_reconnect, tr!("ws_auto_reconnect"));
            ui.add_space(8.0);
            ui.checkbox(&mut self.heartbeat, tr!("ws_heartbeat"));
            ui.add(egui::DragValue::new(&mut self.heartbeat_sec).range(5..=300).speed(1).suffix(format!(" {}", tr!("ws_heartbeat_sec"))));
            ui.add_space(4.0);
            ui.add_enabled_ui(self.heartbeat, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.heartbeat_msg)
                        .desired_width(140.0)
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
        let lbl_send = tr!("ws_btn_send");
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
                        ui.add_enabled_ui(is_connected, |ui| {
                            if ui.button(&lbl_send).clicked() && !self.send_input.is_empty() {
                                self.send_msg();
                            }
                        });
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field_at(ui, w, field_h, "ws_input_scroll", &mut self.send_input);
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
                                &tr!("ws_save_default"),
                                self.output.clone(),
                            );
                        }
                    });
                });
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field_at(ui, w, field_h, "ws_output_scroll", &mut self.output);
            }
        });
    }
}

// ── Implementation ────────────────────────────────────────────────────

impl WebSocketTool {
    fn connect(&mut self) {
        self.error.clear();

        let url = self.url.trim().to_string();
        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            self.error = tr!("ws_invalid_url");
            return;
        }

        if self.connected.load(Ordering::Relaxed) {
            self.disconnect();
        }

        let (cmd_tx, cmd_rx) = mpsc::channel::<WsCmd>();
        let (ev_tx, ev_rx) = mpsc::channel::<WsEvent>();
        let connected = Arc::clone(&self.connected);
        let auto_reconnect = self.auto_reconnect;
        let heartbeat = self.heartbeat;
        let heartbeat_sec = self.heartbeat_sec;
        let heartbeat_msg = self.heartbeat_msg.clone();

        self.cmd_tx = Some(cmd_tx);
        self.event_rx = Some(ev_rx);
        self.connected.store(true, Ordering::Relaxed);

        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_sys"), tr!("ws_connecting")));

        let handler = std::thread::spawn(move || {
            handler_loop(&url, &cmd_rx, &ev_tx, &connected, auto_reconnect, heartbeat, heartbeat_sec, &heartbeat_msg);
        });
        self.handler = Some(handler);
    }

    fn disconnect(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(WsCmd::Stop);
        }
        self.connected.store(false, Ordering::Relaxed);
        self.cmd_tx = None;
        self.handler = None;
    }

    fn send_msg(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(WsCmd::Send(self.send_input.clone()));
        }
    }

    fn poll_events(&mut self) {
        if let Some(rx) = &self.event_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    WsEvent::Status(s) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_sys"), s));
                    }
                    WsEvent::Recv(s) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_recv"), s));
                    }
                    WsEvent::Sent(s) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_sent"), s));
                    }
                    WsEvent::Error(s) => {
                        self.output.push_str(&format!("[{}] {} {}\n", now_str(), tr!("ws_msg_error"), s));
                    }
                }
            }
        }
    }
}

// ── Handler thread ────────────────────────────────────────────────────

fn handler_loop(
    url: &str,
    cmd_rx: &mpsc::Receiver<WsCmd>,
    ev_tx: &mpsc::Sender<WsEvent>,
    connected: &AtomicBool,
    auto_reconnect: bool,
    heartbeat: bool,
    heartbeat_sec: u64,
    heartbeat_msg: &str,
) {
    loop {
        let mut ws = match tungstenite::connect(url) {
            Ok((ws, _)) => ws,
            Err(e) => {
                let _ = ev_tx.send(WsEvent::Error(format!("Connect failed: {}", e)));
                if auto_reconnect && !should_stop(cmd_rx, std::time::Duration::from_secs(3)) {
                    let _ = ev_tx.send(WsEvent::Status(tr!("ws_reconnecting")));
                    continue;
                }
                break;
            }
        };

        // Non-blocking reads so the loop can check commands/heartbeat
        match ws.get_mut() {
            MaybeTlsStream::Plain(s) => { let _ = s.set_nonblocking(true); }
            _ => {}
        }

        let _ = ev_tx.send(WsEvent::Status(tr!("ws_connected")));
        let mut last_hb = std::time::Instant::now();

        loop {
            // ── Check commands ──
            match cmd_rx.try_recv() {
                Ok(WsCmd::Stop) => { let _ = ws.close(None); break; }
                Ok(WsCmd::Send(text)) => {
                    if ws.send(Message::Text(text.clone())).is_ok() {
                        let _ = ev_tx.send(WsEvent::Sent(text));
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {}
            }

            // ── Heartbeat ──
            if heartbeat && last_hb.elapsed().as_secs() >= heartbeat_sec {
                let _ = ws.send(Message::Text(heartbeat_msg.to_string()));
                last_hb = std::time::Instant::now();
            }

            // ── Read messages ──
            match ws.read() {
                Ok(msg) => match msg {
                    Message::Text(t) => { let _ = ev_tx.send(WsEvent::Recv(t)); }
                    Message::Binary(b) => {
                        let _ = ev_tx.send(WsEvent::Recv(format!("[binary {}B]", b.len())));
                    }
                    Message::Ping(d) => { let _ = ws.send(Message::Pong(d)); }
                    Message::Close(_) => {
                        let _ = ev_tx.send(WsEvent::Status("Server closed connection".into()));
                        break;
                    }
                    _ => {}
                },
                Err(tungstenite::Error::Io(ref e))
                    if e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    let _ = ev_tx.send(WsEvent::Error(format!("{}", e)));
                    break;
                }
            }
        }

        connected.store(false, Ordering::Relaxed);
        let _ = ev_tx.send(WsEvent::Status(tr!("ws_disconnected")));

        if !auto_reconnect || should_stop(cmd_rx, std::time::Duration::from_secs(3)) {
            break;
        }
        let _ = ev_tx.send(WsEvent::Status(tr!("ws_reconnecting")));
    }
}

/// Wait `dur` for a Stop command. Returns true if Stop received.
fn should_stop(cmd_rx: &mpsc::Receiver<WsCmd>, dur: std::time::Duration) -> bool {
    match cmd_rx.recv_timeout(dur) {
        Ok(WsCmd::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => true,
        _ => false,
    }
}

fn now_str() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}
