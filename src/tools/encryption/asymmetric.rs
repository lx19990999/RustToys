use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};

pub struct AsymmetricEncryption {
    input: String,
    output: String,
    error: String,
    encrypt: bool,
    public_key: String,
    private_key: String,
    pending_file: Pending<String>,
    pending_pub_file: Pending<String>,
    pending_priv_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for AsymmetricEncryption {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            encrypt: true,
            public_key: String::new(),
            private_key: String::new(),
            pending_file: Pending::default(),
            pending_pub_file: Pending::default(),
            pending_priv_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Tool for AsymmetricEncryption {
    fn name(&self) -> String { tr!("asym_name") }
    fn description(&self) -> String { tr!("asym_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encryption }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let err_prefix = tr!("err_error_reading");
        let prev_input = self.input.clone();
        let prev_encrypt = self.encrypt;
        let prev_pub = self.public_key.clone();
        let prev_priv = self.private_key.clone();

        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            crate::tools::async_utils::open_dropped_text_async(&mut self.pending_file, path);
        }
        if let Some(text) = self.pending_file.poll() {
            if text.starts_with(&err_prefix) {
                self.error = text;
            } else {
                self.input = text;
            }
        }
        if let Some(text) = self.pending_pub_file.poll() {
            if text.starts_with(&err_prefix) {
                self.error = text;
            } else {
                self.public_key = text;
            }
        }
        if let Some(text) = self.pending_priv_file.poll() {
            if text.starts_with(&err_prefix) {
                self.error = text;
            } else {
                self.private_key = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }

        // Mode selector
        ui.horizontal(|ui| {
            let label_encrypt_pub = tr!("asym_encrypt_pub");
            let label_decrypt_priv = tr!("asym_decrypt_priv");
            ui.radio_value(&mut self.encrypt, true, &label_encrypt_pub);
            ui.radio_value(&mut self.encrypt, false, &label_decrypt_priv);
        });
        ui.add_space(2.0);

        // Public Key
        ui.label(tr!("asym_public_key"));
        ui.horizontal(|ui| {
            if ui.button(tr!("btn_paste")).clicked() {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    Ok(text) => self.public_key = text,
                    Err(e) => self.error = tr!("err_clipboard", e),
                }
            }
            if ui.button(tr!("btn_open_file")).clicked() {
                open_file_async(&mut self.pending_pub_file, &tr!("asym_open_pub"), "PEM", &["pem", "pub", "key"]);
            }
            if ui.button(tr!("btn_clear")).clicked() {
                self.public_key.clear();
            }
            if ui.button(tr!("btn_copy")).clicked() && !self.public_key.is_empty() {
                ui.ctx().copy_text(self.public_key.clone());
            }
            if ui.button(tr!("btn_save_as")).clicked() && !self.public_key.is_empty() {
                save_file_async(&mut self.save_pending, &tr!("asym_save_pub"), "PEM", &["pem"], &tr!("asym_pub_pem"), self.public_key.clone());
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("rsa_pub_scroll")
            .max_height(60.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.public_key)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                );
            });

        // Private Key
        ui.label(tr!("asym_private_key"));
        ui.horizontal(|ui| {
            if ui.button(tr!("btn_paste")).clicked() {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    Ok(text) => self.private_key = text,
                    Err(e) => self.error = tr!("err_clipboard", e),
                }
            }
            if ui.button(tr!("btn_open_file")).clicked() {
                open_file_async(&mut self.pending_priv_file, &tr!("asym_open_priv"), "PEM", &["pem", "key"]);
            }
            if ui.button(tr!("btn_clear")).clicked() {
                self.private_key.clear();
            }
            if ui.button(tr!("btn_copy")).clicked() && !self.private_key.is_empty() {
                ui.ctx().copy_text(self.private_key.clone());
            }
            if ui.button(tr!("btn_save_as")).clicked() && !self.private_key.is_empty() {
                save_file_async(&mut self.save_pending, &tr!("asym_save_priv"), "PEM", &["pem"], &tr!("asym_priv_pem"), self.private_key.clone());
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("rsa_priv_scroll")
            .max_height(60.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.private_key)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                );
            });

        // Generate button
        if ui.button(tr!("asym_gen_keypair")).clicked() {
            self.generate_key_pair();
        }
        ui.add_space(4.0);

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                let input_label = if self.encrypt { tr!("sym_input_plain") } else { tr!("sym_input_b64") };

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, "Open file", "All", &["*"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(input_label);

                egui::ScrollArea::vertical()
                    .id_salt("rsa_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Output
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                let output_label = if self.encrypt { tr!("sym_output_b64") } else { tr!("sym_output_plain") };

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        save_file_async(&mut self.save_pending, &tr!("save_as_title"), "Text", &["txt"], &tr!("default_output_txt"), self.output.clone());
                    }
                });
                ui.add_space(2.0);
                ui.label(output_label);

                egui::ScrollArea::vertical()
                    .id_salt("rsa_output_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.output)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
        });

        if self.input != prev_input || self.encrypt != prev_encrypt
            || self.public_key != prev_pub || self.private_key != prev_priv
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

impl AsymmetricEncryption {
    fn generate_key_pair(&mut self) {
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};

        let mut rng = rand::thread_rng();
        match rsa::RsaPrivateKey::new(&mut rng, 2048) {
            Ok(private_key) => {
                let public_key = private_key.to_public_key();
                match private_key.to_pkcs8_pem(LineEnding::LF) {
                    Ok(pem) => self.private_key = pem.to_string(),
                    Err(e) => {
                        self.error = tr!("asym_pem_encode_error", e);
                        return;
                    }
                }
                match public_key.to_public_key_pem(LineEnding::LF) {
                    Ok(pem) => self.public_key = pem,
                    Err(e) => {
                        self.error = tr!("asym_pem_encode_error", e);
                        return;
                    }
                }
                self.error.clear();
            }
            Err(e) => self.error = tr!("asym_key_gen_error", e),
        }
    }

    fn convert(&mut self) {
        self.error.clear();
        self.output.clear();

        if self.encrypt {
            self.do_encrypt();
        } else {
            self.do_decrypt();
        }
    }

    fn do_encrypt(&mut self) {
        use rsa::pkcs8::DecodePublicKey;
        use rsa::Oaep;
        use sha2::Sha256;
        use base64::Engine;
        use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, AeadCore}};
        use aes_gcm::aead::Nonce;

        let public_key = match rsa::RsaPublicKey::from_public_key_pem(self.public_key.trim()) {
            Ok(pk) => pk,
            Err(e) => {
                self.error = tr!("asym_invalid_pub", e);
                return;
            }
        };

        // Generate random AES-256 key and nonce
        let mut rng = rand::thread_rng();
        let aes_key = Aes256Gcm::generate_key(&mut rng);
        let nonce_bytes = Aes256Gcm::generate_nonce(&mut rng);
        let nonce = Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);

        // Encrypt data with AES-256-GCM
        let cipher = match Aes256Gcm::new_from_slice(&aes_key) {
            Ok(c) => c,
            Err(e) => {
                self.error = tr!("asym_aes_init_error", e);
                return;
            }
        };
        let aes_ciphertext = match cipher.encrypt(nonce, self.input.as_bytes()) {
            Ok(ct) => ct,
            Err(e) => {
                self.error = tr!("asym_aes_encrypt_error", e);
                return;
            }
        };

        // Encrypt AES key with RSA-OAEP
        let padding = Oaep::new::<Sha256>();
        let encrypted_key = match public_key.encrypt(&mut rng, padding, &aes_key) {
            Ok(k) => k,
            Err(e) => {
                self.error = tr!("asym_encrypt_error", e);
                return;
            }
        };

        // Format: [encrypted_key_len:4 bytes BE][encrypted_key][nonce:12 bytes][aes_ciphertext]
        let mut output = Vec::new();
        let key_len = encrypted_key.len() as u32;
        output.extend_from_slice(&key_len.to_be_bytes());
        output.extend_from_slice(&encrypted_key);
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&aes_ciphertext);

        self.output = base64::engine::general_purpose::STANDARD.encode(&output);
    }

    fn do_decrypt(&mut self) {
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::Oaep;
        use sha2::Sha256;
        use base64::Engine;
        use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
        use aes_gcm::aead::Nonce;

        let private_key = match rsa::RsaPrivateKey::from_pkcs8_pem(self.private_key.trim()) {
            Ok(pk) => pk,
            Err(e) => {
                self.error = tr!("asym_invalid_priv", e);
                return;
            }
        };

        let data = match base64::engine::general_purpose::STANDARD.decode(self.input.trim()) {
            Ok(b) => b,
            Err(e) => {
                self.error = tr!("asym_base64_error", e);
                return;
            }
        };

        // Parse format: [encrypted_key_len:4 bytes BE][encrypted_key][nonce:12 bytes][aes_ciphertext]
        if data.len() < 4 {
            self.error = tr!("asym_hybrid_too_short");
            return;
        }
        let key_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + key_len + 12 {
            self.error = tr!("asym_hybrid_truncated");
            return;
        }
        let encrypted_key = &data[4..4 + key_len];
        let nonce_bytes = &data[4 + key_len..4 + key_len + 12];
        let aes_ciphertext = &data[4 + key_len + 12..];
        let nonce = Nonce::<Aes256Gcm>::from_slice(nonce_bytes);

        // Decrypt AES key with RSA-OAEP
        let padding = Oaep::new::<Sha256>();
        let aes_key = match private_key.decrypt(padding, encrypted_key) {
            Ok(k) => k,
            Err(e) => {
                self.error = tr!("asym_decrypt_error", e);
                return;
            }
        };

        // Decrypt data with AES-256-GCM
        let cipher = match Aes256Gcm::new_from_slice(&aes_key) {
            Ok(c) => c,
            Err(e) => {
                self.error = tr!("asym_aes_init_error", e);
                return;
            }
        };
        let plaintext = match cipher.decrypt(nonce, aes_ciphertext) {
            Ok(pt) => pt,
            Err(e) => {
                self.error = tr!("asym_aes_decrypt_error", e);
                return;
            }
        };

        match String::from_utf8(plaintext) {
            Ok(s) => self.output = s,
            Err(e) => self.error = tr!("asym_utf8_error", e),
        }
    }
}
