// Port of test/unit/auth.test.js. The network is a router of URL fragment →
// canned response, and every call is recorded.

use super::*;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct Call {
    url: String,
    headers: Vec<(String, String)>,
    body: Value,
}

#[derive(Clone)]
struct Route {
    status: u16,
    body: String,
}

fn ok(body: Value) -> Route {
    Route { status: 200, body: body.to_string() }
}
fn status(status: u16, body: &str) -> Route {
    Route { status, body: body.into() }
}

#[derive(Clone, Default)]
struct Mock {
    routes: Arc<Mutex<Vec<(String, Route)>>>,
    calls: Arc<Mutex<Vec<Call>>>,
}

impl Mock {
    fn route(&self, frag: &str, r: Route) {
        self.routes.lock().unwrap().push((frag.into(), r));
    }
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
    fn count(&self, frag: &str) -> usize {
        self.calls().iter().filter(|c| c.url.contains(frag)).count()
    }
    fn answer(&self, url: &str, headers: &[(&str, &str)], body: Value) -> Result<Response> {
        self.calls.lock().unwrap().push(Call {
            url: url.into(),
            headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            body,
        });
        let routes = self.routes.lock().unwrap();
        let (_, r) = routes.iter().find(|(f, _)| url.contains(f.as_str())).unwrap_or_else(|| panic!("unrouted fetch: {url}"));
        Ok(Response { status: r.status, body: r.body.clone(), headers: vec![] })
    }
}

impl Http for Mock {
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &Value) -> Result<Response> {
        self.answer(url, headers, body.clone())
    }
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response> {
        self.answer(url, headers, Value::Null)
    }
}

struct Fx {
    _root: TempDir,
    auth: Auth,
    mock: Mock,
    path: PathBuf,
}

fn fx() -> Fx {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("usage-monitor");
    let mock = Mock::default();
    let auth = Auth::new(Some(dir.clone()), Box::new(mock.clone()));
    Fx { _root: root, auth, mock, path: dir.join("auth.json") }
}

impl Fx {
    fn seed(&mut self, t: Value) {
        self.auth.clear();
        fs::create_dir_all(self.path.parent().unwrap()).unwrap();
        fs::write(&self.path, t.to_string()).unwrap();
    }
    fn saved(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(&self.path).unwrap()).unwrap()
    }
}

fn live() -> Value {
    json!({"access_token": "tok-live", "refresh_token": "ref-1", "expires_at": now_ms() + 3_600_000})
}
fn expired() -> Value {
    json!({"access_token": "tok-old", "refresh_token": "ref-1", "expires_at": now_ms() - 1000})
}

fn query(url: &str) -> Vec<(String, String)> {
    url.split_once('?')
        .unwrap()
        .1
        .split('&')
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap();
            (k.to_string(), v.to_string())
        })
        .collect()
}
fn param(url: &str, key: &str) -> String {
    query(url).into_iter().find(|(k, _)| k == key).map(|(_, v)| v).unwrap_or_default()
}
fn is_b64url(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn iso_in(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(now_ms() + ms).unwrap().to_rfc3339()
}
fn header(c: &Call, k: &str) -> String {
    c.headers.iter().find(|(h, _)| h == k).map(|(_, v)| v.clone()).unwrap_or_default()
}

// ---- authorize URL ----

#[test]
fn carries_the_pkce_challenge_and_the_app_identity() {
    let mut f = fx();
    let url = f.auth.begin();
    assert!(url.starts_with("https://claude.ai/oauth/authorize?"));
    assert_eq!(param(&url, "response_type"), "code");
    assert_eq!(param(&url, "code_challenge_method"), "S256");
    assert!(!param(&url, "client_id").is_empty());
    assert!(param(&url, "redirect_uri").contains("platform.claude.com"));
    assert!(param(&url, "scope").contains("user%3Aprofile"));
    assert!(is_b64url(&param(&url, "code_challenge")));
    assert!(is_b64url(&param(&url, "state")));
}

#[test]
fn the_challenge_is_the_sha256_of_the_verifier() {
    // RFC 7636 appendix B
    let v = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    assert_eq!(b64url(&Sha256::digest(v.as_bytes())), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
}

#[test]
fn is_different_every_time() {
    let mut f = fx();
    let (a, b) = (f.auth.begin(), f.auth.begin());
    assert_ne!(param(&a, "code_challenge"), param(&b, "code_challenge"));
    assert_ne!(param(&a, "state"), param(&b, "state"));
}

// ---- completing the login ----

#[test]
fn a_pasted_long_lived_token_skips_the_exchange() {
    let mut f = fx();
    f.auth.complete("  sk-ant-longlived  ").unwrap();
    assert!(f.mock.calls().is_empty());
    assert!(f.auth.is_connected());
    assert_eq!(f.saved()["access_token"], "sk-ant-longlived");
}

#[test]
fn exchanges_code_and_state_for_tokens() {
    let mut f = fx();
    f.mock.route("/oauth/token", ok(json!({"access_token": "A", "refresh_token": "R", "expires_in": 60})));
    f.auth.begin();
    f.auth.complete("the-code#the-state").unwrap();
    let c = &f.mock.calls()[0];
    assert!(c.url.contains("platform.claude.com/v1/oauth/token"));
    assert_eq!(c.body["grant_type"], "authorization_code");
    assert_eq!(c.body["code"], "the-code");
    assert_eq!(c.body["state"], "the-state");
    assert!(is_b64url(c.body["code_verifier"].as_str().unwrap()));
    let saved = f.saved();
    assert_eq!(saved["access_token"], "A");
    assert_eq!(saved["refresh_token"], "R");
    assert!(saved["expires_at"].as_i64().unwrap() > now_ms());
}

#[test]
fn refuses_a_code_once_the_pending_login_has_been_consumed() {
    let mut f = fx();
    f.auth.begin();
    f.auth.complete("a-long-lived-token").unwrap();
    assert_eq!(f.auth.complete("code#state").unwrap_err().message, "no pending auth");
}

#[test]
fn surfaces_a_rejected_exchange() {
    let mut f = fx();
    f.mock.route("/oauth/token", status(400, "bad_verifier"));
    f.auth.begin();
    let e = f.auth.complete("c#s").unwrap_err();
    assert_eq!(e.status, 400);
    assert!(e.message.contains("exchange 400") && e.message.contains("bad_verifier"));
}

#[cfg(unix)]
#[test]
fn stores_the_token_file_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = fx();
    f.auth.complete("sk-ant-secret").unwrap();
    assert_eq!(fs::metadata(&f.path).unwrap().permissions().mode() & 0o777, 0o600);
}

// ---- connection state ----

#[test]
fn tracks_whether_a_token_exists() {
    let mut f = fx();
    assert!(!f.auth.is_connected());
    f.seed(live());
    assert!(f.auth.is_connected());
    f.auth.clear();
    assert!(!f.auth.is_connected());
    assert!(!f.path.exists());
}

#[test]
fn a_corrupt_token_file_reads_as_disconnected() {
    let mut f = fx();
    fs::create_dir_all(f.path.parent().unwrap()).unwrap();
    fs::write(&f.path, "{ not json").unwrap();
    assert!(!f.auth.is_connected());
}

#[test]
fn a_token_that_cannot_be_read_still_counts_as_connected() {
    let mut f = fx();
    // a read failing for anything but "no such file": a disk not up yet
    fs::create_dir_all(&f.path).unwrap();
    assert!(f.auth.is_connected());
    // …and the poll is told to try later, not to send the user to the browser
    assert_eq!(f.auth.fetch_usage().unwrap_err().status, 0);
    fs::remove_dir_all(&f.path).unwrap();
    assert!(!f.auth.is_connected());
}

// ---- usage mapping ----

fn usage_body() -> Value {
    json!({
        "five_hour": {"utilization": 42, "resets_at": iso_in(2 * 3_600_000)},
        "seven_day": {"utilization": 7, "resets_at": iso_in(48 * 3_600_000)},
        "seven_day_opus": {"utilization": 3, "resets_at": null},
    })
}

#[test]
fn maps_the_api_windows_into_percentages_and_countdowns() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/usage", ok(usage_body()));
    let u = f.auth.fetch_usage().unwrap();
    assert_eq!(u.session.pct, 42.0);
    assert!((u.session.reset_ms.unwrap() as f64 / 3_600_000.0 - 2.0).abs() < 0.05);
    assert_eq!(u.week.pct, 7.0);
    // no `limits` array: the legacy per-model windows are the fallback
    assert_eq!(u.scoped, vec![Scoped { label: "Opus".into(), pct: 3.0, reset_ms: None }]);
}

#[test]
fn reads_per_model_weekly_limits_from_the_limits_array() {
    let mut f = fx();
    f.seed(live());
    let at = iso_in(24 * 3_600_000);
    let mut body = usage_body();
    body["limits"] = json!([
        {"kind": "session", "percent": 42, "resets_at": at},
        {"kind": "weekly_all", "percent": 7, "resets_at": at},
        {"kind": "weekly_scoped", "percent": 38, "resets_at": at, "scope": {"model": {"display_name": "Fable"}}},
        {"kind": "weekly_scoped", "percent": "n/a", "resets_at": at, "scope": {"model": {"display_name": "X"}}},
        {"kind": "weekly_scoped", "percent": 1, "resets_at": at, "scope": {"surface": "cowork"}},
    ]);
    f.mock.route("/oauth/usage", ok(body));
    let u = f.auth.fetch_usage().unwrap();
    assert_eq!(u.scoped.len(), 1);
    assert_eq!(u.scoped[0].label, "Fable");
    assert_eq!(u.scoped[0].pct, 38.0);
    assert!((u.scoped[0].reset_ms.unwrap() as f64 / 3_600_000.0 - 24.0).abs() < 0.05);
}

#[test]
fn no_scoped_limits_reads_as_an_empty_list() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/usage", ok(json!({"limits": "nope"})));
    assert!(f.auth.fetch_usage().unwrap().scoped.is_empty());
}

#[test]
fn a_missing_window_reads_as_zero() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/usage", ok(json!({})));
    let u = f.auth.fetch_usage().unwrap();
    assert_eq!(u.session, Window { pct: 0.0, reset_ms: None });
    assert_eq!(u.week, Window { pct: 0.0, reset_ms: None });
}

#[test]
fn a_non_numeric_utilization_is_not_trusted() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/usage", ok(json!({"five_hour": {"utilization": "lots"}})));
    assert_eq!(f.auth.fetch_usage().unwrap().session, Window { pct: 0.0, reset_ms: None });
}

#[test]
fn sends_the_bearer_token_and_the_oauth_beta_headers() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/usage", ok(usage_body()));
    f.auth.fetch_usage().unwrap();
    let c = &f.mock.calls()[0];
    assert_eq!(header(c, "Authorization"), "Bearer tok-live");
    assert_eq!(header(c, "anthropic-beta"), "oauth-2025-04-20");
}

#[test]
fn keeps_the_status_on_a_throttled_response() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/usage", status(429, "slow down"));
    assert_eq!(f.auth.fetch_usage().unwrap_err().status, 429);
}

// ---- token refresh ----

#[test]
fn refreshes_an_expired_token_before_calling_the_api() {
    let mut f = fx();
    f.seed(expired());
    f.mock.route("/oauth/token", ok(json!({"access_token": "fresh", "expires_in": 3600})));
    f.mock.route("/oauth/usage", ok(json!({"five_hour": {"utilization": 1}})));
    f.auth.fetch_usage().unwrap();
    let calls = f.mock.calls();
    assert!(calls[0].url.contains("/oauth/token"));
    assert_eq!(calls[0].body["grant_type"], "refresh_token");
    assert_eq!(header(&calls[1], "Authorization"), "Bearer fresh");
}

#[test]
fn back_to_back_callers_share_one_refresh() {
    let mut f = fx();
    f.seed(expired());
    f.mock.route("/oauth/token", ok(json!({"access_token": "fresh", "refresh_token": "ref-2", "expires_in": 3600})));
    f.mock.route("/oauth/usage", ok(json!({})));
    f.mock.route("/oauth/profile", ok(json!({"account": {"email": "a@b.com"}})));
    f.auth.fetch_usage().unwrap();
    f.auth.fetch_profile().unwrap();
    assert_eq!(f.mock.count("/oauth/token"), 1);
}

#[test]
fn keeps_the_old_refresh_token_when_the_response_omits_one() {
    let mut f = fx();
    f.seed(expired());
    f.mock.route("/oauth/token", ok(json!({"access_token": "fresh", "expires_in": 3600})));
    f.mock.route("/oauth/usage", ok(json!({})));
    f.auth.fetch_usage().unwrap();
    assert_eq!(f.saved()["refresh_token"], "ref-1");
}

#[test]
fn refresh_failures_map_to_dead_or_transient() {
    // a rejected/forbidden grant is a dead session; throttling and server
    // errors are transient — retry, do not log out
    for (got, want) in [(400, 401), (403, 401), (429, 429), (500, 500)] {
        let mut f = fx();
        f.seed(expired());
        f.mock.route("/oauth/token", status(got, ""));
        assert_eq!(f.auth.fetch_usage().unwrap_err().status, want, "refresh {got}");
    }
}

#[test]
fn an_expired_token_with_nothing_to_refresh_is_a_401() {
    let mut f = fx();
    f.seed(json!({"access_token": "x", "refresh_token": null, "expires_at": now_ms() - 1}));
    assert_eq!(f.auth.fetch_usage().unwrap_err().status, 401);
}

#[test]
fn not_being_connected_at_all_is_a_401() {
    let mut f = fx();
    assert_eq!(f.auth.fetch_usage().unwrap_err().status, 401);
}

// ---- account profile ----

fn with_account(f: &mut Fx, account: Value) {
    f.seed(live());
    f.mock.route("/oauth/profile", ok(json!({ "account": account })));
}

#[test]
fn maps_the_account_identity() {
    let mut f = fx();
    with_account(&mut f, json!({"email": "a@b.com", "display_name": "Ana", "has_claude_max": true}));
    assert_eq!(
        f.auth.fetch_profile().unwrap(),
        Profile { email: Some("a@b.com".into()), name: Some("Ana".into()), plan: Some("Max".into()) }
    );
}

#[test]
fn plan_tiers() {
    for (flags, plan) in [
        (json!({"has_claude_max": true, "has_claude_pro": true}), Some("Max")),
        (json!({"has_claude_pro": true}), Some("Pro")),
        (json!({}), None),
    ] {
        let mut f = fx();
        with_account(&mut f, flags.clone());
        assert_eq!(f.auth.fetch_profile().unwrap().plan.as_deref(), plan, "{flags}");
    }
}

#[test]
fn falls_back_to_full_name() {
    let mut f = fx();
    with_account(&mut f, json!({"email": "a@b.com", "full_name": "Ana Full"}));
    assert_eq!(f.auth.fetch_profile().unwrap().name.as_deref(), Some("Ana Full"));
}

#[test]
fn the_profile_is_cached_for_the_session_and_dropped_on_logout() {
    let mut f = fx();
    with_account(&mut f, json!({"email": "a@b.com"}));
    f.auth.fetch_profile().unwrap();
    f.auth.fetch_profile().unwrap();
    assert_eq!(f.mock.count("/profile"), 1);
    f.auth.clear();
    f.seed(live());
    f.auth.fetch_profile().unwrap();
    assert_eq!(f.mock.count("/profile"), 2);
}

#[test]
fn surfaces_a_failed_profile_fetch_with_its_status() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/profile", status(500, ""));
    assert_eq!(f.auth.fetch_profile().unwrap_err().status, 500);
}

#[test]
fn an_empty_profile_payload_yields_nulls() {
    let mut f = fx();
    f.seed(live());
    f.mock.route("/oauth/profile", ok(json!({})));
    assert_eq!(f.auth.fetch_profile().unwrap(), Profile { email: None, name: None, plan: None });
}
