// The background poll: local logs every few seconds, the account's real %
// at most every 90 s (the endpoint is rate-limited). Readings land in a
// shared snapshot, and the UI is woken only when one arrives.

use eframe::egui;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::accounts::Accounts;
use crate::auth::{Auth, Limits, Profile, Ureq};
use crate::usage::{self, Reader, now_ms};


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
    /// the usage API asked us to slow down; the next try is then
    pub retry_at: Option<Instant>,
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

// The account's % only moves when tokens are spent, and the local logs show
// that within a poll. So ask when the session's tokens grew, but never more
// than once per 90 s; otherwise a heartbeat every 5 min. A 429 doubles the
// heartbeat, up to 30 min. (The Electron app's policy.)
const FLOOR_MS: i64 = 90_000;
const HEARTBEAT_MS: i64 = 5 * 60_000;
const MAX_BACKOFF_MS: i64 = 30 * 60_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Read,
    Throttled,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    last_poll: Option<i64>,
    next: i64,
    backoff: i64,
    throttled: bool,
    last_tokens: Option<u64>,
}

impl Schedule {
    /// Resume from what an earlier run saved: within the floor of its last
    /// read, or inside a 429 backoff, a restart waits instead of asking again.
    pub fn new(now: i64, saved: Option<&Cache>) -> Self {
        let fresh = Schedule { last_poll: None, next: now, backoff: HEARTBEAT_MS, throttled: false, last_tokens: None };
        let Some(c) = saved else { return fresh };
        if let Some(until) = c.retry_at.filter(|&t| t > now) {
            return Schedule { last_poll: Some(c.polled_at), next: until, backoff: c.backoff, throttled: true, last_tokens: None };
        }
        if now - c.polled_at < FLOOR_MS {
            return Schedule { last_poll: Some(c.polled_at), next: c.polled_at + FLOOR_MS, ..fresh };
        }
        fresh
    }

    /// Ask now? `tokens` is the session's local token count this poll. While
    /// throttled, only the backoff's end counts.
    pub fn due(&mut self, now: i64, tokens: u64) -> bool {
        let grew = self.last_tokens.is_some_and(|t| tokens > t);
        self.last_tokens = Some(tokens);
        let floor_passed = self.last_poll.is_none_or(|t| now - t >= FLOOR_MS);
        now >= self.next || (grew && floor_passed && !self.throttled)
    }

    pub fn polled(&mut self, now: i64, outcome: Outcome) {
        self.last_poll = Some(now);
        self.throttled = outcome == Outcome::Throttled;
        self.backoff = match outcome {
            Outcome::Read => HEARTBEAT_MS,
            Outcome::Throttled => (self.backoff * 2).min(MAX_BACKOFF_MS),
            Outcome::Failed => self.backoff,
        };
        self.next = now + self.backoff;
    }

    pub fn next_at(&self) -> i64 {
        self.next
    }
}

/// What a run leaves for the next: the last reading (on screen at once on
/// restart) and when it last asked, and any 429 backoff still running.
/// `limits-cache.json`, beside the account's token.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Cache {
    pub polled_at: i64,
    pub fetched_at: Option<i64>,
    pub limits: Option<Limits>,
    pub profile: Option<Profile>,
    pub retry_at: Option<i64>,
    pub backoff: i64,
}

fn cache_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("limits-cache.json")
}

fn load_cache(dir: &std::path::Path) -> Option<Cache> {
    serde_json::from_slice(&std::fs::read(cache_path(dir)).ok()?).ok()
}

fn save_cache(dir: &std::path::Path, c: &Cache) {
    if let Ok(body) = serde_json::to_vec(c) {
        let _ = std::fs::write(cache_path(dir), body);
    }
}

// an epoch time as an Instant in the past (or now)
fn instant_at(epoch_ms: i64, now: i64) -> Instant {
    let ago = Duration::from_millis((now - epoch_ms).max(0) as u64);
    Instant::now().checked_sub(ago).unwrap_or_else(Instant::now)
}

fn run(ctx: egui::Context, settings: Settings, out: Shared) {
    let accounts = Accounts::new(None);
    let account = accounts.active();
    let dir = accounts.data_dir_of(&account.id);
    let mut reader = Reader::new(account.claude_dir.as_ref().map(Into::into));
    let mut auth = Auth::new(Some(dir.clone()), Box::new(Ureq::default()));

    // start from the saved reading: on screen at once, and no call if it's fresh
    let mut cache = load_cache(&dir).unwrap_or_default();
    let mut profile = cache.profile.clone();
    let mut schedule = Schedule::new(now_ms(), Some(&cache).filter(|c| c.polled_at > 0));
    if let Ok(mut s) = out.lock() {
        if let (Some(l), Some(at)) = (&cache.limits, cache.fetched_at) {
            s.limits = Some(l.clone());
            s.limits_at = Some(instant_at(at, now_ms()));
        }
        s.retry_at = cache.retry_at.filter(|&t| t > now_ms()).map(|t| Instant::now() + Duration::from_millis((t - now_ms()) as u64));
        s.profile = profile.clone();
    }

    loop {
        let reading = reader.get_usage(&settings.usage);
        let connected = auth.is_connected();
        let mut fresh = None;
        let mut retry_at = None;
        if connected && schedule.due(now_ms(), reading.session.tokens) {
            let outcome = match auth.fetch_usage() {
                Ok(l) => {
                    fresh = Some(l);
                    Outcome::Read
                }
                Err(e) if e.status == 429 => Outcome::Throttled,
                Err(_) => Outcome::Failed, // offline or a 401: keep the last reading
            };
            let now = now_ms();
            schedule.polled(now, outcome);
            if profile.is_none() {
                profile = auth.fetch_profile().ok();
            }
            cache.polled_at = now;
            cache.profile = profile.clone();
            cache.backoff = schedule.backoff;
            cache.retry_at = None;
            if let Some(l) = &fresh {
                cache.limits = Some(l.clone());
                cache.fetched_at = Some(now);
            }
            if outcome == Outcome::Throttled {
                cache.retry_at = Some(schedule.next_at());
                retry_at = Some(Instant::now() + Duration::from_millis((schedule.next_at() - now).max(0) as u64));
            }
            save_cache(&dir, &cache);
        }
        if let Ok(mut s) = out.lock() {
            s.usage = Some(reading);
            s.loaded = true;
            s.connected = connected;
            if let Some(l) = fresh {
                s.limits = Some(l);
                s.limits_at = Some(Instant::now());
                s.retry_at = None;
            } else if retry_at.is_some() {
                s.retry_at = retry_at;
            }
            if !connected {
                s.limits = None;
            }
            if profile.is_some() {
                s.profile = profile.clone();
            }
        }
        ctx.request_repaint();
        std::thread::sleep(settings.poll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn a_fresh_start_asks_at_once() {
        let mut s = Schedule::new(0, None);
        assert!(s.due(0, 100));
    }

    #[test]
    fn a_restart_within_the_floor_waits_on_the_saved_reading() {
        let saved = Cache { polled_at: 1, ..Default::default() };
        let mut s = Schedule::new(30_000, Some(&saved));
        assert!(!s.due(30_000, 100));
        assert!(s.due(FLOOR_MS + 1, 100));
    }

    #[test]
    fn a_restart_inside_a_429_backoff_waits_for_it_to_end() {
        let saved = Cache { polled_at: 1, retry_at: Some(10 * MIN), backoff: 10 * MIN, ..Default::default() };
        let mut s = Schedule::new(2 * MIN, Some(&saved));
        assert!(!s.due(2 * MIN, 100));
        assert!(!s.due(5 * MIN, 900)); // growth doesn't cut a backoff short
        assert!(s.due(10 * MIN, 900));
    }

    #[test]
    fn growing_tokens_ask_early_but_never_inside_the_floor() {
        let mut s = Schedule::new(0, None);
        s.due(0, 100);
        s.polled(0, Outcome::Read);
        assert!(!s.due(30_000, 200)); // grew, but inside the floor
        assert!(s.due(FLOOR_MS + 1, 300)); // grew, floor passed
    }

    #[test]
    fn idle_tokens_wait_for_the_heartbeat() {
        let mut s = Schedule::new(0, None);
        s.due(0, 100);
        s.polled(0, Outcome::Read);
        assert!(!s.due(2 * MIN, 100));
        assert!(s.due(5 * MIN, 100));
    }

    #[test]
    fn a_429_doubles_the_wait_up_to_half_an_hour() {
        let mut s = Schedule::new(0, None);
        s.polled(0, Outcome::Throttled);
        assert_eq!(s.next_at(), 10 * MIN);
        for _ in 0..5 {
            s.polled(0, Outcome::Throttled);
        }
        assert_eq!(s.next_at(), 30 * MIN);
        s.polled(0, Outcome::Read);
        assert_eq!(s.next_at(), 5 * MIN);
    }

    #[test]
    fn growth_waits_out_a_429_backoff() {
        let mut s = Schedule::new(0, None);
        s.due(0, 100);
        s.polled(0, Outcome::Throttled);
        assert!(!s.due(FLOOR_MS, 200));
        assert!(s.due(10 * MIN, 300));
    }
}
