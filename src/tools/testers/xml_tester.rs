use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashMap;

pub struct XmlTester {
    xsd_input: String,
    xml_input: String,
    output: String,
    error: String,
    severity: Severity,
    prev_xsd: String,
    prev_xml: String,
    pending_file: Pending<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    None,
    Success,
    Warning,
    Error,
}

impl Default for XmlTester {
    fn default() -> Self {
        Self {
            xsd_input: String::new(),
            xml_input: String::new(),
            output: String::new(),
            error: String::new(),
            severity: Severity::None,
            prev_xsd: String::new(),
            prev_xml: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl XmlTester {
    fn do_validate(&mut self) {
        self.error.clear();
        self.output.clear();
        self.severity = Severity::None;

        if self.xsd_input.trim().is_empty() || self.xml_input.trim().is_empty() {
            return;
        }

        // Step 1: Check XML well-formedness
        if let Err(e) = check_well_formed(&self.xml_input) {
            self.error = e;
            self.severity = Severity::Error;
            return;
        }

        // Step 2: Parse XSD and validate XML against it
        match parse_xsd(&self.xsd_input) {
            Ok(xsd) => match validate_against_xsd(&self.xml_input, &xsd) {
                ValidationResult::Valid(info) => {
                    self.output = info;
                    self.severity = Severity::Success;
                }
                ValidationResult::Warning(msg) => {
                    self.output = msg;
                    self.severity = Severity::Warning;
                }
                ValidationResult::Invalid(msg) => {
                    self.error = msg;
                    self.severity = Severity::Error;
                }
            },
            Err(e) => {
                self.error = format!("XSD parse error: {}", e);
                self.severity = Severity::Error;
            }
        }
    }

    fn auto_validate(&mut self) {
        if self.xsd_input != self.prev_xsd || self.xml_input != self.prev_xml {
            self.prev_xsd = self.xsd_input.clone();
            self.prev_xml = self.xml_input.clone();
            self.do_validate();
        }
    }
}

impl Tool for XmlTester {
    fn name(&self) -> &str { "XML / XSD Tester" }
    fn description(&self) -> &str { "Validate XML data via an XSD scheme" }
    fn category(&self) -> ToolCategory { ToolCategory::Testers }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.xsd_input = text;
            }
        }
        let total = ui.available_rect_before_wrap();
        let pad = 4.0;
        let w = total.width();
        let half_w = (w - pad) * 0.5;

        let label_h = 18.0;
        let btn_h = 22.0;
        let space = 2.0;
        let result_h = if self.error.is_empty() && self.output.is_empty() { 0.0 }
            else { (total.height() * 0.3).max(80.0) };
        let top_header_h = label_h + space + btn_h + space + space;

        let cols_h = (total.height() - result_h - pad * 2.0).max(120.0);

        // --- Left column: XSD Input ---
        let left_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new("XSD").strong());
            ui.add_space(space);
            ui.horizontal(|ui| {
                if ui.button("Paste").clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => { self.xsd_input = text; self.error.clear(); }
                        Err(e) => self.error = format!("Clipboard error: {}", e),
                    }
                }
                if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open XSD file", "XSD", &["xsd"]);
                }
                    if ui.button("Clear").clicked() {
                    self.xsd_input.clear();
                    self.output.clear();
                    self.error.clear();
                    self.severity = Severity::None;
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_header_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.xsd_input)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- Right column: XML Input ---
        let right_rect = egui::Rect::from_min_size(
            total.min + egui::vec2(half_w + pad, 0.0),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.label(egui::RichText::new("XML").strong());
            ui.add_space(space);
            ui.horizontal(|ui| {
                if ui.button("Paste").clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => { self.xml_input = text; self.error.clear(); }
                        Err(e) => self.error = format!("Clipboard error: {}", e),
                    }
                }
                if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open XSD file", "XSD", &["xsd"]);
                }
                    if ui.button("Clear").clicked() {
                    self.xml_input.clear();
                    self.output.clear();
                    self.error.clear();
                    self.severity = Severity::None;
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_header_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.xml_input)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // Auto-validate on input change
        self.auto_validate();

        // --- Result area ---
        if self.severity != Severity::None {
            let result_y = total.min.y + cols_h + pad;
            let result_rect = egui::Rect::from_min_size(
                egui::pos2(total.min.x, result_y),
                egui::vec2(w, result_h),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(result_rect), |ui| {
                ui.group(|ui| {
                    let (text, color) = match self.severity {
                        Severity::Success => {
                            ui.colored_label(egui::Color32::from_rgb(0, 150, 0), "XML is compliant to the defined XSD scheme.");
                            (self.output.clone(), egui::Color32::from_rgb(0, 150, 0))
                        }
                        Severity::Warning => {
                            (self.output.clone(), egui::Color32::from_rgb(200, 150, 0))
                        }
                        Severity::Error => {
                            (self.error.clone(), egui::Color32::RED)
                        }
                        Severity::None => (String::new(), egui::Color32::WHITE),
                    };
                    if !text.is_empty() {
                        let scroll_h = (result_h - 48.0).max(20.0);
                        egui::ScrollArea::vertical()
                            .id_salt("xsd_result_scroll")
                            .max_height(scroll_h)
                            .show(ui, |ui| {
                                ui.colored_label(color, &text);
                            });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Copy").clicked() {
                            let t = if self.error.is_empty() { &self.output } else { &self.error };
                            ui.ctx().copy_text(t.clone());
                        }
                        if ui.button("Save As...").clicked() {
                            if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "xml_validation.txt") {
                                let t = if self.error.is_empty() { &self.output } else { &self.error };
                                let _ = std::fs::write(path, t);
                            }
                        }
                    });
                });
            });
        }

        // --- Cheatsheet ---
        let cheat_y = total.min.y + cols_h + result_h + pad;
        let cheat_h = (total.height() - cols_h - result_h - pad * 2.0).max(60.0);
        let cheat_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, cheat_y),
            egui::vec2(w, cheat_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(cheat_rect), |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("XSD Cheat sheet").strong());
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("xsd_cheatsheet_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Element").underline().strong());
                        ui.add_space(2.0);
                        egui::Grid::new("xsd_elements")
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (syntax, desc) in &[
                                    ("<xs:element name='x'>", "Declare an element"),
                                    ("<xs:element name='x' type='xs:string'>", "Element with type"),
                                    ("<xs:element name='x' minOccurs='0'>", "Optional element"),
                                    ("<xs:element name='x' maxOccurs='unbounded'>", "Unlimited repetition"),
                                    ("<xs:element name='x' minOccurs='1' maxOccurs='1'>", "Required exactly once"),
                                ] {
                                    ui.label(egui::RichText::new(*syntax).monospace().color(egui::Color32::from_rgb(0, 120, 200)));
                                    ui.label(*desc);
                                    ui.end_row();
                                }
                            });

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Simple types").underline().strong());
                        ui.add_space(2.0);
                        egui::Grid::new("xsd_types")
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (syntax, desc) in &[
                                    ("xs:string", "String"),
                                    ("xs:integer", "Integer number"),
                                    ("xs:decimal", "Decimal number"),
                                    ("xs:boolean", "True/false"),
                                    ("xs:date", "Date (YYYY-MM-DD)"),
                                    ("xs:dateTime", "Date and time"),
                                    ("xs:time", "Time"),
                                    ("xs:duration", "Duration"),
                                    ("xs:base64Binary", "Base64 binary"),
                                    ("xs:hexBinary", "Hex binary"),
                                    ("xs:anyURI", "URI"),
                                    ("xs:normalizedString", "Normalized string"),
                                    ("xs:token", "Token (trimmed)"),
                                    ("xs:positiveInteger", "Positive integer"),
                                    ("xs:nonNegativeInteger", "Non-negative integer"),
                                ] {
                                    ui.label(egui::RichText::new(*syntax).monospace().color(egui::Color32::from_rgb(0, 120, 200)));
                                    ui.label(*desc);
                                    ui.end_row();
                                }
                            });

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Compositors").underline().strong());
                        ui.add_space(2.0);
                        egui::Grid::new("xsd_compositors")
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (syntax, desc) in &[
                                    ("<xs:sequence>", "Elements must appear in order"),
                                    ("<xs:choice>", "Only one element can appear"),
                                    ("<xs:all>", "All elements in any order"),
                                    ("<xs:group ref='x'>", "Reference a group"),
                                ] {
                                    ui.label(egui::RichText::new(*syntax).monospace().color(egui::Color32::from_rgb(0, 120, 200)));
                                    ui.label(*desc);
                                    ui.end_row();
                                }
                            });

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Attributes").underline().strong());
                        ui.add_space(2.0);
                        egui::Grid::new("xsd_attrs")
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (syntax, desc) in &[
                                    ("<xs:attribute name='x' type='xs:string'>", "Declare attribute"),
                                    ("<xs:attribute name='x' use='required'>", "Required attribute"),
                                    ("<xs:attribute name='x' default='v'>", "Default value"),
                                    ("<xs:attribute name='x' fixed='v'>", "Fixed value"),
                                ] {
                                    ui.label(egui::RichText::new(*syntax).monospace().color(egui::Color32::from_rgb(0, 120, 200)));
                                    ui.label(*desc);
                                    ui.end_row();
                                }
                            });

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Structure").underline().strong());
                        ui.add_space(2.0);
                        egui::Grid::new("xsd_structure")
                            .num_columns(2)
                            .spacing([16.0, 2.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (syntax, desc) in &[
                                    ("<xs:schema>", "Root element of XSD"),
                                    ("<xs:complexType name='x'>", "Complex type definition"),
                                    ("<xs:simpleType name='x'>", "Simple type definition"),
                                    ("<xs:restriction base='xs:string'>", "Restrict base type"),
                                    ("<xs:enumeration value='x'>", "Enumeration value"),
                                    ("<xs:minLength value='1'>", "Min string length"),
                                    ("<xs:maxLength value='100'>", "Max string length"),
                                    ("<xs:pattern value='...'>", "Regex pattern"),
                                    ("<xs:annotation><xs:documentation>", "Documentation"),
                                ] {
                                    ui.label(egui::RichText::new(*syntax).monospace().color(egui::Color32::from_rgb(0, 120, 200)));
                                    ui.label(*desc);
                                    ui.end_row();
                                }
                            });
                    });
            });
        });
    }
}

// --- Validation logic ---

fn check_well_formed(xml: &str) -> Result<(), String> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => return Ok(()),
            Err(e) => return Err(format!("XML is not well-formed at position {}: {}", reader.buffer_position(), e)),
            _ => {}
        }
        buf.clear();
    }
}

// --- XSD Schema representation ---

struct XsdSchema {
    root_element: String,
    elements: HashMap<String, XsdElement>,
}

struct XsdElement {
    element_type: Option<String>,
    children: Vec<(String, u32, u32)>, // (name, min, max)  max=u32::MAX for unbounded
    attributes: Vec<XsdAttribute>,
    simple_type: Option<String>,
}

struct XsdAttribute {
    name: String,
    attr_type: Option<String>,
    use_required: bool,
}

enum ValidationResult {
    Valid(String),
    Warning(String),
    Invalid(String),
}

fn parse_xsd(xsd: &str) -> Result<XsdSchema, String> {
    let mut reader = Reader::from_str(xsd);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut elements: HashMap<String, XsdElement> = HashMap::new();
    let mut root_element = String::new();
    let mut element_stack: Vec<String> = Vec::new(); // stack of element names
    let mut type_stack: Vec<String> = Vec::new(); // stack of element types being defined
    let mut current_type_name = String::new();
    let mut current_children: Vec<(String, u32, u32)> = Vec::new();
    let mut current_attrs: Vec<XsdAttribute> = Vec::new();
    let mut current_simple_type: Option<String> = None;
    let mut in_element_def = false;
    let mut in_simple_type = false;
    let mut enum_values: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag_name.as_str() {
                    "xs:element" | "xsd:element" | "element" => {
                        let mut name = String::new();
                        let mut type_name = String::new();
                        let mut min_occurs: u32 = 1;
                        let mut max_occurs: u32 = 1;
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let val = String::from_utf8_lossy(&attr.value);
                            match key.as_ref() {
                                "name" => name = val.to_string(),
                                "type" => type_name = val.to_string(),
                                "minOccurs" => {
                                    min_occurs = val.parse().unwrap_or(1);
                                }
                                "maxOccurs" => {
                                    max_occurs = if val == "unbounded" { u32::MAX } else { val.parse().unwrap_or(1) };
                                }
                                _ => {}
                            }
                        }

                        if in_element_def {
                            // Child element definition
                            if !name.is_empty() {
                                current_children.push((name, min_occurs, max_occurs));
                            }
                        } else if !name.is_empty() {
                            // Top-level element definition
                            root_element = name.clone();
                            in_element_def = true;
                            current_type_name = name;
                            current_children.clear();
                            current_attrs.clear();
                            current_simple_type = if type_name.is_empty() { None } else { Some(type_name) };
                        }

                        // If it's a self-closing element (Empty event), handle immediately
                        if matches!(reader.read_event_into(&mut buf), Ok(Event::Empty(_))) {
                            // Already handled above
                        }
                    }
                    "xs:complexType" | "xsd:complexType" | "complexType" => {
                        let mut name = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        if !name.is_empty() && !in_element_def {
                            // Named complex type
                            element_stack.push(current_type_name.clone());
                            type_stack.push("complexType".to_string());
                            current_type_name = name;
                            current_children.clear();
                            current_attrs.clear();
                            current_simple_type = None;
                        }
                    }
                    "xs:simpleType" | "xsd:simpleType" | "simpleType" => {
                        in_simple_type = true;
                        enum_values.clear();
                    }
                    "xs:restriction" | "xsd:restriction" | "restriction" => {}
                    "xs:attribute" | "xsd:attribute" | "attribute" => {
                        let mut name = String::new();
                        let mut attr_type = String::new();
                        let mut use_val = String::new();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let val = String::from_utf8_lossy(&attr.value);
                            match key.as_ref() {
                                "name" => name = val.to_string(),
                                "type" => attr_type = val.to_string(),
                                "use" => use_val = val.to_string(),
                                _ => {}
                            }
                        }
                        if !name.is_empty() {
                            current_attrs.push(XsdAttribute {
                                name,
                                attr_type: if attr_type.is_empty() { None } else { Some(attr_type) },
                                use_required: use_val == "required",
                            });
                        }
                    }
                    "xs:enumeration" | "xsd:enumeration" | "enumeration" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"value" {
                                enum_values.push(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag_name.as_str() {
                    "xs:element" | "xsd:element" | "element" => {
                        if in_element_def && element_stack.is_empty() {
                            // End of root element definition
                            let elem = XsdElement {
                                element_type: current_simple_type.take(),
                                children: std::mem::take(&mut current_children),
                                attributes: std::mem::take(&mut current_attrs),
                                simple_type: if in_simple_type && !enum_values.is_empty() {
                                    Some(format!("enum: {}", enum_values.join(", ")))
                                } else {
                                    None
                                },
                            };
                            elements.insert(current_type_name.clone(), elem);
                            in_element_def = false;
                        }
                    }
                    "xs:complexType" | "xsd:complexType" | "complexType" => {
                        if !element_stack.is_empty() {
                            // Named complex type ended
                            if let Some(name) = element_stack.pop() {
                                let elem = XsdElement {
                                    element_type: current_simple_type.take(),
                                    children: std::mem::take(&mut current_children),
                                    attributes: std::mem::take(&mut current_attrs),
                                    simple_type: None,
                                };
                                elements.insert(current_type_name.clone(), elem);
                                current_type_name = name;
                            }
                            type_stack.pop();
                        }
                    }
                    "xs:simpleType" | "xsd:simpleType" | "simpleType" => {
                        in_simple_type = false;
                    }
                    "xs:restriction" | "xsd:restriction" | "restriction" => {}
                    _ => {}
                }
            }
            Err(e) => return Err(format!("XSD parse error at position {}: {}", reader.buffer_position(), e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(XsdSchema { root_element, elements })
}

fn validate_against_xsd(xml: &str, xsd: &XsdSchema) -> ValidationResult {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut element_stack: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Track which children have been seen for each parent
    let mut child_count: HashMap<(String, String), u32> = HashMap::new(); // (parent, child) -> count

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = extract_local_name(&e.name());
                let line = reader.buffer_position();

                if element_stack.is_empty() {
                    // Root element
                    if tag != xsd.root_element {
                        return ValidationResult::Invalid(format!(
                            "Expected root element '{}' but found '{}' (pos {})",
                            xsd.root_element, tag, line
                        ));
                    }
                } else {
                    // Child element - validate against parent's definition
                    let parent = element_stack.last().unwrap().clone();
                    if let Some(parent_def) = xsd.elements.get(&parent) {
                        let valid_child = parent_def.children.iter().find(|(name, _, _)| *name == tag);
                        if valid_child.is_none() && !parent_def.children.is_empty() {
                            errors.push(format!(
                                "Element '{}' is not a valid child of '{}' (pos {})",
                                tag, parent, line
                            ));
                        } else if let Some((_, min, max)) = valid_child {
                            let key = (parent.clone(), tag.clone());
                            let count = child_count.entry(key).or_insert(0);
                            *count += 1;
                            if *count > *max && *max != u32::MAX {
                                errors.push(format!(
                                    "Element '{}' occurs {} times in '{}', max is {} (pos {})",
                                    tag, count, parent, max, line
                                ));
                            }
                        }
                    }

                    // Validate attributes
                    if let Some(elem_def) = xsd.elements.get(&tag) {
                        let mut seen_attrs: Vec<String> = Vec::new();
                        for attr in e.attributes().flatten() {
                            let attr_name = extract_local_name(&attr.key);
                            seen_attrs.push(attr_name.clone());
                            let defined = elem_def.attributes.iter().find(|a| a.name == attr_name);
                            if defined.is_none() && !elem_def.attributes.is_empty() {
                                warnings.push(format!(
                                    "Attribute '{}' on element '{}' is not defined in XSD (pos {})",
                                    attr_name, tag, line
                                ));
                            }
                        }
                        // Check required attributes
                        for attr_def in &elem_def.attributes {
                            if attr_def.use_required && !seen_attrs.contains(&attr_def.name) {
                                errors.push(format!(
                                    "Required attribute '{}' is missing on element '{}' (pos {})",
                                    attr_def.name, tag, line
                                ));
                            }
                        }
                    }
                }
                element_stack.push(tag);
            }
            Ok(Event::End(_)) => {
                element_stack.pop();
            }
            Err(e) => {
                return ValidationResult::Invalid(format!("XML error at position {}: {}", reader.buffer_position(), e));
            }
            _ => {}
        }
        buf.clear();
    }

    // Check min occurs
    for ((parent, child), count) in &child_count {
        if let Some(parent_def) = xsd.elements.get(parent) {
            if let Some((_, min, _)) = parent_def.children.iter().find(|(name, _, _)| *name == *child) {
                if count < min && *min > 0 {
                    warnings.push(format!(
                        "Element '{}' in '{}' occurs {} times, minimum is {}",
                        child, parent, count, min
                    ));
                }
            }
        }
    }

    if !errors.is_empty() {
        return ValidationResult::Invalid(errors.join("\n"));
    }

    if !warnings.is_empty() {
        return ValidationResult::Warning(warnings.join("\n"));
    }

    // Build info string
    let mut info = String::new();
    info.push_str(&format!("Root element: {}\n", xsd.root_element));
    info.push_str(&format!("Defined elements: {}", xsd.elements.len()));
    for (name, elem) in &xsd.elements {
        if !elem.children.is_empty() {
            info.push_str(&format!("\n  {} → [{}]", name,
                elem.children.iter()
                    .map(|(n, min, max)| {
                        let card = if *max == u32::MAX {
                            if *min == 0 { "*".to_string() } else { format!("{}+", min) }
                        } else if *min == *max {
                            min.to_string()
                        } else {
                            format!("{}-{}", min, max)
                        };
                        format!("{}({})", n, card)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !elem.attributes.is_empty() {
            info.push_str(&format!("\n  {} @ [{}]", name,
                elem.attributes.iter()
                    .map(|a| format!("{}{}", a.name, if a.use_required { "(req)" } else { "" }))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    ValidationResult::Valid(info)
}

fn extract_local_name(name: &quick_xml::name::QName) -> String {
    let bytes = name.as_ref();
    // Handle namespace prefix: ns:localname
    if let Some(pos) = bytes.iter().position(|&b| b == b':') {
        String::from_utf8_lossy(&bytes[pos + 1..]).to_string()
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}
