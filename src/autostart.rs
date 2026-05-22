use std::io;

const APP_NAME: &str = "rusttoys";

pub fn set(enable: bool) -> io::Result<()> {
    let exe = std::env::current_exe()?;

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let key = format!(
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run"
        );
        if enable {
            let status = Command::new("reg")
                .args(["add", &key, "/v", APP_NAME, "/t", "REG_SZ",
                       "/d", &format!("\"{}\"", exe.display()), "/f"])
                .status()?;
            if !status.success() {
                return Err(io::Error::new(io::ErrorKind::Other, "reg add failed"));
            }
        } else {
            let _ = Command::new("reg")
                .args(["delete", &key, "/v", APP_NAME, "/f"])
                .status();
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dir = dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home dir"))?
            .join(".config/autostart");
        let path = dir.join(format!("{}.desktop", APP_NAME));
        if enable {
            std::fs::create_dir_all(&dir)?;
            let content = format!(
                "[Desktop Entry]\n\
                 Type=Application\n\
                 Name=RustToys\n\
                 Exec={}\n\
                 Icon={}\n\
                 Terminal=false\n\
                 X-GNOME-Autostart-enabled=true\n",
                exe.display(),
                APP_NAME
            );
            std::fs::write(&path, content)?;
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let dir = dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home dir"))?
            .join("Library/LaunchAgents");
        let path = dir.join(format!("com.{}.autostart.plist", APP_NAME));
        if enable {
            std::fs::create_dir_all(&dir)?;
            let content = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" \
"http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.{}.autostart</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#,
                APP_NAME,
                exe.display()
            );
            std::fs::write(&path, content)?;
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }

    Ok(())
}
