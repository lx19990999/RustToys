use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use crate::i18n::Language;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn label(&self) -> String {
        let key = match self {
            Self::Light => "theme_light",
            Self::Dark => "theme_dark",
            Self::System => "theme_system",
        };
        crate::i18n::tr(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub dpi: f32,
    #[serde(default)]
    pub lastsavefolder: Option<String>,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub autostart: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            dpi: 0.0,
            lastsavefolder: None,
            language: Language::default(),
            autostart: false,
        }
    }
}

impl Config {
    fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|mut p| {
            p.push(".config");
            p.push("rusttoys.json");
            p
        })
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, text);
        }
    }
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

pub fn init() {
    let cfg = Config::load();
    crate::i18n::init(cfg.language);
    let mut guard = CONFIG.lock().unwrap();
    *guard = Some(cfg);
}

pub fn get() -> Config {
    let guard = CONFIG.lock().unwrap();
    guard.clone().unwrap_or_default()
}

pub fn update<F: FnOnce(&mut Config)>(f: F) {
    let mut guard = CONFIG.lock().unwrap();
    if let Some(ref mut cfg) = *guard {
        f(cfg);
        cfg.save();
    }
}

pub fn get_save_folder() -> Option<PathBuf> {
    let guard = CONFIG.lock().unwrap();
    guard.as_ref().and_then(|c| c.lastsavefolder.as_ref().map(PathBuf::from))
}

pub fn set_save_folder(path: &std::path::Path) {
    if let Some(dir) = path.parent() {
        update(|cfg| {
            cfg.lastsavefolder = Some(dir.to_string_lossy().to_string());
        });
    }
}
