use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;

pub struct Sidebar {
    pub search: String,
    pub selected_category: Option<ToolCategory>,
    pub selected_tool: usize,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected_category: None,
            selected_tool: 0,
        }
    }
}

impl Sidebar {
    pub fn show(&mut self, ui: &mut egui::Ui, tools: &mut [Box<dyn Tool>]) {
        let busy = tools.get(self.selected_tool).map_or(false, |t| t.is_busy());

        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(tr!("search_tools"))
                .desired_width(f32::INFINITY),
        );
        ui.separator();

        let search_lower = self.search.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for category in ToolCategory::all() {
                let tools_in_cat: Vec<(usize, String, String)> = tools
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.category() == *category)
                    .filter(|(_, t)| {
                        search_lower.is_empty()
                            || t.name().to_lowercase().contains(&search_lower)
                            || t.description().to_lowercase().contains(&search_lower)
                    })
                    .map(|(i, t)| (i, t.name(), t.description()))
                    .collect();

                if tools_in_cat.is_empty() {
                    continue;
                }

                let cat_label = category.label();
                let _is_selected = self.selected_category == Some(*category);

                ui.collapsing(cat_label, |ui| {
                    for (idx, name, desc) in tools_in_cat {
                        let selected = self.selected_tool == idx;
                        let response = if busy && !selected {
                            ui.add_enabled(false, egui::SelectableLabel::new(false, &name))
                        } else {
                            ui.selectable_label(selected, &name)
                        };
                        if !busy && response.clicked() {
                            self.selected_tool = idx;
                            self.selected_category = Some(*category);
                        }
                        response.on_hover_text(if busy && !selected {
                            tr!("locked_hover", &desc)
                        } else {
                            desc
                        });
                    }
                });
            }
        });
    }
}
