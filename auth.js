// Claude OAuth (same flow as Claude Code) + the authoritative usage endpoint.
const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')

const CLIENT_ID = '9d1c250a-e61b-44d9-88ed-5944d1962f5e'
const REDIRECT = 'https://platform.claude.com/oauth/code/callback'
const AUTHORIZE = 'https://claude.ai/oauth/authorize'
const TOKEN_URL = 'https://platform.claude.com/v1/oauth/token'
const USAGE_URL = 'https://api.anthropic.com/api/oauth/usage'
const PROFILE_URL = 'https://api.anthropic.com/api/oauth/profile'
const SCOPE = 'org:create_api_key user:profile user:inference'
const UA = 'claude-cli/2.1.181 (external, cli)'
// when CLAUDE_CONFIG_DIR is set (e.g. via direnv for multi-account setups),
// keep the widget's data alongside that account's Claude config
const DATA_DIR = process.env.CLAUDE_CONFIG_DIR
  ? path.join(process.env.CLAUDE_CONFIG_DIR, 'usage-monitor')
  : path.join(os.homedir(), '.claude-usage-monitor')
// mutable: with several accounts configured, each one keeps its token in its
// own dir and the widget switches between them at runtime
let tokenPath = path.join(DATA_DIR, 'auth.json')

const b64url = (buf) =>
  buf.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')

let tokens = null // { access_token, refresh_token, expires_at }
let pending = null // { verifier, state }
let profile = null // { email, name, plan } — cached account identity

// point the module at another account's dir — the cached token and profile
// belong to the previous one, so both are dropped
function setDataDir(dir) {
  tokenPath = path.join(dir || DATA_DIR, 'auth.json')
  tokens = null
  profile = null
  pending = null
}

function load() {
  if (tokens) return tokens
  try {
    tokens = JSON.parse(fs.readFileSync(tokenPath, 'utf8'))
  } catch {
    tokens = null
  }
  return tokens
}
function save(t) {
  tokens = t
  try {
    fs.mkdirSync(path.dirname(tokenPath), { recursive: true })
    fs.writeFileSync(tokenPath, JSON.stringify(t, null, 2), { mode: 0o600 })
  } catch {}
}
function clear() {
  tokens = null
  profile = null
  try {
    fs.unlinkSync(tokenPath)
  } catch {}
}
function isConnected() {
  return !!load()
}

// Step 1: build the authorize URL (opens in the browser)
function begin() {
  const verifier = b64url(crypto.randomBytes(32))
  const challenge = b64url(crypto.createHash('sha256').update(verifier).digest())
  const state = b64url(crypto.randomBytes(32))
  pending = { verifier, state }
  const params = {
    code: 'true',
    client_id: CLIENT_ID,
    response_type: 'code',
    redirect_uri: REDIRECT,
    scope: SCOPE,
    code_challenge: challenge,
    code_challenge_method: 'S256',
    state,
  }
  const query = Object.entries(params)
    .map(([k, v]) => `${k}=${encodeURIComponent(v)}`)
    .join('&')
  return `${AUTHORIZE}?${query}`
}

// Step 2: exchange the pasted "code#state" for tokens
async function complete(pasted) {
  const raw = String(pasted).trim()
  // A directly-pasted long-lived token (e.g. from `claude setup-token`) skips
  // the rate-limited code exchange entirely.
  if (!raw.includes('#')) {
    save({ access_token: raw, refresh_token: null, expires_at: Date.now() + 365 * 864e5 })
    pending = null
    return
  }
  if (!pending) throw new Error('no pending auth')
  const [code, returnedState] = raw.split('#')
  const res = await fetch(TOKEN_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'User-Agent': UA },
    body: JSON.stringify({
      grant_type: 'authorization_code',
      code,
      state: returnedState || pending.state,
      code_verifier: pending.verifier,
      client_id: CLIENT_ID,
      redirect_uri: REDIRECT,
    }),
  })
  if (!res.ok) {
    const body = await res.text().catch(() => '')
    throw new Error(`exchange ${res.status}: ${body.slice(0, 150)}`)
  }
  const j = await res.json()
  save({
    access_token: j.access_token,
    refresh_token: j.refresh_token,
    expires_at: Date.now() + (j.expires_in || 3600) * 1000,
  })
  pending = null
}

async function refresh() {
  const t = load()
  if (!t?.refresh_token) throw Object.assign(new Error('no refresh token'), { status: 401 })
  const res = await fetch(TOKEN_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'User-Agent': UA },
    body: JSON.stringify({
      grant_type: 'refresh_token',
      refresh_token: t.refresh_token,
      client_id: CLIENT_ID,
    }),
  })
  if (!res.ok) {
    // a rejected grant (expired/revoked refresh token) means the session is
    // dead — surface it as 401 so callers clear the token and prompt login.
    // transient failures (429, 5xx) keep their status so we retry, not logout.
    const dead = res.status === 400 || res.status === 403
    throw Object.assign(new Error('refresh failed'), { status: dead ? 401 : res.status })
  }
  const j = await res.json()
  save({
    access_token: j.access_token,
    refresh_token: j.refresh_token || t.refresh_token,
    expires_at: Date.now() + (j.expires_in || 3600) * 1000,
  })
}

async function validToken() {
  const t = load()
  if (!t) throw Object.assign(new Error('not connected'), { status: 401 })
  if (!t.expires_at || t.expires_at - Date.now() < 60000) await refresh()
  return load().access_token
}

function win(o) {
  return o && typeof o.utilization === 'number'
    ? { pct: o.utilization, resetMs: o.resets_at ? Date.parse(o.resets_at) - Date.now() : null }
    : null
}

// Weekly limits scoped to one model (e.g. "Fable" on Max) live in the `limits`
// array. The older seven_day_<model> windows are kept as a fallback for plans
// that still report them that way.
function scopedWeeks(j) {
  const out = []
  for (const l of Array.isArray(j.limits) ? j.limits : []) {
    if (l?.kind !== 'weekly_scoped' || typeof l.percent !== 'number') continue
    const label = l.scope?.model?.display_name
    if (!label) continue
    out.push({
      label,
      pct: l.percent,
      resetMs: l.resets_at ? Date.parse(l.resets_at) - Date.now() : null,
    })
  }
  if (out.length) return out
  for (const [key, label] of [
    ['seven_day_opus', 'Opus'],
    ['seven_day_sonnet', 'Sonnet'],
  ]) {
    const w = win(j[key])
    if (w) out.push({ label, ...w })
  }
  return out
}

// Step 3: fetch the authoritative usage
async function fetchUsage() {
  const token = await validToken()
  const res = await fetch(USAGE_URL, {
    headers: {
      Authorization: `Bearer ${token}`,
      'anthropic-beta': 'oauth-2025-04-20',
      'anthropic-version': '2023-06-01',
      'User-Agent': UA,
    },
  })
  if (!res.ok) {
    const body = await res.text().catch(() => '')
    throw Object.assign(new Error(`usage ${res.status}: ${body.slice(0, 150)}`), {
      status: res.status,
    })
  }
  const j = await res.json()
  return {
    session: win(j.five_hour) || { pct: 0, resetMs: null },
    week: win(j.seven_day) || { pct: 0, resetMs: null },
    scoped: scopedWeeks(j),
  }
}

// The logged-in account's identity (email + plan tier). Cached for the session;
// only changes on logout/login, so we don't re-fetch on every usage poll.
async function fetchProfile() {
  if (profile) return profile
  const token = await validToken()
  const res = await fetch(PROFILE_URL, {
    headers: {
      Authorization: `Bearer ${token}`,
      'anthropic-beta': 'oauth-2025-04-20',
      'anthropic-version': '2023-06-01',
      'User-Agent': UA,
    },
  })
  if (!res.ok) {
    throw Object.assign(new Error(`profile ${res.status}`), { status: res.status })
  }
  const a = (await res.json()).account || {}
  profile = {
    email: a.email || null,
    name: a.display_name || a.full_name || null,
    plan: a.has_claude_max ? 'Max' : a.has_claude_pro ? 'Pro' : null,
  }
  return profile
}

module.exports = { begin, complete, fetchUsage, fetchProfile, clear, isConnected, setDataDir }
