// Updates: the latest GitHub release against this build's version. A quiet
// check shortly after launch and every 6 hours only speaks up when there's
// something newer; Settings > Check for updates asks out loud. Installing runs
// the same install.sh the README gives (macOS); elsewhere it opens the
// release page.

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
    /// the app and opens the new one, so this one quits to make way.
    pub fn install(&self) {
        if !cfg!(target_os = "macos") {
            crate::app::open_url(&format!("https://github.com/{REPO}/releases/latest"));
            return;
        }
        self.set(Update::Updating);
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

#[cfg(test)]
mod tests {
    use super::newer;

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(newer("v1.10.0", "1.9.9"));
        assert!(newer("v2.0.0", "1.99.99"));
        assert!(!newer("v1.2.3", "1.2.3"));
        assert!(!newer("v1.2.2", "1.2.3"));
        assert!(newer("v0.1.1", "0.1.0"));
    }
}
