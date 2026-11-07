// What the Electron app left behind and the native app never reads: its
// Chromium profile (the cache under the OS's app-data folder, by the app's
// current and earlier names) and its own files in the data dir. Removed
// once per launch, by the released app only: a development build leaves
// them, so an upgrade from the Electron app can still be tried on this machine.

use std::path::{Path, PathBuf};

/// The Electron app's profile folders, by the names it has gone by.
const PROFILES: &[&str] = &["clauddy", "Clauddy", "claude-usage-monitor", "claude-usage-pet"];

/// The Electron app's files in the data dir: the window's spot, the
/// `./pet` debug channel, the Linux autostart icon and the profile it kept
/// there under CLAUDE_CONFIG_DIR.
const DATA_FILES: &[&str] = &["window.json", "debug.json", "icon.png", "electron"];

/// Clear the leftovers in the background. Quiet on any failure.
pub fn clear() {
    let Ok(exe) = std::env::current_exe() else { return };
    if exe.components().any(|c| c.as_os_str() == "target") {
        return; // a development build
    }
    std::thread::spawn(|| {
        if let Some(root) = app_data() {
            clear_profiles(&root);
        }
        clear_data_files(&crate::auth::default_data_dir());
    });
}

/// Where Electron keeps an app's profile on this OS.
fn app_data() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::home_dir().map(|h| h.join("Library/Application Support"))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::home_dir().map(|h| h.join(".config")))
    }
}

/// Remove the profile folders that are Chromium's: a folder by one of the
/// names that holds anything else belongs to someone else and stays.
fn clear_profiles(root: &Path) {
    for name in PROFILES {
        let dir = root.join(name);
        if dir.join("Code Cache").is_dir() || dir.join("Local State").is_file() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn clear_data_files(data: &Path) {
    for name in DATA_FILES {
        let path = data.join(name);
        let _ = if path.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn electron_profiles_go_and_anything_else_stays() {
        let root = tempfile::tempdir().unwrap();
        for name in ["claude-usage-monitor", "claude-usage-pet"] {
            std::fs::create_dir_all(root.path().join(name).join("Code Cache")).unwrap();
        }
        // a folder by the same name that isn't a Chromium profile
        std::fs::create_dir_all(root.path().join("clauddy")).unwrap();
        std::fs::write(root.path().join("clauddy/notes.txt"), "mine").unwrap();
        clear_profiles(root.path());
        assert!(!root.path().join("claude-usage-monitor").exists());
        assert!(!root.path().join("claude-usage-pet").exists());
        assert!(root.path().join("clauddy/notes.txt").exists());
    }

    #[test]
    fn the_electron_files_go_and_the_shared_ones_stay() {
        let data = tempfile::tempdir().unwrap();
        for name in ["window.json", "debug.json", "icon.png", "config.json", "auth.json", "alerts.json"] {
            std::fs::write(data.path().join(name), "{}").unwrap();
        }
        std::fs::create_dir_all(data.path().join("electron/GPUCache")).unwrap();
        std::fs::create_dir_all(data.path().join("accounts/a")).unwrap();
        clear_data_files(data.path());
        let mut left: Vec<String> = std::fs::read_dir(data.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["accounts", "alerts.json", "auth.json", "config.json"]);
    }
}
