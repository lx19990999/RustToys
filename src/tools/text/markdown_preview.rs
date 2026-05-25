use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use crate::tools::io_layout;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub struct MarkdownPreview {
    input: String,
    html_output: String,
    theme: usize, // 0=GitHub Light, 1=GitHub Dark
    sync_scroll: bool,
    scroll_ratio: f32,
    left_content_h: f32,
    left_viewport_h: f32,
    right_content_h: f32,
    right_viewport_h: f32,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
    save_result: String,
}

impl Default for MarkdownPreview {
    fn default() -> Self {
        Self {
            input: String::new(),
            html_output: String::new(),
            theme: 0,
            sync_scroll: false,
            scroll_ratio: 0.0,
            left_content_h: 0.0,
            left_viewport_h: 0.0,
            right_content_h: 0.0,
            right_viewport_h: 0.0,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
            save_result: String::new(),
        }
    }
}

impl Tool for MarkdownPreview {
    fn name(&self) -> String { tr!("md_name") }
    fn description(&self) -> String { tr!("md_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Text }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let err_reading = tr!("err_error_reading");
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&err_reading) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.save_result = text;
        }

        self.update_html();

        let lbl_theme = tr!("md_theme_label");
        let lbl_github_light = tr!("md_github_light");
        let lbl_github_dark = tr!("md_github_dark");
        let lbl_sync_scroll = tr!("md_sync_scroll");
        ui.horizontal(|ui| {
            ui.label(&lbl_theme);
            ui.radio_value(&mut self.theme, 0, &lbl_github_light);
            ui.radio_value(&mut self.theme, 1, &lbl_github_dark);
            ui.separator();
            ui.checkbox(&mut self.sync_scroll, &lbl_sync_scroll);
        });
        ui.add_space(4.0);

        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_markdown = tr!("md_markdown_label");
        let lbl_preview = tr!("md_preview_label");
        let lbl_copy_html = tr!("btn_copy_html");
        let lbl_save_html = tr!("btn_save_html");

        let body_h = ui.available_height().max(120.0);
        let label_h = ui.text_style_height(&egui::TextStyle::Body) + io_layout::ROW_GAP;
        let opt_h = io_layout::option_row_height(ui);

        io_layout::two_column_io_with_height(ui, body_h, |ui, w, col| match col {
            io_layout::IoColumn::Left => {
                ui.label(egui::RichText::new(&lbl_markdown).strong());
                ui.add_space(io_layout::ROW_GAP);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(&lbl_paste).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.html_output = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(&lbl_open).clicked() {
                        open_file_async(
                            &mut self.pending_file,
                            &tr!("md_open_title"),
                            &tr!("md_filter_md"),
                            &["md", "markdown"],
                        );
                    }
                    if ui.button(&lbl_clear).clicked() {
                        self.input.clear();
                    }
                    if ui.button(&lbl_copy).clicked() && !self.input.is_empty() {
                        ui.ctx().copy_text(self.input.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() && !self.input.is_empty() {
                        crate::tools::async_utils::save_file_async(
                            &mut self.save_pending,
                            &tr!("save_as_title"),
                            &tr!("md_filter_md"),
                            &["md"],
                            &tr!("md_save_default"),
                            self.input.clone(),
                        );
                    }
                });
                io_layout::row_spacer(ui, opt_h);
                let text_h = (body_h - label_h - opt_h).max(40.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(w, text_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_width(w);
                        ui.set_max_width(w);
                        ui.set_height(text_h);
                        ui.set_max_height(text_h);
                        let mut scroll = egui::ScrollArea::vertical()
                            .id_salt("md_input_scroll")
                            .auto_shrink([false, false]);
                        if self.sync_scroll {
                            scroll = scroll.vertical_scroll_offset(md_offset_from_ratio(
                                self.scroll_ratio,
                                self.left_content_h,
                                self.left_viewport_h,
                            ));
                        }
                        let out = scroll.show(ui, |ui| {
                            ui.set_width(w);
                            ui.add(
                                egui::TextEdit::multiline(&mut self.input)
                                    .desired_width(w)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                        if self.sync_scroll {
                            self.left_content_h = out.content_size.y;
                            self.left_viewport_h = out.inner_rect.height();
                            md_update_scroll_ratio(
                                &mut self.scroll_ratio,
                                out.state.offset.y,
                                out.content_size.y,
                                out.inner_rect.height(),
                            );
                        }
                    },
                );
            }
            io_layout::IoColumn::Right => {
                ui.label(egui::RichText::new(&lbl_preview).strong());
                ui.add_space(io_layout::ROW_GAP);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(&lbl_copy_html).clicked() && !self.html_output.is_empty() {
                        ui.ctx().copy_text(self.html_output.clone());
                    }
                    if ui.button(&lbl_save_html).clicked() && !self.html_output.is_empty() {
                        crate::tools::async_utils::save_file_async(
                            &mut self.save_pending,
                            &tr!("save_as_title"),
                            &tr!("md_filter_html"),
                            &["html"],
                            &tr!("md_save_html"),
                            self.html_output.clone(),
                        );
                    }
                    if !self.save_result.is_empty() {
                        ui.colored_label(egui::Color32::from_rgb(0, 180, 0), &self.save_result);
                    }
                });
                io_layout::row_spacer(ui, opt_h);
                let preview_h = (body_h - label_h - opt_h).max(40.0);
                let is_dark = self.theme == 1;
                ui.allocate_ui_with_layout(
                    egui::vec2(w, preview_h),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_width(w);
                        ui.set_max_width(w);
                        ui.set_height(preview_h);
                        ui.set_max_height(preview_h);
                        let mut scroll = egui::ScrollArea::vertical()
                            .id_salt("md_preview_scroll")
                            .auto_shrink([false, false]);
                        if self.sync_scroll {
                            scroll = scroll.vertical_scroll_offset(md_offset_from_ratio(
                                self.scroll_ratio,
                                self.right_content_h,
                                self.right_viewport_h,
                            ));
                        }
                        let out = scroll.show(ui, |ui| {
                            ui.set_width(w);
                            render_markdown(ui, &self.input, is_dark);
                        });
                        if self.sync_scroll {
                            self.right_content_h = out.content_size.y;
                            self.right_viewport_h = out.inner_rect.height();
                            md_update_scroll_ratio(
                                &mut self.scroll_ratio,
                                out.state.offset.y,
                                out.content_size.y,
                                out.inner_rect.height(),
                            );
                        }
                    },
                );
            }
        });
    }
}

#[derive(Clone, Copy)]
struct MdColors {
    heading: egui::Color32,
    body: egui::Color32,
    code_bg: egui::Color32,
    quote: egui::Color32,
    link: egui::Color32,
}

impl MdColors {
    fn new(is_dark: bool) -> Self {
        if is_dark {
            Self {
                heading: egui::Color32::WHITE,
                body: egui::Color32::from_rgb(200, 200, 200),
                code_bg: egui::Color32::from_rgb(40, 44, 52),
                quote: egui::Color32::from_rgb(120, 130, 140),
                link: egui::Color32::from_rgb(88, 166, 255),
            }
        } else {
            Self {
                heading: egui::Color32::BLACK,
                body: egui::Color32::from_rgb(50, 50, 50),
                code_bg: egui::Color32::from_rgb(246, 248, 250),
                quote: egui::Color32::from_rgb(100, 110, 120),
                link: egui::Color32::from_rgb(9, 105, 218),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum InlineMark {
    Normal,
    Strong,
    Emphasis,
    Code,
    Strike,
    Link,
}

struct InlineBuffer {
    marks: Vec<InlineMark>,
    buf: String,
    spans: Vec<egui::RichText>,
    colors: MdColors,
}

impl InlineBuffer {
    fn new(colors: MdColors) -> Self {
        Self {
            marks: vec![InlineMark::Normal],
            buf: String::new(),
            spans: Vec::new(),
            colors,
        }
    }

    fn current_mark(&self) -> InlineMark {
        *self.marks.last().unwrap_or(&InlineMark::Normal)
    }

    fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    fn flush_span(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.buf);
        let rt = match self.current_mark() {
            InlineMark::Normal => egui::RichText::new(text).color(self.colors.body),
            InlineMark::Strong => egui::RichText::new(text).strong().color(self.colors.body),
            InlineMark::Emphasis => egui::RichText::new(text).italics().color(self.colors.body),
            InlineMark::Code => egui::RichText::new(text)
                .monospace()
                .background_color(self.colors.code_bg)
                .color(self.colors.body),
            InlineMark::Strike => egui::RichText::new(text).strikethrough().color(self.colors.body),
            InlineMark::Link => egui::RichText::new(text).underline().color(self.colors.link),
        };
        self.spans.push(rt);
    }

    fn flush_line(&mut self, ui: &mut egui::Ui) {
        self.flush_span();
        if self.spans.is_empty() {
            return;
        }
        let spans = std::mem::take(&mut self.spans);
        ui.horizontal_wrapped(|ui| {
            for span in spans {
                ui.label(span);
            }
        });
    }

    fn push_mark(&mut self, mark: InlineMark) {
        self.flush_span();
        self.marks.push(mark);
    }

    fn pop_mark(&mut self, mark: InlineMark) {
        self.flush_span();
        if self.marks.len() > 1 {
            if self.marks.pop() != Some(mark) {
                if self.marks.last() != Some(&InlineMark::Normal) {
                    let _ = self.marks.pop();
                }
            }
        }
        if self.marks.is_empty() {
            self.marks.push(InlineMark::Normal);
        }
    }
}

fn parser_options() -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts
}

fn heading_size(level: HeadingLevel) -> f32 {
    match level {
        HeadingLevel::H1 => 28.0,
        HeadingLevel::H2 => 22.0,
        HeadingLevel::H3 => 18.0,
        HeadingLevel::H4 => 16.0,
        HeadingLevel::H5 => 15.0,
        HeadingLevel::H6 => 14.0,
    }
}

fn render_markdown(ui: &mut egui::Ui, md: &str, is_dark: bool) {
    let colors = MdColors::new(is_dark);
    let mut inline = InlineBuffer::new(colors);
    let mut list_depth: usize = 0;
    let mut in_codeblock = false;
    let mut code_buf = String::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row: Vec<String> = Vec::new();
    let mut table_cell = String::new();
    let mut in_table = false;
    let mut table_id: usize = 0;
    let mut heading_level: Option<HeadingLevel> = None;

    for event in Parser::new_ext(md, parser_options()) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading { level, .. } => {
                    inline.flush_line(ui);
                    heading_level = Some(level);
                }
                Tag::BlockQuote => {
                    inline.flush_line(ui);
                }
                Tag::CodeBlock(kind) => {
                    inline.flush_line(ui);
                    in_codeblock = true;
                    code_buf.clear();
                    if let CodeBlockKind::Fenced(lang) = kind {
                        if !lang.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("{lang}"))
                                    .small()
                                    .monospace()
                                    .color(colors.quote),
                            );
                        }
                    }
                }
                Tag::List(_) => {
                    inline.flush_line(ui);
                    list_depth += 1;
                }
                Tag::Item => {
                    inline.flush_line(ui);
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    ui.label(egui::RichText::new(format!("{indent}• ")).color(colors.body));
                }
                Tag::Emphasis => inline.push_mark(InlineMark::Emphasis),
                Tag::Strong => inline.push_mark(InlineMark::Strong),
                Tag::Strikethrough => inline.push_mark(InlineMark::Strike),
                Tag::Link { .. } => inline.push_mark(InlineMark::Link),
                Tag::Table(_) => {
                    inline.flush_line(ui);
                    in_table = true;
                    table_rows.clear();
                }
                Tag::TableHead => {}
                Tag::TableRow => table_row.clear(),
                Tag::TableCell => table_cell.clear(),
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    if in_table {
                        // cell handled separately
                    } else if heading_level.is_some() {
                        inline.flush_span();
                        if let Some(level) = heading_level.take() {
                            let text: String = inline
                                .spans
                                .iter()
                                .map(|r| r.text().to_string())
                                .collect::<Vec<_>>()
                                .join("");
                            inline.spans.clear();
                            if !text.is_empty() {
                                ui.label(
                                    egui::RichText::new(text)
                                        .strong()
                                        .color(colors.heading)
                                        .size(heading_size(level)),
                                );
                            }
                        }
                    } else {
                        inline.flush_line(ui);
                    }
                }
                TagEnd::Heading(level) => {
                    inline.flush_span();
                    let text: String = inline
                        .spans
                        .drain(..)
                        .map(|r| r.text().to_string())
                        .collect::<Vec<_>>()
                        .join("");
                    if !text.is_empty() {
                        ui.label(
                            egui::RichText::new(text)
                                .strong()
                                .color(colors.heading)
                                .size(heading_size(level)),
                        );
                    }
                    heading_level = None;
                }
                TagEnd::CodeBlock => {
                    in_codeblock = false;
                    if !code_buf.is_empty() {
                        ui.label(
                            egui::RichText::new(code_buf.trim_end())
                                .monospace()
                                .background_color(colors.code_bg)
                                .color(colors.body),
                        );
                        code_buf.clear();
                    }
                    ui.add_space(4.0);
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    ui.add_space(2.0);
                }
                TagEnd::Item => inline.flush_line(ui),
                TagEnd::Emphasis => inline.pop_mark(InlineMark::Emphasis),
                TagEnd::Strong => inline.pop_mark(InlineMark::Strong),
                TagEnd::Strikethrough => inline.pop_mark(InlineMark::Strike),
                TagEnd::Link => inline.pop_mark(InlineMark::Link),
                TagEnd::Table => {
                    if !table_rows.is_empty() {
                        render_table(ui, &table_rows, &colors, table_id);
                        table_id += 1;
                    }
                    in_table = false;
                }
                TagEnd::TableHead => {}
                TagEnd::TableRow => {
                    if !table_row.is_empty() {
                        table_rows.push(table_row.clone());
                    }
                    table_row.clear();
                }
                TagEnd::TableCell => {
                    table_row.push(table_cell.trim().to_string());
                    table_cell.clear();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_codeblock {
                    code_buf.push_str(&text);
                } else if in_table {
                    table_cell.push_str(&text);
                } else {
                    inline.push_str(&text);
                }
            }
            Event::Code(text) => {
                inline.push_mark(InlineMark::Code);
                inline.push_str(&text);
                inline.pop_mark(InlineMark::Code);
            }
            Event::SoftBreak => inline.push_str(" "),
            Event::HardBreak => inline.flush_line(ui),
            Event::Rule => {
                inline.flush_line(ui);
                ui.separator();
            }
            Event::Html(_) | Event::InlineHtml(_) => {}
            _ => {}
        }
    }
    inline.flush_line(ui);
}

fn md_scroll_max(content_h: f32, viewport_h: f32) -> f32 {
    (content_h - viewport_h).max(0.0)
}

fn md_offset_from_ratio(ratio: f32, content_h: f32, viewport_h: f32) -> f32 {
    ratio.clamp(0.0, 1.0) * md_scroll_max(content_h, viewport_h)
}

fn md_update_scroll_ratio(ratio: &mut f32, offset_y: f32, content_h: f32, viewport_h: f32) {
    let max = md_scroll_max(content_h, viewport_h);
    if max <= 0.0 {
        return;
    }
    let actual = (offset_y / max).clamp(0.0, 1.0);
    if (actual - *ratio).abs() > 0.002 {
        *ratio = actual;
    }
}

fn render_table(ui: &mut egui::Ui, rows: &[Vec<String>], colors: &MdColors, table_id: usize) {
    if rows.is_empty() {
        return;
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
    ui.add_space(4.0);
    egui::Grid::new(egui::Id::new("md_table").with(table_id))
        .num_columns(ncols)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for (ri, row) in rows.iter().enumerate() {
                for ci in 0..ncols {
                    let cell = row.get(ci).map(|s| s.as_str()).unwrap_or("");
                    if ri == 0 {
                        ui.label(egui::RichText::new(cell).strong().color(colors.heading));
                    } else {
                        ui.label(egui::RichText::new(cell).color(colors.body));
                    }
                }
                ui.end_row();
            }
        });
    ui.add_space(4.0);
}

impl MarkdownPreview {
    fn update_html(&mut self) {
        let parser = Parser::new_ext(&self.input, parser_options());
        let mut html_buf = String::new();
        pulldown_cmark::html::push_html(&mut html_buf, parser);
        self.html_output = html_buf;
    }
}
