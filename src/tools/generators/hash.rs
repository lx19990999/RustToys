use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::Pending;
use sha2::{Sha256, Sha384, Sha512, Digest};
use sha1::Sha1;
use md5::Md5;
use std::sync::mpsc;

struct HashResult {
    md5: String,
    sha1: String,
    sha256: String,
    sha384: String,
    sha512: String,
}

enum HashMsg {
    Progress(f32),
    Done(HashResult),
}

pub struct HashGenerator {
    input: String,
    uppercase: bool,
    error: String,
    md5: String,
    sha1: String,
    sha256: String,
    sha384: String,
    sha512: String,
    verify_checksum: String,
    is_file: bool,
    file_path: String,
    // Background computation
    computing: bool,
    progress: f32,
    hash_rx: Option<mpsc::Receiver<HashMsg>>,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for HashGenerator {
    fn default() -> Self {
        Self {
            input: String::new(),
            uppercase: true,
            error: String::new(),
            md5: String::new(),
            sha1: String::new(),
            sha256: String::new(),
            sha384: String::new(),
            sha512: String::new(),
            verify_checksum: String::new(),
            is_file: false,
            file_path: String::new(),
            computing: false,
            progress: 0.0,
            hash_rx: None,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Tool for HashGenerator {
    fn name(&self) -> String { tr!("hash_name") }
    fn description(&self) -> String { tr!("hash_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Generators }

    fn is_busy(&self) -> bool { self.computing }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            if !self.computing {
                self.file_path = path.to_string_lossy().to_string();
                self.input = self.file_path.clone();
                self.is_file = true;
                self.start_file_hash(path);
            }
        }
        if let Some(text) = self.pending_file.poll() {
            self.verify_checksum = text;
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        // Poll background thread
        // Poll background thread
        let mut msgs = Vec::new();
        if let Some(rx) = &self.hash_rx {
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
        }
        for msg in msgs {
            match msg {
                HashMsg::Progress(p) => self.progress = p,
                HashMsg::Done(result) => {
                    self.set_results(result);
                    self.computing = false;
                    self.progress = 1.0;
                    self.hash_rx = None;
                }
            }
        }
        if self.computing {
            ui.ctx().request_repaint();
        }

        let prev_input = self.input.clone();
        let prev_uppercase = self.uppercase;

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    let lbl_paste = tr!("btn_paste");
                    if ui.button(lbl_paste).clicked() && !self.computing {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => {
                                self.input = text;
                                self.is_file = false;
                                self.file_path.clear();
                            }
                            Err(e) => {
                                self.input = tr!("err_clipboard", e);
                            }
                        }
                    }
                    let lbl_open = tr!("btn_open_file");
                    if ui.button(lbl_open).clicked() && !self.computing {
                        let title = tr!("save_as_title");
                        let filter_all = tr!("save_filter_all");
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title(&title)
                            .add_filter(&filter_all, &["*"])
                            .pick_file()
                        {
                            self.file_path = path.to_string_lossy().to_string();
                            self.input = self.file_path.clone();
                            self.is_file = true;
                            self.start_file_hash(path);
                        }
                    }
                    let lbl_clear = tr!("btn_clear");
                    if ui.button(lbl_clear).clicked() && !self.computing {
                        self.input.clear();
                        self.file_path.clear();
                        self.is_file = false;
                        self.md5.clear();
                        self.sha1.clear();
                        self.sha256.clear();
                        self.sha384.clear();
                        self.sha512.clear();
                        self.verify_checksum.clear();
                    }
                });
                ui.add_space(2.0);

                let lbl_upper = tr!("label_uppercase");
                ui.checkbox(&mut self.uppercase, &lbl_upper);
                ui.add_space(2.0);

                if self.is_file {
                    ui.label(tr!("hash_file_label", &self.file_path));
                } else {
                    ui.label(tr!("hash_input_text"));

                    egui::ScrollArea::vertical()
                        .id_salt("hash_input_scroll")
                        .max_height(120.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.input)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                }

                // Progress bar
                if self.computing {
                    ui.add_space(8.0);
                    ui.label(tr!("hash_computing"));
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                    ui.ctx().request_repaint();
                }

                ui.add_space(4.0);
                ui.label(tr!("hash_verify_label"));

                egui::ScrollArea::horizontal()
                    .id_salt("hash_verify_scroll")
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.verify_checksum)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Hashes
            cols[1].vertical(|ui| {
                ui.add_space(2.0);
                Self::hash_row(ui, "MD5", &self.md5, &self.verify_checksum, &mut self.save_pending);
                Self::hash_row(ui, "SHA-1", &self.sha1, &self.verify_checksum, &mut self.save_pending);
                Self::hash_row(ui, "SHA-256", &self.sha256, &self.verify_checksum, &mut self.save_pending);
                Self::hash_row(ui, "SHA-384", &self.sha384, &self.verify_checksum, &mut self.save_pending);
                Self::hash_row(ui, "SHA-512", &self.sha512, &self.verify_checksum, &mut self.save_pending);

                // Copy All
                if !self.md5.is_empty() && !self.computing {
                    ui.add_space(8.0);
                    let lbl_copy_all = tr!("btn_copy_all");
                    if ui.button(lbl_copy_all).clicked() {
                        let all = format!(
                            "MD5:     {}\nSHA-1:   {}\nSHA-256: {}\nSHA-384: {}\nSHA-512: {}",
                            self.md5, self.sha1, self.sha256, self.sha384, self.sha512
                        );
                        ui.ctx().copy_text(all);
                    }
                    let lbl_save_as = tr!("btn_save_as");
                    if ui.button(lbl_save_as).clicked() {
                        let title = tr!("hash_save_single");
                        let filter_text = tr!("save_filter_text");
                        let default_name = tr!("hash_save_default");
                        let all = format!(
                            "MD5:     {}\nSHA-1:   {}\nSHA-256: {}\nSHA-384: {}\nSHA-512: {}",
                            self.md5, self.sha1, self.sha256, self.sha384, self.sha512
                        );
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &title, &filter_text, &["txt"], &default_name, all);
                    }
                }
            });
        });

        // Auto-generate for text (sync, fast)
        if !self.is_file && !self.computing
            && (self.input != prev_input || self.uppercase != prev_uppercase)
        {
            self.generate_text();
        }
    }
}

impl HashGenerator {
    fn set_results(&mut self, r: HashResult) {
        self.md5 = self.fmt_hash(r.md5);
        self.sha1 = self.fmt_hash(r.sha1);
        self.sha256 = self.fmt_hash(r.sha256);
        self.sha384 = self.fmt_hash(r.sha384);
        self.sha512 = self.fmt_hash(r.sha512);
    }

    fn fmt_hash(&self, s: String) -> String {
        if self.uppercase { s.to_uppercase() } else { s }
    }

    fn generate_text(&mut self) {
        if self.input.is_empty() {
            self.md5.clear();
            self.sha1.clear();
            self.sha256.clear();
            self.sha384.clear();
            self.sha512.clear();
            return;
        }
        let bytes = self.input.as_bytes();
        self.md5 = self.fmt_hash(hex::encode(Md5::digest(bytes)));
        self.sha1 = self.fmt_hash(hex::encode(Sha1::digest(bytes)));
        self.sha256 = self.fmt_hash(hex::encode(Sha256::digest(bytes)));
        self.sha384 = self.fmt_hash(hex::encode(Sha384::digest(bytes)));
        self.sha512 = self.fmt_hash(hex::encode(Sha512::digest(bytes)));
    }

    fn start_file_hash(&mut self, path: std::path::PathBuf) {
        self.computing = true;
        self.progress = 0.0;
        self.md5.clear();
        self.sha1.clear();
        self.sha256.clear();
        self.sha384.clear();
        self.sha512.clear();

        let (tx, rx) = mpsc::channel::<HashMsg>();
        self.hash_rx = Some(rx);

        std::thread::spawn(move || {
            let file = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(HashMsg::Done(HashResult {
                        md5: format!("Error: {}", e),
                        sha1: String::new(), sha256: String::new(),
                        sha384: String::new(), sha512: String::new(),
                    }));
                    return;
                }
            };

            let total = file.metadata().map(|m| m.len()).unwrap_or(0) as f32;
            use std::io::Read;
            let mut reader = std::io::BufReader::new(file);

            let mut md5h = Md5::new();
            let mut sha1h = Sha1::new();
            let mut sha256h = Sha256::new();
            let mut sha384h = Sha384::new();
            let mut sha512h = Sha512::new();

            let mut buf = [0u8; 1024 * 1024]; // 1MB chunks
            let mut processed: f32 = 0.0;

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        md5h.update(&buf[..n]);
                        sha1h.update(&buf[..n]);
                        sha256h.update(&buf[..n]);
                        sha384h.update(&buf[..n]);
                        sha512h.update(&buf[..n]);
                        processed += n as f32;
                        if total > 0.0 {
                            let _ = tx.send(HashMsg::Progress((processed / total).min(1.0)));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(HashMsg::Done(HashResult {
                            md5: format!("Read error: {}", e),
                            sha1: String::new(), sha256: String::new(),
                            sha384: String::new(), sha512: String::new(),
                        }));
                        return;
                    }
                }
            }

            let _ = tx.send(HashMsg::Done(HashResult {
                md5: hex::encode(md5h.finalize()),
                sha1: hex::encode(sha1h.finalize()),
                sha256: hex::encode(sha256h.finalize()),
                sha384: hex::encode(sha384h.finalize()),
                sha512: hex::encode(sha512h.finalize()),
            }));
        });
    }

    fn hash_row(ui: &mut egui::Ui, label: &str, value: &str, verify: &str, save_pending: &mut Pending<String>) {
        let matches = if !verify.is_empty() && !value.is_empty() {
            value.to_lowercase() == verify.trim().to_lowercase()
        } else {
            false
        };

        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let mut display = value.to_string();
            ui.add(
                egui::TextEdit::singleline(&mut display)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false),
            );

            if !value.is_empty() {
                let lbl_copy = tr!("btn_copy");
                if ui.button(lbl_copy).clicked() {
                    ui.ctx().copy_text(value.to_string());
                }
                let lbl_save = tr!("btn_save_as");
                if ui.button(lbl_save).clicked() {
                    let title = tr!("hash_save_single");
                    let filter_text = tr!("save_filter_text");
                    crate::tools::async_utils::save_file_async(save_pending, &title, &filter_text, &["txt"], &format!("{}.txt", label.to_lowercase().replace('-', "")), value.to_string());
                }
            }
        });

        if matches {
            ui.colored_label(egui::Color32::from_rgb(0, 180, 0), tr!("hash_matches", label));
        }
        ui.add_space(2.0);
    }
}
