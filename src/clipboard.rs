//! OS clipboard read helper.
//!
//! On Linux Wayland, `arboard` alone often fails while the windowing stack
//! (egui-winit) reads via `smithay-clipboard`. Paste buttons use this module
//! so behaviour matches Ctrl+V in text fields.

use std::sync::Mutex;

#[cfg(not(target_os = "android"))]
use arboard::Clipboard;

#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "macos", target_os = "ios"))
))]
use arboard::{GetExtLinux, LinuxClipboardKind};

static CLIPBOARD: Mutex<ClipboardState> = Mutex::new(ClipboardState::empty());

struct ClipboardState {
    #[cfg(not(target_os = "android"))]
    arboard: Option<Clipboard>,
    #[cfg(target_os = "linux")]
    smithay: Option<smithay_clipboard::Clipboard>,
}

/// RGBA8 pixel data from the clipboard (width × height × 4 bytes).
pub struct ClipboardImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl ClipboardState {
    const fn empty() -> Self {
        Self {
            #[cfg(not(target_os = "android"))]
            arboard: None,
            #[cfg(target_os = "linux")]
            smithay: None,
        }
    }
}

/// Call once at startup (e.g. from [`eframe::CreationContext`]).
pub fn init(display_handle: Option<raw_window_handle::RawDisplayHandle>) {
    let mut state = CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());

    #[cfg(not(target_os = "android"))]
    {
        state.arboard = Clipboard::new().ok();
    }

    #[cfg(target_os = "linux")]
    if let Some(raw_window_handle::RawDisplayHandle::Wayland(display)) = display_handle {
        // SAFETY: display pointer comes from winit/eframe and outlives the app.
        state.smithay =
            Some(unsafe { smithay_clipboard::Clipboard::new(display.display.as_ptr()) });
    }
}

/// Read plain text from the system clipboard.
pub fn read_text() -> Result<String, String> {
    let mut state = CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());

    #[cfg(target_os = "linux")]
    if let Some(clipboard) = state.smithay.as_mut() {
        if let Ok(text) = clipboard.load() {
            if !text.is_empty() {
                return Ok(normalize(text));
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    if let Some(clipboard) = state.arboard.as_mut() {
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                return Ok(normalize(text));
            }
        }
        #[cfg(all(
            unix,
            not(any(target_os = "android", target_os = "macos", target_os = "ios"))
        ))]
        if let Ok(text) = clipboard
            .get()
            .clipboard(LinuxClipboardKind::Primary)
            .text()
        {
            if !text.is_empty() {
                return Ok(normalize(text));
            }
        }
    }

    #[cfg(unix)]
    if let Some(text) = read_text_via_command() {
        if !text.is_empty() {
            return Ok(normalize(text));
        }
    }

    #[cfg(not(target_os = "android"))]
    if let Some(clipboard) = state.arboard.as_mut() {
        return clipboard
            .get_text()
            .map(normalize)
            .map_err(|e| e.to_string());
    }

    Err(empty_clipboard_message())
}

/// Read an image from the system clipboard as RGBA8 pixels.
pub fn read_image() -> Result<ClipboardImage, String> {
    let mut state = CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());

    #[cfg(not(target_os = "android"))]
    if let Some(clipboard) = state.arboard.as_mut() {
        if let Ok(img) = clipboard.get_image() {
            return Ok(ClipboardImage {
                width: img.width,
                height: img.height,
                rgba: img.bytes.into_owned(),
            });
        }
    }

    #[cfg(unix)]
    if let Some(img) = read_image_via_command() {
        return Ok(img);
    }

    Err(empty_clipboard_message())
}

fn empty_clipboard_message() -> String {
    "The clipboard contents were not available in the requested format or the clipboard is empty."
        .to_string()
}

fn normalize(text: String) -> String {
    text.replace("\r\n", "\n")
}

#[cfg(unix)]
fn read_text_via_command() -> Option<String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        run_command_text(&["wl-paste", "--no-newline"])
    } else if cfg!(target_os = "macos") {
        run_command_text(&["pbpaste"])
    } else {
        run_command_text(&["xclip", "-o", "-selection", "clipboard"])
            .or_else(|| run_command_text(&["xsel", "--clipboard", "--output"]))
    }
}

#[cfg(unix)]
fn read_image_via_command() -> Option<ClipboardImage> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        for mime in ["image/png", "image/jpeg", "image/bmp", "image/webp"] {
            if let Some(bytes) = run_command_bytes(&["wl-paste", "-t", mime]) {
                if let Some(img) = decode_image_bytes(&bytes) {
                    return Some(img);
                }
            }
        }
        None
    } else if cfg!(target_os = "macos") {
        run_command_bytes(&["pngpaste", "-"])
            .and_then(|bytes| decode_image_bytes(&bytes))
    } else {
        run_command_bytes(&["xclip", "-o", "-selection", "clipboard", "-t", "image/png"])
            .and_then(|bytes| decode_image_bytes(&bytes))
    }
}

fn decode_image_bytes(bytes: &[u8]) -> Option<ClipboardImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(ClipboardImage {
        width: width as usize,
        height: height as usize,
        rgba: rgba.into_raw(),
    })
}

#[cfg(unix)]
fn run_command_text(args: &[&str]) -> Option<String> {
    let program = args.first()?;
    let output = std::process::Command::new(program)
        .args(&args[1..])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(unix)]
fn run_command_bytes(args: &[&str]) -> Option<Vec<u8>> {
    let program = args.first()?;
    let output = std::process::Command::new(program)
        .args(&args[1..])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(output.stdout)
}
