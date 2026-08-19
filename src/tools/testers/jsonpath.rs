use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use crate::tools::io_layout;
use serde_json::Value;

pub struct JsonPathTester {
    json_input: String,
    path_input: String,
    output: String,
    /// Clipboard / file I/O messages.
    status_error: String,
    json_error: String,
    path_error: String,
    prev_json: String,
    prev_path: String,
    match_count: usize,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for JsonPathTester {
    fn default() -> Self {
        Self {
            json_input: String::new(),
            path_input: String::new(),
            output: String::new(),
            status_error: String::new(),
            json_error: String::new(),
            path_error: String::new(),
            prev_json: String::new(),
            prev_path: String::new(),
            match_count: 0,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl JsonPathTester {
    fn do_evaluate(&mut self) {
        self.json_error.clear();
        self.path_error.clear();
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
                            self.output = tr!("jp_no_match");
                        } else {
                            let formatted: Vec<String> = results.iter()
                                .map(|v| serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()))
                                .collect();
                            self.output = formatted.join("\n");
                        }
                    }
                    Err(e) => self.path_error = e,
                }
            }
            Err(e) => self.json_error = tr!("jy_json_parse_error", e),
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
    fn name(&self) -> String { tr!("jp_name") }
    fn description(&self) -> String { tr!("jp_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Testers }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let err_reading = tr!("err_error_reading");
        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            crate::tools::async_utils::open_dropped_text_async(&mut self.pending_file, path);
        }
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&err_reading) {
                self.json_input = text;
                self.status_error.clear();
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.status_error = text;
        }

        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_json = tr!("jp_json_label");
        let lbl_result = tr!("jp_result_label");
        let lbl_match_plural = tr!("jp_match_plural");

        let cols_h = (ui.available_height() * 0.45).max(120.0);
        let opt_h = io_layout::option_row_height(ui);
        io_layout::error_slot(ui, &self.status_error, 1);
        io_layout::error_slot(ui, &self.json_error, 2);
        io_layout::two_column_io_with_height(ui, cols_h, |ui, w, col| match col {
            io_layout::IoColumn::Left => {
                ui.label(egui::RichText::new(&lbl_json).strong());
                ui.add_space(io_layout::ROW_GAP);
                ui.horizontal(|ui| {
                    if ui.button(&lbl_paste).clicked() {
                        match crate::clipboard::read_text() {
                            Ok(text) => {
                                self.json_input = text;
                                self.status_error.clear();
                            }
                            Err(e) => self.status_error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(&lbl_open).clicked() {
                        open_file_async(
                            &mut self.pending_file,
                            &tr!("btn_open_file"),
                            &tr!("jp_json_label"),
                            &["json"],
                        );
                    }
                    if ui.button(&lbl_clear).clicked() {
                        self.json_input.clear();
                        self.output.clear();
                        self.status_error.clear();
                        self.json_error.clear();
                        self.path_error.clear();
                        self.match_count = 0;
                    }
                });
                io_layout::row_spacer(ui, opt_h);
                io_layout::multiline_field(ui, w, "jp_json_scroll", &mut self.json_input);
            }
            io_layout::IoColumn::Right => {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&lbl_result).strong());
                    if self.match_count > 0 {
                        ui.label(
                            egui::RichText::new(tr!(
                                "jp_match_count",
                                self.match_count,
                                if self.match_count == 1 { "" } else { &lbl_match_plural }
                            ))
                            .small()
                            .color(egui::Color32::GRAY),
                        );
                    }
                });
                ui.add_space(io_layout::ROW_GAP);
                ui.horizontal(|ui| {
                    if ui.button(&lbl_copy).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() && !self.output.is_empty() {
                        crate::tools::async_utils::save_file_async(
                            &mut self.save_pending,
                            &tr!("save_as_title"),
                            &tr!("jp_json_label"),
                            &["json"],
                            &tr!("jp_save_default"),
                            self.output.clone(),
                        );
                    }
                });
                io_layout::row_spacer(ui, opt_h);
                io_layout::multiline_field(ui, w, "jp_output_scroll", &mut self.output);
            }
        });

        ui.add_space(4.0);
        let lbl_jp_query = tr!("jp_query_label");
        let lbl_jp_hint = tr!("jp_hint");
        ui.horizontal(|ui| {
            ui.label(&lbl_jp_query);
            let path_response = ui.add(
                egui::TextEdit::singleline(&mut self.path_input)
                    .id_salt("jp_path_input")
                    .desired_width(ui.available_width())
                    .hint_text(&lbl_jp_hint),
            );
            let keep_path_focus = path_response.has_focus();
            self.auto_evaluate();
            if keep_path_focus {
                path_response.request_focus();
            }
        });
        io_layout::error_slot(ui, &self.path_error, 1);

        let lbl_cheatsheet = tr!("jp_cheatsheet");
        ui.group(|ui| {
            ui.label(egui::RichText::new(&lbl_cheatsheet).strong());
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
    }
}

// --- JSONPath Evaluator ---

fn evaluate_jsonpath(root: &Value, path: &str) -> Result<Vec<Value>, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err(tr!("jp_empty_expr"));
    }
    if !path.starts_with('$') {
        return Err(tr!("jp_must_start_dollar"));
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
