use std::sync::mpsc;
use std::path::PathBuf;
use eframe::egui;

/// A pending async result that can be polled each frame.
pub struct Pending<T> {
    rx: Option<mpsc::Receiver<T>>,
}

impl<T> Default for Pending<T> {
    fn default() -> Self {
        Self { rx: None }
    }
}

impl<T> Pending<T> {
    /// Set up a receiver channel. Call this when spawning a background task.
    pub fn set_receiver(&mut self, rx: mpsc::Receiver<T>) {
        self.rx = Some(rx);
    }

    /// Poll for the latest result. Returns Some if a result is available.
    /// Drains all pending values and returns only the last one.
    pub fn poll(&self) -> Option<T> {
        let rx = self.rx.as_ref()?;
        let mut last = None;
        while let Ok(val) = rx.try_recv() {
            last = Some(val);
        }
        last
    }

    /// Check if a task is in progress.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }
}

/// Open a file dialog (blocking) then read file content in a background thread.
/// Returns immediately. Results arrive via Pending::poll().
pub fn open_file_async(
    pending: &mut Pending<String>,
    title: &str,
    filter_name: &str,
    extensions: &[&str],
) {
    let path = rfd::FileDialog::new()
        .set_title(title)
        .add_filter(filter_name, extensions)
        .pick_file();
    if let Some(path) = path {
        let (tx, rx) = mpsc::channel();
        pending.set_receiver(rx);
        std::thread::spawn(move || {
            match std::fs::read_to_string(&path) {
                Ok(text) => { let _ = tx.send(text); }
                Err(e) => { let _ = tx.send(format!("Error reading file: {}", e)); }
            }
        });
    }
}

/// Take the first dropped file path from egui input, if any.
pub fn take_dropped_file(ctx: &egui::Context) -> Option<PathBuf> {
    ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()))
}

/// Read a dropped file as text in a background thread (for text-based tools).
pub fn open_dropped_text_async(pending: &mut Pending<String>, path: PathBuf) {
    let (tx, rx) = mpsc::channel();
    pending.set_receiver(rx);
    std::thread::spawn(move || {
        match std::fs::read_to_string(&path) {
            Ok(text) => { let _ = tx.send(text); }
            Err(e) => { let _ = tx.send(format!("Error reading file: {}", e)); }
        }
    });
}

/// Read a dropped file as raw bytes in a background thread (for binary tools).
pub fn open_dropped_binary_async(pending: &mut Pending<Vec<u8>>, path: PathBuf) {
    let (tx, rx) = mpsc::channel();
    pending.set_receiver(rx);
    std::thread::spawn(move || {
        match std::fs::read(&path) {
            Ok(bytes) => { let _ = tx.send(bytes); }
            Err(_) => { let _ = tx.send(Vec::new()); }
        }
    });
}


/// Config-aware save file dialog. Uses lastsavefolder from config as initial
/// directory, and records the chosen path back to config.
/// The default filename gets a yyyyMMddHHmmss timestamp appended to avoid overwrites.
pub fn save_file_dialog(
    title: &str,
    filter_name: &str,
    extensions: &[&str],
    default_name: &str,
) -> Option<PathBuf> {
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let stamped_name = if let Some(dot_pos) = default_name.rfind('.') {
        format!("{}_{}.{}", &default_name[..dot_pos], timestamp, &default_name[dot_pos + 1..])
    } else {
        format!("{}_{}", default_name, timestamp)
    };
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .add_filter(filter_name, extensions)
        .add_filter("All files", &["*"])
        .set_file_name(&stamped_name);
    if let Some(dir) = crate::config::get_save_folder() {
        dialog = dialog.set_directory(&dir);
    }
    let path = dialog.save_file();
    if let Some(ref p) = path {
        crate::config::set_save_folder(p);
    }
    path
}

/// Async save: show dialog (blocking), then write file in a background thread.
/// `data` is a String (moved into the closure, no clone on the main thread).
pub fn save_file_async(
    pending: &mut Pending<String>,
    title: &str,
    filter_name: &str,
    extensions: &[&str],
    default_name: &str,
    data: String,
) {
    if let Some(path) = save_file_dialog(title, filter_name, extensions, default_name) {
        let (tx, rx) = mpsc::channel();
        pending.set_receiver(rx);
        std::thread::spawn(move || {
            match std::fs::write(&path, data) {
                Ok(_) => { let _ = tx.send(format!("Saved: {}", path.display())); }
                Err(e) => { let _ = tx.send(format!("Save failed: {}", e)); }
            }
        });
    }
}

/// Async save for binary data (Vec<u8>), e.g. images.
pub fn save_file_binary_async(
    pending: &mut Pending<String>,
    title: &str,
    filter_name: &str,
    extensions: &[&str],
    default_name: &str,
    data: Vec<u8>,
) {
    if let Some(path) = save_file_dialog(title, filter_name, extensions, default_name) {
        let (tx, rx) = mpsc::channel();
        pending.set_receiver(rx);
        std::thread::spawn(move || {
            match std::fs::write(&path, data) {
                Ok(_) => { let _ = tx.send(format!("Saved: {}", path.display())); }
                Err(e) => { let _ = tx.send(format!("Save failed: {}", e)); }
            }
        });
    }
}
