// Port of test/unit/usage.test.js. Each test gets its own config dir and
// reader, so fixtures never leak between tests.

use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

const MIN: i64 = 60_000;
const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;

static SEQ: AtomicUsize = AtomicUsize::new(0);

fn budget() -> Config {
    Config { session_token_budget: Some(1000.0), weekly_token_budget: Some(10_000.0), ..Default::default() }
}

fn cfg() -> Config {
    Config { active_threshold_ms: Some(20_000), sleep_threshold_ms: Some(300_000), ..budget() }
}

fn iso(ts: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts).unwrap().to_rfc3339()
}

#[derive(Default)]
struct L {
    ts: Option<i64>,
    model: Option<&'static str>,
    tokens: u64,
    id: Option<&'static str>,
    request_id: Option<&'static str>,
}

// one assistant line as Claude Code writes it
fn line(l: L) -> String {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    json!({
        "type": "assistant",
        "timestamp": iso(l.ts.unwrap_or_else(now_ms)),
        "requestId": l.request_id.map(str::to_string).unwrap_or(format!("req-{n}")),
        "message": {
            "id": l.id.map(str::to_string).unwrap_or(format!("msg-{n}")),
            "model": l.model.unwrap_or("claude-opus-5"),
            "usage": { "input_tokens": l.tokens, "output_tokens": 0 },
        },
    })
    .to_string()
}

fn tok(tokens: u64) -> String {
    line(L { tokens, ..Default::default() })
}

fn tok_at(ago: i64, tokens: u64) -> String {
    line(L { ts: Some(now_ms() - ago), tokens, ..Default::default() })
}

struct Fx {
    root: TempDir,
    reader: Reader,
}

impl Fx {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let reader = Reader::new(Some(root.path().to_path_buf()));
        Fx { root, reader }
    }
    fn write_log(&self, lines: &[String]) -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        self.write_log_in(lines, "session.jsonl", &format!("-proj-{n}"))
    }
    fn write_log_in(&self, lines: &[String], name: &str, project: &str) -> PathBuf {
        let dir = self.root.path().join("projects").join(project);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join(name);
        fs::write(&file, format!("{}\n", lines.join("\n"))).unwrap();
        file
    }
    fn usage(&mut self, c: &Config) -> Usage {
        self.reader.get_usage(c)
    }
    // a real dir under the fixture, encoded the way Claude Code names projects
    fn real_dir(&self, rel: &str) -> PathBuf {
        let d = self.root.path().join("fs").join(rel);
        fs::create_dir_all(&d).unwrap();
        d
    }
}

fn enc(p: &Path) -> String {
    // macOS tempdirs live under /var → /private/var; resolve so the walk matches
    let p = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    encode_segment(&p.to_string_lossy())
}

fn enc_missing(parent: &Path, tail: &str) -> String {
    format!("{}-{}", enc(parent), encode_segment(tail))
}

fn age(file: &Path, ms: i64) {
    let t = SystemTime::now() - std::time::Duration::from_millis(ms as u64);
    File::options().write(true).open(file).unwrap().set_modified(t).unwrap();
}

// ---- token accounting ----

#[test]
fn sums_all_four_usage_fields() {
    let u = json!({"input_tokens": 1, "output_tokens": 2, "cache_read_input_tokens": 4, "cache_creation_input_tokens": 8});
    assert_eq!(tokens_of(&u), 15);
}

#[test]
fn treats_missing_fields_as_zero() {
    assert_eq!(tokens_of(&json!({"input_tokens": 5})), 5);
    assert_eq!(tokens_of(&json!({})), 0);
}

#[test]
fn adds_up_entries_across_a_log() {
    let mut fx = Fx::new();
    fx.write_log(&[tok(100), tok(250)]);
    let u = fx.usage(&budget());
    assert_eq!(u.month_tokens, 350);
    assert_eq!(u.today_tokens, 350);
}

// ---- log decoding ----

#[test]
fn ignores_everything_that_is_not_a_billable_assistant_turn() {
    let mut fx = Fx::new();
    fx.write_log(&[
        tok(100),
        json!({"type": "user", "message": {"content": "hi"}}).to_string(),
        json!({"type": "assistant", "message": {"model": "claude-opus-5"}}).to_string(),
        line(L { tokens: 999, model: Some("<synthetic>"), ..Default::default() }),
        json!({"type": "assistant", "message": {"usage": {"input_tokens": 7}}}).to_string(),
    ]);
    assert_eq!(fx.usage(&budget()).month_tokens, 100);
}

#[test]
fn survives_malformed_and_truncated_lines() {
    let mut fx = Fx::new();
    fx.write_log(&[tok(10), "not json at all".into(), "{\"type\":\"assist".into(), tok(20)]);
    assert_eq!(fx.usage(&budget()).month_tokens, 30);
}

#[test]
fn reads_only_what_a_growing_log_appends_even_mid_line() {
    use std::io::Write;
    let mut fx = Fx::new();
    let file = fx.write_log(&[tok(10)]);
    assert_eq!(fx.usage(&budget()).month_tokens, 10);
    let next = line(L { tokens: 20, model: Some("claude-opus-5-ção"), ..Default::default() });
    let bytes = next.as_bytes();
    let cut = next.find('ç').unwrap() + 1; // inside a UTF-8 character
    let mut f = File::options().append(true).open(&file).unwrap();
    f.write_all(&bytes[..cut]).unwrap();
    assert_eq!(fx.usage(&budget()).month_tokens, 10);
    f.write_all(&bytes[cut..]).unwrap();
    f.write_all(b"\n").unwrap();
    assert_eq!(fx.usage(&budget()).month_tokens, 30);
}

#[test]
fn starts_over_when_a_log_shrinks() {
    let mut fx = Fx::new();
    let file = fx.write_log(&[tok(10), tok(20)]);
    assert_eq!(fx.usage(&budget()).month_tokens, 30);
    fs::write(&file, format!("{}\n", tok(5))).unwrap();
    assert_eq!(fx.usage(&budget()).month_tokens, 5);
}

#[test]
fn deduplicates_a_turn_recorded_in_more_than_one_file() {
    let mut fx = Fx::new();
    let dup = || line(L { tokens: 500, id: Some("msg-same"), request_id: Some("req-same"), ..Default::default() });
    fx.write_log_in(&[dup()], "a.jsonl", "-p");
    fx.write_log_in(&[dup()], "b.jsonl", "-p");
    assert_eq!(fx.usage(&budget()).month_tokens, 500);
}

// ---- model labels ----

#[test]
fn model_labels() {
    for (model, label) in [
        ("claude-opus-5", "Opus 5"),
        ("claude-opus-4-8", "Opus 4.8"),
        ("claude-sonnet-5", "Sonnet 5"),
        ("claude-haiku-4-5-20251001", "Haiku 4.5"),
        ("claude-3-5-sonnet-20241022", "Sonnet 3.5"),
        ("claude-fable-5", "Fable 5"),
        ("claude-opus-4-20250514", "Opus 4"),
    ] {
        assert_eq!(label_for(Some(model)), label, "{model}");
    }
}

#[test]
fn passes_unknown_models_through_and_handles_nothing() {
    assert_eq!(label_for(Some("some-other-model")), "some-other-model");
    assert_eq!(label_for(None), "desconhecido");
}

#[test]
fn aggregates_weekly_tokens_per_label() {
    let mut fx = Fx::new();
    let m = |tokens, model| line(L { tokens, model: Some(model), ..Default::default() });
    fx.write_log(&[m(100, "claude-opus-5"), m(50, "claude-opus-5"), m(20, "claude-sonnet-5")]);
    assert_eq!(
        fx.usage(&budget()).by_model,
        vec![Labeled { label: "Opus 5".into(), tokens: 150 }, Labeled { label: "Sonnet 5".into(), tokens: 20 }]
    );
}

// ---- project labels ----

#[test]
fn resolves_an_existing_path_down_to_its_basename() {
    let fx = Fx::new();
    let dir = fx.real_dir("my-projects/claude-usage-monitor");
    assert_eq!(project_label_uncached(&enc(&dir)), "claude-usage-monitor");
}

#[test]
fn handles_a_dash_inside_a_directory_name() {
    let fx = Fx::new();
    fx.real_dir("my/x");
    let target = fx.real_dir("my-projects");
    assert_eq!(project_label_uncached(&enc(&target)), "my-projects");
}

#[test]
fn handles_a_dot_inside_a_directory_name() {
    let fx = Fx::new();
    let dir = fx.real_dir("nord/.worktrees/sup-441");
    assert_eq!(project_label_uncached(&enc(&dir)), "sup-441");
}

#[test]
fn falls_back_to_the_raw_tail_when_the_project_is_gone() {
    let fx = Fx::new();
    let parent = fx.real_dir("my-projects");
    assert!(project_label_uncached(&enc_missing(&parent, "deleted-thing")).contains("deleted-thing"));
}

#[test]
fn does_not_eat_into_the_name_of_a_directory_that_no_longer_exists() {
    let fx = Fx::new();
    fx.real_dir("q/identity");
    let q = fx.root.path().join("fs/q");
    let label = project_label_uncached(&enc_missing(&q, "identity-web"));
    assert_ne!(label, "web");
    assert!(label.contains("identity-web"), "{label}");
}

// ---- usage by project ----

#[test]
fn aggregates_weekly_tokens_per_project_directory() {
    let mut fx = Fx::new();
    let a = fx.real_dir("alpha");
    let b = fx.real_dir("beta");
    fx.write_log_in(&[tok(100), tok(50)], "session.jsonl", &enc(&a));
    fx.write_log_in(&[tok(20)], "session.jsonl", &enc(&b));
    assert_eq!(
        fx.usage(&budget()).by_project,
        vec![Labeled { label: "alpha".into(), tokens: 150 }, Labeled { label: "beta".into(), tokens: 20 }]
    );
}

#[test]
fn folds_everything_past_the_top_five_into_one_other_row() {
    let mut fx = Fx::new();
    for i in 0..8u64 {
        fx.write_log_in(&[tok(100 - i)], "session.jsonl", &format!("-nope-p{i}"));
    }
    let list = fx.usage(&budget()).by_project;
    assert_eq!(list.len(), 6);
    assert_eq!(list[5].label, "other · 3");
    assert_eq!(list[5].tokens, 95 + 94 + 93);
}

#[test]
fn by_project_counts_only_the_weekly_window() {
    let mut fx = Fx::new();
    let old = fx.real_dir("stale");
    fx.write_log_in(&[tok_at(20 * DAY, 999)], "session.jsonl", &enc(&old));
    assert!(fx.usage(&budget()).by_project.is_empty());
}

// ---- the 5-hour session window ----

#[test]
fn session_opens_at_the_first_turn_and_counts_only_what_falls_inside() {
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(2 * HOUR, 100), tok_at(HOUR, 200)]);
    let s = fx.usage(&budget()).session;
    assert!(s.active);
    assert_eq!(s.tokens, 300);
    assert!((s.reset_ms as f64 / HOUR as f64 - 3.0).abs() < 0.05);
    assert_eq!(s.pct, 30.0);
}

#[test]
fn session_windows_chain_a_new_one_opens_only_after_the_previous_expires() {
    // the 6h-old turn opens a window until 1h ago; the 2h-old turn falls
    // inside it, so only the 1h-old turn opens the live window
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(6 * HOUR, 700), tok_at(2 * HOUR, 100), tok_at(HOUR, 200)]);
    let s = fx.usage(&budget()).session;
    assert_eq!(s.tokens, 200);
    assert!((s.reset_ms as f64 / HOUR as f64 - 4.0).abs() < 0.05);
}

#[test]
fn session_is_inactive_once_the_window_has_fully_elapsed() {
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(7 * HOUR, 100)]);
    let s = fx.usage(&budget()).session;
    assert!(!s.active);
    assert_eq!(s.tokens, 0);
}

#[test]
fn caps_the_percentage_at_100() {
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(MIN, 5000)]);
    assert_eq!(fx.usage(&budget()).session.pct, 100.0);
}

// ---- time buckets ----

#[test]
fn separates_today_the_week_and_the_month() {
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(MIN, 1), tok_at(3 * DAY, 10), tok_at(20 * DAY, 100), tok_at(60 * DAY, 1000)]);
    let u = fx.usage(&budget());
    assert_eq!(u.today_tokens, 1);
    assert_eq!(u.week.tokens, 11);
    assert_eq!(u.month_tokens, 111);
}

#[test]
fn lands_each_entry_in_its_own_day_of_the_30_day_map() {
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(MIN, 5), tok_at(2 * DAY, 7)]);
    let d = fx.usage(&budget()).days30;
    assert_eq!(d.len(), 30);
    assert_eq!(d[29], 5);
    assert_eq!(d[27], 7);
}

#[test]
fn reports_the_recent_rate_over_the_last_five_minutes() {
    let mut fx = Fx::new();
    fx.write_log(&[tok_at(MIN, 500), tok_at(30 * MIN, 9999)]);
    assert_eq!(fx.usage(&budget()).tokens_per_min, 100);
}

#[test]
fn weekly_anchor_sets_the_week_start_and_reset() {
    let mut fx = Fx::new();
    let now = now_ms();
    // anchor 2 days ago: the week opened then and resets in 5 days
    let anchor = iso(now - 2 * DAY);
    fx.write_log(&[tok_at(HOUR, 10), tok_at(3 * DAY, 100)]);
    let c = Config { weekly_anchor_iso: Some(anchor), ..budget() };
    let u = fx.reader.get_usage_at(&c, now);
    assert_eq!(u.week.tokens, 10);
    assert_eq!(u.week.reset_ms, Some(5 * DAY));
}

// ---- activity detection ----

fn tool_line(name: &str) -> String {
    json!({"type": "assistant", "timestamp": iso(now_ms()),
        "message": {"model": "claude-opus-5", "content": [{"type": "tool_use", "name": name}]}})
    .to_string()
}

#[test]
fn maps_tools_to_activities() {
    let fx = Fx::new();
    for (tool, expected) in [
        ("Edit", "editing"),
        ("Write", "editing"),
        ("Read", "reading"),
        ("Grep", "reading"),
        ("Bash", "running"),
        ("WebSearch", "researching"),
        ("Task", "delegating"),
        ("TodoWrite", "planning"),
        ("AskUserQuestion", "waiting"),
        ("SomeNewTool", "working"),
    ] {
        assert_eq!(detect_activity(&fx.write_log(&[tool_line(tool)])).as_deref(), Some(expected), "{tool}");
    }
}

#[test]
fn reads_the_most_recent_message_not_the_first() {
    let fx = Fx::new();
    assert_eq!(detect_activity(&fx.write_log(&[tool_line("Read"), tool_line("Bash")])).as_deref(), Some("running"));
}

#[test]
fn within_one_message_the_last_tool_block_wins() {
    let fx = Fx::new();
    let batched = json!({"type": "assistant", "message": {"model": "claude-opus-5", "content": [
        {"type": "text", "text": "let me look"},
        {"type": "tool_use", "name": "Read"},
        {"type": "tool_use", "name": "Bash"},
    ]}})
    .to_string();
    assert_eq!(detect_activity(&fx.write_log(&[batched])).as_deref(), Some("running"));
}

#[test]
fn plan_mode_outranks_whatever_tool_ran_last() {
    let fx = Fx::new();
    let f = fx.write_log(&[json!({"type": "permission-mode", "mode": "plan"}).to_string(), tool_line("Edit")]);
    assert_eq!(detect_activity(&f).as_deref(), Some("planning"));
}

#[test]
fn no_tool_use_means_no_signal_and_a_missing_file_is_not_a_crash() {
    let fx = Fx::new();
    assert_eq!(detect_activity(&fx.write_log(&[tok(10)])), None);
    assert_eq!(detect_activity(&fx.root.path().join("nope.jsonl")), None);
}

// ---- open turns ----

fn prompt(text: &str) -> String {
    json!({"type": "user", "message": {"role": "user", "content": text}}).to_string()
}
fn call(name: &str) -> String {
    json!({"type": "assistant", "message": {"stop_reason": "tool_use", "content": [{"type": "tool_use", "name": name}]}})
        .to_string()
}
fn result() -> String {
    json!({"type": "user", "message": {"content": [{"type": "tool_result"}]}}).to_string()
}
fn answer() -> String {
    json!({"type": "assistant", "message": {"stop_reason": "end_turn", "content": [{"type": "text", "text": "done"}]}})
        .to_string()
}
fn duration() -> String {
    json!({"type": "system", "subtype": "turn_duration"}).to_string()
}

#[test]
fn a_tool_call_its_result_or_a_prompt_leave_the_turn_open() {
    assert_eq!(turn_open(&[prompt("fix it")]), Some(true));
    assert_eq!(turn_open(&[prompt("fix it"), call("Bash")]), Some(true));
    assert_eq!(turn_open(&[prompt("fix it"), call("Bash"), result()]), Some(true));
}

#[test]
fn a_final_answer_its_turn_duration_or_an_interruption_close_it() {
    assert_eq!(turn_open(&[call("Bash"), result(), answer()]), Some(false));
    let title = json!({"type": "ai-title", "title": "x"}).to_string();
    assert_eq!(turn_open(&[answer(), duration(), title]), Some(false));
    assert_eq!(turn_open(&[call("Bash"), prompt("[Request interrupted by user for tool use]")]), Some(false));
    let refusal = json!({"type": "assistant", "message": {"stop_reason": "refusal", "content": []}}).to_string();
    assert_eq!(turn_open(&[prompt("hi"), refusal]), Some(false));
}

#[test]
fn slash_commands_shell_input_notices_and_meta_lines_open_nothing() {
    for text in [
        "<command-name>/model</command-name>",
        "<local-command-stdout>Set model</local-command-stdout>",
        "<bash-input>ls</bash-input>",
        "<task-notification> <task-id>1</task-id>",
        "This session is being continued from a previous conversation",
    ] {
        assert_eq!(turn_open(&[answer(), duration(), prompt(text)]), Some(false), "{text}");
    }
    let meta = json!({"type": "user", "isMeta": true, "message": {"content": "caveat"}}).to_string();
    assert_eq!(turn_open(&[answer(), meta]), Some(false));
    let side = json!({"type": "user", "isSidechain": true, "message": {"content": "sub"}}).to_string();
    assert_eq!(turn_open(&[answer(), side]), Some(false));
    let text = json!({"type": "user", "message": {"content": [{"type": "text", "text": "go"}]}}).to_string();
    assert_eq!(turn_open(&[answer(), text]), Some(true));
}

#[test]
fn a_tail_with_no_turn_in_it_says_nothing() {
    assert_eq!(turn_open(&[]), None);
    let away = json!({"type": "system", "subtype": "away_summary"}).to_string();
    assert_eq!(turn_open(&["{broken".into(), away]), None);
    assert_eq!(turn_open(&[json!({"type": "assistant", "message": {}}).to_string()]), None);
}

#[test]
fn a_long_quiet_command_still_counts_as_working() {
    let mut fx = Fx::new();
    age(&fx.write_log(&[prompt("build it"), call("Bash")]), 3 * MIN);
    let u = fx.usage(&cfg());
    assert!(u.active && !u.sleeping);
    assert_eq!(u.activity.as_deref(), Some("running"));
}

#[test]
fn the_model_thinking_after_a_tool_result_still_counts_as_working() {
    let mut fx = Fx::new();
    age(&fx.write_log(&[prompt("go"), call("Read"), result()]), 2 * MIN);
    let u = fx.usage(&cfg());
    assert!(u.active);
    assert_eq!(u.activity.as_deref(), Some("reading"));
    age(&fx.write_log(&[prompt("go")]), MIN);
    let u = fx.usage(&cfg());
    assert!(u.active);
    assert_eq!(u.activity.as_deref(), Some("working"));
}

#[test]
fn a_finished_turn_rests_at_once_even_if_the_log_was_just_written() {
    let mut fx = Fx::new();
    fx.write_log(&[prompt("go"), call("Bash"), result(), answer(), duration()]);
    let u = fx.usage(&cfg());
    assert!(!u.active);
    assert_eq!(u.activity, None);
}

#[test]
fn an_open_turn_in_another_session_wins_over_a_finished_newest_one() {
    let mut fx = Fx::new();
    age(&fx.write_log(&[prompt("long"), call("Task")]), 4 * MIN);
    fx.write_log(&[prompt("quick"), answer(), duration()]);
    let u = fx.usage(&cfg());
    assert!(u.active);
    assert_eq!(u.activity.as_deref(), Some("delegating"));
}

#[test]
fn an_open_turn_untouched_for_30_minutes_was_abandoned() {
    let mut fx = Fx::new();
    age(&fx.write_log(&[prompt("go"), call("Bash")]), 31 * MIN);
    let u = fx.usage(&cfg());
    assert!(!u.active);
    assert!(u.sleeping);
}

// ---- switching accounts and activity state ----

#[test]
fn set_claude_dir_reads_another_account_and_none_goes_back() {
    let mut fx = Fx::new();
    fx.write_log(&[tok(10)]);
    let home = fx.root.path().to_path_buf();
    let other = TempDir::new().unwrap();
    fx.reader.set_claude_dir(Some(other.path().to_path_buf()));
    // a second subscription with no sessions yet must not inherit the first's
    assert_eq!(fx.usage(&budget()).month_tokens, 0);
    fx.reader.set_claude_dir(Some(home));
    assert!(fx.usage(&budget()).month_tokens > 0);
}

#[test]
fn a_log_just_written_means_claude_is_active_and_awake() {
    let mut fx = Fx::new();
    fx.write_log(&[tok(10)]);
    let u = fx.usage(&cfg());
    assert!(u.active && !u.sleeping);
}

#[test]
fn an_empty_log_directory_reports_zeroes() {
    let mut fx = Fx::new();
    fs::create_dir_all(fx.root.path().join("projects/empty-proj")).unwrap();
    let u = fx.usage(&budget());
    assert_eq!(u.days30.len(), 30);
    assert_eq!(u.month_tokens, 0);
    assert_eq!(u.last_activity_ms, None);
    assert!(u.sleeping && u.ts > 0);
}
