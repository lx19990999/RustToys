use eframe::egui;
use crate::tr;

pub trait Tool {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn category(&self) -> ToolCategory;
    fn ui(&mut self, ui: &mut egui::Ui);
    fn is_busy(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Converters,
    Encoders,
    Encryption,
    Formatters,
    Generators,
    Graphic,
    Testers,
    Text,
}

impl ToolCategory {
    pub fn label(&self) -> String {
        let key = match self {
            Self::Converters => "cat_converters",
            Self::Encoders => "cat_encoders",
            Self::Encryption => "cat_encryption",
            Self::Formatters => "cat_formatters",
            Self::Generators => "cat_generators",
            Self::Graphic => "cat_graphic",
            Self::Testers => "cat_testers",
            Self::Text => "cat_text",
        };
        tr!(key)
    }

    pub fn all() -> &'static [ToolCategory] {
        &[
            Self::Converters,
            Self::Encoders,
            Self::Encryption,
            Self::Formatters,
            Self::Generators,
            Self::Graphic,
            Self::Testers,
            Self::Text,
        ]
    }
}
