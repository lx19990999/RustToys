use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::Pending;

pub struct UuidGenerator {
    output: String,
    count: usize,
    uppercase: bool,
    with_hyphens: bool,
    version: usize, // 0=v1, 1=v4, 2=v7
    save_result: String,
    pending_file: Pending<String>,
}

impl Default for UuidGenerator {
    fn default() -> Self {
        Self {
            output: String::new(),
            count: 0,
            uppercase: false,
            with_hyphens: true,
            version: 1,
            save_result: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl UuidGenerator {
    fn do_generate(&mut self) {
        self.save_result.clear();
        let mut uuids = Vec::new();
        for _ in 0..self.count {
            let id = match self.version {
                0 => {
                    let ts = uuid::Timestamp::now(uuid::NoContext);
                    uuid::Uuid::new_v1(ts, &[0x01, 0x23, 0x45, 0x67, 0x89, 0xAB])
                }
                1 => uuid::Uuid::new_v4(),
                2 => uuid::Uuid::now_v7(),
                _ => uuid::Uuid::new_v4(),
            };

            let mut s = if self.with_hyphens {
                id.to_string()
            } else {
                id.simple().to_string()
            };

            if self.uppercase {
                s = s.to_uppercase();
            }
            uuids.push(s);
        }
        self.output = uuids.join("\n");
    }
}

impl Tool for UuidGenerator {
    fn name(&self) -> String { tr!("uuid_name") }
    fn description(&self) -> String { tr!("uuid_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Generators }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            self.save_result = text;
        }
        if self.count == 0 {
            self.count = 1;
            self.with_hyphens = true;
        }

        ui.horizontal(|ui| {
            ui.label(tr!("label_count"));
            ui.add(egui::DragValue::new(&mut self.count).range(1..=100).speed(1));
        });
        ui.add_space(4.0);

        let lbl_v1 = tr!("uuid_v1");
        let lbl_v4 = tr!("uuid_v4");
        let lbl_v7 = tr!("uuid_v7");
        ui.horizontal(|ui| {
            ui.label(tr!("uuid_version"));
            ui.radio_value(&mut self.version, 0, &lbl_v1);
            ui.radio_value(&mut self.version, 1, &lbl_v4);
            ui.radio_value(&mut self.version, 2, &lbl_v7);
        });
        ui.add_space(4.0);

        let lbl_upper = tr!("label_uppercase");
        let lbl_hyphens = tr!("uuid_with_hyphens");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.uppercase, &lbl_upper);
            ui.checkbox(&mut self.with_hyphens, &lbl_hyphens);
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let lbl_generate = tr!("btn_generate");
            if ui.button(lbl_generate).clicked() {
                self.do_generate();
            }
            let lbl_refresh = tr!("btn_refresh");
            if ui.button(lbl_refresh).clicked() {
                self.do_generate();
            }
        });

        ui.add_space(4.0);
        ui.label(tr!("label_output"));
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                let lbl_copy = tr!("btn_copy");
                if ui.button(lbl_copy).clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                let lbl_save_as = tr!("btn_save_as");
                if ui.button(lbl_save_as).clicked() {
                    let title = tr!("save_as_title");
                    let filter_text = tr!("save_filter_text");
                    let default_name = tr!("uuid_save_default");
                    crate::tools::async_utils::save_file_async(&mut self.pending_file, &title, &filter_text, &["txt"], &default_name, self.output.clone());
                }
            });
        }
        if !self.save_result.is_empty() {
            ui.colored_label(egui::Color32::GREEN, &self.save_result);
        }
    }
}
