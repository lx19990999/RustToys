use eframe::egui;
use crate::tool::{Tool, ToolCategory};

#[derive(Default)]
pub struct UuidGenerator {
    output: String,
    count: usize,
    uppercase: bool,
    with_hyphens: bool,
    version: usize, // 0=v1, 1=v4, 2=v7
}

impl UuidGenerator {
    fn do_generate(&mut self) {
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
    fn name(&self) -> &str { "UUID Generator" }
    fn description(&self) -> &str { "Generate UUIDs version 1, 4 (GUID) and 7" }
    fn category(&self) -> ToolCategory { ToolCategory::Generators }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if self.count == 0 {
            self.count = 1;
            self.with_hyphens = true;
        }

        ui.horizontal(|ui| {
            ui.label("Count:");
            ui.add(egui::DragValue::new(&mut self.count).range(1..=100).speed(1));
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Version:");
            ui.radio_value(&mut self.version, 0, "v1 (time-based)");
            ui.radio_value(&mut self.version, 1, "v4 (random)");
            ui.radio_value(&mut self.version, 2, "v7 (Unix time-based)");
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.uppercase, "Uppercase");
            ui.checkbox(&mut self.with_hyphens, "With hyphens");
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("Generate").clicked() {
                self.do_generate();
            }
            if ui.button("Refresh").clicked() {
                self.do_generate();
            }
        });

        ui.add_space(4.0);
        ui.label("Output:");
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button("Save As...").clicked() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "uuids.txt") {
                        let _ = std::fs::write(path, &self.output);
                    }
                }
            });
        }
    }
}
