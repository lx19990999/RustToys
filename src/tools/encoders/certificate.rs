use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, save_file_async};
use base64::Engine;

pub struct CertificateDecoder {
    input: String,
    output: String,
    error: String,
    save_result: String,
    pending_file: Pending<String>,
}

impl Default for CertificateDecoder {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            save_result: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl Tool for CertificateDecoder {
    fn name(&self) -> String { tr!("cert_name") }
    fn description(&self) -> String { tr!("cert_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            self.save_result = text;
        }

        let prev_input = self.input.clone();

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title(&tr!("cert_open_title"))
                            .add_filter(&tr!("cert_filter"), &["pem", "crt", "cer", "der", "key"])
                            .add_filter(&tr!("save_filter_all"), &["*"])
                            .pick_file()
                        {
                            match std::fs::read(&path) {
                                Ok(bytes) => {
                                    if let Ok(text) = std::str::from_utf8(&bytes) {
                                        if text.contains("-----BEGIN") {
                                            self.input = text.to_string();
                                        } else {
                                            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                            self.input = format!("-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
                                                wrap_lines(&b64, 64));
                                        }
                                    } else {
                                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                        self.input = format!("-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
                                            wrap_lines(&b64, 64));
                                    }
                                }
                                Err(e) => self.error = tr!("err_file_read", e),
                            }
                        }
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                        self.save_result.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("cert_input_label"));

                egui::ScrollArea::vertical()
                    .id_salt("cert_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        save_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("cert_save_default"), self.output.clone());
                    }
                });
                if !self.save_result.is_empty() {
                    ui.colored_label(egui::Color32::GREEN, &self.save_result);
                }
                ui.add_space(2.0);
                ui.label(tr!("cert_output_label"));

                egui::ScrollArea::vertical()
                    .id_salt("cert_output_scroll")
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

        if self.input != prev_input {
            self.decode();
        }
    }
}

impl CertificateDecoder {
    fn decode(&mut self) {
        self.error.clear();
        self.output.clear();

        let pem = self.input.trim();
        if pem.is_empty() { return; }

        let mut info = String::new();
        let mut block_index = 0;

        let mut lines = pem.lines().peekable();
        while let Some(line) = lines.next() {
            if let Some(block_type) = extract_block_header(line, "-----BEGIN ") {
                block_index += 1;
                if block_index > 1 {
                    info.push_str(&format!("\n{}\n\n", "=".repeat(60)));
                }

                let mut b64_data = String::new();
                for inner_line in lines.by_ref() {
                    if extract_block_header(inner_line, "-----END ").is_some() { break; }
                    let t = inner_line.trim();
                    if !t.is_empty() { b64_data.push_str(t); }
                }

                info.push_str(&tr!("cert_block", block_type));
                info.push('\n');
                info.push_str(&tr!("cert_base64_len", b64_data.len()));
                info.push('\n');

                match base64::engine::general_purpose::STANDARD.decode(&b64_data) {
                    Ok(der) => {
                        info.push_str(&tr!("cert_der_data", der.len()));
                        info.push('\n');
                        self.parse_x509(&der, &mut info);
                    }
                    Err(e) => {
                        info.push_str(&tr!("cert_base64_error", e));
                        info.push('\n');
                    }
                }
            }
        }

        if block_index == 0 {
            let cleaned: String = pem.chars().filter(|c| !c.is_whitespace()).collect();
            match base64::engine::general_purpose::STANDARD.decode(&cleaned) {
                Ok(der) => {
                    info.push_str(&tr!("cert_raw_der"));
                    info.push('\n');
                    info.push_str(&tr!("cert_der_data", der.len()));
                    info.push('\n');
                    self.parse_x509(&der, &mut info);
                }
                Err(_) => {
                    self.error = tr!("cert_no_pem");
                    return;
                }
            }
        }

        self.output = info;
    }

    fn parse_x509(&self, der: &[u8], info: &mut String) {
        use x509_parser::prelude::FromDer;
        match x509_parser::certificate::X509Certificate::from_der(der) {
            Ok((_, cert)) => {
                info.push_str(&tr!("cert_x509_header"));
                info.push('\n');
                info.push_str(&tr!("cert_subject", format_name(cert.subject())));
                info.push('\n');
                info.push_str(&tr!("cert_issuer", format_name(cert.issuer())));
                info.push('\n');
                info.push_str(&tr!("cert_serial", cert.raw_serial_as_string()));
                info.push('\n');

                let vf = cert.validity().not_before.to_rfc2822().unwrap_or_default();
                let vt = cert.validity().not_after.to_rfc2822().unwrap_or_default();
                info.push_str(&tr!("cert_valid", vf, vt));
                info.push('\n');

                if let Some(dur) = cert.validity().time_to_expiration() {
                    let days = dur.whole_seconds() / 86400;
                    info.push_str(&tr!("cert_status_valid", days));
                } else {
                    info.push_str(&tr!("cert_status_expired"));
                }
                info.push('\n');

                info.push_str(&tr!("cert_version", cert.version().0 + 1));
                info.push('\n');

                match cert.public_key().parsed() {
                    Ok(pk) => {
                        use x509_parser::public_key::PublicKey;
                        let alg = match pk {
                            PublicKey::RSA(k) => tr!("pktype_rsa", k.key_size() * 8),
                            PublicKey::EC(_) => tr!("pktype_ec"),
                            PublicKey::DSA(_) => tr!("pktype_dsa"),
                            PublicKey::GostR3410(_) => "GOST R 34.10-94".into(),
                            PublicKey::GostR3410_2012(_) => "GOST R 34.10-2012".into(),
                            PublicKey::Unknown(_) => tr!("pktype_unknown"),
                        };
                        info.push_str(&tr!("cert_pubkey", alg));
                        info.push('\n');
                    }
                    Err(e) => {
                        info.push_str(&tr!("cert_pubkey_error", e));
                        info.push('\n');
                    }
                }

                info.push_str(&tr!("cert_sigalg", oid_label(&cert.signature_algorithm.algorithm)));
                info.push('\n');

                let exts = cert.extensions();
                if !exts.is_empty() {
                    info.push_str(&tr!("cert_extensions", exts.len()));
                    info.push('\n');
                    for ext in exts {
                        let crit = if ext.critical { tr!("cert_critical") } else { String::new() };
                        info.push_str(&format!("  - {}{}\n", oid_label(&ext.oid), crit));
                    }
                }
            }
            Err(e) => {
                info.push_str(&tr!("cert_parse_error", e));
                info.push('\n');
            }
        }
    }
}

fn extract_block_header<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    line.find(prefix).map(|pos| {
        let rest = &line[pos + prefix.len()..];
        rest.trim_end_matches('-').trim()
    })
}

fn format_name(name: &x509_parser::x509::X509Name) -> String {
    let mut parts = Vec::new();
    for rdn in name.iter_rdn() {
        for attr in rdn.iter() {
            if let Ok(s) = attr.attr_value().as_str() {
                parts.push(format!("{}={}", oid_short(&attr.attr_type()), s));
            }
        }
    }
    if parts.is_empty() { format!("{}", name) } else { parts.join(", ") }
}

fn oid_short(oid: &x509_parser::oid_registry::Oid) -> &'static str {
    let s = oid.to_id_string();
    match s.as_str() {
        "2.5.4.3" => "CN",
        "2.5.4.6" => "C",
        "2.5.4.7" => "L",
        "2.5.4.8" => "ST",
        "2.5.4.10" => "O",
        "2.5.4.11" => "OU",
        _ => "",
    }
}

fn oid_label(oid: &x509_parser::oid_registry::Oid) -> String {
    let s = oid.to_id_string();
    match s.as_str() {
        "2.5.4.3" => tr!("oid_cn"),
        "2.5.4.6" => tr!("oid_c"),
        "2.5.4.7" => tr!("oid_l"),
        "2.5.4.8" => tr!("oid_st"),
        "2.5.4.10" => tr!("oid_o"),
        "2.5.4.11" => tr!("oid_ou"),
        "1.2.840.113549.1.1.1" => tr!("oid_rsa_enc"),
        "1.2.840.113549.1.1.5" => tr!("oid_sha1_rsa"),
        "1.2.840.113549.1.1.11" => tr!("oid_sha256_rsa"),
        "1.2.840.113549.1.1.12" => tr!("oid_sha384_rsa"),
        "1.2.840.113549.1.1.13" => tr!("oid_sha512_rsa"),
        "1.2.840.113549.1.1.10" => tr!("oid_rsassa_pss"),
        "1.2.840.10045.2.1" => tr!("oid_ec_pubkey"),
        "1.2.840.10045.4.3.2" => tr!("oid_sha256_ecdsa"),
        "1.2.840.10045.4.3.3" => tr!("oid_sha384_ecdsa"),
        "1.2.840.10045.4.3.4" => tr!("oid_sha512_ecdsa"),
        "2.5.29.14" => tr!("oid_ski"),
        "2.5.29.15" => tr!("oid_ku"),
        "2.5.29.17" => tr!("oid_san"),
        "2.5.29.19" => tr!("oid_bc"),
        "2.5.29.31" => tr!("oid_crl_dp"),
        "2.5.29.32" => tr!("oid_cp"),
        "2.5.29.35" => tr!("oid_aki"),
        "2.5.29.37" => tr!("oid_eku"),
        "1.3.6.1.5.5.7.1.1" => tr!("oid_aia"),
        other => other.into(),
    }
}

fn wrap_lines(s: &str, width: usize) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && i % width == 0 { result.push('\n'); }
        result.push(c);
    }
    result
}
