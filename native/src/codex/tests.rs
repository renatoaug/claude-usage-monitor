// Port of test/unit/codex.test.js.

use super::*;
use std::time::SystemTime;
use tempfile::TempDir;

fn now() -> i64 {
    parse_date("2026-09-12T20:00:00Z").unwrap()
}

fn token_count(ts: &str, total: u64, limits: Value) -> String {
    json!({"timestamp": ts, "type": "event_msg",
        "payload": {"type": "token_count", "info": {"total_token_usage": {"total_tokens": total}}, "rate_limits": limits}})
    .to_string()
}

fn codex_limits() -> Value {
    let n = now() as f64 / 1000.0;
    json!({"limit_id": "codex", "plan_type": "plus",
        "primary": {"used_percent": 40, "window_minutes": 300, "resets_at": n + 3600.0},
        "secondary": {"used_percent": 31, "window_minutes": 10080, "resets_at": n + 86400.0}})
}

fn premium() -> Value {
    json!({"limit_id": "premium", "plan_type": "plus", "primary": null, "secondary": null})
}

struct Fx {
    root: TempDir,
    codex: Codex,
}

impl Fx {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("sessions/2026/09/12")).unwrap();
        let mut codex = Codex::new(Some(root.path().to_path_buf()));
        codex.set_bin(None); // never spawn the real Codex
        Fx { root, codex }
    }
    fn write(&self, name: &str, lines: &[String], mtime: i64) -> PathBuf {
        let f = self.root.path().join("sessions/2026/09/12").join(name);
        fs::write(&f, format!("{}\n", lines.join("\n"))).unwrap();
        let t = SystemTime::UNIX_EPOCH + Duration::from_millis(mtime as u64);
        File::options().write(true).open(&f).unwrap().set_modified(t).unwrap();
        f
    }
    // a stand-in app-server: answers initialize, then the given read response
    #[cfg(unix)]
    fn stub_bin(&self, read_response: Value) -> PathBuf {
        let f = self.root.path().join(format!("stub-{}", rand_suffix()));
        let script = format!(
            "#!/bin/sh\necho \"$1\" > \"$0.args\"\necho run >> \"$0.runs\"\necho \"$CODEX_HOME\" > \"$0.home\"\nwhile read -r line; do\n  case \"$line\" in\n    *'\"id\":1'*) echo 'noise'; echo '{{\"id\":1,\"result\":{{}}}}' ;;\n    *'\"id\":2'*) printf '%s\\n' '{}' ;;\n  esac\ndone\n",
            read_response
        );
        fs::write(&f, script).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&f, fs::Permissions::from_mode(0o755)).unwrap();
        f
    }
    fn stale(&self) {
        let n = now() as f64 / 1000.0;
        self.write(
            "rollout-l.jsonl",
            &[token_count(
                "2026-09-12T10:00:00Z",
                10,
                json!({"plan_type": "prolite", "primary": {"used_percent": 85, "window_minutes": 10080, "resets_at": n + 3600.0}, "secondary": null}),
            )],
            now() - 10_000,
        );
    }
}

#[cfg(unix)]
fn rand_suffix() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    format!("{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst))
}

#[cfg(unix)]
fn live_result() -> Value {
    let n = now() as f64 / 1000.0;
    json!({"rateLimitsByLimitId": {"codex": {"planType": "prolite",
        "primary": {"usedPercent": 40, "windowDurationMins": 10080, "resetsAt": n + 86400.0}, "secondary": null}}})
}

#[test]
fn returns_none_when_codex_is_not_installed() {
    let mut c = Codex::new(Some(PathBuf::from("/no/such/codex")));
    c.set_bin(None);
    assert!(c.get_usage(now()).is_none());
}

#[test]
fn reads_limits_tokens_model_and_activity_from_a_rollout() {
    let mut fx = Fx::new();
    fx.write(
        "rollout-a.jsonl",
        &[
            json!({"type": "turn_context", "payload": {"model": "gpt-6", "cwd": "/x/proj"}}).to_string(),
            token_count("2026-09-12T19:00:00Z", 100, codex_limits()),
            token_count("2026-09-12T19:00:01Z", 100, codex_limits()), // repeated count
            token_count("2026-09-12T19:30:00Z", 250, premium()),      // null windows: ignored
            "not json {\"token_count\"".into(),
            "{\"type\":\"response_item\",\"payload\":{}}".into(),
        ],
        now() - 10_000,
    );
    let u = fx.codex.get_usage(now()).unwrap();
    assert!(u.active);
    assert_eq!(u.plan.as_deref(), Some("plus"));
    assert_eq!(u.session, Some(Window { pct: Some(40.0), reset_ms: Some(3_600_000), expired: false }));
    assert_eq!(u.weekly.unwrap().pct, Some(31.0));
    assert_eq!(u.tokens_5h, 250);
    assert_eq!(u.tokens_week, 250);
}

#[test]
fn a_weekly_only_plan_puts_its_week_in_the_session_slot() {
    let mut fx = Fx::new();
    let n = now() as f64 / 1000.0;
    fx.write(
        "rollout-w.jsonl",
        &[token_count(
            "2026-09-12T19:00:00Z",
            100,
            json!({"limit_id": "codex", "plan_type": "prolite", "primary": {"used_percent": 52, "window_minutes": 10080, "resets_at": n + 86400.0}, "secondary": null}),
        )],
        now() - 10_000,
    );
    let u = fx.codex.get_usage(now()).unwrap();
    assert!(u.weekly_only);
    assert_eq!(u.session, Some(Window { pct: Some(52.0), reset_ms: Some(86_400_000), expired: false }));
    assert!(u.weekly.is_none());
}

#[test]
fn sorts_windows_by_length_not_by_slot() {
    let mut fx = Fx::new();
    let mut l = codex_limits();
    let (p, s) = (l["primary"].clone(), l["secondary"].clone());
    l["primary"] = s;
    l["secondary"] = p;
    fx.write("rollout-s.jsonl", &[token_count("2026-09-12T19:00:00Z", 100, l)], now() - 10_000);
    let u = fx.codex.get_usage(now()).unwrap();
    assert!(!u.weekly_only);
    assert_eq!(u.session.unwrap().pct, Some(40.0));
    assert_eq!(u.weekly.unwrap().pct, Some(31.0));
}

#[test]
fn reads_only_what_was_appended_and_survives_a_partial_line() {
    let mut fx = Fx::new();
    let f = fx.write("rollout-b.jsonl", &[token_count("2026-09-12T19:00:00Z", 100, codex_limits())], now() - 10_000);
    assert_eq!(fx.codex.get_usage(now()).unwrap().tokens_week, 100);
    let next = token_count("2026-09-12T19:10:00Z", 180, codex_limits());
    let mut h = File::options().append(true).open(&f).unwrap();
    h.write_all(&next.as_bytes()[..20]).unwrap();
    assert_eq!(fx.codex.get_usage(now()).unwrap().tokens_week, 100);
    h.write_all(&next.as_bytes()[20..]).unwrap();
    h.write_all(b"\n").unwrap();
    assert_eq!(fx.codex.get_usage(now()).unwrap().tokens_week, 180);
    // truncated or replaced: read again from scratch
    fs::write(&f, format!("{}\n", token_count("2026-09-12T19:20:00Z", 5, Value::Null))).unwrap();
    assert_eq!(fx.codex.get_usage(now()).unwrap().tokens_week, 5);
}

#[test]
fn an_idle_codex_keeps_the_last_pct_and_a_passed_reset_is_unknown_not_zero() {
    let mut fx = Fx::new();
    fx.write("rollout-c.jsonl", &[token_count("2026-09-12T10:00:00Z", 10, codex_limits())], now() - 3_600_000);
    let later = now() + 2 * 3_600_000; // past the 5h reset, before the weekly one
    let u = fx.codex.get_usage(later).unwrap();
    assert!(!u.active);
    assert_eq!(u.session, Some(Window { pct: None, reset_ms: None, expired: true }));
    assert_eq!(u.weekly.unwrap().pct, Some(31.0));
}

#[test]
fn ignores_rollouts_older_than_the_30_day_map() {
    let mut fx = Fx::new();
    fx.write("rollout-old.jsonl", &[token_count("2026-08-01T10:00:00Z", 10, codex_limits())], now() - 40 * DAY);
    let u = fx.codex.get_usage(now()).unwrap();
    assert!(u.session.is_none());
    assert!(u.last_seen.is_none());
    assert_eq!(u.month_tokens, 0);
}

#[test]
fn attributes_each_delta_to_the_model_and_project_in_effect() {
    let mut fx = Fx::new();
    let ctx = |model: &str, cwd: &str| json!({"type": "turn_context", "payload": {"model": model, "cwd": cwd}}).to_string();
    let mut lines = vec![
        ctx("gpt-6", "/w/alpha"),
        token_count("2026-09-12T19:00:00Z", 100, Value::Null),
        ctx("gpt-6-mini", "/w/beta"),
        token_count("2026-09-12T19:10:00Z", 130, Value::Null),
        token_count("2026-09-10T09:00:00Z", 900, Value::Null), // before today, still this week
    ];
    for i in 1..=6u64 {
        lines.push(ctx("gpt-6", &format!("/w/p{i}")));
        lines.push(token_count("2026-09-12T18:00:00Z", 1000 + i * 10, Value::Null));
    }
    fx.write("rollout-d.jsonl", &lines, now() - 10_000);
    let u = fx.codex.get_usage(now()).unwrap();
    assert_eq!(u.by_model.iter().find(|m| m.label == "gpt-6-mini").unwrap().tokens, 800);
    assert_eq!(u.by_project.iter().find(|p| p.label == "alpha").unwrap().tokens, 100);
    assert!(u.by_project.last().unwrap().label.starts_with("other · "));
    assert_eq!(u.by_project.len(), 6);
    assert_eq!(u.days30.len(), 30);
    assert_eq!(u.days30.iter().sum::<u64>(), u.month_tokens);
}

#[test]
fn an_event_with_no_model_still_counts() {
    let mut fx = Fx::new();
    fx.write("rollout-e.jsonl", &[token_count("2026-09-12T19:00:00Z", 50, Value::Null)], now() - 10_000);
    assert_eq!(fx.codex.get_usage(now()).unwrap().by_model, vec![Labeled { label: "unknown".into(), tokens: 50 }]);
}

#[test]
fn detect_finds_nothing_without_rollouts() {
    let mut fx = Fx::new();
    assert_eq!(fx.codex.detect(), (false, None));
}

#[test]
fn detect_finds_a_session_and_its_plan_however_old() {
    let mut fx = Fx::new();
    fx.write("rollout-f.jsonl", &[token_count("2026-06-01T10:00:00Z", 10, codex_limits())], now() - 90 * DAY);
    assert_eq!(fx.codex.detect(), (true, Some("plus".into())));
}

#[test]
fn detect_a_session_without_limits_is_found_plan_unknown() {
    let mut fx = Fx::new();
    fx.write("rollout-g.jsonl", &[token_count("2026-09-12T10:00:00Z", 10, premium())], now() - 10_000);
    assert_eq!(fx.codex.detect(), (true, None));
}

#[test]
#[cfg(unix)] // the stub app-server is a shell script
fn a_live_reading_beats_an_older_rollout() {
    let mut fx = Fx::new();
    fx.stale();
    let bin = fx.stub_bin(json!({"id": 2, "result": live_result()}));
    fx.codex.set_bin(Some(bin.clone()));
    fx.codex.refresh_blocking(now());
    let u = fx.codex.get_usage(now()).unwrap();
    assert_eq!(u.session.unwrap().pct, Some(40.0));
    assert!(u.weekly_only);
    assert_eq!(u.plan.as_deref(), Some("prolite"));
    assert_eq!(u.limits_at, Some(now()));
    assert_eq!(fs::read_to_string(format!("{}.args", bin.display())).unwrap().trim(), "app-server");
    assert_eq!(fs::read_to_string(format!("{}.home", bin.display())).unwrap().trim(), fx.root.path().to_string_lossy());
}

#[test]
#[cfg(unix)] // the stub app-server is a shell script
fn falls_back_to_the_single_bucket_when_it_is_not_keyed() {
    let mut fx = Fx::new();
    fx.stale();
    let only = live_result()["rateLimitsByLimitId"]["codex"].clone();
    fx.codex.set_bin(Some(fx.stub_bin(json!({"id": 2, "result": {"rateLimits": only}}))));
    fx.codex.refresh_blocking(now());
    assert_eq!(fx.codex.get_usage(now()).unwrap().session.unwrap().pct, Some(40.0));
}

#[test]
#[cfg(unix)] // the stub app-server is a shell script
fn a_rollout_newer_than_the_live_read_wins() {
    let mut fx = Fx::new();
    fx.write("rollout-l.jsonl", &[token_count("2026-09-12T19:30:00Z", 10, codex_limits())], now() - 10_000);
    fx.codex.set_bin(Some(fx.stub_bin(json!({"id": 2, "result": live_result()}))));
    fx.codex.refresh_blocking(now() - 3_600_000);
    fx.codex.set_bin(None); // and no new live read on the way
    let u = fx.codex.get_usage(now()).unwrap();
    assert_eq!(u.session.unwrap().pct, Some(40.0));
    assert_eq!(u.weekly.unwrap().pct, Some(31.0));
}

#[test]
#[cfg(unix)] // the stub app-server is a shell script
fn a_failed_read_keeps_the_last_good_one() {
    let mut fx = Fx::new();
    fx.stale();
    fx.codex.set_bin(Some(fx.stub_bin(json!({"id": 2, "result": live_result()}))));
    fx.codex.refresh_blocking(now());
    fx.codex.set_bin(Some(fx.stub_bin(json!({"id": 2, "error": {"message": "not logged in"}}))));
    fx.codex.refresh_blocking(now() + 1);
    fx.codex.set_bin(None);
    assert_eq!(fx.codex.get_usage(now()).unwrap().session.unwrap().pct, Some(40.0));
}

#[test]
fn a_missing_binary_falls_back_to_the_logs() {
    let mut fx = Fx::new();
    fx.stale();
    fx.codex.set_bin(Some(fx.root.path().join("no-such-codex")));
    fx.codex.refresh_blocking(now());
    fx.codex.set_bin(None);
    assert_eq!(fx.codex.get_usage(now()).unwrap().session.unwrap().pct, Some(85.0));
}

// wait (up to 5 s) for `ok`, polling: a fixed sleep is flaky under load
#[cfg(unix)]
fn eventually(mut ok: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    ok()
}

#[test]
#[cfg(unix)] // the stub app-server is a shell script
fn get_usage_asks_at_most_once_per_interval() {
    let mut fx = Fx::new();
    fx.stale();
    let bin = fx.stub_bin(json!({"id": 2, "result": live_result()}));
    fx.codex.set_bin(Some(bin.clone()));
    let runs = || fs::read_to_string(format!("{}.runs", bin.display())).unwrap_or_default().lines().count();
    fx.codex.get_usage(now());
    assert!(eventually(|| runs() == 1));
    // the answer lands on a later poll; within the interval nothing is spawned
    assert!(eventually(|| fx.codex.get_usage(now() + 60_000).unwrap().session.unwrap().pct == Some(40.0)));
    assert_eq!(runs(), 1);
    fx.codex.get_usage(now() + 120_000);
    assert!(eventually(|| runs() == 2));
}

fn task(kind: &str) -> String {
    json!({"timestamp": "2026-09-12T19:50:00Z", "type": "event_msg", "payload": {"type": kind}}).to_string()
}

#[test]
fn an_open_task_keeps_codex_working_through_a_quiet_stretch() {
    let mut fx = Fx::new();
    // a long command: nothing written for five minutes, the task still open
    fx.write("rollout-a.jsonl", &[task("task_started"), token_count("2026-09-12T19:54:00Z", 10, codex_limits())], now() - 5 * 60_000);
    assert!(fx.codex.get_usage(now()).unwrap().active);
}

#[test]
fn a_finished_or_interrupted_task_rests_at_once() {
    for end in ["task_complete", "turn_aborted"] {
        let mut fx = Fx::new();
        fx.write("rollout-a.jsonl", &[task("task_started"), task(end)], now() - 5_000);
        assert!(!fx.codex.get_usage(now()).unwrap().active, "{end}");
    }
}

#[test]
fn any_recent_session_with_an_open_task_counts() {
    let mut fx = Fx::new();
    fx.write("rollout-a.jsonl", &[task("task_started")], now() - 10 * 60_000);
    fx.write("rollout-b.jsonl", &[task("task_started"), task("task_complete")], now() - 5_000);
    assert!(fx.codex.get_usage(now()).unwrap().active);
}

#[test]
fn an_open_task_untouched_for_half_an_hour_was_abandoned() {
    let mut fx = Fx::new();
    fx.write("rollout-a.jsonl", &[task("task_started")], now() - 31 * 60_000);
    assert!(!fx.codex.get_usage(now()).unwrap().active);
}

#[test]
fn a_rollout_without_tasks_falls_back_to_the_last_minute() {
    let mut fx = Fx::new();
    fx.write("rollout-a.jsonl", &[token_count("2026-09-12T19:59:30Z", 10, codex_limits())], now() - 30_000);
    assert!(fx.codex.get_usage(now()).unwrap().active);
    let mut fx = Fx::new();
    fx.write("rollout-a.jsonl", &[token_count("2026-09-12T19:58:00Z", 10, codex_limits())], now() - 2 * 60_000);
    assert!(!fx.codex.get_usage(now()).unwrap().active);
}
