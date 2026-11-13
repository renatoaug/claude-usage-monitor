// Claude OAuth (same flow as Claude Code) + the authoritative usage endpoint.
// Port of auth.js. Blocking: the app calls it from its poll thread, one call
// at a time, so a refresh can never race another and rotate the token twice.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::usage::{now_ms, parse_date};

const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const REDIRECT: &str = "https://platform.claude.com/oauth/code/callback";
const AUTHORIZE: &str = "https://claude.ai/oauth/authorize";
const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
const SCOPE: &str = "org:create_api_key user:profile user:inference";
const UA: &str = "claude-cli/2.1.181 (external, cli)";

/// An HTTP status, or 0 for "try again later" (never "log in again").
#[derive(Debug, Clone, PartialEq)]
pub struct AuthError {
    pub status: u16,
    pub message: String,
}

impl AuthError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        AuthError { status, message: message.into() }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type Result<T> = std::result::Result<T, AuthError>;

pub struct Response {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

impl Response {
    fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

/// The network, swappable so the tests can route requests.
pub trait Http: Send {
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &Value) -> Result<Response>;
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response>;
}

pub struct Ureq(ureq::Agent);

impl Default for Ureq {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(20))).build().new_agent();
        Ureq(agent)
    }
}

fn read(res: std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Response> {
    // a network failure is transient: status 0, retry later
    let mut res = res.map_err(|e| AuthError::new(0, e.to_string()))?;
    let headers = res.headers().iter().map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())).collect();
    let status = res.status().as_u16();
    let body = res.body_mut().read_to_string().unwrap_or_default();
    Ok(Response { status, body, headers })
}

impl Http for Ureq {
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &Value) -> Result<Response> {
        let mut req = self.0.post(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        read(req.send_json(body))
    }
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response> {
        let mut req = self.0.get(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        read(req.call())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub pct: f64,
    pub reset_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scoped {
    pub label: String,
    pub pct: f64,
    pub reset_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Limits {
    /// the 5-hour window; on a weekly-only plan, the week in its place
    pub session: Window,
    pub week: Window,
    pub scoped: Vec<Scoped>,
    /// the plan has no 5-hour window, only weekly ones
    #[serde(default)]
    pub weekly_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub email: Option<String>,
    pub name: Option<String>,
    pub plan: Option<String>,
}

struct Pending {
    verifier: String,
    state: String,
}

/// The default data dir: alongside CLAUDE_CONFIG_DIR when that's set (one
/// Claude config per account via direnv), else ~/.claude-usage-monitor.
pub fn default_data_dir() -> PathBuf {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(d) if !d.is_empty() => PathBuf::from(d).join("usage-monitor"),
        _ => std::env::home_dir().unwrap_or_default().join(".claude-usage-monitor"),
    }
}

pub struct Auth {
    http: Box<dyn Http>,
    token_path: PathBuf,
    tokens: Option<Tokens>,
    pending: Option<Pending>,
    profile: Option<Profile>,
    // the file exists but couldn't be read (a disk still waking up): that
    // must not read as a logout
    unreadable: bool,
    rate_limit: Vec<(String, String)>,
}

fn b64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn random_b64() -> String {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).expect("system randomness");
    b64url(&buf)
}

fn encode_uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The usage response as limits. A plan without the 5-hour window (no
/// `five_hour` at all, not an idle one with no `resets_at`) shows its week in
/// the session's place.
fn limits(j: &Value) -> Limits {
    let empty = || Window { pct: 0.0, reset_ms: None };
    let week = j.get("seven_day").and_then(window);
    let weekly_only = j.get("five_hour").is_none_or(Value::is_null) && week.is_some();
    Limits {
        session: if weekly_only { week.clone() } else { j.get("five_hour").and_then(window) }.unwrap_or_else(empty),
        week: week.unwrap_or_else(empty),
        scoped: scoped_weeks(j),
        weekly_only,
    }
}

fn window(o: &Value) -> Option<Window> {
    let pct = o.get("utilization")?.as_f64()?;
    let reset_ms = o.get("resets_at").and_then(Value::as_str).and_then(parse_date).map(|t| t - now_ms());
    Some(Window { pct, reset_ms })
}

// Weekly limits scoped to one model (e.g. "Fable" on Max) live in `limits`;
// the older seven_day_<model> windows are the fallback.
fn scoped_weeks(j: &Value) -> Vec<Scoped> {
    let mut out: Vec<Scoped> = j
        .get("limits")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|l| l.get("kind").and_then(Value::as_str) == Some("weekly_scoped"))
        .filter_map(|l| {
            let pct = l.get("percent")?.as_f64()?;
            let label = l.pointer("/scope/model/display_name")?.as_str().filter(|s| !s.is_empty())?;
            let reset_ms = l.get("resets_at").and_then(Value::as_str).and_then(parse_date).map(|t| t - now_ms());
            Some(Scoped { label: label.into(), pct, reset_ms })
        })
        .collect();
    if out.is_empty() {
        for (key, label) in [("seven_day_opus", "Opus"), ("seven_day_sonnet", "Sonnet")] {
            if let Some(w) = j.get(key).and_then(window) {
                out.push(Scoped { label: label.into(), pct: w.pct, reset_ms: w.reset_ms });
            }
        }
    }
    out
}

impl Auth {
    pub fn new(data_dir: Option<PathBuf>, http: Box<dyn Http>) -> Self {
        Auth {
            http,
            token_path: data_dir.unwrap_or_else(default_data_dir).join("auth.json"),
            tokens: None,
            pending: None,
            profile: None,
            unreadable: false,
            rate_limit: Vec::new(),
        }
    }

    /// Point at another account's dir: the cached token and profile go.
    pub fn set_data_dir(&mut self, dir: Option<PathBuf>) {
        self.token_path = dir.unwrap_or_else(default_data_dir).join("auth.json");
        self.tokens = None;
        self.profile = None;
        self.pending = None;
        self.unreadable = false;
    }

    fn load(&mut self) -> Option<&Tokens> {
        if self.tokens.is_none() {
            let raw = match fs::read_to_string(&self.token_path) {
                Ok(raw) => {
                    self.unreadable = false;
                    Some(raw)
                }
                Err(e) => {
                    self.unreadable = e.kind() != ErrorKind::NotFound;
                    None
                }
            };
            // read but unparseable really is dead
            self.tokens = raw.and_then(|r| serde_json::from_str(&r).ok());
        }
        self.tokens.as_ref()
    }

    fn save(&mut self, t: Tokens) {
        if let Some(dir) = self.token_path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let body = serde_json::to_string_pretty(&t).unwrap_or_default();
        let _ = write_private(&self.token_path, body.as_bytes());
        self.tokens = Some(t);
    }

    pub fn clear(&mut self) {
        self.tokens = None;
        self.profile = None;
        self.unreadable = false;
        let _ = fs::remove_file(&self.token_path);
    }

    /// A token we merely failed to read still counts as connected.
    pub fn is_connected(&mut self) -> bool {
        self.load().is_some() || self.unreadable
    }

    /// Step 1: the authorize URL to open in the browser.
    pub fn begin(&mut self) -> String {
        let verifier = random_b64();
        let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
        let state = random_b64();
        let params = [
            ("code", "true"),
            ("client_id", CLIENT_ID),
            ("response_type", "code"),
            ("redirect_uri", REDIRECT),
            ("scope", SCOPE),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("state", &state),
        ];
        let query: Vec<String> = params.iter().map(|(k, v)| format!("{k}={}", encode_uri_component(v))).collect();
        let url = format!("{AUTHORIZE}?{}", query.join("&"));
        self.pending = Some(Pending { verifier, state });
        url
    }

    /// Step 2: exchange the pasted "code#state" for tokens. A pasted
    /// long-lived token (`claude setup-token`) skips the exchange.
    pub fn complete(&mut self, pasted: &str) -> Result<()> {
        let raw = pasted.trim();
        if !raw.contains('#') {
            self.save(Tokens { access_token: raw.into(), refresh_token: None, expires_at: Some(now_ms() + 365 * 86_400_000) });
            self.pending = None;
            return Ok(());
        }
        let Some(pending) = &self.pending else {
            return Err(AuthError::new(0, "no pending auth"));
        };
        let (code, returned) = raw.split_once('#').unwrap_or((raw, ""));
        let returned = returned.split('#').next().unwrap_or("");
        let body = json!({
            "grant_type": "authorization_code",
            "code": code,
            "state": if returned.is_empty() { pending.state.as_str() } else { returned },
            "code_verifier": pending.verifier,
            "client_id": CLIENT_ID,
            "redirect_uri": REDIRECT,
        });
        let res = self.http.post_json(TOKEN_URL, &[("User-Agent", UA)], &body)?;
        if !res.ok() {
            let snippet: String = res.body.chars().take(150).collect();
            return Err(AuthError::new(res.status, format!("exchange {}: {snippet}", res.status)));
        }
        let j = res.json();
        self.save(Tokens {
            access_token: j["access_token"].as_str().unwrap_or("").into(),
            refresh_token: j["refresh_token"].as_str().map(str::to_string),
            expires_at: Some(now_ms() + j["expires_in"].as_i64().filter(|&s| s > 0).unwrap_or(3600) * 1000),
        });
        self.pending = None;
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        let Some(t) = self.load().cloned() else {
            return Err(AuthError::new(401, "no refresh token"));
        };
        let Some(refresh) = t.refresh_token.filter(|r| !r.is_empty()) else {
            return Err(AuthError::new(401, "no refresh token"));
        };
        let body = json!({ "grant_type": "refresh_token", "refresh_token": refresh, "client_id": CLIENT_ID });
        let res = self.http.post_json(TOKEN_URL, &[("User-Agent", UA)], &body)?;
        if !res.ok() {
            // a rejected grant means the session is dead → 401 (log in again);
            // throttling and server errors keep their status (retry)
            let dead = res.status == 400 || res.status == 403;
            return Err(AuthError::new(if dead { 401 } else { res.status }, "refresh failed"));
        }
        let j = res.json();
        self.save(Tokens {
            access_token: j["access_token"].as_str().unwrap_or("").into(),
            refresh_token: j["refresh_token"].as_str().map(str::to_string).or(Some(refresh)),
            expires_at: Some(now_ms() + j["expires_in"].as_i64().filter(|&s| s > 0).unwrap_or(3600) * 1000),
        });
        Ok(())
    }

    fn valid_token(&mut self) -> Result<String> {
        let unreadable = {
            let has = self.load().is_some();
            !has && self.unreadable
        };
        let Some(t) = self.load().cloned() else {
            return Err(if unreadable { AuthError::new(0, "token unreadable") } else { AuthError::new(401, "not connected") });
        };
        if t.expires_at.is_none_or(|at| at - now_ms() < 60_000) {
            self.refresh()?;
        }
        Ok(self.load().map(|t| t.access_token.clone()).unwrap_or_default())
    }

    fn api_get(&mut self, url: &str) -> Result<Response> {
        let token = self.valid_token()?;
        let bearer = format!("Bearer {token}");
        self.http.get(url, &[("Authorization", &bearer), ("anthropic-beta", "oauth-2025-04-20"), ("anthropic-version", "2023-06-01"), ("User-Agent", UA)])
    }

    /// Step 3: the authoritative usage.
    pub fn fetch_usage(&mut self) -> Result<Limits> {
        let res = self.api_get(USAGE_URL)?;
        if !res.ok() {
            let snippet: String = res.body.chars().take(150).collect();
            return Err(AuthError::new(res.status, format!("usage {}: {snippet}", res.status)));
        }
        let rl: Vec<(String, String)> =
            res.headers.iter().filter_map(|(k, v)| k.strip_prefix("anthropic-ratelimit-").map(|k| (k.to_string(), v.clone()))).collect();
        if !rl.is_empty() {
            self.rate_limit = rl;
        }
        let j = res.json();
        Ok(limits(&j))
    }

    /// The account's identity, cached until logout.
    pub fn fetch_profile(&mut self) -> Result<Profile> {
        if let Some(p) = &self.profile {
            return Ok(p.clone());
        }
        let res = self.api_get(PROFILE_URL)?;
        if !res.ok() {
            return Err(AuthError::new(res.status, format!("profile {}", res.status)));
        }
        let j = res.json();
        let a = &j["account"];
        let s = |k: &str| a.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
        let flag = |k: &str| a.get(k).and_then(Value::as_bool).unwrap_or(false);
        let p = Profile {
            email: s("email"),
            name: s("display_name").or_else(|| s("full_name")),
            plan: if flag("has_claude_max") {
                Some("Max".into())
            } else if flag("has_claude_pro") {
                Some("Pro".into())
            } else {
                None
            },
        };
        self.profile = Some(p.clone());
        Ok(p)
    }
}

/// Write a file only its owner can read (the token).
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        opts.mode(0o600);
        let mut f = opts.open(path)?;
        // an existing file keeps its old mode on open: tighten it
        f.set_permissions(fs::Permissions::from_mode(0o600))?;
        return f.write_all(bytes);
    }
    #[allow(unreachable_code)]
    opts.open(path)?.write_all(bytes)
}

#[cfg(test)]
mod tests;
