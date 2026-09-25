const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')

// Cursor keeps no usage in local files. Its app stores a session token in
// `state.vscdb` (SQLite), and the same dashboard endpoints cursor.com uses
// answer with it — unofficial, so every field is read defensively. Activity
// comes from the agent transcripts under ~/.cursor/projects, which log each
// tool call much like Claude Code's own logs.
const API = 'https://api2.cursor.sh/aiserver.v1.DashboardService/'
const ACTIVE_MS = 60000 // a transcript that says nothing counts only while it's this fresh
// an unfinished turn this long without a write was abandoned (Cursor closed mid-run)
const OPEN_TURN_MS = 30 * 60000
const RECENT_TRANSCRIPTS = 8
const FETCH_MS = 5 * 60000 // a monthly budget moves slowly; don't hammer the API
const RETRY_MS = 60000
const TAIL_BYTES = 64 * 1024

function defaultStateDb() {
  const tail = ['Cursor', 'User', 'globalStorage', 'state.vscdb']
  if (process.platform === 'darwin')
    return path.join(os.homedir(), 'Library', 'Application Support', ...tail)
  if (process.platform === 'win32')
    return path.join(process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming'), ...tail)
  return path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), ...tail)
}

let homeDir = path.join(os.homedir(), '.cursor')
let stateDb = defaultStateDb()
let fetchImpl = (...args) => globalThis.fetch(...args)
let limits = null // last good reading: { ts, plan, endMs, total, api }
let lastTry = -Infinity
let lastFailed = false
let signedOut = false
let inflight = null

// tests point these elsewhere; a change of place forgets the old reading
function configureCursor(opts = {}) {
  if ('fetch' in opts) fetchImpl = opts.fetch || ((...args) => globalThis.fetch(...args))
  if (!('home' in opts) && !('stateDb' in opts)) return
  if ('home' in opts) homeDir = opts.home || path.join(os.homedir(), '.cursor')
  if ('stateDb' in opts) stateDb = opts.stateDb || defaultStateDb()
  limits = null
  lastTry = -Infinity
  lastFailed = false
  signedOut = false
  inflight = null
}

const text = (v) => (v == null ? null : typeof v === 'string' ? v : Buffer.from(v).toString('utf8'))

// Electron's Node has `node:sqlite`; Bun, which runs the tests, has its own
// with the same prepare/get/close
function openReadOnly(file) {
  try {
    const { DatabaseSync } = require('node:sqlite')
    return new DatabaseSync(file, { readOnly: true })
  } catch (err) {
    if (!process.versions.bun) throw err
    const { Database } = require('bun:sqlite')
    return new Database(file, { readonly: true })
  }
}

// the app's own session: its token, plus the plan it last cached
function readAuth() {
  if (!fs.existsSync(stateDb)) return null
  let db
  try {
    db = openReadOnly(stateDb)
    const q = db.prepare('SELECT value FROM ItemTable WHERE key = ?')
    const get = (k) => text(q.get(k)?.value)
    return { token: get('cursorAuth/accessToken'), plan: get('cursorAuth/stripeMembershipType') }
  } catch {
    return null
  } finally {
    db?.close()
  }
}

// Settings → Connect: only a signed-in app has something to read
function detectCursor() {
  const auth = readAuth()
  return { found: !!auth?.token, plan: auth?.plan || null }
}

async function post(method, token) {
  const res = await fetchImpl(API + method, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      'Connect-Protocol-Version': '1',
    },
    body: '{}',
    signal: AbortSignal.timeout(15000),
  })
  if (!res.ok) throw new Error(`cursor ${method}: HTTP ${res.status}`)
  return res.json()
}

const pctOf = (v) => (typeof v === 'number' && Number.isFinite(v) ? v : null)

// One fetch at a time. The token is read fresh each time: the Cursor app
// refreshes it on its own, so there is nothing to renew here.
function refreshLimits(now = Date.now()) {
  if (inflight) return inflight
  lastTry = now
  inflight = (async () => {
    const auth = readAuth()
    signedOut = !auth?.token
    if (signedOut) return
    const [usage, info] = await Promise.all([
      post('GetCurrentPeriodUsage', auth.token),
      post('GetPlanInfo', auth.token).catch(() => null),
    ])
    const pu = usage?.planUsage || {}
    limits = {
      ts: now,
      plan: info?.planInfo?.planName?.toLowerCase() || auth.plan || null,
      endMs: Number(usage?.billingCycleEnd) || null,
      total: pctOf(pu.totalPercentUsed),
      api: pctOf(pu.apiPercentUsed),
    }
  })()
    .then(() => {
      lastFailed = false
    })
    .catch((err) => {
      lastFailed = true
      console.error('cursor:', err.message)
    })
    .finally(() => {
      inflight = null
    })
  return inflight
}

// the billing cycle as a window; past its end, the % belongs to the old cycle
function cycleNow(pct, endMs, now) {
  const expired = endMs != null && endMs <= now
  return {
    pct: expired ? null : pct,
    resetMs: endMs != null && !expired ? endMs - now : null,
    expired,
  }
}

// ---- activity ---------------------------------------------------------------
const ACTIVITY = {
  Read: 'reading',
  ReadFile: 'reading',
  Grep: 'reading',
  Glob: 'reading',
  LS: 'reading',
  ListDir: 'reading',
  SemanticSearch: 'reading',
  ReadLints: 'reading',
  Write: 'editing',
  StrReplace: 'editing',
  Edit: 'editing',
  MultiEdit: 'editing',
  ApplyPatch: 'editing',
  Delete: 'editing',
  EditNotebook: 'editing',
  Shell: 'running',
  WebSearch: 'researching',
  WebFetch: 'researching',
  Task: 'delegating',
  TodoWrite: 'planning',
  CreatePlan: 'planning',
  AskQuestion: 'waiting',
}

// what the newest transcript lines say: a finished turn means nothing is
// running (null), an open one names its scene, and undefined means no telling
function activityOf(lines) {
  for (let i = lines.length - 1; i >= 0; i--) {
    let o
    try {
      o = JSON.parse(lines[i])
    } catch {
      continue
    }
    if (o.type === 'turn_ended') return null
    if (o.role === 'assistant') {
      const blocks = Array.isArray(o.message?.content) ? o.message.content : []
      const tool = blocks.findLast((b) => b?.type === 'tool_use')
      return (tool && ACTIVITY[tool.name]) || 'working'
    }
    if (o.role === 'user') return 'working' // just asked: the agent is thinking
  }
  return undefined
}

const dirs = (dir) => {
  try {
    return fs.readdirSync(dir, { withFileTypes: true }).filter((d) => !d.name.startsWith('.'))
  } catch {
    return []
  }
}

// every transcript, subagents included: projects/<p>/agent-transcripts/<id>/…
function transcripts() {
  const out = []
  for (const p of dirs(path.join(homeDir, 'projects'))) {
    if (!p.isDirectory()) continue
    const root = path.join(homeDir, 'projects', p.name, 'agent-transcripts')
    for (const id of dirs(root)) {
      if (!id.isDirectory()) continue
      const dir = path.join(root, id.name)
      for (const f of dirs(dir)) {
        if (f.isFile() && f.name.endsWith('.jsonl')) out.push(path.join(dir, f.name))
        else if (f.isDirectory() && f.name === 'subagents') {
          for (const s of dirs(path.join(dir, f.name)))
            if (s.isFile() && s.name.endsWith('.jsonl')) out.push(path.join(dir, f.name, s.name))
        }
      }
    }
  }
  return out
}

function tailLines(file, size) {
  const len = Math.min(size, TAIL_BYTES)
  const buf = Buffer.alloc(len)
  const fd = fs.openSync(file, 'r')
  try {
    fs.readSync(fd, buf, 0, len, size - len)
  } finally {
    fs.closeSync(fd)
  }
  const lines = buf.toString('utf8').split('\n')
  if (len < size) lines.shift() // cut mid-line
  return lines.filter(Boolean)
}

// A turn stays open while a long command runs or the model thinks, without a
// write, so an open turn in any recent transcript is Cursor at work. A
// transcript with nothing to say counts only while it's fresh.
function readActivity(now) {
  const recent = []
  let lastSeen = null
  for (const file of transcripts()) {
    try {
      const st = fs.statSync(file)
      if (lastSeen == null || st.mtimeMs > lastSeen) lastSeen = st.mtimeMs
      if (now - st.mtimeMs < OPEN_TURN_MS) recent.push({ file, st })
    } catch {}
  }
  recent.sort((a, b) => b.st.mtimeMs - a.st.mtimeMs)
  for (const { file, st } of recent.slice(0, RECENT_TRANSCRIPTS)) {
    let said
    try {
      said = activityOf(tailLines(file, st.size))
    } catch {}
    const activity = said === undefined ? (now - st.mtimeMs < ACTIVE_MS ? 'working' : null) : said
    if (activity) return { active: true, activity, lastSeen }
  }
  return { active: false, activity: null, lastSeen }
}

// The poll's view of Cursor: activity from disk every tick, limits from the
// last fetch (a new one starts in the background when it's due).
function getCursorUsage(now = Date.now()) {
  if (now - lastTry >= (limits && !lastFailed ? FETCH_MS : RETRY_MS)) refreshLimits(now)
  return {
    ...readActivity(now),
    plan: limits?.plan ?? null,
    session: limits ? cycleNow(limits.total, limits.endMs, now) : null,
    api: limits ? cycleNow(limits.api, limits.endMs, now) : null,
    limitsAt: limits?.ts ?? null,
    signedOut,
  }
}

module.exports = { getCursorUsage, refreshLimits, detectCursor, configureCursor, activityOf }
