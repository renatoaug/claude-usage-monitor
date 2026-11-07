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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub session: Session,
    pub week: Week,
    pub today_tokens: u64,
    pub by_model: Vec<Labeled>,
    pub by_project: Vec<Labeled>,
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
    key: String,
}

#[derive(Default)]
struct FileState {
    offset: u64,
    rest: Vec<u8>,
    entries: Vec<Entry>,
}

struct LogFile {
    path: PathBuf,
    size: u64,
    mtime: i64,
}

/// Reads one Claude config dir's logs. The file cache is keyed by absolute
/// path, so switching dirs and back stays instant; the label cache is keyed
/// by directory *name*, which can repeat across dirs, so it is dropped.
pub struct Reader {
    claude_dir: PathBuf,
    files: HashMap<PathBuf, FileState>,
    labels: HashMap<String, String>,
}

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
        Reader { claude_dir: claude_dir.unwrap_or_else(default_claude_dir), files: HashMap::new(), labels: HashMap::new() }
    }

    /// Point the reader at another account's logs (None = the default dir).
    pub fn set_claude_dir(&mut self, dir: Option<PathBuf>) {
        let next = dir.unwrap_or_else(default_claude_dir);
        if next == self.claude_dir {
            return;
        }
        self.claude_dir = next;
        self.labels.clear();
    }

    fn projects_dir(&self) -> PathBuf {
        self.claude_dir.join("projects")
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

        let projects_dir = self.projects_dir();
        let mut files = Vec::new();
        walk_jsonl(&projects_dir, &mut files, scan_cutoff);

        let newest = files.iter().max_by_key(|f| f.mtime);
        let last_mtime = newest.map(|f| f.mtime).unwrap_or(0);
        let newest_file = newest.map(|f| f.path.clone());

        let mut seen = HashSet::new();
        let (mut today, mut week, mut month, mut last5m) = (0u64, 0u64, 0u64, 0u64);
        let mut by_model = Tally::default();
        let mut by_project = Tally::default();
        let mut days30 = vec![0u64; DAYS];
        let mut recent: Vec<(i64, u64)> = Vec::new();

        for f in &files {
            let proj = project_dir_of(&projects_dir, &f.path).map(|d| project_label(&mut self.labels, &d));
            let entries = parse_file(&mut self.files, &f.path, f.size);
            for e in entries {
                if e.ts < start30 {
                    continue;
                }
                if e.key != ":" && !seen.insert(e.key.clone()) {
                    continue;
                }
                let t = e.tokens;
                month += t;
                let idx = ((e.ts - start30) / DAY_MS).clamp(0, DAYS as i64 - 1) as usize;
                days30[idx] += t;
                if e.ts >= week_start {
                    week += t;
                    by_model.add(label_for(e.model.as_deref()), t);
                    if let Some(p) = &proj {
                        by_project.add(p.clone(), t);
                    }
                }
                if e.ts >= today_ms {
                    today += t;
                }
                if e.ts >= recent_cutoff {
                    recent.push((e.ts, t));
                }
                if e.ts >= five_min_ago {
                    last5m += t;
                }
            }
        }

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

        // ranked, then everything past the top N folded into one row
        let ranked = by_project.ranked();
        let mut by_project: Vec<Labeled> = ranked.iter().take(TOP_PROJECTS).cloned().collect();
        let rest = &ranked[ranked.len().min(TOP_PROJECTS)..];
        if !rest.is_empty() {
            by_project.push(Labeled { label: format!("other · {}", rest.len()), tokens: rest.iter().map(|p| p.tokens).sum() });
        }

        Usage {
            session,
            week: Week { tokens: week, pct: pct(week, weekly_budget), reset_ms: week_reset_ms },
            today_tokens: today,
            by_model: by_model.ranked(),
            by_project,
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
fn resolve_project_path(dir_name: &str) -> (Option<PathBuf>, Option<String>) {
    let mut cur = PathBuf::from("/");
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

/// The readable name for a project directory: its basename when the path
/// still exists, otherwise the raw tail. A wrong name is worse than an ugly one.
fn project_label(cache: &mut HashMap<String, String>, dir_name: &str) -> String {
    if let Some(hit) = cache.get(dir_name) {
        return hit.clone();
    }
    let (full, consumed) = resolve_project_path(dir_name);
    let label = match full {
        Some(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string_lossy().into_owned()),
        None => {
            let raw = consumed.as_deref().unwrap_or(dir_name).trim_start_matches('-');
            let chars: Vec<char> = raw.chars().collect();
            if chars.len() > LABEL_MAX { format!("…{}", chars[chars.len() - (LABEL_MAX - 1)..].iter().collect::<String>()) } else { raw.to_string() }
        }
    };
    cache.insert(dir_name.to_string(), label.clone());
    label
}

#[cfg(test)]
pub fn project_label_uncached(dir_name: &str) -> String {
    project_label(&mut HashMap::new(), dir_name)
}

fn project_dir_of(projects_dir: &Path, file: &Path) -> Option<String> {
    let rel = file.strip_prefix(projects_dir).ok()?;
    let first = rel.components().next()?;
    Some(first.as_os_str().to_string_lossy().into_owned())
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
            out.push(LogFile { path, size: meta.len(), mtime });
        }
    }
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
    let req = nonempty(&obj.request_id).or_else(|| nonempty(&obj.uuid)).unwrap_or("");
    entries.push(Entry { ts, model: model.map(str::to_string), tokens, key: format!("{}:{}", nonempty(&msg.id).unwrap_or(""), req) });
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
