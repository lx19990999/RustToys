use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use aes::cipher::{BlockEncrypt, BlockDecrypt, KeyInit, generic_array::GenericArray};
use base64::Engine;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Algorithm {
    Aes128,
    Aes192,
    Aes256,
    Des,
}

impl Algorithm {
    fn key_len(&self) -> usize {
        match self {
            Self::Aes128 => 16,
            Self::Aes192 => 24,
            Self::Aes256 => 32,
            Self::Des => 8,
        }
    }
    fn label(&self) -> &'static str {
        match self {
            Self::Aes128 => "AES-128",
            Self::Aes192 => "AES-192",
            Self::Aes256 => "AES-256",
            Self::Des => "DES",
        }
    }
}

pub struct SymmetricEncryption {
    input: String,
    output: String,
    error: String,
    encrypt: bool,
    algorithm: Algorithm,
    key: String,
    iv: String,
    pending_file: Pending<String>,
}

impl Default for SymmetricEncryption {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            encrypt: true,
            algorithm: Algorithm::Aes128,
            key: String::new(),
            iv: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl Tool for SymmetricEncryption {
    fn name(&self) -> &str { "对称加密 (AES / DES)" }
    fn description(&self) -> &str { "AES and DES encryption/decryption using CBC mode" }
    fn category(&self) -> ToolCategory { ToolCategory::Encryption }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }

        ui.horizontal(|ui| {
            ui.label("Algorithm:");
            let alg_label = self.algorithm.label();
            egui::ComboBox::from_id_salt("sym_alg")
                .selected_text(alg_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.algorithm, Algorithm::Aes128, "AES-128");
                    ui.selectable_value(&mut self.algorithm, Algorithm::Aes192, "AES-192");
                    ui.selectable_value(&mut self.algorithm, Algorithm::Aes256, "AES-256");
                    ui.selectable_value(&mut self.algorithm, Algorithm::Des, "DES");
                });
            ui.separator();
            ui.radio_value(&mut self.encrypt, true, "Encrypt");
            ui.radio_value(&mut self.encrypt, false, "Decrypt");
        });
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            ui.label("Key:");
            ui.add(egui::TextEdit::singleline(&mut self.key).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
        });
        ui.horizontal(|ui| {
            ui.label("IV: ");
            ui.add(egui::TextEdit::singleline(&mut self.iv).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_encrypt = self.encrypt;
        let prev_alg = self.algorithm;
        let prev_key = self.key.clone();
        let prev_iv = self.iv.clone();

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                let input_label = if self.encrypt { "Input (plain text):" } else { "Input (Base64):" };
                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = format!("Clipboard error: {}", e),
                        }
                    }
                    if ui.button("Open File...").clicked() {
                        open_file_async(&mut self.pending_file, "Open file", "All", &["*"]);
                    }
                    if ui.button("Clear").clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(input_label);
                egui::ScrollArea::vertical().id_salt("sym_input_scroll").auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut self.input)
                            .desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                    });
            });

            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }
                let output_label = if self.encrypt { "Output (Base64):" } else { "Output (plain text):" };
                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.output.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "output.txt") {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(output_label);
                egui::ScrollArea::vertical().id_salt("sym_output_scroll").auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut self.output)
                            .desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                    });
            });
        });

        if self.input != prev_input || self.encrypt != prev_encrypt
            || self.algorithm != prev_alg || self.key != prev_key || self.iv != prev_iv
        {
            if !self.input.trim().is_empty() {
                self.convert();
            } else {
                self.output.clear();
                self.error.clear();
            }
        }
    }
}

impl SymmetricEncryption {
    fn convert(&mut self) {
        self.error.clear();
        self.output.clear();

        let key_bytes = self.key.as_bytes();
        let iv_bytes = self.iv.as_bytes();
        let expected_key = self.algorithm.key_len();

        if key_bytes.len() > expected_key {
            self.error = format!("Key too long (max {} bytes for {})", expected_key, self.algorithm.label());
            return;
        }
        if iv_bytes.len() > 16 {
            self.error = "IV too long (max 16 bytes)".to_string();
            return;
        }

        let mut key_padded = vec![0u8; expected_key];
        let klen = key_bytes.len().min(expected_key);
        key_padded[..klen].copy_from_slice(&key_bytes[..klen]);

        let mut iv_padded = vec![0u8; 16];
        let ilen = iv_bytes.len().min(16);
        iv_padded[..ilen].copy_from_slice(&iv_bytes[..ilen]);

        let result = match self.algorithm {
            Algorithm::Aes128 => {
                let cipher = aes::Aes128::new_from_slice(&key_padded[..16]).map_err(|e| format!("Key error: {}", e));
                match cipher {
                    Ok(c) => if self.encrypt { cbc_encrypt(&c, &iv_padded, self.input.as_bytes()) } else { cbc_decrypt(&c, &iv_padded, &self.input) },
                    Err(e) => Err(e),
                }
            }
            Algorithm::Aes192 => {
                let cipher = aes::Aes192::new_from_slice(&key_padded[..24]).map_err(|e| format!("Key error: {}", e));
                match cipher {
                    Ok(c) => if self.encrypt { cbc_encrypt(&c, &iv_padded, self.input.as_bytes()) } else { cbc_decrypt(&c, &iv_padded, &self.input) },
                    Err(e) => Err(e),
                }
            }
            Algorithm::Aes256 => {
                let cipher = aes::Aes256::new_from_slice(&key_padded).map_err(|e| format!("Key error: {}", e));
                match cipher {
                    Ok(c) => if self.encrypt { cbc_encrypt(&c, &iv_padded, self.input.as_bytes()) } else { cbc_decrypt(&c, &iv_padded, &self.input) },
                    Err(e) => Err(e),
                }
            }
            Algorithm::Des => {
                let cipher = des::Des::new_from_slice(&key_padded[..8]).map_err(|e| format!("Key error: {}", e));
                match cipher {
                    Ok(c) => if self.encrypt { cbc_encrypt(&c, &iv_padded, self.input.as_bytes()) } else { cbc_decrypt(&c, &iv_padded, &self.input) },
                    Err(e) => Err(e),
                }
            }
        };

        match result {
            Ok(s) => self.output = s,
            Err(e) => self.error = e,
        }
    }
}

// ── Manual CBC + PKCS7 ──────────────────────────────────────────────────────

fn cbc_encrypt<C: BlockEncrypt>(cipher: &C, iv: &[u8], data: &[u8]) -> Result<String, String> {
    let bs = 16;
    let padded = pkcs7_pad(data, bs);
    let mut prev = iv[..bs].to_vec();
    let mut out = Vec::with_capacity(padded.len());

    for chunk in padded.chunks(bs) {
        let mut block = [0u8; 16];
        for i in 0..bs {
            block[i] = chunk[i] ^ prev[i];
        }
        let mut ga = GenericArray::clone_from_slice(&block);
        cipher.encrypt_block(&mut ga);
        out.extend_from_slice(&ga);
        prev = ga.to_vec();
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(&out))
}

fn cbc_decrypt<C: BlockDecrypt>(cipher: &C, iv: &[u8], data: &str) -> Result<String, String> {
    let bs = 16;
    let ciphertext = base64::engine::general_purpose::STANDARD.decode(data.trim())
        .map_err(|e| format!("Base64 error: {}", e))?;

    if ciphertext.len() % bs != 0 || ciphertext.is_empty() {
        return Err("Invalid ciphertext length".to_string());
    }

    let mut prev = iv[..bs].to_vec();
    let mut out = Vec::with_capacity(ciphertext.len());

    for chunk in ciphertext.chunks(bs) {
        let mut ga = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut ga);
        for i in 0..bs {
            out.push(ga[i] ^ prev[i]);
        }
        prev = chunk.to_vec();
    }

    let unpadded = pkcs7_unpad(&out)?;
    String::from_utf8(unpadded).map_err(|e| format!("UTF-8 error: {}", e))
}

fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.resize(data.len() + pad_len, pad_len as u8);
    padded
}

fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("Empty data".to_string());
    }
    let pad_len = *data.last().unwrap() as usize;
    if pad_len == 0 || pad_len > 16 || pad_len > data.len() {
        return Err("Invalid padding".to_string());
    }
    for &b in &data[data.len() - pad_len..] {
        if b as usize != pad_len {
            return Err("Invalid padding".to_string());
        }
    }
    Ok(data[..data.len() - pad_len].to_vec())
}
