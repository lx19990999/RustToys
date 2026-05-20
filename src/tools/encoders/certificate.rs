use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use base64::Engine;

#[derive(Default)]
pub struct CertificateDecoder {
    input: String,
    output: String,
    error: String,
}

impl Tool for CertificateDecoder {
    fn name(&self) -> &str { "Certificate Decoder" }
    fn description(&self) -> &str { "Decode PEM/DER certificates and display their properties" }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();

        ui.columns(2, |cols| {
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = format!("Clipboard error: {}", e),
                        }
                    }
                    if ui.button("Open File...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Open certificate")
                            .add_filter("PEM/DER", &["pem", "crt", "cer", "der", "key"])
                            .add_filter("All files", &["*"])
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
                                Err(e) => self.error = format!("File read error: {}", e),
                            }
                        }
                    }
                    if ui.button("Clear").clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label("PEM Certificate:");

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
                    if ui.button("Copy").clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.output.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "cert_info.txt") {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label("Certificate Info:");

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

                info.push_str(&format!("Block: {}\n", block_type));
                info.push_str(&format!("Base64 length: {} chars\n", b64_data.len()));

                match base64::engine::general_purpose::STANDARD.decode(&b64_data) {
                    Ok(der) => {
                        info.push_str(&format!("DER data: {} bytes\n", der.len()));
                        self.parse_x509(&der, &mut info);
                    }
                    Err(e) => info.push_str(&format!("Base64 error: {}\n", e)),
                }
            }
        }

        if block_index == 0 {
            let cleaned: String = pem.chars().filter(|c| !c.is_whitespace()).collect();
            match base64::engine::general_purpose::STANDARD.decode(&cleaned) {
                Ok(der) => {
                    info.push_str("Raw DER certificate\n");
                    info.push_str(&format!("DER data: {} bytes\n", der.len()));
                    self.parse_x509(&der, &mut info);
                }
                Err(_) => {
                    self.error = "No PEM block found. Expected -----BEGIN ...----- header.".into();
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
                info.push_str("\n--- X.509 Certificate ---\n");
                info.push_str(&format!("Subject: {}\n", format_name(cert.subject())));
                info.push_str(&format!("Issuer:  {}\n", format_name(cert.issuer())));
                info.push_str(&format!("Serial:  {}\n", cert.raw_serial_as_string()));

                let vf = cert.validity().not_before.to_rfc2822().unwrap_or_default();
                let vt = cert.validity().not_after.to_rfc2822().unwrap_or_default();
                info.push_str(&format!("Valid:   {} - {}\n", vf, vt));

                if let Some(dur) = cert.validity().time_to_expiration() {
                    let days = dur.whole_seconds() / 86400;
                    info.push_str(&format!("Status:  Valid ({} days remaining)\n", days));
                } else {
                    info.push_str("Status:  EXPIRED\n");
                }

                info.push_str(&format!("Version: v{}\n", cert.version().0 + 1));

                match cert.public_key().parsed() {
                    Ok(pk) => {
                        use x509_parser::public_key::PublicKey;
                        let alg = match pk {
                            PublicKey::RSA(k) => format!("RSA ({} bits)", k.key_size() * 8),
                            PublicKey::EC(_) => "EC".into(),
                            PublicKey::DSA(_) => "DSA".into(),
                            PublicKey::GostR3410(_) => "GOST R 34.10-94".into(),
                            PublicKey::GostR3410_2012(_) => "GOST R 34.10-2012".into(),
                            PublicKey::Unknown(_) => "Unknown".into(),
                        };
                        info.push_str(&format!("PubKey:  {}\n", alg));
                    }
                    Err(e) => info.push_str(&format!("PubKey:  parse error: {}\n", e)),
                }

                info.push_str(&format!("SigAlg:  {}\n", oid_label(&cert.signature_algorithm.algorithm)));

                let exts = cert.extensions();
                if !exts.is_empty() {
                    info.push_str(&format!("\nExtensions ({}):\n", exts.len()));
                    for ext in exts {
                        let crit = if ext.critical { " [CRITICAL]" } else { "" };
                        info.push_str(&format!("  - {}{}\n", oid_label(&ext.oid), crit));
                    }
                }
            }
            Err(e) => info.push_str(&format!("\nX.509 parse error: {}\n", e)),
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
        "2.5.4.3" => "CN (Common Name)".into(),
        "2.5.4.6" => "C (Country)".into(),
        "2.5.4.7" => "L (Locality)".into(),
        "2.5.4.8" => "ST (State)".into(),
        "2.5.4.10" => "O (Organization)".into(),
        "2.5.4.11" => "OU (Org Unit)".into(),
        "1.2.840.113549.1.1.1" => "RSA Encryption".into(),
        "1.2.840.113549.1.1.5" => "SHA-1 with RSA".into(),
        "1.2.840.113549.1.1.11" => "SHA-256 with RSA".into(),
        "1.2.840.113549.1.1.12" => "SHA-384 with RSA".into(),
        "1.2.840.113549.1.1.13" => "SHA-512 with RSA".into(),
        "1.2.840.113549.1.1.10" => "RSASSA-PSS".into(),
        "1.2.840.10045.2.1" => "EC Public Key".into(),
        "1.2.840.10045.4.3.2" => "SHA-256 with ECDSA".into(),
        "1.2.840.10045.4.3.3" => "SHA-384 with ECDSA".into(),
        "1.2.840.10045.4.3.4" => "SHA-512 with ECDSA".into(),
        "2.5.29.14" => "Subject Key Identifier".into(),
        "2.5.29.15" => "Key Usage".into(),
        "2.5.29.17" => "Subject Alt Name".into(),
        "2.5.29.19" => "Basic Constraints".into(),
        "2.5.29.31" => "CRL Distribution Points".into(),
        "2.5.29.32" => "Certificate Policies".into(),
        "2.5.29.35" => "Authority Key Identifier".into(),
        "2.5.29.37" => "Extended Key Usage".into(),
        "1.3.6.1.5.5.7.1.1" => "Authority Info Access".into(),
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
