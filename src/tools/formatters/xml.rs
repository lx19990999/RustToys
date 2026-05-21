use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};

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
        if let Some(text) = self.pending_file.poll() {
            let err_prefix = tr!("err_error_reading");
            if !text.starts_with(&err_prefix) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        let prev_input = self.input.clone();
        let prev_indent = self.indent_size;
        let prev_attrs = self.attrs_new_line;
        let prev_minify = self.minify;

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("xf_open_title"), "XML", &["xml"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(tr!("label_indent"));
                    ui.add(egui::DragValue::new(&mut self.indent_size).range(1..=8).speed(1));
                    let label_minify = tr!("label_minify");
                    ui.checkbox(&mut self.minify, &label_minify);
                    let label_attrs_newline = tr!("xf_attrs_newline");
                    ui.checkbox(&mut self.attrs_new_line, &label_attrs_newline);
                });
                ui.add_space(2.0);
                ui.label(tr!("xf_input_label"));

                egui::ScrollArea::vertical()
                    .id_salt("xml_input_scroll")
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

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        save_file_async(&mut self.save_pending, &tr!("save_as_title"), "XML", &["xml"], &tr!("xf_save_default"), self.output.clone());
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_output"));

                egui::ScrollArea::vertical()
                    .id_salt("xml_output_scroll")
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

        // Auto-format
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

        // Validate first
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
    if attrs.is_empty() {
        return String::new();
    }
    if !new_line {
        let parts: Vec<String> = attrs.iter()
            .map(|a| {
                let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
                let val = a.unescape_value().unwrap_or_default();
                format!("{}=\"{}\"", key, val)
            })
            .collect();
        return format!(" {}", parts.join(" "));
    }
    let attr_indent = indent.repeat(depth + 1);
    let parts: Vec<String> = attrs.iter()
        .map(|a| {
            let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
            let val = a.unescape_value().unwrap_or_default();
            format!("{}\n{}{}=\"{}\"", "", &attr_indent, key, val)
        })
        .collect();
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
                if !text.is_empty() {
                    output.push_str(&format!("{}{}\n", indent.repeat(depth), text));
                }
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
                let attrs: Vec<String> = e.attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
                        let val = a.unescape_value().unwrap_or_default();
                        format!("{}=\"{}\"", key, val)
                    })
                    .collect();
                if attrs.is_empty() {
                    output.push_str(&format!("<{}>", name));
                } else {
                    output.push_str(&format!("<{} {}>", name, attrs.join(" ")));
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                output.push_str(&format!("</{}>", name));
            }
            Ok(quick_xml::events::Event::Empty(ref e)) => {
                let name = e.name().as_ref().to_owned();
                let name = std::str::from_utf8(&name).unwrap_or("?");
                let attrs: Vec<String> = e.attributes()
                    .filter_map(|a| a.ok())
                    .map(|a| {
                        let key = std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
                        let val = a.unescape_value().unwrap_or_default();
                        format!("{}=\"{}\"", key, val)
                    })
                    .collect();
                if attrs.is_empty() {
                    output.push_str(&format!("<{}/>", name));
                } else {
                    output.push_str(&format!("<{} {}/>", name, attrs.join(" ")));
                }
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
