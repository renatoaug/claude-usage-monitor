// Port of test/unit/cursor.test.js.

use super::*;
use crate::auth::{AuthError, Response};
use crate::usage::parse_date;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

const DAY: i64 = 86_400_000;

fn now() -> i64 {
    parse_date("2026-09-24T12:00:00Z").unwrap()
}

fn end() -> i64 {
    now() + 10 * DAY
}

enum Answer {
    Json(Value),
    Status(u16),
    Offline,
}

/// a request the double saw: the endpoint and its headers
type Call = (String, Vec<(String, String)>);

// a fetch double: answers per endpoint, and remembers what it was asked
#[derive(Default)]
struct Api {
    answers: HashMap<&'static str, Answer>,
    calls: Mutex<Vec<Call>>,
}

impl Api {
    fn new(answers: Vec<(&'static str, Answer)>) -> Arc<Self> {
        Arc::new(Api { answers: answers.into_iter().collect(), calls: Mutex::default() })
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl Http for Api {
    fn post_json(&self, url: &str, headers: &[(&str, &str)], _body: &Value) -> crate::auth::Result<Response> {
        self.calls.lock().unwrap().push((url.into(), headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()));
        let ok = |status: u16, body: String| Ok(Response { status, body, headers: vec![] });
        match self.answers.get(url.rsplit('/').next().unwrap()) {
            Some(Answer::Json(v)) => ok(200, v.to_string()),
            Some(Answer::Status(s)) => ok(*s, String::new()),
            Some(Answer::Offline) => Err(AuthError { status: 0, message: "offline".into() }),
            None => ok(200, "null".into()),
        }
    }
    fn get(&self, _: &str, _: &[(&str, &str)]) -> crate::auth::Result<Response> {
        unreachable!()
    }
}

fn usage() -> Value {
    json!({
        "billingCycleStart": (now() - 20 * DAY).to_string(),
        "billingCycleEnd": end().to_string(),
        "planUsage": {"totalPercentUsed": 42.5, "autoPercentUsed": 10, "apiPercentUsed": 61},
    })
}

fn plan() -> Value {
    json!({"planInfo": {"planName": "Pro", "billingCycleEnd": end().to_string()}})
}

fn good() -> Arc<Api> {
    Api::new(vec![("GetCurrentPeriodUsage", Answer::Json(usage())), ("GetPlanInfo", Answer::Json(plan()))])
}

struct Fx {
    root: TempDir,
    cursor: Cursor,
}

impl Fx {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let fx = Fx { cursor: Cursor::at(root.path().join(".cursor"), root.path().join("state.vscdb"), Api::new(vec![])), root };
        fx.state_db(&[("cursorAuth/accessToken", Sql::Text("tok-1".into())), ("cursorAuth/stripeMembershipType", Sql::Text("free".into()))]);
        fx
    }
    fn state_db(&self, values: &[(&str, Sql)]) {
        let file = self.root.path().join("state.vscdb");
        let _ = fs::remove_file(&file);
        let db = Connection::open(&file).unwrap();
        db.execute("CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)", []).unwrap();
        for (k, v) in values {
            db.execute("INSERT INTO ItemTable (key, value) VALUES (?, ?)", rusqlite::params![k, v]).unwrap();
        }
    }
    fn api(&mut self, api: &Arc<Api>) {
        self.cursor.http = api.clone();
    }
    fn transcript(&self, project: &str, id: &str, lines: &[Value], mtime: i64, sub: Option<&str>) {
        let mut dir = self.root.path().join(".cursor/projects").join(project).join("agent-transcripts").join(id);
        if sub.is_some() {
            dir = dir.join("subagents");
        }
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join(format!("{}.jsonl", sub.unwrap_or(id)));
        let body: Vec<String> = lines.iter().map(Value::to_string).collect();
        fs::write(&f, format!("{}\n", body.join("\n"))).unwrap();
        let t = SystemTime::UNIX_EPOCH + Duration::from_millis(mtime as u64);
        File::options().write(true).open(&f).unwrap().set_modified(t).unwrap();
    }
    fn usage(&mut self, at: i64) -> CursorUsage {
        // activity only: no background fetch
        self.cursor.last_try = Some(at);
        self.cursor.get_usage(at)
    }
}

fn user() -> Value {
    json!({"role": "user", "message": {"content": [{"type": "text", "text": "hi"}]}})
}

fn tool(name: &str) -> Value {
    json!({"role": "assistant", "message": {"content": [{"type": "text", "text": "…"}, {"type": "tool_use", "name": name, "input": {}}]}})
}

fn ended() -> Value {
    json!({"type": "turn_ended", "status": "success"})
}

// ---- detect -------------------------------------------------------------------

#[test]
fn finds_a_signed_in_app_with_its_cached_plan() {
    assert_eq!(Fx::new().cursor.detect(), (true, Some("free".into())));
}

#[test]
fn an_app_without_a_token_is_not_connectable() {
    let fx = Fx::new();
    fx.state_db(&[("cursorAuth/stripeMembershipType", Sql::Text("free".into()))]);
    assert_eq!(fx.cursor.detect(), (false, Some("free".into())));
}

#[test]
fn no_app_or_an_unreadable_file_finds_nothing() {
    let mut fx = Fx::new();
    fx.cursor.state_db = fx.root.path().join("missing.vscdb");
    assert_eq!(fx.cursor.detect(), (false, None));
    let junk = fx.root.path().join("junk.vscdb");
    fs::write(&junk, "not sqlite").unwrap();
    fx.cursor.state_db = junk;
    assert_eq!(fx.cursor.detect(), (false, None));
}

#[test]
fn a_token_stored_as_bytes_reads_the_same() {
    let fx = Fx::new();
    fx.state_db(&[("cursorAuth/accessToken", Sql::Blob(b"tok-b".to_vec()))]);
    assert!(fx.cursor.detect().0);
}

// ---- limits -------------------------------------------------------------------

#[test]
fn the_billing_cycle_is_the_window_with_the_api_pool_beside_it() {
    let mut fx = Fx::new();
    let api = good();
    fx.api(&api);
    fx.cursor.refresh_blocking(now());
    let u = fx.usage(now() + 1000);
    assert_eq!(u.plan.as_deref(), Some("pro"));
    assert_eq!(u.session, Some(Window { pct: Some(42.5), reset_ms: Some(end() - now() - 1000), expired: false }));
    assert_eq!(u.api.unwrap().pct, Some(61.0));
    assert_eq!(u.limits_at, Some(now()));
    assert!(!u.signed_out);
    let calls = api.calls.lock().unwrap();
    assert!(calls[0].1.contains(&("Authorization".into(), "Bearer tok-1".into())));
}

#[test]
fn past_the_cycle_end_the_old_pct_is_unknown_never_zero() {
    let mut fx = Fx::new();
    fx.api(&good());
    fx.cursor.refresh_blocking(now());
    assert_eq!(fx.usage(end() + 1).session, Some(Window { pct: None, reset_ms: None, expired: true }));
}

#[test]
fn the_plan_falls_back_to_the_cached_one_and_odd_fields_read_as_unknown() {
    let mut fx = Fx::new();
    let odd = json!({"billingCycleEnd": "soon", "planUsage": {"totalPercentUsed": "lots"}});
    fx.api(&Api::new(vec![("GetCurrentPeriodUsage", Answer::Json(odd)), ("GetPlanInfo", Answer::Status(500))]));
    fx.cursor.refresh_blocking(now());
    let u = fx.usage(now());
    assert_eq!(u.plan.as_deref(), Some("free"));
    assert_eq!(u.session, Some(Window { pct: None, reset_ms: None, expired: false }));
}

#[test]
fn signed_out_no_fetch_and_it_says_so() {
    let mut fx = Fx::new();
    fx.state_db(&[]);
    let api = good();
    fx.api(&api);
    fx.cursor.refresh_blocking(now());
    assert_eq!(api.count(), 0);
    let u = fx.usage(now());
    assert!(u.signed_out);
    assert!(u.session.is_none());
}

#[test]
fn a_failed_fetch_keeps_the_last_reading_and_retries_sooner() {
    let mut fx = Fx::new();
    fx.api(&good());
    fx.cursor.refresh_blocking(now());
    fx.api(&Api::new(vec![("GetCurrentPeriodUsage", Answer::Status(401))]));
    fx.cursor.refresh_blocking(now() + 5 * 60_000);
    let u = fx.usage(now() + 5 * 60_000 + 1000);
    assert_eq!(u.session.unwrap().pct, Some(42.5));
    assert_eq!(u.limits_at, Some(now()));
    fx.cursor.last_try = Some(now() + 5 * 60_000);
    assert!(fx.cursor.due(now() + 6 * 60_000));
}

#[test]
fn polls_are_throttled_one_fetch_per_5_minutes_one_in_flight() {
    let mut fx = Fx::new();
    fx.api(&good());
    fx.cursor.refresh_blocking(now());
    assert!(!fx.cursor.due(now() + 60_000));
    assert!(fx.cursor.due(now() + 5 * 60_000));
    // a fetch in the background holds the next one back
    fx.cursor.maybe_refresh(now() + 5 * 60_000);
    assert!(!fx.cursor.due(now() + 20 * 60_000));
}

#[test]
fn a_fetch_in_the_background_lands_on_a_later_tick() {
    let mut fx = Fx::new();
    let api = good();
    fx.api(&api);
    fx.cursor.get_usage(now());
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while fx.cursor.limits.is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
        fx.cursor.collect();
    }
    assert_eq!(fx.cursor.get_usage(now()).session.unwrap().pct, Some(42.5));
}

#[test]
fn errors_are_logged_and_the_next_try_comes_after_a_minute() {
    let mut fx = Fx::new();
    let api = Api::new(vec![("GetCurrentPeriodUsage", Answer::Offline)]);
    fx.api(&api);
    fx.cursor.refresh_blocking(now());
    assert!(fx.cursor.last_failed);
    assert!(fx.usage(now()).session.is_none());
    fx.cursor.last_try = Some(now());
    assert!(!fx.cursor.due(now() + 30_000));
    assert!(fx.cursor.due(now() + 60_000));
}

// ---- activity -----------------------------------------------------------------

#[test]
fn no_transcripts_never_seen_not_active() {
    let u = Fx::new().usage(now());
    assert!(!u.active && u.activity.is_none() && u.last_seen.is_none());
}

#[test]
fn the_newest_tool_call_names_the_scene() {
    let mut fx = Fx::new();
    fx.transcript("proj", "a", &[user(), tool("Read")], now() - 60 * 60_000, None);
    fx.transcript("proj", "b", &[user(), tool("StrReplace")], now() - 5000, None);
    let u = fx.usage(now());
    assert!(u.active);
    assert_eq!(u.activity, Some("editing"));
    assert_eq!(u.last_seen, Some(now() - 5000));
}

#[test]
fn a_subagent_transcript_counts_and_unknown_tools_just_work() {
    let mut fx = Fx::new();
    fx.transcript("proj", "c", &[user(), tool("Task")], now() - 20_000, None);
    fx.transcript("proj", "c", &[tool("SomethingNew")], now() - 1000, Some("sub-1"));
    assert_eq!(fx.usage(now()).activity, Some("working"));
}

#[test]
fn a_finished_turn_rests_an_open_one_works_through_a_long_quiet_command() {
    let mut fx = Fx::new();
    fx.transcript("proj", "d", &[user(), tool("Shell"), ended()], now() - 5000, None);
    let u = fx.usage(now());
    assert!(!u.active && u.activity.is_none());
    // no write for two minutes, but the turn hasn't ended: still running
    fx.transcript("proj", "e", &[user(), tool("Shell")], now() - 120_000, None);
    fx.transcript("proj", "d", &[user(), tool("Shell"), ended()], now() - 130_000, None);
    let u = fx.usage(now());
    assert!(u.active);
    assert_eq!(u.activity, Some("running"));
    assert_eq!(u.last_seen, Some(now() - 120_000));
}

#[test]
fn an_open_turn_untouched_for_30_minutes_was_abandoned() {
    let mut fx = Fx::new();
    fx.transcript("proj", "g", &[user(), tool("Shell")], now() - 31 * 60_000, None);
    let u = fx.usage(now());
    assert!(!u.active && u.activity.is_none());
}

#[test]
fn a_transcript_with_nothing_to_say_counts_only_while_fresh() {
    let mut fx = Fx::new();
    fx.transcript("proj", "h", &[json!({"note": "x"})], now() - 5000, None);
    assert_eq!(fx.usage(now()).activity, Some("working"));
    fx.transcript("proj", "h", &[json!({"note": "x"})], now() - 120_000, None);
    assert!(!fx.usage(now()).active);
}

#[test]
fn only_the_tail_of_a_long_transcript_is_read() {
    let mut fx = Fx::new();
    let filler = json!({"role": "assistant", "message": {"content": [{"type": "text", "text": "x".repeat(70_000)}]}});
    fx.transcript("proj", "f", &[filler, user(), tool("WebSearch")], now() - 5000, None);
    assert_eq!(fx.usage(now()).activity, Some("researching"));
}

#[test]
fn activity_of_user_turns_think_broken_lines_are_skipped() {
    assert_eq!(activity_of(&[user().to_string()]), Some(Some("working")));
    assert_eq!(activity_of(&["{broken".to_string(), tool("Grep").to_string()]), Some(Some("reading")));
    assert_eq!(activity_of(&["{broken"]), None);
    assert_eq!(activity_of(&[json!({"role": "assistant", "message": {}}).to_string()]), Some(Some("working")));
}
