use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};

pub struct YamlProperties {
    input: String,
    output: String,
    error: String,
    to_properties: bool,
    pending_file: Pending<String>,
}

impl Default for YamlProperties {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            to_properties: true,
            pending_file: Pending::default(),
        }
    }
}

impl Tool for YamlProperties {
    fn name(&self) -> &str { "YAML <> Properties" }
    fn description(&self) -> &str { "Convert between YAML and .properties formats" }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.to_properties, true, "YAML → Properties");
            ui.radio_value(&mut self.to_properties, false, "Properties → YAML");
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_to_properties = self.to_properties;

        ui.columns(2, |cols| {
            // Left: Input panel
            cols[0].vertical(|ui| {
                let input_label = if self.to_properties { "Input YAML:" } else { "Input Properties:" };

                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = format!("Clipboard error: {}", e),
                        }
                    }
                    if ui.button("Open File...").clicked() {
                        open_file_async(&mut self.pending_file, "Open file", "Data", &["yaml", "yml", "properties"]);
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
                    .id_salt("yp_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Output panel
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                let output_label = if self.to_properties { "Output Properties:" } else { "Output YAML:" };

                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.output.is_empty() {
                        let ext = if self.to_properties { "properties" } else { "yaml" };
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", ext, &[ext], &format!("output.{}", ext)) {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(output_label);

                egui::ScrollArea::vertical()
                    .id_salt("yp_output_scroll")
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

        // Auto-convert when input or direction changes
        if self.input != prev_input || self.to_properties != prev_to_properties {
            if !self.input.trim().is_empty() {
                self.convert();
            } else {
                self.output.clear();
                self.error.clear();
            }
        }
    }
}

impl YamlProperties {
    fn convert(&mut self) {
        self.error.clear();
        self.output.clear();

        if self.to_properties {
            match yaml_text_to_properties(&self.input) {
                Ok(s) => self.output = s,
                Err(e) => self.error = format!("YAML parse error: {}", e),
            }
        } else {
            match properties_to_yaml(&self.input) {
                Ok(s) => self.output = s,
                Err(e) => self.error = format!("Properties parse error: {}", e),
            }
        }
    }
}

// ── Parsed line types ────────────────────────────────────────────────────────

enum PropLine {
    Comment(String),
    KeyValue(String, String),
}

// ── YAML text → Properties (preserves comments) ─────────────────────────────

fn yaml_text_to_properties(input: &str) -> Result<String, String> {
    let mut lines: Vec<String> = Vec::new();

    // indent-stack: tracks (indent_level, current_key) for active nesting
    let mut stack: Vec<(usize, String)> = Vec::new();

    for raw_line in input.lines() {
        // ── blank line ──
        if raw_line.trim().is_empty() {
            continue;
        }

        // ── indent ──
        let indent = raw_line.len() - raw_line.trim_start().len();
        let content = raw_line.trim();

        // ── comment ──
        if content.starts_with('#') {
            lines.push(content.to_string());
            continue;
        }

        // ── pop stack to parent indent ──
        while let Some(&(si, _)) = stack.last() {
            if si >= indent {
                stack.pop();
            } else {
                break;
            }
        }

        // ── parse key / value ──
        let (key, value_opt, inline_comment) = match parse_yaml_kv(content)? {
            Some(kv) => kv,
            None => continue, // skip bare sequence items
        };

        // Build the full dot-path
        let full_key = if stack.is_empty() {
            key.clone()
        } else {
            let prefix = stack.iter()
                .map(|(_, k)| k.as_str())
                .collect::<Vec<_>>()
                .join(".");
            format!("{}.{}", prefix, key)
        };

        // Emit inline comment before the key-value line
        if let Some(comment) = inline_comment {
            lines.push(comment);
        }

        match value_opt {
            Some(v) => {
                lines.push(format!("{}={}", full_key, escape_properties(&v)));
                // Adjust stack: pop same-level entry if any, then push
                if let Some(&(si, _)) = stack.last() {
                    if si == indent {
                        stack.pop();
                    }
                }
                stack.push((indent, key));
            }
            None => {
                // No value — just update stack position at this indent
                if let Some(&(si, _)) = stack.last() {
                    if si == indent {
                        stack.pop();
                    }
                }
                stack.push((indent, key));
            }
        }
    }

    Ok(lines.join("\n"))
}

/// Parse a YAML line into (key, optional_value, optional_inline_comment).
/// Returns Ok(None) for lines that should be skipped (bare sequence items).
fn parse_yaml_kv(line: &str) -> Result<Option<(String, Option<String>, Option<String>)>, String> {
    // "- key: value" (sequence item treated as mapping)
    let line = line.strip_prefix("- ").unwrap_or(line);

    if let Some(pos) = line.find(": ") {
        let potential_key = line[..pos].trim();
        // A valid YAML mapping key: no spaces, and if there's another colon
        // before ": " this is likely a plain scalar (e.g. "optional:nacos:foo")
        if !potential_key.contains(' ') && !potential_key.contains(':') {
            let raw_value = line[pos + 2..].trim();
            let (val_part, comment) = split_inline_comment(raw_value);
            if val_part.is_empty() {
                return Ok(Some((potential_key.to_string(), None, comment)));
            } else {
                return Ok(Some((potential_key.to_string(), Some(unquote_yaml(&val_part)), comment)));
            }
        }
        // else: fall through — treat as skip
    }
    if line.ends_with(':') {
        return Ok(Some((line[..line.len() - 1].trim().to_string(), None, None)));
    }
    // Plain scalar or unrecognized — skip
    Ok(None)
}

/// Split a YAML value string at the first `#` that appears outside of quotes.
/// Returns (value_part, optional_comment).
fn split_inline_comment(s: &str) -> (String, Option<String>) {
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = s.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if in_double => { chars.next(); } // skip escaped char
            '#' if !in_single && !in_double => {
                let value = s[..i].trim_end().to_string();
                let comment = s[i..].to_string();
                return (value, Some(comment));
            }
            _ => {}
        }
    }
    (s.to_string(), None)
}

/// Strip surrounding quotes from a YAML scalar.
fn unquote_yaml(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn escape_properties(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out
}

// ── Properties → YAML (preserves comments) ──────────────────────────────────

fn properties_to_yaml(input: &str) -> Result<String, String> {
    let raw = join_continuation_lines(input);
    let entries = parse_prop_lines(&raw)?;

    // Build a nested tree from dot-separated keys.
    // Process in reverse so that comments preceding a key are attached to it.
    let mut tree = PropNode::new();
    let mut pending_comments: Vec<String> = Vec::new();

    for entry in entries.iter().rev() {
        match entry {
            PropLine::Comment(text) => {
                pending_comments.push(text.clone());
            }
            PropLine::KeyValue(path, value) => {
                let parts: Vec<&str> = path.split('.').collect();
                let node = tree.get_or_create(&parts);
                node.value = Some(value.clone());
                if !pending_comments.is_empty() {
                    for c in pending_comments.drain(..).rev() {
                        node.comments.push(c);
                    }
                }
            }
        }
    }

    // Any remaining comments (before the first key) go to root
    for c in pending_comments.drain(..).rev() {
        tree.comments.push(c);
    }

    let mut out = String::new();
    tree.write_yaml(&mut out, 0);
    Ok(out.trim_end().to_string())
}

// ── Properties line parsing ──────────────────────────────────────────────────

fn parse_prop_lines(lines: &[String]) -> Result<Vec<PropLine>, String> {
    let mut result = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            result.push(PropLine::Comment(trimmed.to_string()));
            continue;
        }
        let (key, value) = parse_kv_line(trimmed)?;
        result.push(PropLine::KeyValue(key, value));
    }
    Ok(result)
}

fn join_continuation_lines(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    for line in input.split('\n') {
        if line.ends_with('\\') {
            current.push_str(&line[..line.len() - 1]);
            current.push('\n');
        } else {
            current.push_str(line);
            result.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

fn parse_kv_line(line: &str) -> Result<(String, String), String> {
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'=' || bytes[i] == b':' {
            let key = line[..i].trim().to_string();
            let value = unescape_properties(&line[i + 1..].trim());
            return Ok((key, value));
        }
        i += 1;
    }
    Ok((line.trim().to_string(), String::new()))
}

fn unescape_properties(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('f') => out.push('\x0C'),
                Some('\\') => out.push('\\'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(cp) {
                            out.push(ch);
                        }
                    }
                }
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ── PropNode: nested tree for Properties → YAML ─────────────────────────────

struct PropNode {
    value: Option<String>,
    children: Vec<(String, PropNode)>,
    array: Vec<PropNode>,
    comments: Vec<String>,
}

impl PropNode {
    fn new() -> Self {
        Self { value: None, children: Vec::new(), array: Vec::new(), comments: Vec::new() }
    }

    fn get_or_create(&mut self, parts: &[&str]) -> &mut PropNode {
        self.get_or_create_impl(parts, 0, None)
    }

    fn get_or_create_impl(&mut self, parts: &[&str], depth: usize, arr_idx: Option<usize>) -> &mut PropNode {
        if depth >= parts.len() {
            return self;
        }
        let segment = parts[depth];

        // Check for [N] array index at this segment
        let (base_key, arr_index) = if let Some(bracket) = segment.find('[') {
            let base = &segment[..bracket];
            let idx_str = &segment[bracket + 1..segment.len() - 1];
            let idx: usize = idx_str.parse().unwrap_or(0);
            (base.to_string(), Some(idx))
        } else {
            (segment.to_string(), None)
        };

        let is_last = depth == parts.len() - 1;

        // If we're inside an array element and this is the last segment
        if let Some(ai) = arr_idx {
            while self.array.len() <= ai {
                self.array.push(PropNode::new());
            }
            if is_last {
                return &mut self.array[ai];
            } else {
                return self.array[ai].get_or_create_impl(parts, depth + 1, None);
            }
        }

        if is_last {
            if let Some(ai) = arr_index {
                // Array leaf: import[0] = value
                let child = self.get_or_create_child(&base_key);
                while child.array.len() <= ai {
                    child.array.push(PropNode::new());
                }
                &mut child.array[ai]
            } else {
                self.get_or_create_child(&base_key)
            }
        } else {
            if let Some(ai) = arr_index {
                // Intermediate array: import[0].key = ...
                let child = self.get_or_create_child(&base_key);
                while child.array.len() <= ai {
                    child.array.push(PropNode::new());
                }
                child.array[ai].get_or_create_impl(parts, depth + 1, None)
            } else {
                let child = self.get_or_create_child(&base_key);
                child.get_or_create_impl(parts, depth + 1, None)
            }
        }
    }

    fn get_or_create_child(&mut self, key: &str) -> &mut PropNode {
        let idx = self.children.iter().position(|(k, _)| *k == key);
        let idx = match idx {
            Some(i) => i,
            None => {
                self.children.push((key.to_string(), PropNode::new()));
                self.children.len() - 1
            }
        };
        &mut self.children[idx].1
    }

    fn write_yaml(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);

        for comment in &self.comments {
            out.push_str(&format!("{}{}\n", pad, comment));
        }

        for (key, child) in &self.children {
            for comment in &child.comments {
                out.push_str(&format!("{}{}\n", pad, comment));
            }

            if !child.array.is_empty() {
                // This key is an array
                out.push_str(&format!("{}{}:\n", pad, key));
                let item_pad = "  ".repeat(indent + 1);
                for item in &child.array {
                    for comment in &item.comments {
                        out.push_str(&format!("{}{}\n", item_pad, comment));
                    }
                    if let Some(ref v) = item.value {
                        if item.children.is_empty() {
                            out.push_str(&format!("{}- {}\n", item_pad, format_yaml_scalar(v)));
                        } else {
                            // First key on same line as dash, rest indented further
                            let mut keys: Vec<&String> = item.children.iter().map(|(k, _)| k).collect();
                            keys.sort();
                            let mut first = true;
                            for k in &keys {
                                let c = item.children.iter().find(|(ck, _)| *ck == **k).map(|(_, c)| c).unwrap();
                                if first {
                                    if let Some(ref v) = c.value {
                                        out.push_str(&format!("{}- {}: {}\n", item_pad, k, format_yaml_scalar(v)));
                                    } else {
                                        out.push_str(&format!("{}- {}:\n", item_pad, k));
                                        c.write_yaml(out, indent + 2);
                                    }
                                    first = false;
                                } else {
                                    let deeper = "  ".repeat(indent + 2);
                                    for comment in &c.comments {
                                        out.push_str(&format!("{}{}\n", deeper, comment));
                                    }
                                    if let Some(ref v) = c.value {
                                        out.push_str(&format!("{}{}: {}\n", deeper, k, format_yaml_scalar(v)));
                                    } else {
                                        out.push_str(&format!("{}{}:\n", deeper, k));
                                        c.write_yaml(out, indent + 3);
                                    }
                                }
                            }
                        }
                    } else if !item.children.is_empty() {
                        // Object item in array
                        let mut keys: Vec<&String> = item.children.iter().map(|(k, _)| k).collect();
                        keys.sort();
                        let mut first = true;
                        for k in &keys {
                            let c = item.children.iter().find(|(ck, _)| *ck == **k).map(|(_, c)| c).unwrap();
                            for comment in &c.comments {
                                out.push_str(&format!("{}{}\n", "  ".repeat(if first { indent + 1 } else { indent + 2 }), comment));
                            }
                            if first {
                                if let Some(ref v) = c.value {
                                    out.push_str(&format!("{}- {}: {}\n", item_pad, k, format_yaml_scalar(v)));
                                } else {
                                    out.push_str(&format!("{}- {}:\n", item_pad, k));
                                    c.write_yaml(out, indent + 2);
                                }
                                first = false;
                            } else {
                                let deeper = "  ".repeat(indent + 2);
                                if let Some(ref v) = c.value {
                                    out.push_str(&format!("{}{}: {}\n", deeper, k, format_yaml_scalar(v)));
                                } else {
                                    out.push_str(&format!("{}{}:\n", deeper, k));
                                    c.write_yaml(out, indent + 3);
                                }
                            }
                        }
                    } else {
                        out.push_str(&format!("{}-\n", item_pad));
                    }
                }
            } else if let Some(ref v) = child.value {
                out.push_str(&format!("{}{}: {}\n", pad, key, format_yaml_scalar(v)));
            } else if !child.children.is_empty() || !child.array.is_empty() {
                out.push_str(&format!("{}{}:\n", pad, key));
                child.write_yaml(out, indent + 1);
            } else {
                out.push_str(&format!("{}{}:\n", pad, key));
            }
        }
    }
}

fn format_yaml_scalar(v: &str) -> String {
    if v.is_empty() {
        return "''".to_string();
    }
    if v.contains('\n') {
        return format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
    }
    // Quote if the value looks like it could be misinterpreted
    if v.contains(':')
        || v.contains('#')
        || v.contains('{')
        || v.contains('}')
        || v.contains('[')
        || v.contains(']')
        || v.contains(',')
        || v.contains('&')
        || v.contains('*')
        || v.contains('?')
        || v.contains('|')
        || v.contains('>')
        || v.contains('!')
        || v.contains('%')
        || v.contains('@')
        || v.contains('`')
        || v == "true"
        || v == "false"
        || v == "null"
        || v == "yes"
        || v == "no"
        || v == "~"
        || v.starts_with(' ')
        || v.ends_with(' ')
        || v.starts_with('-')
    {
        return format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""));
    }
    v.to_string()
}
