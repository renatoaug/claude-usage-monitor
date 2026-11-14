// Updates: the latest GitHub release against this build's version. A quiet
// check shortly after launch and every 6 hours only speaks up when there's
// something newer; Settings > Check for updates asks out loud. Installing runs
// the same install.sh the README gives (macOS); on Windows and Linux the app
// downloads the release's archive, puts the new binary in its own place and
// starts it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui::Context;

const REPO: &str = "renatoaug/claude-usage-monitor";
const FIRST: Duration = Duration::from_secs(8);
const EVERY: Duration = Duration::from_secs(6 * 3600);

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Update {
    #[default]
    Idle,
    Checking,
    UpToDate,
    /// a newer release, by its tag ("v1.2.3")
    Available(String),
    Updating,
    Failed(String),
}

/// Whether tag `a` ("v1.2.3") is newer than version `b` ("1.2.0").
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> { v.trim_start_matches('v').split(['.', '-']).take(3).map(|p| p.parse().unwrap_or(0)).collect() };
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..3 {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

fn latest_tag() -> Result<String, String> {
    let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().new_agent();
    let mut res = agent
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("User-Agent", "Clauddy")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    let body: serde_json::Value = res.body_mut().read_json().map_err(|e| e.to_string())?;
    body["tag_name"].as_str().map(str::to_string).ok_or_else(|| "no release found".into())
}

#[derive(Clone)]
pub struct Updater {
    state: Arc<Mutex<Update>>,
    ctx: Context,
}

impl Updater {
    /// Starts the quiet check: it only ever sets `Available`.
    pub fn new(ctx: &Context) -> Self {
        let u = Updater { state: Arc::default(), ctx: ctx.clone() };
        let quiet = u.clone();
        std::thread::Builder::new()
            .name("updates".into())
            .spawn(move || {
                // a development build (under target/) is always behind: no nagging dot
                if std::env::current_exe().is_ok_and(|e| e.components().any(|c| c.as_os_str() == "target")) {
                    return;
                }
                std::thread::sleep(FIRST);
                loop {
                    if let Ok(tag) = latest_tag()
                        && newer(&tag, env!("CARGO_PKG_VERSION"))
                    {
                        quiet.set(Update::Available(tag));
                    }
                    std::thread::sleep(EVERY);
                }
            })
            .ok();
        u
    }

    fn set(&self, s: Update) {
        if let Ok(mut st) = self.state.lock() {
            *st = s;
        }
        self.ctx.request_repaint();
    }

    pub fn state(&self) -> Update {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Ask now, and say what came back.
    pub fn check(&self) {
        self.set(Update::Checking);
        let me = self.clone();
        std::thread::spawn(move || {
            me.set(match latest_tag() {
                Ok(tag) if newer(&tag, env!("CARGO_PKG_VERSION")) => Update::Available(tag),
                Ok(_) => Update::UpToDate,
                Err(e) => Update::Failed(e),
            });
        });
    }

    /// Install the latest release. macOS runs install.sh, which replaces
    /// the app and opens the new one, so this one quits to make way. Windows
    /// and Linux swap the binary in place (see `replace`) and start the new
    /// one. Anywhere else, or a development build, gets the release page.
    pub fn install(&self) {
        let Update::Available(tag) = self.state() else { return };
        let dev = std::env::current_exe().is_ok_and(|e| e.components().any(|c| c.as_os_str() == "target"));
        let Some(asset) = asset(&tag).filter(|_| !dev) else {
            crate::app::open_url(&format!("https://github.com/{REPO}/releases/latest"));
            return;
        };
        self.set(Update::Updating);
        if !cfg!(target_os = "macos") {
            let me = self.clone();
            std::thread::spawn(move || match std::env::current_exe().map_err(|e| e.to_string()).and_then(|exe| replace(exe, &tag, asset)) {
                // quit only once the new one is running, or nothing would be
                Ok(exe) => match std::process::Command::new(exe).spawn() {
                    Ok(_) => std::process::exit(0),
                    Err(e) => me.set(Update::Failed(format!("updated, but couldn't restart: {e}"))),
                },
                Err(e) => me.set(Update::Failed(e)),
            });
            return;
        }
        let cmd = format!("curl -fsSL https://raw.githubusercontent.com/{REPO}/main/install.sh | bash");
        if std::process::Command::new("/bin/bash").args(["-lc", &cmd]).spawn().is_ok() {
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(1500));
                std::process::exit(0);
            });
        } else {
            self.set(Update::Failed("couldn't run the installer".into()));
        }
    }
}

/// The release archive for this OS, and the binary's path inside it.
fn asset(tag: &str) -> Option<(String, &'static str)> {
    let v = tag.trim_start_matches('v');
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some((format!("Clauddy-{v}-win-x64.zip"), "Clauddy.exe"))
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some((format!("Clauddy-{v}-linux-x64.tar.gz"), "Clauddy/clauddy"))
    } else if cfg!(target_os = "macos") {
        Some((String::new(), "")) // install.sh knows its own way
    } else {
        None
    }
}

/// Download `tag`'s archive, unpack it and put its binary where this one
/// is. Returns the path to start.
fn replace(exe: std::path::PathBuf, tag: &str, (file, inside): (String, &str)) -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("clauddy-update-{}", tag.trim_start_matches('v')));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join(&file);
    let agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().new_agent();
    let mut res = agent
        .get(format!("https://github.com/{REPO}/releases/download/{tag}/{file}"))
        .header("User-Agent", "Clauddy")
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut out = std::fs::File::create(&archive).map_err(|e| e.to_string())?;
    std::io::copy(&mut res.body_mut().as_reader(), &mut out).map_err(|e| format!("download failed: {e}"))?;
    drop(out);
    // tar unpacks both: Windows 10+ ships bsdtar, which reads zip too
    let ok = std::process::Command::new("tar").arg("-xf").arg(&archive).arg("-C").arg(&dir).status().is_ok_and(|s| s.success());
    if !ok {
        return Err("couldn't unpack the update".into());
    }
    swap(&exe, &dir.join(inside))?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(exe)
}

/// Put `new` at `exe`. A running binary can't be overwritten on Windows but
/// can be renamed, so the current one steps aside to `<exe>.old` first (and
/// comes back if the copy fails); `clear_old` removes it on the next launch.
fn swap(exe: &std::path::Path, new: &std::path::Path) -> Result<(), String> {
    if !new.is_file() {
        return Err("the update has no app inside".into());
    }
    let old = old_path(exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old).map_err(|e| format!("couldn't replace the app: {e}"))?;
    // copy, not rename: the temp dir can be on another drive
    if let Err(e) = std::fs::copy(new, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(format!("couldn't replace the app: {e}"));
    }
    Ok(())
}

fn old_path(exe: &std::path::Path) -> std::path::PathBuf {
    exe.with_extension("old")
}

/// Remove what the last update moved aside. The old process may still be
/// closing, so this waits a moment and runs off the UI thread.
pub fn clear_old() {
    let Ok(exe) = std::env::current_exe() else { return };
    let old = old_path(&exe);
    if !old.exists() {
        return;
    }
    std::thread::spawn(move || {
        for _ in 0..5 {
            std::thread::sleep(Duration::from_secs(3));
            if std::fs::remove_file(&old).is_ok() {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{newer, old_path, replace, swap};

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(newer("v1.10.0", "1.9.9"));
        assert!(newer("v2.0.0", "1.99.99"));
        assert!(!newer("v1.2.3", "1.2.3"));
        assert!(!newer("v1.2.2", "1.2.3"));
        assert!(newer("v0.1.1", "0.1.0"));
    }

    #[test]
    fn an_update_puts_the_new_binary_in_place_and_keeps_the_old_aside() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Clauddy.exe");
        let new = dir.path().join("unpacked");
        std::fs::write(&exe, "old build").unwrap();
        std::fs::write(&new, "new build").unwrap();
        swap(&exe, &new).unwrap();
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new build");
        assert_eq!(std::fs::read_to_string(old_path(&exe)).unwrap(), "old build");
        // a second update replaces the leftover from the first
        std::fs::write(&new, "newer build").unwrap();
        swap(&exe, &new).unwrap();
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "newer build");
        assert_eq!(std::fs::read_to_string(old_path(&exe)).unwrap(), "new build");
    }

    #[test]
    fn an_archive_without_the_app_leaves_the_current_one_alone() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Clauddy.exe");
        std::fs::write(&exe, "old build").unwrap();
        assert!(swap(&exe, &dir.path().join("missing")).is_err());
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "old build");
        assert!(!old_path(&exe).exists());
    }

    // downloads a real release: run by hand when the updater changes
    #[test]
    #[ignore = "downloads a release from GitHub"]
    fn a_real_release_unpacks_into_place() {
        let dir = tempfile::tempdir().unwrap();
        for (file, inside, name) in
            [("Clauddy-2.2.0-win-x64.zip", "Clauddy.exe", "Clauddy.exe"), ("Clauddy-2.2.0-linux-x64.tar.gz", "Clauddy/clauddy", "clauddy")]
        {
            let exe = dir.path().join(name);
            std::fs::write(&exe, "old build").unwrap();
            assert_eq!(replace(exe.clone(), "v2.2.0", (file.into(), inside)).unwrap(), exe);
            assert!(std::fs::metadata(&exe).unwrap().len() > 1_000_000, "{file}: the real binary is in place");
            assert_eq!(std::fs::read_to_string(old_path(&exe)).unwrap(), "old build");
        }
    }
}
