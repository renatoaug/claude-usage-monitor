// Open at login, for the released app only: a
// development build (its own bundle id, or a binary under target/) never
// adds itself.

/// Make sure Clauddy starts with the session. Quiet on any failure.
pub fn open_at_login() {
    let Ok(exe) = std::env::current_exe() else { return };
    if exe.components().any(|c| c.as_os_str() == "target") {
        return; // a development build
    }
    #[cfg(target_os = "macos")]
    mac::register();
    #[cfg(target_os = "windows")]
    {
        // the per-user Run key: no admin, and Settings > Apps > Startup lists it
        let _ = std::process::Command::new("reg")
            .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "Clauddy", "/t", "REG_SZ", "/d"])
            .arg(format!("\"{}\"", exe.display()))
            .arg("/f")
            .status();
    }
    #[cfg(target_os = "linux")]
    {
        // the XDG autostart entry, rewritten only when it changes
        let dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::home_dir().unwrap_or_default().join(".config"))
            .join("autostart");
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=Clauddy\nComment=A cute pixel-art desktop pet that tracks your Claude Code usage\nExec=\"{}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        let file = dir.join("clauddy.desktop");
        if std::fs::read_to_string(&file).ok().as_deref() != Some(entry.as_str()) {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(file, entry);
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2_foundation::NSBundle;
    use objc2_service_management::{SMAppService, SMAppServiceStatus};

    pub fn register() {
        // the released bundle only; the dev one is app.clauddy.dev
        if NSBundle::mainBundle().bundleIdentifier().is_none_or(|id| id.to_string() != "app.clauddy") {
            return;
        }
        // SAFETY: plain calls on the app's own login-item service
        unsafe {
            let service = SMAppService::mainAppService();
            // once turned off in System Settings it stays off: only a new install registers
            if service.status() == SMAppServiceStatus::NotRegistered {
                let _ = service.registerAndReturnError();
            }
        }
    }
}
