use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};
use serde_json::Value;
use base64::Engine;


pub struct JwtDecoder {
    // Decode mode
    token: String,
    header: String,
    payload: String,
    error: String,
    // Encode mode
    encode_input: String,
    encode_secret: String,
    encode_output: String,
    // Shared
    encode_mode: bool,
    // Verification
    verify_secret: String,
    verify_result: String,
    verify_valid: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for JwtDecoder {
    fn default() -> Self {
        Self {
            token: String::new(),
            header: String::new(),
            payload: String::new(),
            error: String::new(),
            encode_input: String::new(),
            encode_secret: String::new(),
            encode_output: String::new(),
            encode_mode: false,
            verify_secret: String::new(),
            verify_result: String::new(),
            verify_valid: false,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}


impl Tool for JwtDecoder {
    fn name(&self) -> String { tr!("jwt_name") }
    fn description(&self) -> String { tr!("jwt_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.token = text;
                if !self.encode_mode {
                    self.decode();
                }
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        let label_decode = tr!("label_decode");
        let label_encode = tr!("label_encode");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.encode_mode, false, &label_decode);
            ui.radio_value(&mut self.encode_mode, true, &label_encode);
        });
        ui.add_space(4.0);

        if self.encode_mode {
            self.ui_encode(ui);
        } else {
            self.ui_decode(ui);
        }
    }
}

impl JwtDecoder {
    fn ui_decode(&mut self, ui: &mut egui::Ui) {
        let prev_token = self.token.clone();
        let prev_secret = self.verify_secret.clone();

        // Input area
        ui.horizontal(|ui| {
            if ui.button(tr!("btn_paste")).clicked() {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    Ok(text) => self.token = text,
                    Err(e) => self.error = tr!("err_clipboard", e),
                }
            }
            if ui.button(tr!("btn_open_file")).clicked() {
                    open_file_async(&mut self.pending_file, &tr!("jwt_save_token"), &tr!("save_filter_text"), &["txt"]);
            }
            if ui.button(tr!("btn_clear")).clicked() {
                self.token.clear();
                self.header.clear();
                self.payload.clear();
                self.error.clear();
                self.verify_result.clear();
                self.verify_valid = false;
            }
        });
        ui.add_space(2.0);
        ui.label(tr!("jwt_token_label"));

        egui::ScrollArea::vertical()
            .id_salt("jwt_token_scroll")
            .max_height(100.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.token)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace),
                );
            });

        ui.add_space(4.0);

        // Verification settings
        ui.collapsing(tr!("jwt_verification"), |ui| {
            ui.horizontal(|ui| {
                ui.label(tr!("jwt_secret_label"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.verify_secret)
                        .desired_width(f32::INFINITY)
                        .password(true),
                );
            });
            ui.label(tr!("jwt_skip_verify_hint"));
            if !self.verify_result.is_empty() {
                if self.verify_valid {
                    ui.colored_label(egui::Color32::from_rgb(0, 180, 0), &self.verify_result);
                } else {
                    ui.colored_label(egui::Color32::RED, &self.verify_result);
                }
            }
        });

        ui.add_space(4.0);

        // Error display
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::RED, &self.error);
            ui.add_space(4.0);
        }

        // Output: Header and Payload
        ui.columns(2, |cols| {
            // Left: Header
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr!("jwt_header_label"));
                    if !self.header.is_empty() && ui.small_button(tr!("btn_copy")).clicked() {
                        ui.ctx().copy_text(self.header.clone());
                    }
                    if !self.header.is_empty() && ui.small_button(tr!("btn_save_as")).clicked() {
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("jwt_save_header"), "JSON", &["json"], &tr!("jwt_header_json"), self.header.clone());
                    }
                });

                egui::ScrollArea::vertical()
                    .id_salt("jwt_header_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.header)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Payload
            cols[1].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr!("jwt_payload_label"));
                    if !self.payload.is_empty() && ui.small_button(tr!("btn_copy")).clicked() {
                        ui.ctx().copy_text(self.payload.clone());
                    }
                    if !self.payload.is_empty() && ui.small_button(tr!("btn_save_as")).clicked() {
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("jwt_save_payload"), "JSON", &["json"], &tr!("jwt_payload_json"), self.payload.clone());
                    }
                });

                egui::ScrollArea::vertical()
                    .id_salt("jwt_payload_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.payload)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
        });

        // Auto-convert
        if self.token != prev_token || self.verify_secret != prev_secret {
            self.decode();
        }
    }

    fn ui_encode(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.encode_input.clone();
        let prev_secret = self.encode_secret.clone();

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.encode_input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("jwt_save_token"), &tr!("save_filter_text"), &["txt"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.encode_input.clear();
                        self.encode_output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("jwt_encode_payload"));

                egui::ScrollArea::vertical()
                    .id_salt("jwt_encode_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.encode_input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });

                ui.add_space(4.0);
                ui.label(tr!("jwt_alg_label"));
                ui.horizontal(|ui| {
                    ui.label(tr!("jwt_secret_label"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.encode_secret)
                            .desired_width(f32::INFINITY)
                            .password(true),
                    );
                });
            });

            // Right: Output
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.encode_output.is_empty() {
                        ui.ctx().copy_text(self.encode_output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.encode_output.is_empty() {
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), "JWT", &["jwt"], &tr!("jwt_save_token"), self.encode_output.clone());
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("jwt_encoded_label"));

                egui::ScrollArea::vertical()
                    .id_salt("jwt_encode_output_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.encode_output)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
        });

        // Auto-convert
        if self.encode_input != prev_input || self.encode_secret != prev_secret {
            self.encode();
        }
    }

    fn decode(&mut self) {
        self.error.clear();
        self.header.clear();
        self.payload.clear();
        self.verify_result.clear();
        self.verify_valid = false;

        let trimmed = self.token.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let parts: Vec<String> = trimmed.split('.').map(String::from).collect();
        if parts.len() < 2 {
            self.error = tr!("jwt_invalid");
            return;
        }

        // Decode header
        match base64_decode_url_safe(&parts[0]) {
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(val) => self.header = serde_json::to_string_pretty(&val).unwrap(),
                Err(e) => self.error = tr!("jwt_header_parse_error", e),
            },
            Err(e) => self.error = tr!("jwt_header_decode_error", e),
        }

        // Decode payload
        match base64_decode_url_safe(&parts[1]) {
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(val) => self.payload = serde_json::to_string_pretty(&val).unwrap(),
                Err(e) => {
                    if self.error.is_empty() {
                        self.error = tr!("jwt_payload_parse_error", e);
                    }
                }
            },
            Err(e) => {
                if self.error.is_empty() {
                    self.error = tr!("jwt_payload_decode_error", e);
                }
            }
        }

        // Signature verification
        if !self.verify_secret.is_empty() {
            if parts.len() < 3 {
                self.verify_result = tr!("jwt_no_sig");
                self.verify_valid = false;
            } else {
                self.verify_signature(&parts[0], &parts[1], &parts[2]);
            }
        }
    }

    fn verify_signature(&mut self, header_b64: &str, payload_b64: &str, signature_b64: &str) {
        use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

        // Read algorithm from header
        let alg = match base64_decode_url_safe(header_b64) {
            Ok(bytes) => {
                match serde_json::from_slice::<Value>(&bytes) {
                    Ok(val) => match val.get("alg").and_then(|a| a.as_str()) {
                        Some("HS256") => Algorithm::HS256,
                        Some("HS384") => Algorithm::HS384,
                        Some("HS512") => Algorithm::HS512,
                        Some("RS256") => Algorithm::RS256,
                        Some("RS384") => Algorithm::RS384,
                        Some("RS512") => Algorithm::RS512,
                        Some("ES256") => Algorithm::ES256,
                        Some("ES384") => Algorithm::ES384,
                        Some("PS256") => Algorithm::PS256,
                        Some("PS384") => Algorithm::PS384,
                        Some("PS512") => Algorithm::PS512,
                        Some(other) => {
                            self.verify_result = tr!("jwt_unsupported_alg", other);
                            self.verify_valid = false;
                            return;
                        }
                        None => {
                            self.verify_result = tr!("jwt_no_alg");
                            self.verify_valid = false;
                            return;
                        }
                    },
                    Err(_) => {
                        self.verify_result = tr!("jwt_cannot_parse_alg");
                        self.verify_valid = false;
                        return;
                    }
                }
            }
            Err(_) => {
                self.verify_result = tr!("jwt_cannot_decode_alg");
                self.verify_valid = false;
                return;
            }
        };

        let mut validation = Validation::new(alg);
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.required_spec_claims.clear();
        validation.validate_aud = false;

        let token = format!("{}.{}.{}", header_b64, payload_b64, signature_b64);
        let key = DecodingKey::from_secret(self.verify_secret.as_bytes());

        match decode::<Value>(&token, &key, &validation) {
            Ok(_) => {
                self.verify_result = tr!("jwt_sig_valid");
                self.verify_valid = true;
            }
            Err(e) => {
                self.verify_result = tr!("jwt_verify_failed", e);
                self.verify_valid = false;
            }
        }
    }

    fn encode(&mut self) {
        self.error.clear();
        self.encode_output.clear();

        if self.encode_input.trim().is_empty() {
            return;
        }

        let payload: Value = match serde_json::from_str(&self.encode_input) {
            Ok(v) => v,
            Err(e) => {
                self.error = tr!("jwt_invalid_json", e);
                return;
            }
        };

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&header).unwrap());
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());

        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<sha2::Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.encode_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(format!("{}.{}", header_b64, payload_b64).as_bytes());
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(mac.finalize().into_bytes());

        self.encode_output = format!("{}.{}.{}", header_b64, payload_b64, signature);
    }
}

fn base64_decode_url_safe(input: &str) -> Result<Vec<u8>, String> {
    let padded = match input.len() % 4 {
        2 => format!("{}==", input),
        3 => format!("{}=", input),
        _ => input.to_string(),
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&padded)
        .map_err(|e| e.to_string())
}
