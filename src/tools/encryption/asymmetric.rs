use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};

pub struct AsymmetricEncryption {
    input: String,
    output: String,
    error: String,
    encrypt: bool,
    public_key: String,
    private_key: String,
    pending_file: Pending<String>,
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
        }
    }
}

impl Tool for AsymmetricEncryption {
    fn name(&self) -> &str { "非对称加密 (RSA)" }
    fn description(&self) -> &str { "RSA encryption/decryption with public/private keys" }
    fn category(&self) -> ToolCategory { ToolCategory::Encryption }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }

        // Mode selector
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.encrypt, true, "Encrypt (use Public Key)");
            ui.radio_value(&mut self.encrypt, false, "Decrypt (use Private Key)");
        });
        ui.add_space(2.0);

        // Public Key
        ui.label("Public Key (PEM):");
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
        ui.label("Private Key (PEM):");
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
        if ui.button("  Generate Key Pair  ").clicked() {
            self.generate_key_pair();
        }
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_encrypt = self.encrypt;
        let prev_pub = self.public_key.clone();
        let prev_priv = self.private_key.clone();

        ui.columns(2, |cols| {
            // Left: Input
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
                        self.error = format!("PEM encode error: {}", e);
                        return;
                    }
                }
                match public_key.to_public_key_pem(LineEnding::LF) {
                    Ok(pem) => self.public_key = pem,
                    Err(e) => {
                        self.error = format!("PEM encode error: {}", e);
                        return;
                    }
                }
                self.error.clear();
            }
            Err(e) => self.error = format!("Key generation error: {}", e),
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

        let public_key = match rsa::RsaPublicKey::from_public_key_pem(self.public_key.trim()) {
            Ok(pk) => pk,
            Err(e) => {
                self.error = format!("Invalid public key PEM: {}", e);
                return;
            }
        };

        let padding = Oaep::new::<Sha256>();
        let mut rng = rand::thread_rng();
        match public_key.encrypt(&mut rng, padding, self.input.as_bytes()) {
            Ok(ciphertext) => self.output = base64::engine::general_purpose::STANDARD.encode(&ciphertext),
            Err(e) => self.error = format!("Encrypt error: {}", e),
        }
    }

    fn do_decrypt(&mut self) {
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::Oaep;
        use sha2::Sha256;
        use base64::Engine;

        let private_key = match rsa::RsaPrivateKey::from_pkcs8_pem(self.private_key.trim()) {
            Ok(pk) => pk,
            Err(e) => {
                self.error = format!("Invalid private key PEM: {}", e);
                return;
            }
        };

        let ciphertext = match base64::engine::general_purpose::STANDARD.decode(self.input.trim()) {
            Ok(b) => b,
            Err(e) => {
                self.error = format!("Base64 error: {}", e);
                return;
            }
        };

        let padding = Oaep::new::<Sha256>();
        match private_key.decrypt(padding, &ciphertext) {
            Ok(plaintext) => {
                match String::from_utf8(plaintext) {
                    Ok(s) => self.output = s,
                    Err(e) => self.error = format!("UTF-8 error: {}", e),
                }
            }
            Err(e) => self.error = format!("Decrypt error: {}", e),
        }
    }
}
