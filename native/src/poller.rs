// The background poll: local logs every few seconds, the account's real %
// at most every 90 s (the endpoint is rate-limited). Readings land in a
// shared snapshot, and the UI is woken only when one arrives.

use eframe::egui;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::accounts::{Account, Accounts};
use crate::alerts::{self, Scope};
use crate::prefs::Prefs;
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
    /// the first poll has finished: until then the screens show skeletons
    pub loaded: bool,
    /// the usage API asked us to slow down; the next try is then
    pub retry_at: Option<Instant>,
    /// the Claude accounts, and the one on screen
    pub accounts: Vec<Account>,
    pub active: String,
    pub login: Login,
    /// Codex, when it's tracked: its reading, and whether this machine has
    /// Codex at all (and on what plan)
    pub codex: Option<crate::codex::CodexUsage>,
    pub codex_found: Option<(bool, Option<String>)>,
    /// Cursor, the same way: found means its app is signed in here
    pub cursor: Option<crate::cursor::CursorUsage>,
    pub cursor_found: Option<(bool, Option<String>)>,
}

/// Where a browser login stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Login {
    #[default]
    Idle,
    /// the browser is open; waiting for the pasted code
    AwaitingCode,
    Failed(String),
}

/// What the UI asks the poll thread to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Switch(String),
    /// a fresh, logged-out account: switch to it and start its login
    Add,
    Remove(String),
    BeginLogin,
    CompleteLogin(String),
    /// forget the active account's token (the account stays listed)
    Logout,
    /// Settings were saved: the alerts follow the new thresholds
    Prefs(Prefs),
}

pub type Shared = Arc<Mutex<Snapshot>>;

pub fn spawn(ctx: egui::Context, settings: Settings) -> (Shared, Sender<Command>) {
    let shared: Shared = Arc::default();
    let out = shared.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("poller".into())
        .spawn(move || Worker::new(ctx, settings, out).run(rx))
        .expect("poll thread");
    (shared, tx)
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

struct Worker {
    ctx: egui::Context,
    settings: Settings,
    out: Shared,
    accounts: Accounts,
    dir: std::path::PathBuf,
    reader: Reader,
    auth: Auth,
    cache: Cache,
    schedule: Schedule,
    profile: Option<Profile>,
    alerts: alerts::State,
    prefs: Prefs,
    /// consecutive 401s: one can be a refresh racing another, two is a dead token
    auth_fails: u32,
    codex: crate::codex::Codex,
    cursor: crate::cursor::Cursor,
}

impl Worker {
    fn new(ctx: egui::Context, settings: Settings, out: Shared) -> Self {
        let accounts = Accounts::new(None);
        // folders left by an add abandoned mid-login (never one with a token)
        accounts.prune_orphans();
        let dir = accounts.data_dir_of(&accounts.active_id());
        let mut w = Worker {
            ctx,
            settings,
            out,
            reader: Reader::new(None),
            auth: Auth::new(Some(dir.clone()), Box::new(Ureq::default())),
            dir,
            accounts,
            cache: Cache::default(),
            schedule: Schedule::new(now_ms(), None),
            profile: None,
            alerts: alerts::State::default(),
            prefs: Prefs::load(),
            auth_fails: 0,
            codex: crate::codex::Codex::new(None),
            cursor: crate::cursor::Cursor::new(),
        };
        let found = (w.codex.detect(), w.cursor.detect());
        if let Ok(mut s) = w.out.lock() {
            s.codex_found = Some(found.0);
            s.cursor_found = Some(found.1);
        }
        w.bind();
        w
    }

    /// Point everything at the active account: its logs, its token, its
    /// saved reading. Only the active account is polled.
    fn bind(&mut self) {
        let account = self.accounts.active();
        self.dir = self.accounts.data_dir_of(&account.id);
        self.reader.set_claude_dir(account.claude_dir.as_ref().map(Into::into));
        self.auth.set_data_dir(Some(self.dir.clone()));
        // start from the saved reading: on screen at once, and no call if it's fresh
        self.cache = load_cache(&self.dir).unwrap_or_default();
        self.profile = self.cache.profile.clone();
        // only the active account can notify; its alerts are its own
        self.alerts = alerts::State::load(&self.dir);
        self.auth_fails = 0;
        self.schedule = Schedule::new(now_ms(), Some(&self.cache).filter(|c| c.polled_at > 0));
        let now = now_ms();
        if let Ok(mut s) = self.out.lock() {
            s.limits = self.cache.limits.clone().filter(|_| self.cache.fetched_at.is_some());
            s.limits_at = self.cache.fetched_at.map(|at| instant_at(at, now));
            s.retry_at = self.cache.retry_at.filter(|&t| t > now).map(|t| Instant::now() + Duration::from_millis((t - now) as u64));
            s.profile = self.profile.clone();
            s.accounts = self.accounts.list();
            s.active = account.id;
            s.connected = self.auth.is_connected();
        }
    }

    fn set_login(&self, login: Login) {
        if let Ok(mut s) = self.out.lock() {
            s.login = login;
        }
    }

    fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::Switch(id) => {
                if self.accounts.set_active(&id).is_some() {
                    self.set_login(Login::Idle);
                    self.bind();
                }
            }
            Command::Add => {
                let id = self.accounts.add(None);
                self.accounts.set_active(&id);
                self.bind();
                self.handle(Command::BeginLogin);
            }
            Command::Remove(id) => {
                self.accounts.remove(&id);
                if let Ok(mut s) = self.out.lock() {
                    s.accounts = self.accounts.list();
                }
            }
            Command::Prefs(p) => {
                if (p.heads_up, p.near_limit) != (self.prefs.heads_up, self.prefs.near_limit) {
                    self.alerts.rearm();
                    self.alerts.save(&self.dir);
                }
                if p.codex && !self.prefs.codex {
                    let found = self.codex.detect();
                    if let Ok(mut s) = self.out.lock() {
                        s.codex_found = Some(found);
                    }
                }
                if p.cursor && !self.prefs.cursor {
                    let found = self.cursor.detect();
                    if let Ok(mut s) = self.out.lock() {
                        s.cursor_found = Some(found);
                    }
                }
                self.prefs = p;
            }
            Command::Logout => {
                self.auth.clear();
                self.cache = Cache::default();
                let _ = std::fs::remove_file(cache_path(&self.dir));
                self.profile = None;
                if let Ok(mut s) = self.out.lock() {
                    s.connected = false;
                    s.limits = None;
                    s.profile = None;
                    s.retry_at = None;
                }
            }
            Command::BeginLogin => {
                crate::app::open_url(&self.auth.begin());
                self.set_login(Login::AwaitingCode);
            }
            Command::CompleteLogin(code) => match self.auth.complete(&code) {
                Ok(()) => {
                    self.set_login(Login::Idle);
                    // a fresh login reads at once, and names the account
                    self.cache = Cache::default();
                    self.schedule = Schedule::new(now_ms(), None);
                    self.profile = self.auth.fetch_profile().ok();
                    let id = self.accounts.active_id();
                    if let Some(email) = self.profile.as_ref().and_then(|p| p.email.as_deref()) {
                        self.accounts.label(&id, Some(email));
                    }
                    if let Ok(mut s) = self.out.lock() {
                        s.accounts = self.accounts.list();
                        s.profile = self.profile.clone();
                        s.connected = true;
                    }
                }
                Err(e) => self.set_login(Login::Failed(e.message)),
            },
        }
        self.ctx.request_repaint();
    }

    fn tick(&mut self) {
        let reading = self.reader.get_usage(&self.settings.usage);
        let connected = self.auth.is_connected();
        let mut fresh = None;
        let mut retry_at = None;
        if connected && self.schedule.due(now_ms(), reading.session.tokens) {
            let outcome = match self.auth.fetch_usage() {
                Ok(l) => {
                    fresh = Some(l);
                    Outcome::Read
                }
                Err(e) if e.status == 429 => Outcome::Throttled,
                Err(e) if e.status == 401 => {
                    // a refresh that raced another comes back 401 on a live
                    // session: only the second in a row means the token is dead
                    self.auth_fails += 1;
                    if self.auth_fails >= 2 {
                        self.lose_access();
                    }
                    Outcome::Failed
                }
                Err(_) => Outcome::Failed, // offline: keep the last reading
            };
            if outcome == Outcome::Read {
                self.auth_fails = 0;
            }
            let now = now_ms();
            self.schedule.polled(now, outcome);
            if self.profile.is_none() {
                self.profile = self.auth.fetch_profile().ok();
            }
            let c = &mut self.cache;
            c.polled_at = now;
            c.profile = self.profile.clone();
            c.backoff = self.schedule.backoff;
            c.retry_at = None;
            if let Some(l) = &fresh {
                c.limits = Some(l.clone());
                c.fetched_at = Some(now);
            }
            if outcome == Outcome::Throttled {
                c.retry_at = Some(self.schedule.next_at());
                retry_at = Some(Instant::now() + Duration::from_millis((self.schedule.next_at() - now).max(0) as u64));
            }
            save_cache(&self.dir, &self.cache);
        }
        let codex = if self.prefs.codex { self.codex.get_usage(now_ms()) } else { None };
        let cursor = self.prefs.cursor.then(|| self.cursor.get_usage(now_ms()));
        self.check_alerts(&reading, codex.as_ref(), cursor.as_ref());
        if let Ok(mut s) = self.out.lock() {
            s.codex = codex;
            s.cursor = cursor;
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
            if self.profile.is_some() {
                s.profile = self.profile.clone();
            }
        }
        self.ctx.request_repaint();
    }

    /// The token is dead: forget it, say so once, and ask for a login.
    fn lose_access(&mut self) {
        self.auth.clear();
        self.auth_fails = 0;
        if self.prefs.alerts {
            alerts::send(&alerts::auth_lost());
        }
        if let Ok(mut s) = self.out.lock() {
            s.connected = false;
            s.limits = None;
        }
    }

    /// The account's real % when there's a reading still in force, else the
    /// local estimate (as the Electron app did).
    fn scopes(&self, reading: &usage::Usage) -> Vec<Scope> {
        let now = now_ms();
        let live = |w: &crate::auth::Window, at: i64| -> Option<(f64, Option<i64>)> {
            let left = w.reset_ms.map(|r| r - (now - at));
            (left.is_none_or(|l| l > 0)).then_some((w.pct, left))
        };
        let scope = |label: &str, r: Option<(f64, Option<i64>)>, session: bool| Scope {
            label: label.into(),
            pct: r.map(|x| x.0),
            reset_ms: r.and_then(|x| x.1),
            session,
        };
        match (&self.cache.limits, self.cache.fetched_at) {
            (Some(l), Some(at)) => {
                let mut v = vec![scope("Session", live(&l.session, at), true), scope("Weekly usage", live(&l.week, at), false)];
                for sc in &l.scoped {
                    let w = crate::auth::Window { pct: sc.pct, reset_ms: sc.reset_ms };
                    v.push(scope(&format!("{} weekly", sc.label), live(&w, at), false));
                }
                v
            }
            _ => vec![
                scope("Session", reading.session.active.then_some((reading.session.pct, Some(reading.session.reset_ms))), true),
                scope("Weekly usage", Some((reading.week.pct, reading.week.reset_ms)), false),
            ],
        }
    }

    fn check_alerts(
        &mut self,
        reading: &usage::Usage,
        codex: Option<&crate::codex::CodexUsage>,
        cursor: Option<&crate::cursor::CursorUsage>,
    ) {
        let mut scopes = self.scopes(reading);
        let claude_session = scopes.first().and_then(|s| s.pct);
        let scope = |label: &str, w: &Option<crate::codex::Window>, session: bool| Scope {
            label: label.into(),
            pct: w.as_ref().and_then(|w| w.pct),
            reset_ms: w.as_ref().and_then(|w| w.reset_ms),
            session,
        };
        // Codex has its own windows, so its own keys ("Codex session:80")
        if let Some(c) = codex {
            scopes.push(scope(if c.weekly_only { "Codex weekly" } else { "Codex session" }, &c.session, true));
            scopes.push(scope("Codex weekly", &c.weekly, false));
        }
        // Cursor's budget is the billing cycle; its API-model pool fills on its own
        if let Some(c) = cursor {
            scopes.push(scope("Cursor usage", &c.session, false));
            scopes.push(scope("Cursor API models", &c.api, false));
        }
        let thresholds = [self.prefs.heads_up, self.prefs.near_limit];
        let (notes, dirty) = self.alerts.check(self.prefs.alerts, &thresholds, &scopes);
        let (reset, reset_dirty) = self.alerts.window_reset(self.prefs.alerts, claude_session);
        for n in notes.iter().chain(reset.iter()) {
            alerts::send(n);
        }
        if dirty || reset_dirty {
            self.alerts.save(&self.dir);
        }
    }

    // poll, then wait for the next poll or a command; a command is answered
    // at once and followed by a fresh poll, so its effect shows right away
    fn run(mut self, rx: Receiver<Command>) {
        loop {
            self.tick();
            match rx.recv_timeout(self.settings.poll) {
                Ok(cmd) => {
                    self.handle(cmd);
                    while let Ok(more) = rx.try_recv() {
                        self.handle(more);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
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
