// Multiple Claude subscriptions in one widget.
//
// An account is a token plus, optionally, the Claude config dir whose logs it
// reads. The first one ("default") keeps the paths the app has always used, so
// an existing install carries over untouched; every extra account gets its own
// folder under `accounts/` for auth.json and alerts.json.
//
// Only the active account is polled — and therefore only it can notify.
const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')

const BASE_DIR = process.env.CLAUDE_CONFIG_DIR
  ? path.join(process.env.CLAUDE_CONFIG_DIR, 'usage-monitor')
  : path.join(os.homedir(), '.claude-usage-monitor')
const FILE = path.join(BASE_DIR, 'accounts.json')
const DEFAULT_ID = 'default'

function defaults() {
  return { active: DEFAULT_ID, accounts: [{ id: DEFAULT_ID, label: null, claudeDir: null }] }
}

function sane(j) {
  const list = (Array.isArray(j?.accounts) ? j.accounts : [])
    .filter((a) => a && typeof a.id === 'string' && a.id)
    .map((a) => ({
      id: a.id,
      label: typeof a.label === 'string' && a.label ? a.label : null,
      claudeDir: typeof a.claudeDir === 'string' && a.claudeDir ? a.claudeDir : null,
    }))
  // the default account can be removed like any other, so it is only recreated
  // when nothing is left — an empty list would leave the widget with no token
  if (!list.length) list.push(defaults().accounts[0])
  const active = list.some((a) => a.id === j?.active) ? j.active : list[0].id
  return { active, accounts: list }
}

// re-read on every call instead of caching: the file is a handful of rows, it
// is read only when the account list is shown or changed, and a stale cache
// here would mean the widget polling a token it thinks it still has
function load() {
  try {
    return sane(JSON.parse(fs.readFileSync(FILE, 'utf8')))
  } catch {
    return defaults() // first run, or a corrupted file
  }
}

function save(state) {
  try {
    fs.mkdirSync(BASE_DIR, { recursive: true })
    fs.writeFileSync(FILE, JSON.stringify(state, null, 2))
  } catch {}
}

// where an account's own files (auth.json, alerts.json) live
function dataDirOf(id) {
  return id === DEFAULT_ID ? BASE_DIR : path.join(BASE_DIR, 'accounts', id)
}

function list() {
  return load().accounts.map((a) => ({ ...a }))
}

function activeId() {
  return load().active
}

function active() {
  const s = load()
  return s.accounts.find((a) => a.id === s.active) || s.accounts[0]
}

function find(s, id) {
  return s.accounts.find((a) => a.id === id)
}

function setActive(id) {
  const s = load()
  const hit = find(s, id)
  if (!hit) return null
  s.active = id
  save(s)
  return hit
}

// a fresh, logged-out account — the caller switches to it and runs the usual
// browser login, which is what gives it an identity
function add(claudeDir) {
  const s = load()
  // two clicks inside the same millisecond used to mint the same id, and the
  // duplicate then looked like the account we were already on
  let id = `acct-${Date.now().toString(36)}`
  while (s.accounts.some((a) => a.id === id)) {
    id = `acct-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`
  }
  s.accounts.push({ id, label: null, claudeDir: claudeDir || null })
  save(s)
  return id
}

// the login flow knows the email; that's a better name than "Account 2"
function label(id, text) {
  const s = load()
  const a = find(s, id)
  if (!a || a.label === (text || null)) return
  a.label = text || null
  save(s)
}

function setClaudeDir(id, dir) {
  const s = load()
  const a = find(s, id)
  if (!a) return
  a.claudeDir = dir || null
  save(s)
}

// removing an account takes its token with it — leaving a stray auth.json
// behind would be a credential nobody can see or revoke from the UI
function remove(id) {
  const s = load()
  if (id === s.active) return false // switch away first: the widget is showing it
  const i = s.accounts.findIndex((a) => a.id === id)
  if (i < 0) return false
  if (s.accounts.length < 2) return false // never leave the widget with no account
  s.accounts.splice(i, 1)
  save(s)
  try {
    if (id === DEFAULT_ID) {
      // its folder is the app's own data dir: drop the credentials, not the
      // settings, the account list or the simulator file that live beside them
      for (const f of ['auth.json', 'alerts.json'])
        fs.rmSync(path.join(dataDirOf(id), f), { force: true })
    } else {
      fs.rmSync(dataDirOf(id), { recursive: true, force: true })
    }
  } catch {}
  return true
}

// Folders left behind under `accounts/` by slots that are no longer listed —
// an add abandoned mid-login, mostly, since saving the alert state recreates
// the folder on its way out. Only ever drops a folder with no auth.json in it,
// so a token is never destroyed by a bad read of accounts.json.
function pruneOrphans() {
  const root = path.join(BASE_DIR, 'accounts')
  let dirs = []
  try {
    dirs = fs.readdirSync(root, { withFileTypes: true })
  } catch {
    return 0 // no extra accounts were ever added
  }
  const known = new Set(list().map((a) => a.id))
  let n = 0
  for (const d of dirs) {
    if (!d.isDirectory() || known.has(d.name)) continue
    const dir = path.join(root, d.name)
    if (fs.existsSync(path.join(dir, 'auth.json'))) continue
    try {
      fs.rmSync(dir, { recursive: true, force: true })
      n++
    } catch {}
  }
  return n
}

module.exports = {
  BASE_DIR,
  DEFAULT_ID,
  dataDirOf,
  list,
  active,
  activeId,
  setActive,
  add,
  label,
  setClaudeDir,
  remove,
  pruneOrphans,
}
