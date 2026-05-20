use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use serde_json::Value;

pub struct JsonPathTester {
    json_input: String,
    path_input: String,
    output: String,
    error: String,
    prev_json: String,
    prev_path: String,
    match_count: usize,
    pending_file: Pending<String>,
}

impl Default for JsonPathTester {
    fn default() -> Self {
        Self {
            json_input: String::new(),
            path_input: String::new(),
            output: String::new(),
            error: String::new(),
            prev_json: String::new(),
            prev_path: String::new(),
            match_count: 0,
            pending_file: Pending::default(),
        }
    }
}

impl JsonPathTester {
    fn do_evaluate(&mut self) {
        self.error.clear();
        self.output.clear();
        self.match_count = 0;

        if self.json_input.trim().is_empty() || self.path_input.trim().is_empty() {
            return;
        }

        match serde_json::from_str::<Value>(&self.json_input) {
            Ok(val) => {
                match evaluate_jsonpath(&val, &self.path_input) {
                    Ok(results) => {
                        self.match_count = results.len();
                        if results.is_empty() {
                            self.output = "No match found".to_string();
                        } else {
                            let formatted: Vec<String> = results.iter()
                                .map(|v| serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
                                .collect();
                            self.output = formatted.join("\n");
                        }
                    }
                    Err(e) => self.error = e,
                }
            }
            Err(e) => self.error = format!("JSON parse error: {}", e),
        }
    }

    fn auto_evaluate(&mut self) {
        if self.json_input != self.prev_json || self.path_input != self.prev_path {
            self.prev_json = self.json_input.clone();
            self.prev_path = self.path_input.clone();
            self.do_evaluate();
        }
    }
}

impl Tool for JsonPathTester {
    fn name(&self) -> &str { "JSONPath Tester" }
    fn description(&self) -> &str { "Test JSONPath queries against JSON data" }
    fn category(&self) -> ToolCategory { ToolCategory::Testers }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.json_input = text;
                self.error.clear();
            }
        }

        let total = ui.available_rect_before_wrap();
        let pad = 4.0;
        let w = total.width();
        let half_w = (w - pad) * 0.5;

        // Layout constants
        let label_h = 18.0;
        let btn_h = 22.0;
        let space = 2.0;
        let query_h = 24.0;
        let error_h = if self.error.is_empty() { 0.0 } else { 16.0 };
        let top_header_h = label_h + space + btn_h + space + space; // label + buttons + padding
        let cheat_header_h = 20.0 + space; // "Cheat sheet" label + separator

        let cols_h = (total.height() * 0.55).max(120.0);
        let cheat_h = (total.height() - cols_h - query_h - error_h - cheat_header_h - pad * 3.0).max(60.0);

        // --- Left column: JSON Input ---
        let left_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new("JSON").strong());
            ui.add_space(space);
            ui.horizontal(|ui| {
                if ui.button("Paste").clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => { self.json_input = text; self.error.clear(); }
                        Err(e) => self.error = format!("Clipboard error: {}", e),
                    }
                }
                if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open JSON file", "JSON", &["json"]);
                }
                if ui.button("Clear").clicked() {
                    self.json_input.clear();
                    self.output.clear();
                    self.error.clear();
                    self.match_count = 0;
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_header_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.json_input)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- Right column: Test Result ---
        let right_rect = egui::Rect::from_min_size(
            total.min + egui::vec2(half_w + pad, 0.0),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Test result").strong());
                if self.match_count > 0 {
                    ui.label(
                        egui::RichText::new(format!("({} match{})", self.match_count, if self.match_count == 1 { "" } else { "es" }))
                            .small().color(egui::Color32::GRAY),
                    );
                }
            });
            ui.add_space(space);
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() && !self.output.is_empty() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button("Save As...").clicked() && !self.output.is_empty() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "JSON", &["json"], "jsonpath_result.json") {
                        let _ = std::fs::write(path, &self.output);
                    }
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_header_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.output)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- JSONPath query bar ---
        let query_y = total.min.y + cols_h + pad;
        let query_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, query_y),
            egui::vec2(w, query_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(query_rect), |ui| {
            ui.horizontal(|ui| {
                ui.label("JSONPath:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.path_input)
                        .desired_width(ui.available_width())
                        .hint_text("$.store.book[*].author"),
                );
            });
        });

        if !self.error.is_empty() {
            let error_y = query_y + query_h;
            let error_rect = egui::Rect::from_min_size(
                egui::pos2(total.min.x, error_y),
                egui::vec2(w, error_h),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(error_rect), |ui| {
                ui.colored_label(egui::Color32::RED, &self.error);
            });
        }

        // Auto-evaluate on input change
        self.auto_evaluate();

        // --- Cheatsheet fills remaining height ---
        let cheat_y = query_y + query_h + error_h + pad;
        let cheat_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, cheat_y),
            egui::vec2(w, cheat_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(cheat_rect), |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Cheat sheet").strong());
                ui.separator();

                let entries: &[(&str, &str)] = &[
                    ("$", "The root object or array."),
                    ("@", "Used for filter expressions. Refers to the current node for further processing."),
                    ("object.property", "Dot-notated child"),
                    ("['object'].['property']", "Bracket-notated child or children"),
                    ("..property", "Performs a deep scan for the specified property in all available objects. Always returns a list, even for a single match."),
                    ("*", "Wildcard. Selects all elements in an object or array, regardless of their names or indexes."),
                    ("[n]", "Selects the n-th element from an array. Indexes start from 0."),
                    ("[n1,n2]", "Selects n1 and n2 array items. Returns a list."),
                    ("[start:end:step]", "Array slice operator"),
                    ("?(expression)", "Selects all elements in an object or array that match the specified boolean expression. Returns a list."),
                    ("(expression)", "Script expression."),
                ];

                egui::ScrollArea::vertical()
                    .id_salt("jp_cheatsheet_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("jp_cheatsheet")
                            .num_columns(2)
                            .spacing([16.0, 3.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (syntax, desc) in entries {
                                    ui.label(egui::RichText::new(*syntax).monospace().strong());
                                    ui.label(*desc);
                                    ui.end_row();
                                }
                            });
                    });
            });
        });
    }
}

// --- JSONPath Evaluator ---

fn evaluate_jsonpath(root: &Value, path: &str) -> Result<Vec<Value>, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Empty JSONPath expression".to_string());
    }
    if !path.starts_with('$') {
        return Err("JSONPath must start with '$'".to_string());
    }
    let tokens = tokenize_jsonpath(path)?;
    let mut results = vec![root.clone()];
    execute_tokens(&tokens, &mut results)?;
    Ok(results)
}

#[derive(Debug, Clone)]
enum Token {
    Dot(String),
    Bracket(BracketContent),
    RecursiveDescent(String),
}

#[derive(Debug, Clone)]
enum BracketContent {
    Index(isize),
    Union(Vec<isize>),
    Wildcard,
    Slice(Option<isize>, Option<isize>),
    Filter(FilterExpr),
    Name(String),
}

#[derive(Debug, Clone)]
enum FilterExpr {
    Exists(String),
    Compare {
        path: String,
        op: FilterOp,
        value: FilterValue,
    },
}

#[derive(Debug, Clone, Copy)]
enum FilterOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone)]
enum FilterValue {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

fn tokenize_jsonpath(path: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;

    if chars[i] == '$' {
        i += 1;
    }

    while i < chars.len() {
        match chars[i] {
            '.' => {
                i += 1;
                if i < chars.len() && chars[i] == '.' {
                    i += 1;
                    let mut name = String::new();
                    while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                        name.push(chars[i]);
                        i += 1;
                    }
                    tokens.push(Token::RecursiveDescent(name));
                } else {
                    let mut name = String::new();
                    while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                        name.push(chars[i]);
                        i += 1;
                    }
                    if name.is_empty() {
                        return Err("Expected property name after '.'".to_string());
                    }
                    tokens.push(Token::Dot(name));
                }
            }
            '[' => {
                i += 1;
                let mut content = String::new();
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '[' {
                        depth += 1;
                    } else if chars[i] == ']' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    content.push(chars[i]);
                    i += 1;
                }
                if depth != 0 {
                    return Err("Unclosed bracket".to_string());
                }
                let bracket = parse_bracket_content(&content)?;
                tokens.push(Token::Bracket(bracket));
            }
            ' ' => { i += 1; }
            _ => {
                return Err(format!("Unexpected character '{}' at position {}", chars[i], i));
            }
        }
    }

    Ok(tokens)
}

fn parse_bracket_content(s: &str) -> Result<BracketContent, String> {
    let s = s.trim();

    if s == "*" {
        return Ok(BracketContent::Wildcard);
    }

    if s.starts_with('?') {
        let filter = parse_filter(&s[1..].trim())?;
        return Ok(BracketContent::Filter(filter));
    }

    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        return Ok(BracketContent::Name(s[1..s.len()-1].to_string()));
    }

    if s.contains(',') {
        let indices: Result<Vec<isize>, _> = s.split(',')
            .map(|v| v.trim().parse::<isize>())
            .collect();
        match indices {
            Ok(v) => return Ok(BracketContent::Union(v)),
            Err(_) => return Err(format!("Invalid union index: {}", s)),
        }
    }

    if s.contains(':') {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        let start = if parts[0].trim().is_empty() {
            None
        } else {
            Some(parts[0].trim().parse::<isize>().map_err(|_| format!("Invalid slice start: {}", parts[0]))?)
        };
        let end = if parts.len() > 1 && parts[1].trim().is_empty() {
            None
        } else {
            Some(parts[1].trim().parse::<isize>().map_err(|_| format!("Invalid slice end: {}", parts[1]))?)
        };
        return Ok(BracketContent::Slice(start, end));
    }

    let idx: isize = s.parse().map_err(|_| format!("Invalid index: {}", s))?;
    Ok(BracketContent::Index(idx))
}

fn parse_filter(s: &str) -> Result<FilterExpr, String> {
    let s = s.trim().trim_start_matches('(').trim_end_matches(')').trim();

    if !s.starts_with('@') {
        return Err("Filter must start with @".to_string());
    }

    let ops = [(">=", FilterOp::Ge), ("<=", FilterOp::Le), ("!=", FilterOp::Ne),
               ("==", FilterOp::Eq), (">", FilterOp::Gt), ("<", FilterOp::Lt)];

    for (op_str, op) in &ops {
        if let Some(pos) = s.find(op_str) {
            let path_part = s[..pos].trim();
            let val_part = s[pos + op_str.len()..].trim();

            let path = path_part.strip_prefix('@').unwrap_or(path_part)
                .trim_start_matches('.')
                .to_string();

            let value = parse_filter_value(val_part)?;
            return Ok(FilterExpr::Compare { path, op: *op, value });
        }
    }

    let path = s.strip_prefix('@').unwrap_or(s)
        .trim_start_matches('.')
        .to_string();
    Ok(FilterExpr::Exists(path))
}

fn parse_filter_value(s: &str) -> Result<FilterValue, String> {
    let s = s.trim();
    if s == "null" || s == "nil" {
        return Ok(FilterValue::Null);
    }
    if s == "true" {
        return Ok(FilterValue::Bool(true));
    }
    if s == "false" {
        return Ok(FilterValue::Bool(false));
    }
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        return Ok(FilterValue::String(s[1..s.len()-1].to_string()));
    }
    if let Ok(n) = s.parse::<f64>() {
        return Ok(FilterValue::Number(n));
    }
    Err(format!("Invalid filter value: {}", s))
}

fn execute_tokens(tokens: &[Token], current: &mut Vec<Value>) -> Result<(), String> {
    for token in tokens {
        let mut next = Vec::new();
        for item in current.iter() {
            match token {
                Token::Dot(key) => {
                    if let Some(v) = item.get(key) {
                        next.push(v.clone());
                    }
                }
                Token::Bracket(content) => {
                    match content {
                        BracketContent::Wildcard => {
                            match item {
                                Value::Array(arr) => next.extend(arr.clone()),
                                Value::Object(obj) => next.extend(obj.values().cloned()),
                                _ => {}
                            }
                        }
                        BracketContent::Index(idx) => {
                            if let Some(v) = get_by_index(item, *idx) {
                                next.push(v);
                            }
                        }
                        BracketContent::Union(indices) => {
                            for idx in indices {
                                if let Some(v) = get_by_index(item, *idx) {
                                    next.push(v);
                                }
                            }
                        }
                        BracketContent::Slice(start, end) => {
                            if let Value::Array(arr) = item {
                                let len = arr.len() as isize;
                                let s = start.unwrap_or(0);
                                let e = end.unwrap_or(len);
                                let s = normalize_index(s, len);
                                let e = normalize_index(e, len);
                                let (s, e) = if s <= e { (s, e) } else { (e, s) };
                                for idx in s..e {
                                    if idx >= 0 && (idx as usize) < arr.len() {
                                        next.push(arr[idx as usize].clone());
                                    }
                                }
                            }
                        }
                        BracketContent::Filter(filter) => {
                            if let Value::Array(arr) = item {
                                for elem in arr {
                                    if evaluate_filter(elem, filter) {
                                        next.push(elem.clone());
                                    }
                                }
                            }
                        }
                        BracketContent::Name(name) => {
                            if let Some(v) = item.get(name) {
                                next.push(v.clone());
                            }
                        }
                    }
                }
                Token::RecursiveDescent(key) => {
                    collect_descendants(item, key, &mut next);
                }
            }
        }
        *current = next;
    }
    Ok(())
}

fn get_by_index(val: &Value, idx: isize) -> Option<Value> {
    match val {
        Value::Array(arr) => {
            let len = arr.len() as isize;
            let actual = if idx < 0 { len + idx } else { idx };
            if actual >= 0 && (actual as usize) < arr.len() {
                Some(arr[actual as usize].clone())
            } else {
                None
            }
        }
        Value::Object(obj) => {
            obj.get(&idx.to_string()).cloned()
        }
        _ => None,
    }
}

fn normalize_index(idx: isize, len: isize) -> isize {
    if idx < 0 { (len + idx).max(0) } else { idx.min(len) }
}

fn collect_descendants(val: &Value, key: &str, results: &mut Vec<Value>) {
    if key.is_empty() || key == "*" {
        if let Value::Object(obj) = val {
            for v in obj.values() {
                results.push(v.clone());
                collect_descendants(v, key, results);
            }
        } else if let Value::Array(arr) = val {
            for v in arr {
                results.push(v.clone());
                collect_descendants(v, key, results);
            }
        }
    } else {
        if let Value::Object(obj) = val {
            if let Some(v) = obj.get(key) {
                results.push(v.clone());
            }
            for v in obj.values() {
                collect_descendants(v, key, results);
            }
        } else if let Value::Array(arr) = val {
            for v in arr {
                collect_descendants(v, key, results);
            }
        }
    }
}

fn evaluate_filter(val: &Value, filter: &FilterExpr) -> bool {
    match filter {
        FilterExpr::Exists(path) => {
            navigate_path(val, path).is_some()
        }
        FilterExpr::Compare { path, op, value } => {
            if let Some(v) = navigate_path(val, path) {
                compare_values(&v, *op, value)
            } else {
                false
            }
        }
    }
}

fn navigate_path(val: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(val.clone());
    }
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = val.clone();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        if let Some(v) = current.get(part) {
            current = v.clone();
        } else {
            return None;
        }
    }
    Some(current)
}

fn compare_values(left: &Value, op: FilterOp, right: &FilterValue) -> bool {
    match (left, right) {
        (Value::Number(n), FilterValue::Number(r)) => {
            let l = n.as_f64().unwrap_or(0.0);
            match op {
                FilterOp::Eq => (l - r).abs() < f64::EPSILON,
                FilterOp::Ne => (l - r).abs() >= f64::EPSILON,
                FilterOp::Gt => l > *r,
                FilterOp::Ge => l >= *r,
                FilterOp::Lt => l < *r,
                FilterOp::Le => l <= *r,
            }
        }
        (Value::String(s), FilterValue::String(r)) => {
            match op {
                FilterOp::Eq => s == r,
                FilterOp::Ne => s != r,
                FilterOp::Gt => s.as_str() > r.as_str(),
                FilterOp::Ge => s.as_str() >= r.as_str(),
                FilterOp::Lt => s.as_str() < r.as_str(),
                FilterOp::Le => s.as_str() <= r.as_str(),
            }
        }
        (Value::Bool(b), FilterValue::Bool(r)) => {
            match op {
                FilterOp::Eq => b == r,
                FilterOp::Ne => b != r,
                _ => false,
            }
        }
        (Value::Null, FilterValue::Null) => {
            matches!(op, FilterOp::Eq)
        }
        (_, FilterValue::Null) => {
            matches!(op, FilterOp::Ne)
        }
        _ => false,
    }
}
