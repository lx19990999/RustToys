use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};


pub struct NumberBase {
    decimal: String,
    binary: String,
    octal: String,
    hex: String,
    error: String,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
    pending_field: Option<String>,
}

impl Default for NumberBase {
    fn default() -> Self {
        Self {
            decimal: String::new(),
            binary: String::new(),
            octal: String::new(),
            hex: String::new(),
            error: String::new(),
            pending_file: Pending::default(),
            save_pending: Pending::default(),
            pending_field: None,
        }
    }
}


enum PendingAction {
    Paste(&'static str),
    Open(&'static str),
    Clear,
}

impl Tool for NumberBase {
    fn name(&self) -> String { tr!("nb_name") }
    fn description(&self) -> String { tr!("nb_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                if let Some(ref field_name) = self.pending_field.take() {
                    let val = text.trim().to_string();
                    match field_name.as_str() {
                        "decimal" => { self.decimal = val; self.update_from("decimal"); }
                        "binary" => { self.binary = val; self.update_from("binary"); }
                        "octal" => { self.octal = val; self.update_from("octal"); }
                        "hex" => { self.hex = val; self.update_from("hex"); }
                        _ => {}
                    }
                }
            }
        }
        self.error.clear();
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        let mut pending: Option<PendingAction> = None;
        let mut error_msg = String::new();

        // Decimal
        let mut dec_changed = false;
        ui.horizontal(|ui| {
            ui.label(tr!("nb_decimal"));
            if ui.text_edit_singleline(&mut self.decimal).changed() { dec_changed = true; }
            Self::buttons(ui, &mut self.save_pending, &self.decimal, "decimal", &mut pending, &mut error_msg);
        });
        if let Ok(n) = self.decimal.trim().parse::<i64>() {
            ui.horizontal(|ui| { ui.label(tr!("nb_formatted")); ui.monospace(format_number(n)); });
        }

        // Binary
        let mut bin_changed = false;
        ui.horizontal(|ui| {
            ui.label(tr!("nb_binary"));
            if ui.text_edit_singleline(&mut self.binary).changed() { bin_changed = true; }
            Self::buttons(ui, &mut self.save_pending, &self.binary, "binary", &mut pending, &mut error_msg);
        });

        // Octal
        let mut oct_changed = false;
        ui.horizontal(|ui| {
            ui.label(tr!("nb_octal"));
            if ui.text_edit_singleline(&mut self.octal).changed() { oct_changed = true; }
            Self::buttons(ui, &mut self.save_pending, &self.octal, "octal", &mut pending, &mut error_msg);
        });

        // Hex
        let mut hex_changed = false;
        ui.horizontal(|ui| {
            ui.label(tr!("nb_hex"));
            if ui.text_edit_singleline(&mut self.hex).changed() { hex_changed = true; }
            Self::buttons(ui, &mut self.save_pending, &self.hex, "hex", &mut pending, &mut error_msg);
        });

        if !error_msg.is_empty() {
            self.error = error_msg;
        }
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::RED, &self.error);
        }

        // Apply pending actions after UI rendering
        match pending {
            Some(PendingAction::Paste(field)) => {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    Ok(text) => {
                        *self.field_mut(field) = text.trim().to_string();
                        self.update_from(field);
                    }
                    Err(e) => self.error = tr!("err_clipboard", e),
                }
            }
            Some(PendingAction::Open(field)) => {
                self.pending_field = Some(field.to_string());
                open_file_async(&mut self.pending_file, &tr!("nb_open"), &tr!("save_filter_text"), &["txt"]);
            }
            Some(PendingAction::Clear) => {
                self.decimal.clear();
                self.binary.clear();
                self.octal.clear();
                self.hex.clear();
            }
            None => {}
        }

        // Apply text edit changes
        if dec_changed { self.update_from("decimal"); }
        if bin_changed { self.update_from("binary"); }
        if oct_changed { self.update_from("octal"); }
        if hex_changed { self.update_from("hex"); }
    }
}

impl NumberBase {
    fn buttons(
        ui: &mut egui::Ui,
        save_pending: &mut Pending<String>,
        value: &str,
        field: &'static str,
        pending: &mut Option<PendingAction>,
        error_msg: &mut String,
    ) {
        if ui.small_button(tr!("btn_copy")).clicked() && !value.is_empty() {
            ui.ctx().copy_text(value.to_string());
        }
        if ui.small_button(tr!("btn_paste")).clicked() {
            *pending = Some(PendingAction::Paste(field));
        }
        if ui.small_button(tr!("nb_open")).clicked() {
            *pending = Some(PendingAction::Open(field));
        }
        if ui.small_button(tr!("nb_save")).clicked() && !value.is_empty() {
            save_file_async(save_pending, &tr!("nb_save_as"), &tr!("save_filter_text"), &["txt"], &tr!("nb_save_default", field), value.to_string());
        }
        if ui.small_button(tr!("btn_clear")).clicked() {
            *pending = Some(PendingAction::Clear);
        }
        let _ = error_msg;
    }

    fn field_mut(&mut self, name: &str) -> &mut String {
        match name {
            "decimal" => &mut self.decimal,
            "binary"  => &mut self.binary,
            "octal"   => &mut self.octal,
            "hex"     => &mut self.hex,
            _ => &mut self.decimal,
        }
    }

    fn update_from(&mut self, source: &str) {
        let (raw, radix) = match source {
            "decimal" => (self.decimal.trim().trim_start_matches("0d").replace(",", ""), 10),
            "binary"  => (self.binary.trim().trim_start_matches("0b").to_string(), 2),
            "octal"   => (self.octal.trim().trim_start_matches("0o").to_string(), 8),
            "hex"     => (self.hex.trim().trim_start_matches("0x").to_string(), 16),
            _ => return,
        };

        if raw.is_empty() {
            self.decimal.clear();
            self.binary.clear();
            self.octal.clear();
            self.hex.clear();
            return;
        }

        match i64::from_str_radix(&raw, radix) {
            Ok(n) => {
                self.decimal = format!("{}", n);
                self.binary = format!("{:b}", n);
                self.octal = format!("{:o}", n);
                self.hex = format!("{:X}", n);
                self.error.clear();
            }
            Err(e) => self.error = format!("{}", e),
        }
    }
}

fn format_number(n: i64) -> String {
    let s = format!("{}", n.abs());
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { result.push(','); }
        result.push(c);
    }
    if n < 0 { result.push('-'); }
    result.chars().rev().collect()
}
