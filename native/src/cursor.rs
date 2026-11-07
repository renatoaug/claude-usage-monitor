// Cursor: port of cursor.js. Cursor keeps no usage in local files. Its app
// stores a session token in `state.vscdb` (SQLite), and the same dashboard
// endpoints cursor.com uses answer with it: unofficial, so every field is
// read defensively. Activity comes from the agent transcripts under
// ~/.cursor/projects, which log each tool call much like Claude Code's logs.

use rusqlite::{Connection, OpenFlags, types::Value as Sql};
use serde::Serialize;
use serde_json::{Value, json};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use crate::auth::{Http, Ureq};
pub use crate::codex::Window;

const API: &str = "https://api2.cursor.sh/aiserver.v1.DashboardService/";
/// a transcript that says nothing counts only while it's this fresh
const ACTIVE_MS: i64 = 60_000;
/// an unfinished turn this long without a write was abandoned
const OPEN_TURN_MS: i64 = 30 * 60_000;
const RECENT_TRANSCRIPTS: usize = 8;
/// a monthly budget moves slowly; don't hammer the API
const FETCH_MS: i64 = 5 * 60_000;
const RETRY_MS: i64 = 60_000;
const TAIL_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
struct Limits {
    ts: i64,
    plan: Option<String>,
    end_ms: Option<i64>,
    total: Option<f64>,
    api: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorUsage {
    pub active: bool,
    pub activity: Option<&'static str>,
    pub last_seen: Option<i64>,
    pub plan: Option<String>,
    /// the billing month: Cursor's "session"
    pub session: Option<Window>,
    /// the API-models pool, beside the plan's
    pub api: Option<Window>,
    pub limits_at: Option<i64>,
    pub signed_out: bool,
}

enum Outcome {
    SignedOut,
    Read(Limits),
    Failed(String),
}

struct Auth {
    token: Option<String>,
    plan: Option<String>,
}

fn default_state_db() -> PathBuf {
    let home = std::env::home_dir().unwrap_or_default();
    let tail = Path::new("Cursor/User/globalStorage/state.vscdb");
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(tail)
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData/Roaming")).join(tail)
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config")).join(tail)
    }
}

// the app's own session: its token, plus the plan it last cached
fn read_auth(db: &Path) -> Option<Auth> {
    if !db.exists() {
        return None;
    }
    let conn = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).ok()?;
    let mut q = conn.prepare("SELECT value FROM ItemTable WHERE key = ?").ok()?;
    let mut get = |k: &str| -> rusqlite::Result<Option<String>> {
        let v = match q.query_row([k], |r| r.get::<_, Sql>(0)) {
            Ok(v) => v,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(match v {
            Sql::Text(s) => Some(s),
            Sql::Blob(b) => Some(String::from_utf8_lossy(&b).into_owned()),
            Sql::Integer(i) => Some(i.to_string()),
            Sql::Real(f) => Some(f.to_string()),
            Sql::Null => None,
        })
    };
    let token = get("cursorAuth/accessToken").ok()?;
    let plan = get("cursorAuth/stripeMembershipType").ok()?;
    Some(Auth { token: token.filter(|t| !t.is_empty()), plan: plan.filter(|p| !p.is_empty()) })
}

fn post(http: &dyn Http, method: &str, token: &str) -> Result<Value, String> {
    let auth = format!("Bearer {token}");
    let headers = [("Authorization", auth.as_str()), ("Content-Type", "application/json"), ("Connect-Protocol-Version", "1")];
    let res = http.post_json(&format!("{API}{method}"), &headers, &json!({})).map_err(|e| e.to_string())?;
    if !(200..300).contains(&res.status) {
        return Err(format!("cursor {method}: HTTP {}", res.status));
    }
    serde_json::from_str(&res.body).map_err(|e| format!("cursor {method}: {e}"))
}

// a number, or a numeric string (Cursor sends its times as strings)
fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
    .filter(|n: &f64| n.is_finite())
}

// One fetch. The token is read fresh each time: the Cursor app refreshes it
// on its own, so there is nothing to renew here.
fn fetch(http: &dyn Http, db: &Path, now: i64) -> Outcome {
    let Some(Auth { token: Some(token), plan }) = read_auth(db) else { return Outcome::SignedOut };
    let usage = match post(http, "GetCurrentPeriodUsage", &token) {
        Ok(u) => u,
        Err(e) => return Outcome::Failed(e),
    };
    let info = post(http, "GetPlanInfo", &token).ok();
    let pu = usage.get("planUsage");
    let pct = |k: &str| pu.and_then(|p| p.get(k)).and_then(Value::as_f64).filter(|n| n.is_finite());
    let named = info.as_ref().and_then(|i| i.pointer("/planInfo/planName")).and_then(Value::as_str).map(str::to_lowercase);
    Outcome::Read(Limits {
        ts: now,
        plan: named.filter(|p| !p.is_empty()).or(plan),
        end_ms: num(usage.get("billingCycleEnd")).filter(|&n| n != 0.0).map(|n| n as i64),
        total: pct("totalPercentUsed"),
        api: pct("apiPercentUsed"),
    })
}

// the billing cycle as a window; past its end, the % belongs to the old cycle
fn cycle_now(pct: Option<f64>, end_ms: Option<i64>, now: i64) -> Window {
    let expired = end_ms.is_some_and(|e| e <= now);
    Window { pct: if expired { None } else { pct }, reset_ms: end_ms.filter(|_| !expired).map(|e| e - now), expired }
}

// ---- activity ---------------------------------------------------------------

fn scene(tool: &str) -> &'static str {
    match tool {
        "Read" | "ReadFile" | "Grep" | "Glob" | "LS" | "ListDir" | "SemanticSearch" | "ReadLints" => "reading",
        "Write" | "StrReplace" | "Edit" | "MultiEdit" | "ApplyPatch" | "Delete" | "EditNotebook" => "editing",
        "Shell" => "running",
        "WebSearch" | "WebFetch" => "researching",
        "Task" => "delegating",
        "TodoWrite" | "CreatePlan" => "planning",
        "AskQuestion" => "waiting",
        _ => "working",
    }
}

/// What the newest transcript lines say: a finished turn means nothing is
/// running (Some(None)), an open one names its scene, and None means no
/// telling.
pub fn activity_of<S: AsRef<str>>(lines: &[S]) -> Option<Option<&'static str>> {
    for line in lines.iter().rev() {
        let Ok(o) = serde_json::from_str::<Value>(line.as_ref()) else { continue };
        if o.get("type").and_then(Value::as_str) == Some("turn_ended") {
            return Some(None);
        }
        match o.get("role").and_then(Value::as_str) {
            Some("assistant") => {
                let tool = o
                    .pointer("/message/content")
                    .and_then(Value::as_array)
                    .and_then(|b| b.iter().rev().find(|b| b.get("type").and_then(Value::as_str) == Some("tool_use")));
                return Some(Some(tool.and_then(|t| t.get("name")).and_then(Value::as_str).map_or("working", scene)));
            }
            // just asked: the agent is thinking
            Some("user") => return Some(Some("working")),
            _ => {}
        }
    }
    None
}

fn entries(dir: &Path) -> Vec<fs::DirEntry> {
    let Ok(rd) = fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<_> = rd.flatten().filter(|e| !e.file_name().to_string_lossy().starts_with('.')).collect();
    out.sort_by_key(|e| e.file_name());
    out
}

fn is_jsonl(e: &fs::DirEntry) -> bool {
    e.file_type().is_ok_and(|t| t.is_file()) && e.file_name().to_string_lossy().ends_with(".jsonl")
}

fn is_dir(e: &fs::DirEntry) -> bool {
    e.file_type().is_ok_and(|t| t.is_dir())
}

// every transcript, subagents included: projects/<p>/agent-transcripts/<id>/…
fn transcripts(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    for p in entries(&home.join("projects")).iter().filter(|e| is_dir(e)) {
        for id in entries(&p.path().join("agent-transcripts")).iter().filter(|e| is_dir(e)) {
            for f in entries(&id.path()) {
                if is_jsonl(&f) {
                    out.push(f.path());
                } else if is_dir(&f) && f.file_name() == "subagents" {
                    out.extend(entries(&f.path()).iter().filter(|e| is_jsonl(e)).map(|e| e.path()));
                }
            }
        }
    }
    out
}

fn tail_lines(file: &Path, size: u64) -> std::io::Result<Vec<String>> {
    let len = size.min(TAIL_BYTES);
    let mut f = File::open(file)?;
    f.seek(SeekFrom::Start(size - len))?;
    let mut buf = Vec::with_capacity(len as usize);
    f.take(len).read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    if len < size {
        lines.remove(0); // cut mid-line
    }
    lines.retain(|l| !l.is_empty());
    Ok(lines)
}

fn mtime_ms(meta: &fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0)
}

// A turn stays open while a long command runs or the model thinks, without a
// write, so an open turn in any recent transcript is Cursor at work. A
// transcript with nothing to say counts only while it's fresh.
fn read_activity(home: &Path, now: i64) -> (bool, Option<&'static str>, Option<i64>) {
    let mut recent = vec![];
    let mut last_seen: Option<i64> = None;
    for file in transcripts(home) {
        let Ok(meta) = fs::metadata(&file) else { continue };
        let m = mtime_ms(&meta);
        if last_seen.is_none_or(|l| m > l) {
            last_seen = Some(m);
        }
        if now - m < OPEN_TURN_MS {
            recent.push((file, meta.len(), m));
        }
    }
    recent.sort_by_key(|r| std::cmp::Reverse(r.2));
    for (file, size, m) in recent.into_iter().take(RECENT_TRANSCRIPTS) {
        let said = tail_lines(&file, size).ok().and_then(|l| activity_of(&l));
        let activity = match said {
            Some(a) => a,
            None => (now - m < ACTIVE_MS).then_some("working"),
        };
        if activity.is_some() {
            return (true, activity, last_seen);
        }
    }
    (false, None, last_seen)
}

pub struct Cursor {
    home: PathBuf,
    state_db: PathBuf,
    http: Arc<dyn Http + Sync>,
    limits: Option<Limits>,
    last_try: Option<i64>,
    last_failed: bool,
    signed_out: bool,
    pending: Option<Receiver<Outcome>>,
}

impl Cursor {
    pub fn new() -> Self {
        let home = std::env::home_dir().unwrap_or_default().join(".cursor");
        Self::at(home, default_state_db(), Arc::new(Ureq::default()))
    }

    fn at(home: PathBuf, state_db: PathBuf, http: Arc<dyn Http + Sync>) -> Self {
        Cursor { home, state_db, http, limits: None, last_try: None, last_failed: false, signed_out: false, pending: None }
    }

    /// Settings → Connect: only a signed-in app has something to read.
    pub fn detect(&self) -> (bool, Option<String>) {
        match read_auth(&self.state_db) {
            Some(a) => (a.token.is_some(), a.plan),
            None => (false, None),
        }
    }

    // a fetch that came back keeps the last good reading on failure: it ages
    // into stale, never into zero
    fn apply(&mut self, out: Outcome) {
        self.signed_out = matches!(out, Outcome::SignedOut);
        self.last_failed = matches!(out, Outcome::Failed(_));
        match out {
            Outcome::Read(l) => self.limits = Some(l),
            Outcome::Failed(e) => eprintln!("cursor: {e}"),
            Outcome::SignedOut => {}
        }
    }

    fn collect(&mut self) {
        let Some(rx) = &self.pending else { return };
        match rx.try_recv() {
            Ok(out) => {
                self.pending = None;
                self.apply(out);
            }
            Err(mpsc::TryRecvError::Disconnected) => self.pending = None,
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn due(&self, now: i64) -> bool {
        let every = if self.limits.is_some() && !self.last_failed { FETCH_MS } else { RETRY_MS };
        self.pending.is_none() && self.last_try.is_none_or(|t| now - t >= every)
    }

    // fetch in the background, one at a time
    fn maybe_refresh(&mut self, now: i64) {
        self.collect();
        if !self.due(now) {
            return;
        }
        self.last_try = Some(now);
        let (tx, rx) = mpsc::channel();
        let (http, db) = (self.http.clone(), self.state_db.clone());
        std::thread::Builder::new()
            .name("cursor-limits".into())
            .spawn(move || {
                let _ = tx.send(fetch(http.as_ref(), &db, now));
            })
            .ok();
        self.pending = Some(rx);
    }

    /// Fetch now and wait (tests, and the dev flag).
    pub fn refresh_blocking(&mut self, now: i64) {
        self.last_try = Some(now);
        let out = fetch(self.http.as_ref(), &self.state_db, now);
        self.apply(out);
    }

    /// The poll's view of Cursor: activity from disk every tick, limits from
    /// the last fetch (a new one starts in the background when it's due).
    pub fn get_usage(&mut self, now: i64) -> CursorUsage {
        self.maybe_refresh(now);
        let (active, activity, last_seen) = read_activity(&self.home, now);
        let l = self.limits.as_ref();
        CursorUsage {
            active,
            activity,
            last_seen,
            plan: l.and_then(|l| l.plan.clone()),
            session: l.map(|l| cycle_now(l.total, l.end_ms, now)),
            api: l.map(|l| cycle_now(l.api, l.end_ms, now)),
            limits_at: l.map(|l| l.ts),
            signed_out: self.signed_out,
        }
    }
}

#[cfg(test)]
mod tests;
