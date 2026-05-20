use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use regex::Regex;

pub struct SqlFormatter {
    input: String,
    output: String,
    error: String,
    uppercase: bool,
    indent_size: usize,
    dialect: usize, // 0=Standard, 1=MySQL, 2=PostgreSQL, 3=PL/SQL
    leading_comma: bool,
    pending_file: Pending<String>,
}

impl Default for SqlFormatter {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            uppercase: true,
            indent_size: 2,
            dialect: 0,
            leading_comma: false,
            pending_file: Pending::default(),
        }
    }
}

const DIALECTS: [&str; 4] = ["Standard", "MySQL", "PostgreSQL", "PL/SQL"];

impl Tool for SqlFormatter {
    fn name(&self) -> &str { "SQL Formatter" }
    fn description(&self) -> &str { "Format SQL data" }
    fn category(&self) -> ToolCategory { ToolCategory::Formatters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }
        let prev_input = self.input.clone();
        let prev_uppercase = self.uppercase;
        let prev_indent = self.indent_size;
        let prev_dialect = self.dialect;
        let prev_leading = self.leading_comma;

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = format!("Clipboard error: {}", e),
                        }
                    }
                    if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open SQL file", "SQL", &["sql"]);
                    }
                    if ui.button("Clear").clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label("Language:");
                    egui::ComboBox::from_id_salt("sql_dialect")
                        .selected_text(DIALECTS[self.dialect])
                        .show_ui(ui, |ui| {
                            for (i, name) in DIALECTS.iter().enumerate() {
                                ui.selectable_value(&mut self.dialect, i, *name);
                            }
                        });
                    ui.separator();
                    ui.label("Indent:");
                    ui.add(egui::DragValue::new(&mut self.indent_size).range(1..=8).speed(1));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.uppercase, "Uppercase keywords");
                    ui.checkbox(&mut self.leading_comma, "Leading comma");
                });
                ui.add_space(2.0);
                ui.label("Input SQL:");

                egui::ScrollArea::vertical()
                    .id_salt("sql_input_scroll")
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
                    if ui.button("Copy").clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.output.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "SQL", &["sql"], "formatted.sql") {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label("Output:");

                egui::ScrollArea::vertical()
                    .id_salt("sql_output_scroll")
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
        if self.input != prev_input || self.uppercase != prev_uppercase
            || self.indent_size != prev_indent || self.dialect != prev_dialect
            || self.leading_comma != prev_leading
        {
            self.format_sql();
        }
    }
}

impl SqlFormatter {
    fn format_sql(&mut self) {
        self.error.clear();
        if self.input.trim().is_empty() {
            self.output.clear();
            return;
        }
        self.output = format_sql(&self.input, self.uppercase, self.indent_size, self.dialect, self.leading_comma);
    }
}

fn get_keywords(dialect: usize) -> Vec<&'static str> {
    let mut kw = vec![
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "ON",
        "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS",
        "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
        "CREATE", "TABLE", "ALTER", "DROP", "INDEX",
        "UNION", "ALL", "DISTINCT", "AS", "LIKE", "BETWEEN",
        "IS", "NULL", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END",
        "ASC", "DESC", "WITH", "RECURSIVE", "TOP", "FETCH",
    ];
    match dialect {
        1 => {
            kw.extend(&["STRAIGHT_JOIN", "SQL_CALC_FOUND_ROWS", "FORCE INDEX", "IGNORE INDEX", "USE INDEX"]);
        }
        2 => {
            kw.extend(&["ILIKE", "SIMILAR TO", "LATERAL", "GENERATE_SERIES", "RETURNING"]);
        }
        3 => {
            kw.extend(&["DECLARE", "BEGIN", "END", "LOOP", "WHILE", "FOR", "CURSOR", "EXCEPTION", "RAISE"]);
        }
        _ => {}
    }
    kw
}

fn format_sql(sql: &str, uppercase: bool, indent_size: usize, dialect: usize, leading_comma: bool) -> String {
    let keywords = get_keywords(dialect);
    let mut result = sql.to_string();

    // Replace keywords (longest first to avoid partial matches)
    let mut sorted_kw = keywords.clone();
    sorted_kw.sort_by(|a, b| b.len().cmp(&a.len()));

    for kw in &sorted_kw {
        let re = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(kw))).unwrap();
        let replacement = if uppercase { kw.to_string() } else { kw.to_lowercase() };
        result = re.replace_all(&result, replacement.as_str()).to_string();
    }

    let major_keywords = ["SELECT", "FROM", "WHERE", "GROUP BY", "ORDER BY", "HAVING",
        "JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN", "OUTER JOIN", "FULL JOIN",
        "UNION", "INSERT", "UPDATE", "DELETE", "VALUES", "SET", "RETURNING"];

    for kw in &major_keywords {
        let pattern = if uppercase { kw.to_string() } else { kw.to_lowercase() };
        result = result.replace(&pattern, &format!("\n{}", pattern));
    }

    let indent = " ".repeat(indent_size);
    let lines: Vec<&str> = result.lines().collect();
    let mut formatted = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if i == 0 {
            formatted.push_str(trimmed);
        } else if !trimmed.is_empty() {
            formatted.push_str(&format!("\n{}{}", indent, trimmed));
        }
    }

    let mut output = formatted.trim().to_string();

    // Leading comma: move trailing commas to start of next line
    if leading_comma {
        output = apply_leading_comma(&output);
    }

    output
}

fn apply_leading_comma(sql: &str) -> String {
    let lines: Vec<&str> = sql.lines().collect();
    let mut result = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();

        if trimmed.ends_with(',') && i + 1 < lines.len() {
            // Remove trailing comma from this line
            let without_comma = trimmed.trim_end_matches(',');
            result.push_str(without_comma);
            result.push('\n');
            // Add leading comma to next line with same indentation
            let next = lines[i + 1].trim_end();
            let next_indent_len = if next.starts_with(' ') {
                next.len() - next.trim_start().len()
            } else {
                0
            };
            let next_indent = &lines[i + 1].trim_end()[..next_indent_len];
            result.push_str(next_indent);
            result.push_str(", ");
        } else {
            result.push_str(line);
            if i + 1 < lines.len() {
                result.push('\n');
            }
        }
    }

    result
}
