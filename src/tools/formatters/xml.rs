use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};
use crate::tools::io_layout;

pub struct XmlFormatter {
    input: String,
    output: String,
    error: String,
    indent_size: usize,
    attrs_new_line: bool,
    minify: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for XmlFormatter {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            indent_size: 2,
            attrs_new_line: false,
            minify: false,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Tool for XmlFormatter {
    fn name(&self) -> String { tr!("xf_name") }
    fn description(&self) -> String { tr!("xf_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Formatters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();
        let prev_indent = self.indent_size;
        let prev_attrs = self.attrs_new_line;
        let prev_minify = self.minify;

        if let Some(text) = self.pending_file.poll() {
            let err_prefix = tr!("err_error_reading");
            if !text.starts_with(&err_prefix) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }

        let lbl_paste = tr!("btn_paste");
        let lbl_open_file = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_indent = tr!("label_indent");
        let lbl_minify = tr!("label_minify");
        let lbl_attrs_newline = tr!("xf_attrs_newline");
        let lbl_input = tr!("xf_input_label");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_output = tr!("label_output");
        let err_clipboard = tr!("err_clipboard");

        let opt_h = io_layout::option_row_height(ui);
        io_layout::show_error(ui, &self.error);
        io_layout::two_column_io(ui, |ui, w, col| match col {
            io_layout::IoColumn::Left => {
                ui.horizontal(|ui| {
                    if ui.button(&lbl_paste).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = err_clipboard.replace("{}", &e.to_string()),
                        }
                    }
                    if ui.button(&lbl_open_file).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("xf_open_title"), "XML", &["xml"]);
                    }
                    if ui.button(&lbl_clear).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(io_layout::ROW_GAP);
                ui.horizontal(|ui| {
                    ui.label(&lbl_indent);
                    ui.add(egui::DragValue::new(&mut self.indent_size).range(1..=8).speed(1));
                    ui.checkbox(&mut self.minify, &lbl_minify);
                    ui.checkbox(&mut self.attrs_new_line, &lbl_attrs_newline);
                });
                ui.add_space(io_layout::ROW_GAP);
                ui.label(&lbl_input);
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field(ui, w, "xml_input_scroll", &mut self.input);
            }
            io_layout::IoColumn::Right => {
                ui.horizontal(|ui| {
                    if ui.button(&lbl_copy).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() && !self.output.is_empty() {
                        save_file_async(
                            &mut self.save_pending,
                            &tr!("save_as_title"),
                            "XML",
                            &["xml"],
                            &tr!("xf_save_default"),
                            self.output.clone(),
                        );
                    }
                });
                io_layout::row_spacer(ui, opt_h + io_layout::ROW_GAP);
                ui.label(&lbl_output);
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field(ui, w, "xml_output_scroll", &mut self.output);
            }
        });

        if self.input != prev_input || self.indent_size != prev_indent
            || self.attrs_new_line != prev_attrs || self.minify != prev_minify
        {
            self.format();
        }
    }
}

impl XmlFormatter {
    fn format(&mut self) {
        self.error.clear();
        if self.input.trim().is_empty() {
            self.output.clear();
            return;
        }
        {
            let mut reader = quick_xml::Reader::from_str(&self.input);
            let mut buf = Vec::new();
            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(e) => {
                        self.error = tr!("xf_error_pos", reader.buffer_position(), e);
                        self.output.clear();
                        return;
                    }
                    _ => {}
                }
                buf.clear();
            }
        }
        if self.minify {
            self.output = minify_xml(&self.input);
        } else {
            self.output = format_xml(&self.input, self.indent_size, self.attrs_new_line);
        }
    }
}

fn format_attrs(attrs: &[quick_xml::events::attributes::Attribute], indent: &str, depth: usize, new_line: bool) -> String {
    if attrs.is_empty() { return String::new(); }
    if !new_line {
        let parts: Vec<String> = attrs.iter().map(|a| {
            let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
            let val = a.unescape_value().unwrap_or_default();
            format!("{}=\"{}\"", key, val)
        }).collect();
        return format!(" {}", parts.join(" "));
    }
    let attr_indent = indent.repeat(depth + 1);
    let parts: Vec<String> = attrs.iter().map(|a| {
        let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
        let val = a.unescape_value().unwrap_or_default();
        format!("{}\n{}{}=\"{}\"", "", &attr_indent, key, val)
    }).collect();
    parts.join("")
}

fn format_xml(xml: &str, indent_size: usize, attrs_new_line: bool) -> String {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut output = String::new();
    let mut depth: usize = 0;
    let indent = " ".repeat(indent_size);
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                let attrs = format_attrs(&e.attributes().filter_map(|a| a.ok()).collect::<Vec<_>>(), &indent, depth, attrs_new_line);
                output.push_str(&format!("{}<{}{}>\n", indent.repeat(depth), name, attrs));
                depth += 1;
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                if depth > 0 { depth -= 1; }
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                output.push_str(&format!("{}</{}>\n", indent.repeat(depth), name));
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                let attrs = format_attrs(&e.attributes().filter_map(|a| a.ok()).collect::<Vec<_>>(), &indent, depth, attrs_new_line);
                output.push_str(&format!("{}<{}{} />\n", indent.repeat(depth), name, attrs));
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default();
                let text = text.trim();
                if !text.is_empty() { output.push_str(&format!("{}{}\n", indent.repeat(depth), text)); }
            }
            Ok(quick_xml::events::Event::Comment(ref e)) => {
                output.push_str(&format!("{}<!--{}-->\n", indent.repeat(depth), std::str::from_utf8(e).unwrap_or("?")));
            }
            Ok(quick_xml::events::Event::Decl(ref e)) => {
                output.push_str(&format!("<?xml{}?>\n", std::str::from_utf8(&e).unwrap_or("")));
            }
            Ok(quick_xml::events::Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    output.trim().to_string()
}

fn minify_xml(xml: &str) -> String {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut output = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                let attrs: Vec<String> = e.attributes().filter_map(|a| a.ok()).map(|a| {
                    let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
                    let val = a.unescape_value().unwrap_or_default();
                    format!("{}=\"{}\"", key, val)
                }).collect();
                if attrs.is_empty() { output.push_str(&format!("<{}>", name)); }
                else { output.push_str(&format!("<{} {}>", name, attrs.join(" "))); }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                output.push_str(&format!("</{}>", name));
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                let attrs: Vec<String> = e.attributes().filter_map(|a| a.ok()).map(|a| {
                    let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
                    let val = a.unescape_value().unwrap_or_default();
                    format!("{}=\"{}\"", key, val)
                }).collect();
                if attrs.is_empty() { output.push_str(&format!("<{}/>", name)); }
                else { output.push_str(&format!("<{} {}/>", name, attrs.join(" "))); }
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default();
                output.push_str(&text);
            }
            Ok(quick_xml::events::Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    output
}
