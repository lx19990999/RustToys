use eframe::egui;

pub trait Tool {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
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
    pub fn label(&self) -> &'static str {
        match self {
            Self::Converters => "Converters",
            Self::Encoders => "Encoders / Decoders",
            Self::Encryption => "加密/解密",
            Self::Formatters => "Formatters",
            Self::Generators => "Generators",
            Self::Graphic => "Graphic",
            Self::Testers => "Testers",
            Self::Text => "Text",
        }
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
