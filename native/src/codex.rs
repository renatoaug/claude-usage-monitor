// Codex: port of codex.js. Codex (the CLI and the desktop app alike) writes one
// `rollout-*.jsonl` per session under ~/.codex/sessions/YYYY/MM/DD; its
// `token_count` events carry the token totals and the account's rate limits.
// Other ChatGPT products spend the same limits without writing rollouts, so
// the % also comes from Codex's own `codex app-server`, asked in the
// background. Clauddy never touches Codex's login.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use crate::usage::{Labeled, local_midnight, parse_date};

const DAY: i64 = 86_400_000;
const DAYS: usize = 30;
const WEEK: i64 = 7 * DAY;
/// a rollout touched within the last minute: Codex is working, when its
/// tail doesn't say whether a task is open
const ACTIVE_MS: i64 = 60_000;
/// an open task untouched this long was abandoned
const OPEN_TURN_MS: i64 = 30 * 60_000;
/// how many of the newest rollouts to look for an open task in
const RECENT_ROLLOUTS: usize = 8;
const DAY_MIN: i64 = 24 * 60;
const TOP_PROJECTS: usize = 5;
/// how often to ask the app-server for the limits
const LIVE_MS: i64 = 2 * 60_000;
/// a process spawn per try: back off when one fails
const LIVE_RETRY_MS: i64 = 5 * 60_000;
const LIVE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq)]
struct RawWindow {
    pct: f64,
    resets_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
struct Limits {
    ts: i64,
    plan: Option<String>,
    session: Option<RawWindow>,
    weekly: Option<RawWindow>,
}

/// A limit window as it stands now. A window whose reset has passed tells us
/// nothing about today: its % is unknown until Codex writes a fresh one.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub pct: Option<f64>,
    pub reset_ms: Option<i64>,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsage {
    pub active: bool,
    pub last_seen: Option<i64>,
    pub plan: Option<String>,
    /// with no 5-hour window, the week is the budget that binds: it takes
    /// the session's place
    pub session: Option<Window>,
    pub weekly: Option<Window>,
    pub weekly_only: bool,
    pub limits_at: Option<i64>,
    pub tokens_today: u64,
    pub tokens_5h: u64,
    pub tokens_week: u64,
    pub by_model: Vec<Labeled>,
    pub by_project: Vec<Labeled>,
    pub days30: Vec<u64>,
    pub month_tokens: u64,
}

#[derive(Debug, Clone)]
struct Event {
    ts: i64,
    tokens: u64,
    model: Option<String>,
    cwd: Option<String>,
}

#[derive(Default)]
struct Summary {
    offset: u64,
    rest: Vec<u8>,
    last_total: u64,
    events: Vec<Event>,
    limits: Option<Limits>,
    model: Option<String>,
    cwd: Option<String>,
    /// the last task: open (`task_started`) or closed; None before any
    task: Option<bool>,
    mtime: i64,
}

/// Where the codex binary is: the desktop app's bundled CLI first (a GUI
/// app doesn't get the shell's PATH), else `codex` on the PATH.
fn find_bin() -> PathBuf {
    let home = std::env::home_dir().unwrap_or_default();
    for dir in [PathBuf::from("/Applications"), home.join("Applications")] {
        for f in ["ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex", "Codex.app/Contents/Resources/codex"] {
            let p = dir.join(f);
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from("codex")
}

// a token_count's rate limits. Codex reports several buckets; some come with
// null windows (`premium` on Plus): only one with a real window counts. Plus
// has a 5-hour and a weekly window; some plans only the weekly, in the
// `primary` slot: sort them by length, not by slot.
fn limits_of(rl: &Value, ts: i64) -> Option<Limits> {
    let (primary, secondary) = (rl.get("primary").filter(|v| !v.is_null()), rl.get("secondary").filter(|v| !v.is_null()));
    if primary.is_none() && secondary.is_none() {
        return None;
    }
    let mut session = None;
    let mut weekly = None;
    for (w, is_secondary) in [(primary, false), (secondary, true)] {
        let Some(w) = w else { continue };
        let Some(pct) = w.get("used_percent").and_then(Value::as_f64) else { continue };
        let resets_at = w.get("resets_at").and_then(Value::as_f64).map(|s| (s * 1000.0) as i64);
        let long = match w.get("window_minutes").and_then(Value::as_i64) {
            Some(m) => m >= DAY_MIN,
            None => is_secondary,
        };
        let v = RawWindow { pct, resets_at };
        if long {
            weekly = Some(v);
        } else {
            session = Some(v);
        }
    }
    if session.is_none() && weekly.is_none() {
        return None;
    }
    Some(Limits { ts, plan: rl.get("plan_type").and_then(Value::as_str).map(str::to_string), session, weekly })
}

// the app-server reports the same windows as the rollouts, in camelCase
fn limits_from_live(result: &Value, ts: i64) -> Option<Limits> {
    let rl = result.pointer("/rateLimitsByLimitId/codex").or_else(|| result.get("rateLimits")).filter(|v| !v.is_null())?;
    let win = |w: Option<&Value>| match w.filter(|v| !v.is_null()) {
        Some(w) => json!({"used_percent": w["usedPercent"], "window_minutes": w["windowDurationMins"], "resets_at": w["resetsAt"]}),
        None => Value::Null,
    };
    limits_of(&json!({"primary": win(rl.get("primary")), "secondary": win(rl.get("secondary")), "plan_type": rl["planType"]}), ts)
}

/// One `account/rateLimits/read` over the app-server's stdio JSON-RPC.
fn read_live(bin: &Path, codex_home: &Path, ts: i64) -> Option<Limits> {
    let mut child =
        Command::new(bin).arg("app-server").env("CODEX_HOME", codex_home).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let send = |stdin: &mut std::process::ChildStdin, v: Value| writeln!(stdin, "{v}").and_then(|_| stdin.flush());
    let mut out = None;
    if send(&mut stdin, json!({"id": 1, "method": "initialize", "params": {"clientInfo": {"name": "clauddy", "version": "0"}}})).is_ok() {
        let deadline = Instant::now() + LIVE_TIMEOUT;
        while let Ok(line) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            match msg.get("id").and_then(Value::as_i64) {
                Some(1) => {
                    let _ = send(&mut stdin, json!({"method": "initialized"}));
                    let _ = send(&mut stdin, json!({"id": 2, "method": "account/rateLimits/read"}));
                }
                Some(2) => {
                    out = msg.get("result").and_then(|r| limits_from_live(r, ts));
                    break;
                }
                _ => {}
            }
        }
    }
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    out
}

fn ingest_line(sum: &mut Summary, line: &[u8]) {
    // cheap filter before parsing: rollouts are mostly message bodies
    let has = |needle: &[u8]| memchr::memmem::find(line, needle).is_some();
    if !has(b"\"token_count\"") && !has(b"\"turn_context\"") && !has(b"\"task_") && !has(b"\"turn_aborted\"") {
        return;
    }
    let Ok(o) = serde_json::from_slice::<Value>(line) else { return };
    let p = o.get("payload").cloned().unwrap_or(Value::Null);
    let ts = o.get("timestamp").and_then(Value::as_str).and_then(parse_date).unwrap_or(0);
    if o.get("type").and_then(Value::as_str) == Some("turn_context") {
        if let Some(m) = p.get("model").and_then(Value::as_str) {
            sum.model = Some(m.into());
        }
        if let Some(c) = p.get("cwd").and_then(Value::as_str) {
            sum.cwd = Some(c.into());
        }
        return;
    }
    match p.get("type").and_then(Value::as_str) {
        Some("task_started") => sum.task = Some(true),
        Some("task_complete" | "turn_aborted") => sum.task = Some(false),
        _ => {}
    }
    if p.get("type").and_then(Value::as_str) != Some("token_count") {
        return;
    }
    // totals are cumulative per session and often emitted twice: the delta is
    // what this event spent, on the model and project in effect now
    if let Some(total) = p.pointer("/info/total_token_usage/total_tokens").and_then(Value::as_u64)
        && total > sum.last_total
    {
        sum.events.push(Event { ts, tokens: total - sum.last_total, model: sum.model.clone(), cwd: sum.cwd.clone() });
        sum.last_total = total;
    }
    if let Some(lim) = p.get("rate_limits").and_then(|rl| limits_of(rl, ts))
        && sum.limits.as_ref().is_none_or(|l| lim.ts >= l.ts)
    {
        sum.limits = Some(lim);
    }
}

/// Working means a task is open in one of the newest rollouts (each one's
/// last task and when it was written), even when a long command or a slow
/// answer kept it quiet. A rollout with no task in it falls back to:
/// written in the last minute.
fn working(recent: &mut [(Option<bool>, i64)], now: i64) -> bool {
    recent.sort_by_key(|r| std::cmp::Reverse(r.1));
    for (i, &(task, mtime)) in recent.iter().take(RECENT_ROLLOUTS).enumerate() {
        match task {
            Some(true) => return true,
            None if i == 0 => return now - mtime < ACTIVE_MS,
            _ => {}
        }
    }
    false
}

fn mtime_ms(meta: &fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn window_now(w: Option<&RawWindow>, now: i64) -> Option<Window> {
    let w = w?;
    let expired = w.resets_at.is_some_and(|r| r <= now);
    Some(Window { pct: (!expired).then_some(w.pct), reset_ms: w.resets_at.filter(|_| !expired).map(|r| r - now), expired })
}

fn ranked(map: HashMap<String, u64>, order: &[String]) -> Vec<Labeled> {
    // first-seen order breaks ties, as a JS Map does
    let mut rows: Vec<Labeled> = order.iter().filter_map(|k| map.get(k).map(|&t| Labeled { label: k.clone(), tokens: t })).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.tokens));
    rows
}

pub struct Codex {
    dir: PathBuf,
    files: HashMap<PathBuf, Summary>,
    live: Option<Limits>,
    last_try: i64,
    last_failed: bool,
    /// None: find it; Some(None): never spawn (tests); Some(path): that one
    bin: Option<Option<PathBuf>>,
    pending: Option<Receiver<Option<Limits>>>,
}

impl Codex {
    pub fn new(dir: Option<PathBuf>) -> Self {
        let dir = dir.unwrap_or_else(|| match std::env::var_os("CODEX_HOME") {
            Some(d) if !d.is_empty() => PathBuf::from(d),
            _ => std::env::home_dir().unwrap_or_default().join(".codex"),
        });
        Codex { dir, files: HashMap::new(), live: None, last_try: 0, last_failed: false, bin: None, pending: None }
    }

    #[cfg(test)]
    fn set_bin(&mut self, bin: Option<PathBuf>) {
        self.bin = Some(bin);
    }

    fn bin(&self) -> Option<PathBuf> {
        match &self.bin {
            Some(b) => b.clone(),
            None => Some(find_bin()),
        }
    }

    // a finished background read, if one came back: a failed one keeps the
    // last good reading (it ages into stale, never into zero)
    fn collect(&mut self) {
        let Some(rx) = &self.pending else { return };
        if let Ok(result) = rx.try_recv() {
            self.last_failed = result.is_none();
            if result.is_some() {
                self.live = result;
            }
            self.pending = None;
        }
    }

    /// Ask the app-server now and wait for the answer (tests, and the first read).
    #[cfg(test)]
    fn refresh_blocking(&mut self, now: i64) {
        let Some(bin) = self.bin() else { return };
        self.last_try = now;
        let result = read_live(&bin, &self.dir, now);
        self.last_failed = result.is_none();
        if result.is_some() {
            self.live = result;
        }
    }

    // ask the app-server in the background, at most once per interval
    fn maybe_refresh(&mut self, now: i64) {
        self.collect();
        let every = if self.last_failed { LIVE_RETRY_MS } else { LIVE_MS };
        if self.pending.is_some() || now - self.last_try < every {
            return;
        }
        let Some(bin) = self.bin() else { return };
        self.last_try = now;
        let (tx, rx) = mpsc::channel();
        let dir = self.dir.clone();
        std::thread::Builder::new()
            .name("codex-live".into())
            .spawn(move || {
                let _ = tx.send(read_live(&bin, &dir, now));
            })
            .ok();
        self.pending = Some(rx);
    }

    // every rollout, with its stat; `since` drops the ones untouched since
    fn rollouts(&self, since: i64) -> Vec<(PathBuf, u64, i64)> {
        let mut out = vec![];
        fn walk(dir: &Path, depth: u8, since: i64, out: &mut Vec<(PathBuf, u64, i64)>) {
            let Ok(kids) = fs::read_dir(dir) else { return };
            let mut kids: Vec<_> = kids.flatten().collect();
            kids.sort_by_key(|k| k.file_name());
            for k in kids {
                let Ok(ft) = k.file_type() else { continue };
                let p = k.path();
                if ft.is_dir() && depth < 3 {
                    walk(&p, depth + 1, since, out);
                } else if ft.is_file()
                    && p.extension().is_some_and(|e| e == "jsonl")
                    && let Ok(meta) = k.metadata()
                {
                    let m = mtime_ms(&meta);
                    if m >= since {
                        out.push((p, meta.len(), m));
                    }
                }
            }
        }
        walk(&self.dir.join("sessions"), 0, since, &mut out);
        out
    }

    // Rollouts only grow: a poll reads just the bytes appended since the last.
    fn read_file(&mut self, path: &Path, size: u64, mtime: i64) -> &Summary {
        let sum = self.files.entry(path.to_path_buf()).or_default();
        if size < sum.offset {
            *sum = Summary::default(); // shrunk or replaced: start over
        }
        if size > sum.offset
            && let Ok(mut f) = File::open(path)
            && f.seek(SeekFrom::Start(sum.offset)).is_ok()
        {
            let mut buf = std::mem::take(&mut sum.rest);
            if f.take(size - sum.offset).read_to_end(&mut buf).is_ok() {
                sum.offset = size;
                // the last piece may be a line still being written
                let end = memchr::memrchr(b'\n', &buf);
                let (whole, rest) = match end {
                    Some(e) => (buf[..e].to_vec(), buf[e + 1..].to_vec()),
                    None => (vec![], buf),
                };
                sum.rest = rest;
                for line in whole.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
                    ingest_line(sum, line);
                }
            }
        }
        sum.mtime = mtime;
        sum
    }

    /// Is there a Codex session on this machine? The folder alone isn't enough.
    pub fn detect(&mut self) -> (bool, Option<String>) {
        let mut files = self.rollouts(0);
        if files.is_empty() {
            return (false, None);
        }
        files.sort_by_key(|f| std::cmp::Reverse(f.2));
        for (p, size, m) in files.into_iter().take(5) {
            if let Some(plan) = self.read_file(&p, size, m).limits.as_ref().and_then(|l| l.plan.clone()) {
                return (true, Some(plan));
            }
        }
        (true, None)
    }

    pub fn get_usage(&mut self, now: i64) -> Option<CodexUsage> {
        if !self.dir.join("sessions").exists() {
            return None;
        }
        self.maybe_refresh(now);
        let start_of_day = local_midnight(now);
        let start30 = start_of_day - (DAYS as i64 - 1) * DAY;
        let files = self.rollouts(start30);
        let mut seen = HashSet::new();
        let mut limits: Option<Limits> = None;
        let mut latest: Option<i64> = None;
        let (mut today, mut h5, mut week, mut month) = (0u64, 0u64, 0u64, 0u64);
        let mut days30 = vec![0u64; DAYS];
        let (mut by_model, mut model_order) = (HashMap::new(), vec![]);
        let (mut by_project, mut project_order) = (HashMap::new(), vec![]);
        let tally = |map: &mut HashMap<String, u64>, order: &mut Vec<String>, k: String, t: u64| {
            if !map.contains_key(&k) {
                order.push(k.clone());
            }
            *map.entry(k).or_insert(0) += t;
        };
        let mut recent: Vec<(Option<bool>, i64)> = vec![];
        for (p, size, m) in files {
            seen.insert(p.clone());
            let sum = self.read_file(&p, size, m);
            if now - m <= OPEN_TURN_MS {
                recent.push((sum.task, m));
            }
            if let Some(l) = &sum.limits
                && limits.as_ref().is_none_or(|x| l.ts > x.ts)
            {
                limits = Some(l.clone());
            }
            if latest.is_none_or(|t| sum.mtime > t) {
                latest = Some(sum.mtime);
            }
            for e in sum.events.clone() {
                if e.ts < start30 {
                    continue;
                }
                month += e.tokens;
                days30[(((e.ts - start30) / DAY) as usize).min(DAYS - 1)] += e.tokens;
                if e.ts >= start_of_day {
                    today += e.tokens;
                }
                if now - e.ts < 5 * 3_600_000 {
                    h5 += e.tokens;
                }
                if now - e.ts < WEEK {
                    week += e.tokens;
                    tally(&mut by_model, &mut model_order, e.model.clone().unwrap_or_else(|| "unknown".into()), e.tokens);
                    if let Some(cwd) = &e.cwd {
                        let name = Path::new(cwd).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| cwd.clone());
                        tally(&mut by_project, &mut project_order, name, e.tokens);
                    }
                }
            }
        }
        // whichever is newer: a rollout written after the live read beats it
        if let Some(live) = &self.live
            && limits.as_ref().is_none_or(|l| live.ts >= l.ts)
        {
            limits = Some(live.clone());
        }
        self.files.retain(|f, _| seen.contains(f)); // forget files that aged out

        let projects = ranked(by_project, &project_order);
        let mut by_project: Vec<Labeled> = projects.iter().take(TOP_PROJECTS).cloned().collect();
        let rest = &projects[projects.len().min(TOP_PROJECTS)..];
        if !rest.is_empty() {
            by_project.push(Labeled { label: format!("other · {}", rest.len()), tokens: rest.iter().map(|p| p.tokens).sum() });
        }
        let weekly_only = limits.as_ref().is_some_and(|l| l.weekly.is_some() && l.session.is_none());
        Some(CodexUsage {
            active: working(&mut recent, now),
            last_seen: latest,
            plan: limits.as_ref().and_then(|l| l.plan.clone()),
            session: limits.as_ref().and_then(|l| window_now(if weekly_only { l.weekly.as_ref() } else { l.session.as_ref() }, now)),
            weekly: limits.as_ref().filter(|_| !weekly_only).and_then(|l| window_now(l.weekly.as_ref(), now)),
            weekly_only,
            limits_at: limits.as_ref().map(|l| l.ts),
            tokens_today: today,
            tokens_5h: h5,
            tokens_week: week,
            by_model: ranked(by_model, &model_order),
            by_project,
            days30,
            month_tokens: month,
        })
    }
}

#[cfg(test)]
mod tests;
