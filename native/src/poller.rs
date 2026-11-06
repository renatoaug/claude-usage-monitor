// The background poll: local logs every few seconds, the account's real %
// at most every 90 s (the endpoint is rate-limited). Readings land in a
// shared snapshot, and the UI is woken only when one arrives.

use eframe::egui;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::accounts::Accounts;
use crate::auth::{Auth, Limits, Profile, Ureq};
use crate::usage::{self, Reader};

const LIMITS_EVERY: Duration = Duration::from_secs(90);
const LIMITS_BACKOFF: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub usage: usage::Config,
    pub poll: Duration,
    pub fire_threshold: f64,
}

impl Settings {
    /// ~/.claude-usage-monitor/config.json, the file the Electron app writes.
    pub fn load() -> Self {
        let path = crate::auth::default_data_dir().join("config.json");
        let raw: Value = std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
        Settings {
            usage: serde_json::from_value(raw.clone()).unwrap_or_default(),
            poll: Duration::from_millis(raw["pollIntervalMs"].as_u64().filter(|&ms| ms >= 1000).unwrap_or(4000)),
            fire_threshold: raw["fireThreshold"].as_f64().unwrap_or(90.0),
        }
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub usage: Option<usage::Usage>,
    pub limits: Option<Limits>,
    /// when `limits` was read: its reset countdowns are relative to it
    pub limits_at: Option<Instant>,
    pub connected: bool,
    pub profile: Option<Profile>,
    pub error: Option<String>,
    /// the first poll has finished: until then the screens show skeletons
    pub loaded: bool,
}

pub type Shared = Arc<Mutex<Snapshot>>;

pub fn spawn(ctx: egui::Context, settings: Settings) -> Shared {
    let shared: Shared = Arc::default();
    let out = shared.clone();
    std::thread::Builder::new()
        .name("poller".into())
        .spawn(move || run(ctx, settings, out))
        .expect("poll thread");
    shared
}

fn run(ctx: egui::Context, settings: Settings, out: Shared) {
    let accounts = Accounts::new(None);
    let account = accounts.active();
    let mut reader = Reader::new(account.claude_dir.as_ref().map(Into::into));
    let mut auth = Auth::new(Some(accounts.data_dir_of(&account.id)), Box::new(Ureq::default()));
    let mut next_limits = Instant::now();

    loop {
        let reading = reader.get_usage(&settings.usage);
        let connected = auth.is_connected();
        let mut fresh = None;
        let mut error = None;
        let mut profile = None;
        if connected && Instant::now() >= next_limits {
            match auth.fetch_usage() {
                Ok(l) => {
                    fresh = Some(l);
                    next_limits = Instant::now() + LIMITS_EVERY;
                }
                Err(e) => {
                    // throttled or offline: keep the last reading, try later
                    error = Some(e.message);
                    next_limits = Instant::now() + if e.status == 401 { LIMITS_EVERY } else { LIMITS_BACKOFF };
                }
            }
            profile = auth.fetch_profile().ok();
        }
        if let Ok(mut s) = out.lock() {
            s.usage = Some(reading);
            s.loaded = true;
            s.connected = connected;
            if let Some(l) = fresh {
                s.limits = Some(l);
                s.limits_at = Some(Instant::now());
                s.error = None;
            } else if error.is_some() {
                s.error = error;
            }
            if !connected {
                s.limits = None;
            }
            if profile.is_some() {
                s.profile = profile;
            }
        }
        ctx.request_repaint();
        std::thread::sleep(settings.poll);
    }
}
