// Claude Code's local logs: tokens, the 5-hour session window, whether a turn
// is open, and what Claude is doing. Port of usage.js.

use chrono::{Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const DAY_MS: i64 = 24 * 3600 * 1000;
const DAYS: usize = 30;
// how many projects the panel ranks before folding the rest into `other`
const TOP_PROJECTS: usize = 5;
const SESSION_MS: i64 = 5 * 3600 * 1000;
const CHUNK: u64 = 8 * 1024 * 1024;
const LABEL_MAX: usize = 26;
// a turn left open this long without a write was abandoned (a closed
// terminal, a crash) — or a permission prompt nobody answered
const OPEN_TURN_MS: i64 = 30 * 60000;
// parallel sessions and subagents each keep a log; a handful covers them
const RECENT_LOGS: usize = 8;
const TAIL_BYTES: u64 = 65536;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub plan: Option<String>,
    pub session_token_budget: Option<f64>,
    pub weekly_token_budget: Option<f64>,
    pub weekly_anchor_iso: Option<String>,
    pub active_threshold_ms: Option<i64>,
    pub sleep_threshold_ms: Option<i64>,
}

// Plan presets (token budgets ~= 100%). Calibrated for Max 5x from the official
// panel; Pro/Max20x scaled by the plan multiplier. ESTIMATES.
pub fn plan_budget(plan: &str) -> Option<(f64, f64)> {
    match plan {
        "pro" => Some((126e6, 690e6)),
        "max5x" => Some((630e6, 3450e6)),
        "max20x" => Some((2520e6, 13800e6)),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub tokens: u64,
    pub pct: f64,
    pub reset_ms: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Week {
    pub tokens: u64,
    pub pct: f64,
    pub reset_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Labeled {
    pub label: String,
    pub tokens: u64,
}

/// What the panel's lists can cover.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Span {
    Today,
    #[default]
    Week,
    Month,
}

impl Span {
    pub const ALL: [Span; 3] = [Span::Today, Span::Week, Span::Month];

    pub fn index(self) -> usize {
        self as usize
    }

    /// As the section titles say it: "by model · 7 days".
    pub fn label(self) -> &'static str {
        ["today", "7 days", "30 days"][self.index()]
    }
}

/// One project over a span: what it used, on which models, day by day, and
/// in which of its worktrees.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub label: String,
    pub tokens: u64,
    /// every token sent in (fresh, written to the cache, read from it), and
    /// the part read from the cache: the share that cost the least
    pub input: u64,
    pub cache_read: u64,
    pub by_model: Vec<Labeled>,
    /// the last 7 days, oldest first, whatever the span
    pub days: Vec<Labeled>,
    /// by worktree, the project's own checkout among them; empty when no
    /// worktree was used
    pub worktrees: Vec<Labeled>,
}

/// The lists over one span.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub tokens: u64,
    pub by_model: Vec<Labeled>,
    /// ranked, the tail folded into `other`
    pub by_project: Vec<Labeled>,
    /// the ranked projects in detail (not the folded tail)
    pub projects: Vec<Project>,
    /// the heaviest sessions, by tokens
    pub sessions: Vec<Labeled>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub session: Session,
    pub week: Week,
    pub today_tokens: u64,
    pub by_model: Vec<Labeled>,
    pub by_project: Vec<Labeled>,
    /// the lists over each span, in `Span::ALL` order (`by_model` and
    /// `by_project` above are the week's)
    pub periods: Vec<Period>,
    pub days30: Vec<u64>,
    pub month_tokens: u64,
    pub tokens_per_min: u64,
    pub active: bool,
    pub sleeping: bool,
    pub activity: Option<String>,
    /// None when no log was ever written
    pub last_activity_ms: Option<i64>,
    pub ts: i64,
}

impl Usage {
    /// Nothing read yet: zeros, for screens that hold their shape while the
    /// first poll is under way.
    pub fn empty() -> Self {
        Usage {
            session: Session::default(),
            week: Week::default(),
            today_tokens: 0,
            by_model: vec![],
            by_project: vec![],
            periods: vec![Period::default(); Span::ALL.len()],
            days30: vec![0; DAYS],
            month_tokens: 0,
            tokens_per_min: 0,
            active: false,
            sleeping: false,
            activity: None,
            last_activity_ms: None,
            ts: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    ts: i64,
    model: Option<String>,
    tokens: u64,
    /// every token sent in, and the part read from the cache
    input: u64,
    cache_read: u64,
    key: String,
}

#[derive(Default)]
struct FileState {
    offset: u64,
    rest: Vec<u8>,
    entries: Vec<Entry>,
}

#[derive(Clone)]
struct LogFile {
    path: PathBuf,
    size: u64,
    mtime: i64,
    /// which of the reader's projects dirs it is under
    root: usize,
}

/// Reads one Claude config dir's logs (on Windows, when opted in, the default
/// dir also takes in those of Claude Code running in WSL). The file cache is keyed by
/// absolute path, so switching dirs and back stays instant; the label cache
/// is keyed by directory *name*, which can repeat across dirs, so it is dropped.
pub struct Reader {
    claude_dir: PathBuf,
    files: HashMap<PathBuf, FileState>,
    labels: HashMap<String, ProjectRef>,
    /// reading the default dir (the only one WSL's logs join)
    default_dir: bool,
    /// whether to look into WSL too: opted into, on Windows
    wsl: bool,
    /// the WSL dirs last found, written by a discovery thread
    wsl_dirs: Arc<Mutex<Vec<crate::platform::wsl::ClaudeDir>>>,
    /// when a discovery was last started, and whether one is running
    wsl_asked: i64,
    wsl_busy: Arc<AtomicBool>,
    /// what the last walk of each WSL projects dir found
    walked: HashMap<PathBuf, Walked>,
}

// how often to look again for running distros (wsl.exe is a process spawn)
const WSL_RECHECK_MS: i64 = 60_000;
// how often a projects dir behind a slow filesystem is walked whole
const WALK_MS: i64 = 60_000;

pub fn default_claude_dir() -> PathBuf {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => std::env::home_dir().unwrap_or_default().join(".claude"),
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn mtime_ms(meta: &fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0)
}

impl Reader {
    pub fn new(claude_dir: Option<PathBuf>) -> Self {
        let default_dir = claude_dir.is_none();
        Reader {
            claude_dir: claude_dir.unwrap_or_else(default_claude_dir),
            files: HashMap::new(),
            labels: HashMap::new(),
            default_dir,
            wsl: false,
            wsl_dirs: Arc::default(),
            wsl_asked: 0,
            wsl_busy: Arc::default(),
            walked: HashMap::new(),
        }
    }

    /// Also read Claude Code's logs in running WSL distros (Windows only; the
    /// Settings switch). Off, wsl.exe is never run.
    pub fn set_wsl(&mut self, on: bool) {
        self.wsl = cfg!(windows) && on;
        if !self.wsl {
            // turned on again, it looks at once
            self.wsl_asked = 0;
            self.walked.clear();
        }
    }

    /// Point the reader at another account's logs (None = the default dir).
    pub fn set_claude_dir(&mut self, dir: Option<PathBuf>) {
        self.default_dir = dir.is_none();
        let next = dir.unwrap_or_else(default_claude_dir);
        if next == self.claude_dir {
            return;
        }
        self.claude_dir = next;
        self.labels.clear();
    }

    /// Every projects dir to read, each with the filesystem root its project
    /// names are paths from: this machine's, then any running WSL distro's.
    /// The distros are looked for on a thread of their own, so a slow
    /// `wsl.exe` never holds up a poll: until it answers, the last ones found
    /// are read.
    fn projects_dirs(&mut self, now: i64) -> Vec<(PathBuf, PathBuf)> {
        let mut dirs = vec![(PathBuf::from("/"), self.claude_dir.join("projects"))];
        if self.wsl && self.default_dir {
            let due = self.wsl_asked == 0 || now - self.wsl_asked >= WSL_RECHECK_MS || now < self.wsl_asked;
            if due && !self.wsl_busy.swap(true, Ordering::AcqRel) {
                self.wsl_asked = now;
                let (found, busy) = (Arc::clone(&self.wsl_dirs), Arc::clone(&self.wsl_busy));
                std::thread::spawn(move || {
                    let dirs = crate::platform::wsl::claude_dirs();
                    if let Ok(mut f) = found.lock() {
                        *f = dirs;
                    }
                    busy.store(false, Ordering::Release);
                });
            }
            let found = self.wsl_dirs.lock().map(|f| f.clone()).unwrap_or_default();
            dirs.extend(found.iter().map(|d| (d.root.clone(), d.dir.join("projects"))));
        }
        dirs
    }

    pub fn get_usage(&mut self, cfg: &Config) -> Usage {
        self.get_usage_at(cfg, now_ms())
    }

    pub fn get_usage_at(&mut self, cfg: &Config, now: i64) -> Usage {
        let today_ms = local_midnight(now);
        let mut week_start = now - 7 * DAY_MS;
        let mut week_reset_ms = None;
        if let Some(anchor) = cfg.weekly_anchor_iso.as_deref().and_then(parse_date) {
            let period = 7 * DAY_MS;
            let last_reset = anchor + (now - anchor).div_euclid(period) * period;
            week_start = last_reset;
            week_reset_ms = Some(last_reset + period - now);
        }
        let start30 = today_ms - (DAYS as i64 - 1) * DAY_MS;
        let five_min_ago = now - 5 * 60000;
        let recent_cutoff = now - 12 * 3600 * 1000;
        let scan_cutoff = start30 - DAY_MS;

        let roots = self.projects_dirs(now);
        let mut files = Vec::new();
        for (i, (_, projects_dir)) in roots.iter().enumerate() {
            let from = files.len();
            if i == 0 {
                walk_jsonl(projects_dir, &mut files, scan_cutoff);
            } else {
                // WSL's, over 9P: a whole walk costs under 1 ms a directory
                files.extend(walk_cached(self.walked.entry(projects_dir.clone()).or_default(), projects_dir, now, scan_cutoff));
            }
            files[from..].iter_mut().for_each(|f| f.root = i);
        }

        let newest = files.iter().max_by_key(|f| f.mtime);
        let last_mtime = newest.map(|f| f.mtime).unwrap_or(0);
        let newest_file = newest.map(|f| f.path.clone());

        let mut seen = HashSet::new();
        let mut last5m = 0u64;
        let mut spans: [Acc; 3] = Default::default();
        let mut days30 = vec![0u64; DAYS];
        let mut recent: Vec<(i64, u64)> = Vec::new();
        let starts = [today_ms, week_start, start30];
        let week_ago = today_ms - (WEEK_DAYS as i64 - 1) * DAY_MS;

        for f in &files {
            let (fs_root, projects_dir) = &roots[f.root];
            let at = log_place(projects_dir, &f.path);
            let proj = at.as_ref().map(|(dir, _)| project_of(&mut self.labels, fs_root, dir));
            let session = at.map(|(dir, id)| format!("{}|{dir}|{id}", f.root));
            let entries = parse_file(&mut self.files, &f.path, f.size);
            for e in entries {
                if e.ts < start30 {
                    continue;
                }
                if e.key != ":" && !seen.insert(e.key.clone()) {
                    continue;
                }
                let t = e.tokens;
                let idx = ((e.ts - start30) / DAY_MS).clamp(0, DAYS as i64 - 1) as usize;
                days30[idx] += t;
                let day = (e.ts >= week_ago).then(|| ((e.ts - week_ago) / DAY_MS).clamp(0, WEEK_DAYS as i64 - 1) as usize);
                for (acc, start) in spans.iter_mut().zip(starts) {
                    if e.ts >= start {
                        acc.add(e, proj.as_ref(), session.as_deref(), day);
                    }
                }
                if e.ts >= recent_cutoff {
                    recent.push((e.ts, t));
                }
                if e.ts >= five_min_ago {
                    last5m += t;
                }
            }
        }
        let [today, week, month] = spans.each_ref().map(|acc| acc.tokens);

        // 5h session window (like /usage "Current session"): a window opens at
        // the first turn after the previous one expired
        recent.sort_by_key(|e| e.0);
        let mut window: Option<(i64, i64)> = None;
        for &(ts, _) in &recent {
            if window.is_none_or(|(_, end)| ts >= end) {
                window = Some((ts, ts + SESSION_MS));
            }
        }
        let (session_budget, weekly_budget) = match cfg.plan.as_deref().and_then(plan_budget) {
            Some(b) => b,
            None => (cfg.session_token_budget.unwrap_or(0.0), cfg.weekly_token_budget.unwrap_or(0.0)),
        };
        let pct = |tk: u64, budget: f64| if budget > 0.0 { (tk as f64 / budget * 100.0).min(100.0) } else { 0.0 };

        let mut session = Session::default();
        if let Some((start, end)) = window.filter(|&(_, end)| now < end) {
            let tk = recent.iter().filter(|e| e.0 >= start && e.0 < end).map(|e| e.1).sum();
            session = Session { tokens: tk, pct: pct(tk, session_budget), reset_ms: end - now, active: true };
        }

        let last_activity_ms = (last_mtime > 0).then(|| now - last_mtime);
        // Working means a turn is open in one of the recent logs, even when a
        // long command or a slow answer kept it quiet. A log with nothing to
        // say falls back to: written in the last few seconds.
        let mut recent_logs: Vec<&LogFile> = files.iter().filter(|f| now - f.mtime <= OPEN_TURN_MS).collect();
        recent_logs.sort_by_key(|f| std::cmp::Reverse(f.mtime));
        let mut open_file = None;
        let mut newest_says = None;
        for (i, f) in recent_logs.iter().take(RECENT_LOGS).enumerate() {
            let open = tail_lines(&f.path).and_then(|l| turn_open(&l));
            if i == 0 {
                newest_says = open;
            }
            if open == Some(true) {
                open_file = Some(f.path.clone());
                break;
            }
        }
        let active_threshold = cfg.active_threshold_ms.filter(|&v| v != 0).unwrap_or(20000);
        let sleep_threshold = cfg.sleep_threshold_ms.filter(|&v| v != 0).unwrap_or(300000);
        let quiet_rule = newest_says.is_none() && last_activity_ms.is_some_and(|ms| ms <= active_threshold);
        let active = open_file.is_some() || quiet_rule;
        let sleeping = !active && last_activity_ms.is_none_or(|ms| ms >= sleep_threshold);

        let active_file = open_file.clone().or(if quiet_rule { newest_file } else { None });
        let activity = active_file.and_then(|f| detect_activity(&f)).or_else(|| open_file.map(|_| "working".into()));

        // the day split is the last 7 days' whatever the span, so it is read
        // from the widest one
        let day_names: Vec<String> =
            (0..WEEK_DAYS as i64).map(|d| crate::format::local(week_ago + d * DAY_MS + DAY_MS / 2).format("%a %-d").to_string()).collect();
        let periods: Vec<Period> = spans.iter().map(|acc| acc.finish(&spans[2], &day_names)).collect();
        let by_project = periods[Span::Week.index()].by_project.clone();

        Usage {
            session,
            week: Week { tokens: week, pct: pct(week, weekly_budget), reset_ms: week_reset_ms },
            today_tokens: today,
            by_model: periods[Span::Week.index()].by_model.clone(),
            by_project,
            periods,
            days30,
            month_tokens: month,
            tokens_per_min: (last5m as f64 / 5.0).round() as u64,
            active,
            sleeping,
            activity,
            last_activity_ms,
            ts: now,
        }
    }
}

// tokens per label, in first-seen order so ties rank like the JS Map did
#[derive(Default)]
struct Tally {
    index: HashMap<String, usize>,
    rows: Vec<Labeled>,
}

impl Tally {
    fn add(&mut self, label: String, tokens: u64) {
        match self.index.get(&label) {
            Some(&i) => self.rows[i].tokens += tokens,
            None => {
                self.index.insert(label.clone(), self.rows.len());
                self.rows.push(Labeled { label, tokens });
            }
        }
    }
    fn ranked(&self) -> Vec<Labeled> {
        let mut rows = self.rows.clone();
        rows.sort_by_key(|r| std::cmp::Reverse(r.tokens)); // stable
        rows
    }
}

// how many rows a project's worktrees and the sessions take before the fold
const TOP_WORKTREES: usize = 5;
const TOP_SESSIONS: usize = 5;
const WEEK_DAYS: usize = 7;
// a project's own checkout, among its worktrees
const MAIN_CHECKOUT: &str = "main checkout";

/// The top `n` rows, the rest folded into one `other · N` row.
fn folded(ranked: &[Labeled], n: usize) -> Vec<Labeled> {
    let mut rows: Vec<Labeled> = ranked.iter().take(n).cloned().collect();
    let rest = &ranked[ranked.len().min(n)..];
    if !rest.is_empty() {
        rows.push(Labeled { label: format!("other · {}", rest.len()), tokens: rest.iter().map(|p| p.tokens).sum() });
    }
    rows
}

#[derive(Default)]
struct ProjAcc {
    label: String,
    parent: String,
    tokens: u64,
    input: u64,
    cache_read: u64,
    by_model: Tally,
    worktrees: Tally,
    days: [u64; WEEK_DAYS],
}

#[derive(Default)]
struct SessAcc {
    project: String,
    first: i64,
    tokens: u64,
}

/// Everything counted over one span, before it is ranked.
#[derive(Default)]
struct Acc {
    tokens: u64,
    by_model: Tally,
    // by the project's path, so two projects of the same name stay two; in
    // first-seen order so ties rank the same on every poll
    projects: HashMap<String, ProjAcc>,
    project_order: Vec<String>,
    sessions: HashMap<String, SessAcc>,
    session_order: Vec<String>,
}

impl Acc {
    fn add(&mut self, e: &Entry, proj: Option<&ProjectRef>, session: Option<&str>, day: Option<usize>) {
        self.tokens += e.tokens;
        self.by_model.add(label_for(e.model.as_deref()), e.tokens);
        let Some(proj) = proj else { return };
        if !self.projects.contains_key(&proj.key) {
            self.project_order.push(proj.key.clone());
        }
        let p = self.projects.entry(proj.key.clone()).or_default();
        if p.label.is_empty() {
            (p.label, p.parent) = (proj.label.clone(), proj.parent.clone());
        }
        p.tokens += e.tokens;
        p.input += e.input;
        p.cache_read += e.cache_read;
        p.by_model.add(label_for(e.model.as_deref()), e.tokens);
        p.worktrees.add(proj.worktree.clone().unwrap_or_else(|| MAIN_CHECKOUT.into()), e.tokens);
        if let Some(d) = day {
            p.days[d] += e.tokens;
        }
        let Some(session) = session else { return };
        if !self.sessions.contains_key(session) {
            self.session_order.push(session.to_string());
        }
        let s = self.sessions.entry(session.to_string()).or_default();
        if s.first == 0 || e.ts < s.first {
            s.first = e.ts;
        }
        s.project = proj.key.clone();
        s.tokens += e.tokens;
    }

    /// The name a project shows under: its directory's, with its parent's
    /// when another project here has the same.
    fn name(&self, key: &str) -> String {
        let Some(p) = self.projects.get(key) else { return String::new() };
        let twin = self.projects.iter().any(|(k, o)| k != key && o.label == p.label);
        if twin && !p.parent.is_empty() { format!("{}/{}", p.parent, p.label) } else { p.label.clone() }
    }

    /// Ranked and folded. `widest` is the 30 days' count, for the day split.
    fn finish(&self, widest: &Acc, day_names: &[String]) -> Period {
        let mut keys: Vec<&String> = self.project_order.iter().collect();
        keys.sort_by_key(|k| std::cmp::Reverse(self.projects[*k].tokens)); // stable
        let ranked: Vec<Labeled> = keys.iter().map(|k| Labeled { label: self.name(k), tokens: self.projects[*k].tokens }).collect();
        let projects = keys
            .iter()
            .take(TOP_PROJECTS)
            .map(|k| {
                let p = &self.projects[*k];
                let days = widest.projects.get(*k).map(|w| w.days).unwrap_or_default();
                let trees = p.worktrees.ranked();
                Project {
                    label: self.name(k),
                    tokens: p.tokens,
                    input: p.input,
                    cache_read: p.cache_read,
                    by_model: p.by_model.ranked(),
                    days: day_names.iter().zip(days).map(|(name, tokens)| Labeled { label: name.clone(), tokens }).collect(),
                    worktrees: if trees.iter().any(|t| t.label != MAIN_CHECKOUT) { folded(&trees, TOP_WORKTREES) } else { vec![] },
                }
            })
            .collect();
        let sessions: Vec<Labeled> = self
            .session_order
            .iter()
            .map(|k| {
                let s = &self.sessions[k];
                let started = crate::format::local(s.first).format("%a %-I:%M %p");
                Labeled { label: format!("{} · {started}", self.name(&s.project)), tokens: s.tokens }
            })
            .collect();
        let mut top = sessions;
        top.sort_by_key(|r| std::cmp::Reverse(r.tokens)); // stable
        top.truncate(TOP_SESSIONS);
        Period { tokens: self.tokens, by_model: self.by_model.ranked(), by_project: folded(&ranked, TOP_PROJECTS), projects, sessions: top }
    }
}

pub fn local_midnight(now: i64) -> i64 {
    let date = Local.timestamp_millis_opt(now).single().map(|d| d.date_naive()).unwrap_or_default();
    Local.from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default()).earliest().map(|d| d.timestamp_millis()).unwrap_or(now - now.rem_euclid(DAY_MS))
}

/// ISO dates as Date.parse reads them: with a zone, or local time without one.
pub fn parse_date(s: &str) -> Option<i64> {
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(d.timestamp_millis());
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return Local.from_local_datetime(&n).earliest().map(|d| d.timestamp_millis());
        }
    }
    None
}

pub fn label_for(model: Option<&str>) -> String {
    let Some(model) = model.filter(|m| !m.is_empty()) else {
        return "desconhecido".into();
    };
    let m = model.to_lowercase();
    let fam = [("opus", "Opus"), ("sonnet", "Sonnet"), ("haiku", "Haiku"), ("fable", "Fable"), ("mythos", "Mythos")]
        .into_iter()
        .find(|(k, _)| m.contains(k))
        .map(|(_, v)| v);
    let Some(fam) = fam else {
        return model.to_string();
    };
    match family_version(&m).or_else(|| legacy_version(&m)) {
        Some((a, Some(b))) => format!("{fam} {a}.{b}"),
        Some((a, None)) => format!("{fam} {a}"),
        None => fam.to_string(),
    }
}

fn digit_at(b: &[u8], i: usize) -> bool {
    b.get(i).is_some_and(u8::is_ascii_digit)
}

fn digits(b: &[u8], from: usize, n: usize) -> bool {
    (from..from + n).all(|i| digit_at(b, i))
}

// /(opus|sonnet|haiku|fable|mythos)-(\d{1,2})(?:-(\d{1,2}))?(?!\d)/ — version
// after the family: opus-5, opus-4-8, haiku-4-5-<date>
fn family_version(m: &str) -> Option<(String, Option<String>)> {
    let b = m.as_bytes();
    let s = |from: usize, n: usize| m[from..from + n].to_string();
    for i in 0..b.len() {
        for fam in ["opus", "sonnet", "haiku", "fable", "mythos"] {
            if !m[i..].starts_with(fam) || b.get(i + fam.len()) != Some(&b'-') {
                continue;
            }
            let j = i + fam.len() + 1;
            for n1 in [2, 1] {
                if !digits(b, j, n1) {
                    continue;
                }
                let k = j + n1;
                if b.get(k) == Some(&b'-') {
                    for n2 in [2, 1] {
                        if digits(b, k + 1, n2) && !digit_at(b, k + 1 + n2) {
                            return Some((s(j, n1), Some(s(k + 1, n2))));
                        }
                    }
                }
                if !digit_at(b, k) {
                    return Some((s(j, n1), None));
                }
            }
        }
    }
    None
}

// legacy ids put the version before the family: claude-3-5-sonnet → /-(\d)-(\d+)/
fn legacy_version(m: &str) -> Option<(String, Option<String>)> {
    let b = m.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'-' && digit_at(b, i + 1) && b.get(i + 2) == Some(&b'-') && digit_at(b, i + 3) {
            let end = (i + 3..).find(|&k| !digit_at(b, k)).unwrap_or(b.len());
            return Some((m[i + 1..i + 2].to_string(), Some(m[i + 3..end].to_string())));
        }
    }
    None
}

fn encode_segment(name: &str) -> String {
    name.replace(['/', '.'], "-")
}

// Claude Code names each project directory after the working directory with
// every `/` and `.` flattened to `-`, which can't be reversed by string
// surgery. Walk the real filesystem instead: at each level, which actual
// child, encoded the same way, starts the rest? Longest match wins. Returns
// the path when fully resolved, and the remainder before the last step (a
// step that may have eaten into the name of a directory that's gone).
fn resolve_project_path(fs_root: &Path, dir_name: &str) -> (Option<PathBuf>, Option<String>) {
    let mut cur = fs_root.to_path_buf();
    let mut rest = dir_name.to_string();
    let mut consumed = None;
    while !rest.is_empty() {
        let Some(tail) = rest.strip_prefix('-') else { break };
        let Ok(kids) = fs::read_dir(&cur) else { break };
        let mut best: Option<(String, usize)> = None;
        for k in kids.flatten() {
            // symlinks count: /var is one on macOS
            let Ok(ft) = k.file_type() else { continue };
            if !ft.is_dir() && !ft.is_symlink() {
                continue;
            }
            let name = k.file_name().to_string_lossy().into_owned();
            let enc = encode_segment(&name);
            if tail != enc && !tail.starts_with(&format!("{enc}-")) {
                continue;
            }
            if best.as_ref().is_none_or(|(_, len)| enc.len() > *len) {
                best = Some((name, enc.len()));
            }
        }
        let Some((name, len)) = best else { break };
        consumed = Some(tail.to_string());
        cur.push(name);
        rest = tail[len..].to_string();
    }
    (rest.is_empty().then_some(cur), consumed)
}

// where Claude Code keeps the worktrees it makes, inside the project, as the
// directory name spells it (`/.claude/worktrees/` flattened)
const WORKTREES: &str = "--claude-worktrees-";

/// The project a working directory belongs to: the repository it is in, so a
/// session started in a subdirectory or in a worktree counts for its project
/// instead of standing as one of its own. A git worktree's `.git` is a file
/// pointing into the main repository; a directory in no repository is its
/// own project.
fn project_root(dir: &Path) -> PathBuf {
    let home = std::env::home_dir();
    for at in dir.ancestors() {
        // a home kept in git (dotfiles) isn't the project of all under it
        if at != dir && home.as_deref() == Some(at) {
            break;
        }
        let git = at.join(".git");
        let Ok(meta) = fs::metadata(&git) else { continue };
        if meta.is_file()
            && let Ok(body) = fs::read_to_string(&git)
            && let Some(main) = body.trim().strip_prefix("gitdir:").and_then(|p| p.trim().split_once("/.git/worktrees/"))
        {
            return PathBuf::from(main.0);
        }
        return at.to_path_buf();
    }
    dir.to_path_buf()
}

/// The project a log directory belongs to.
#[derive(Clone, Debug, PartialEq)]
struct ProjectRef {
    /// what tells projects apart: the project's path (or, for a path that is
    /// gone, its raw name)
    key: String,
    /// the project directory's name, and its parent's
    label: String,
    parent: String,
    /// the worktree the directory is, when it is one
    worktree: Option<String>,
}

fn name_of(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string_lossy().into_owned())
}

/// The project a log directory stands for: the project its path is in when
/// the path still exists, otherwise the raw tail as its name. A wrong name
/// is worse than an ugly one.
fn project_of(cache: &mut HashMap<String, ProjectRef>, fs_root: &Path, dir_name: &str) -> ProjectRef {
    // the same name can stand for different paths in this machine and in WSL
    let cache_key = format!("{}|{dir_name}", fs_root.display());
    if let Some(hit) = cache.get(&cache_key) {
        return hit.clone();
    }
    // a worktree Claude Code made is under its project, and is often gone
    // by now: what comes before it is the project's own directory
    let (base, tree) = match dir_name.split_once(WORKTREES) {
        Some((project, tree)) => (project, Some(tree.to_string())),
        None => (dir_name, None),
    };
    let (full, consumed) = resolve_project_path(fs_root, base);
    let found = match full {
        Some(p) => {
            let root = project_root(&p);
            // a git worktree kept elsewhere goes by its directory's name
            let worktree = tree.or_else(|| (!p.starts_with(&root)).then(|| name_of(&p)));
            ProjectRef { key: root.to_string_lossy().into_owned(), label: name_of(&root), parent: root.parent().map(name_of).unwrap_or_default(), worktree }
        }
        None => {
            let raw = consumed.as_deref().unwrap_or(base).trim_start_matches('-');
            let chars: Vec<char> = raw.chars().collect();
            let label =
                if chars.len() > LABEL_MAX { format!("…{}", chars[chars.len() - (LABEL_MAX - 1)..].iter().collect::<String>()) } else { raw.to_string() };
            ProjectRef { key: format!("?{base}"), label, parent: String::new(), worktree: tree }
        }
    };
    cache.insert(cache_key, found.clone());
    found
}

#[cfg(test)]
pub fn project_label_uncached(dir_name: &str) -> String {
    project_of(&mut HashMap::new(), Path::new("/"), dir_name).label
}

/// Where a log sits under the projects dir: its project directory and its
/// session (a session's subagents log under a directory named after it).
fn log_place(projects_dir: &Path, file: &Path) -> Option<(String, String)> {
    let rel = file.strip_prefix(projects_dir).ok()?;
    let mut parts = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned());
    let dir = parts.next()?;
    let session = parts.next()?;
    Some((dir, session.trim_end_matches(".jsonl").to_string()))
}

fn walk_jsonl(dir: &Path, out: &mut Vec<LogFile>, cutoff: i64) {
    let Ok(items) = fs::read_dir(dir) else { return };
    // sorted, so a turn replayed into several logs is credited the same way on
    // every poll and platform (dedup keeps the first copy it sees)
    let mut items: Vec<_> = items.flatten().collect();
    items.sort_by_key(|it| it.file_name());
    for it in items {
        let Ok(ft) = it.file_type() else { continue };
        let path = it.path();
        if ft.is_dir() {
            walk_jsonl(&path, out, cutoff);
        } else if ft.is_file() && path.extension().is_some_and(|e| e == "jsonl") {
            let Ok(meta) = it.metadata() else { continue };
            let mtime = mtime_ms(&meta);
            if mtime < cutoff {
                continue;
            }
            out.push(LogFile { path, size: meta.len(), mtime, root: 0 });
        }
    }
}

/// What the last walk of a projects dir behind a slow filesystem found.
#[derive(Default)]
struct Walked {
    at: i64,
    /// each project directory and its mtime at the last look
    projects: HashMap<PathBuf, i64>,
    files: Vec<LogFile>,
}

/// The logs under `dir` without walking all of it on every poll: whole at
/// most once a minute. In between, a poll lists only the top level, walks
/// again the projects whose directory changed (a new session started), and
/// reads again the size of the logs written in the last half hour (a turn
/// going on). A log quiet for longer that grows again (a resumed session) is
/// seen at the next whole walk.
fn walk_cached(w: &mut Walked, dir: &Path, now: i64, cutoff: i64) -> Vec<LogFile> {
    let projects: HashMap<PathBuf, i64> = fs::read_dir(dir)
        .map(|it| it.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())).filter_map(|e| Some((e.path(), mtime_ms(&e.metadata().ok()?)))).collect())
        .unwrap_or_default();
    if w.at == 0 || now - w.at >= WALK_MS || now < w.at {
        w.files.clear();
        walk_jsonl(dir, &mut w.files, cutoff);
        w.projects = projects;
        w.at = now;
        return w.files.clone();
    }
    let mut changed = false;
    for (project, mtime) in projects {
        if w.projects.get(&project) != Some(&mtime) {
            w.files.retain(|f| !f.path.starts_with(&project));
            walk_jsonl(&project, &mut w.files, cutoff);
            w.projects.insert(project, mtime);
            changed = true;
        }
    }
    if changed {
        // the order a whole walk gives, so dedup credits a turn the same way
        w.files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    for f in w.files.iter_mut().filter(|f| now - f.mtime <= OPEN_TURN_MS) {
        if let Ok(meta) = fs::metadata(&f.path) {
            f.size = meta.len();
            f.mtime = mtime_ms(&meta);
        }
    }
    w.files.clone()
}

#[cfg(test)]
pub fn tokens_of(usage: &Value) -> u64 {
    ["input_tokens", "output_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"]
        .iter()
        .map(|k| usage.get(k).and_then(Value::as_u64).unwrap_or(0))
        .sum()
}

#[derive(Deserialize)]
struct RawLine<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<Cow<'a, str>>,
    #[serde(borrow)]
    timestamp: Option<Cow<'a, str>>,
    #[serde(rename = "requestId", borrow)]
    request_id: Option<Cow<'a, str>>,
    #[serde(borrow)]
    uuid: Option<Cow<'a, str>>,
    #[serde(borrow)]
    message: Option<RawMessage<'a>>,
}

#[derive(Deserialize)]
struct RawMessage<'a> {
    #[serde(borrow)]
    id: Option<Cow<'a, str>>,
    #[serde(borrow)]
    model: Option<Cow<'a, str>>,
    #[serde(borrow)]
    timestamp: Option<Cow<'a, str>>,
    usage: Option<RawUsage>,
}

#[derive(Deserialize)]
struct RawUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}

fn nonempty<'a>(s: &'a Option<Cow<'a, str>>) -> Option<&'a str> {
    s.as_deref().filter(|s| !s.is_empty())
}

static USAGE_KEY: std::sync::LazyLock<memchr::memmem::Finder<'static>> = std::sync::LazyLock::new(|| memchr::memmem::Finder::new(b"\"usage\""));

fn ingest_line(entries: &mut Vec<Entry>, line: &[u8]) {
    // most lines are tool results and prompts: skip them without a parse
    if USAGE_KEY.find(line).is_none() {
        return;
    }
    // typed, so the (large) message content is skipped instead of built
    let Ok(obj) = serde_json::from_slice::<RawLine>(line) else { return };
    if obj.kind.as_deref() != Some("assistant") {
        return;
    }
    let Some(msg) = &obj.message else { return };
    let Some(u) = &msg.usage else { return };
    let model = nonempty(&msg.model);
    if model == Some("<synthetic>") {
        return; // Claude Code internal messages
    }
    let Some(ts) = nonempty(&obj.timestamp).or_else(|| nonempty(&msg.timestamp)).and_then(parse_date) else {
        return;
    };
    if ts == 0 {
        return;
    }
    let tokens = [u.input_tokens, u.output_tokens, u.cache_read_input_tokens, u.cache_creation_input_tokens].iter().map(|t| t.unwrap_or(0)).sum();
    // every token sent in: fresh, read from the cache, written to it
    let cache_read = u.cache_read_input_tokens.unwrap_or(0);
    let input = u.input_tokens.unwrap_or(0) + cache_read + u.cache_creation_input_tokens.unwrap_or(0);
    let req = nonempty(&obj.request_id).or_else(|| nonempty(&obj.uuid)).unwrap_or("");
    entries.push(Entry { ts, model: model.map(str::to_string), tokens, input, cache_read, key: format!("{}:{}", nonempty(&msg.id).unwrap_or(""), req) });
}

// Session logs only grow, and a long one passes 500 MB: a poll reads just the
// bytes appended since the last one. A line still being written waits in `rest`.
fn parse_file<'a>(cache: &'a mut HashMap<PathBuf, FileState>, path: &Path, size: u64) -> &'a [Entry] {
    let state = cache.entry(path.to_path_buf()).or_default();
    // shrunk or replaced: start over
    if size < state.offset {
        *state = FileState::default();
    }
    if size <= state.offset {
        return &state.entries;
    }
    let Ok(mut file) = File::open(path) else { return &state.entries };
    if file.seek(SeekFrom::Start(state.offset)).is_err() {
        return &state.entries;
    }
    while state.offset < size {
        let mut chunk = vec![0u8; CHUNK.min(size - state.offset) as usize];
        let n = match file.read(&mut chunk) {
            Ok(0) | Err(_) => break, // retry from the offset next poll
            Ok(n) => n,
        };
        state.offset += n as u64;
        let mut buf = std::mem::take(&mut state.rest);
        buf.extend_from_slice(&chunk[..n]);
        // cut at the last newline in bytes, so a split UTF-8 character survives
        let Some(end) = memchr::memrchr(b'\n', &buf) else {
            state.rest = buf;
            continue;
        };
        state.rest = buf[end + 1..].to_vec();
        let mut from = 0;
        for nl in memchr::memchr_iter(b'\n', &buf[..=end]) {
            if nl > from {
                ingest_line(&mut state.entries, &buf[from..nl]);
            }
            from = nl + 1;
        }
    }
    &state.entries
}

/// The last lines of a session log (the recent tail is plenty), or None.
pub fn tail_lines(path: &Path) -> Option<Vec<String>> {
    let mut f = File::open(path).ok()?;
    let size = f.metadata().ok()?.len();
    let len = size.min(TAIL_BYTES);
    f.seek(SeekFrom::Start(size - len)).ok()?;
    let mut buf = Vec::with_capacity(len as usize);
    f.take(len).read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).split('\n').filter(|l| !l.is_empty()).map(str::to_string).collect())
}

fn activity_by_tool(name: &str) -> &'static str {
    match name {
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => "editing",
        "Read" | "Grep" | "Glob" | "LS" => "reading",
        "Bash" | "BashOutput" | "KillShell" => "running",
        "WebSearch" | "WebFetch" => "researching",
        "Task" | "Agent" => "delegating",
        "TodoWrite" | "ExitPlanMode" => "planning",
        "AskUserQuestion" => "waiting",
        _ => "working",
    }
}

/// What Claude Code is doing now, from the tail of a session log: the latest
/// tool_use → an activity; plan mode wins over everything. None = no signal.
pub fn detect_activity(path: &Path) -> Option<String> {
    let lines = tail_lines(path)?;
    let mut activity: Option<&str> = None;
    let mut perm_mode: Option<String> = None;
    for line in lines.iter().rev().take(80) {
        let Ok(o) = serde_json::from_str::<Value>(line) else { continue };
        let kind = o.get("type").and_then(Value::as_str);
        if perm_mode.is_none() && matches!(kind, Some("permission-mode" | "mode")) {
            perm_mode =
                ["mode", "permissionMode", "value"].iter().find_map(|k| o.get(*k).filter(|v| truthy(Some(v)))).and_then(|v| v.as_str().map(str::to_string));
        }
        if activity.is_none()
            && kind == Some("assistant")
            && let Some(blocks) = o.pointer("/message/content").and_then(Value::as_array)
        {
            let last_tool = blocks.iter().rev().find(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"));
            if let Some(tool) = last_tool {
                activity = Some(activity_by_tool(tool.get("name").and_then(Value::as_str).unwrap_or("")));
            }
        }
        if activity.is_some() && perm_mode.is_some() {
            break;
        }
    }
    if perm_mode.as_deref() == Some("plan") {
        return Some("planning".into());
    }
    activity.map(str::to_string)
}

// User lines that open no turn: slash commands and their output, `!` shell
// input, background-task notices, the compaction summary
const NO_TURN: [&str; 6] = ["<command-name>", "<command-message>", "<local-command-", "<bash-", "<task-notification>", "This session is being continued"];

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        _ => true,
    }
}

fn text_of(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .map(|b| if b.get("type").and_then(Value::as_str) == Some("text") { b.get("text").and_then(Value::as_str).unwrap_or("") } else { "" })
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// Is a turn still open? Claude Code writes nothing while a command runs or
/// the model thinks, so how the log ends decides. Some(true), Some(false), or
/// None when the tail doesn't say.
pub fn turn_open(lines: &[String]) -> Option<bool> {
    for line in lines.iter().rev() {
        let Ok(o) = serde_json::from_str::<Value>(line) else { continue };
        if truthy(o.get("isSidechain")) {
            continue;
        }
        match o.get("type").and_then(Value::as_str) {
            Some("system") => {
                if o.get("subtype").and_then(Value::as_str) == Some("turn_duration") {
                    return Some(false);
                }
            }
            Some("assistant") => {
                let stop = o.pointer("/message/stop_reason");
                if !truthy(stop) {
                    continue;
                }
                let done = matches!(stop.and_then(Value::as_str), Some("end_turn" | "stop_sequence" | "refusal"));
                return Some(!done);
            }
            Some("user") => {
                let content = o.pointer("/message/content");
                if let Some(Value::Array(blocks)) = content
                    && blocks.iter().any(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"))
                {
                    return Some(true);
                }
                if truthy(o.get("isMeta")) || truthy(o.get("isCompactSummary")) {
                    continue;
                }
                let text = text_of(content);
                if text.contains("[Request interrupted by user") {
                    return Some(false);
                }
                let lead = text.trim_start();
                if NO_TURN.iter().any(|p| lead.starts_with(p)) {
                    continue;
                }
                return Some(true);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests;
