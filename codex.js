const { spawn } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')

// Codex (CLI and the desktop app alike) writes one `rollout-*.jsonl` per
// session under ~/.codex/sessions/YYYY/MM/DD. Its `token_count` events carry
// both the token totals and the account's rate limits, so the tokens and history
// come from local files. Other ChatGPT products share those limits without
// writing rollouts, so the % also comes from Codex's own app-server.
let codexDir = process.env.CODEX_HOME || path.join(os.homedir(), '.codex')

function setCodexDir(dir) {
  codexDir = dir || process.env.CODEX_HOME || path.join(os.homedir(), '.codex')
  fileCache.clear()
  liveLimits = null
  lastLiveTry = 0
  lastLiveFailed = false
}

// tests point this at a stub; false turns the live read off
function setCodexBin(bin) {
  codexBin = bin
}

const DAY = 86400000
const DAYS = 30
const WEEK = 7 * DAY
const ACTIVE_MS = 60000 // a rollout touched within the last minute = Codex working
const DAY_MIN = 24 * 60
const TOP_PROJECTS = 5 // same fold as Claude's panel
const LIVE_MS = 2 * 60000 // how often to ask the app-server for the limits
const LIVE_RETRY_MS = 5 * 60000 // a process spawn per try: back off when it fails
const LIVE_TIMEOUT_MS = 15000

let codexBin
let liveLimits = null
let lastLiveTry = 0
let lastLiveFailed = false

// per file: how far we've read, plus what we got out of it. Rollouts only ever
// grow, so a poll reads just the bytes appended since the last one.
const fileCache = new Map()

function blankSummary() {
  return { offset: 0, rest: '', lastTotal: 0, events: [], limits: null, model: null, cwd: null }
}

// a token_count's rate limits. Codex reports several buckets; some (the
// `premium` one on Plus, say) come with null windows — only a bucket with a
// real window is worth showing.
function limitsOf(rl, ts) {
  if (!rl || (!rl.primary && !rl.secondary)) return null
  const win = (w) =>
    w && typeof w.used_percent === 'number'
      ? { pct: w.used_percent, resetsAt: w.resets_at ? w.resets_at * 1000 : null }
      : null
  // Plus reports a 5-hour window and a weekly one; some plans (Pro Lite, say)
  // only the weekly one, in the `primary` slot. Sort by length, not by slot.
  let session = null
  let weekly = null
  for (const w of [rl.primary, rl.secondary]) {
    const v = win(w)
    if (!v) continue
    const long = w.window_minutes != null ? w.window_minutes >= DAY_MIN : w === rl.secondary
    if (long) weekly = v
    else session = v
  }
  return { ts, plan: rl.plan_type || null, session, weekly }
}

// the app-server reports the same windows as the rollouts, in camelCase
function limitsFromLive(result, ts) {
  const rl = result?.rateLimitsByLimitId?.codex || result?.rateLimits
  if (!rl) return null
  const win = (w) =>
    w && {
      used_percent: w.usedPercent,
      window_minutes: w.windowDurationMins,
      resets_at: w.resetsAt,
    }
  return limitsOf(
    { primary: win(rl.primary), secondary: win(rl.secondary), plan_type: rl.planType },
    ts,
  )
}

// the desktop app's bundled CLI first: a GUI app doesn't get the shell's PATH
function findCodexBin() {
  if (codexBin !== undefined) return codexBin
  const home = os.homedir()
  const bundled = ['/Applications', path.join(home, 'Applications')].flatMap((dir) => [
    path.join(dir, 'ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex'),
    path.join(dir, 'Codex.app/Contents/Resources/codex'),
  ])
  return bundled.find((f) => fs.existsSync(f)) || 'codex'
}

// One `account/rateLimits/read` over the app-server's stdio JSON-RPC. Codex keeps
// its own login and token refresh: Clauddy never touches the token.
function readLiveLimits(bin, ts) {
  return new Promise((resolve) => {
    let child
    try {
      child = spawn(bin, ['app-server'], {
        env: { ...process.env, CODEX_HOME: codexDir },
        stdio: ['pipe', 'pipe', 'ignore'],
      })
    } catch {
      resolve(null)
      return
    }
    const finish = (limits) => {
      clearTimeout(timer)
      child.kill()
      resolve(limits)
    }
    const timer = setTimeout(() => finish(null), LIVE_TIMEOUT_MS)
    const send = (msg) => child.stdin.write(`${JSON.stringify(msg)}\n`)
    child.on('error', () => finish(null))
    child.on('exit', () => finish(null))
    child.stdin.on('error', () => finish(null))
    let rest = ''
    child.stdout.on('data', (chunk) => {
      const lines = (rest + chunk).split('\n')
      rest = lines.pop()
      for (const line of lines) {
        let msg
        try {
          msg = JSON.parse(line)
        } catch {
          continue
        }
        if (msg.id === 1) {
          send({ method: 'initialized' })
          send({ id: 2, method: 'account/rateLimits/read' })
        } else if (msg.id === 2) finish(limitsFromLive(msg.result, ts))
      }
    })
    send({
      id: 1,
      method: 'initialize',
      params: { clientInfo: { name: 'clauddy', version: '0' } },
    })
  })
}

// A failed read keeps the last good one: it ages into stale, never into zero.
async function refreshLimits(now = Date.now()) {
  const bin = findCodexBin()
  if (!bin) return
  lastLiveTry = now
  const dir = codexDir
  const limits = await readLiveLimits(bin, now)
  if (dir !== codexDir) return // the source changed while this one was running
  lastLiveFailed = !limits
  if (limits) liveLimits = limits
}

function ingestLine(sum, line) {
  // cheap filter before JSON.parse — rollouts are mostly message bodies
  if (!line.includes('"token_count"') && !line.includes('"turn_context"')) return
  let o
  try {
    o = JSON.parse(line)
  } catch {
    return
  }
  const p = o.payload || {}
  const ts = Date.parse(o.timestamp) || 0
  if (o.type === 'turn_context') {
    if (p.model) sum.model = p.model
    if (p.cwd) sum.cwd = p.cwd
    return
  }
  if (p.type !== 'token_count') return
  // totals are cumulative per session, and the same count is often emitted
  // twice — the delta is what this event actually spent. The model and project
  // are the ones in effect *now*, so a mid-session switch is attributed right.
  const total = p.info?.total_token_usage?.total_tokens
  if (typeof total === 'number' && total > sum.lastTotal) {
    sum.events.push({ ts, tokens: total - sum.lastTotal, model: sum.model, cwd: sum.cwd })
    sum.lastTotal = total
  }
  const lim = limitsOf(p.rate_limits, ts)
  if (lim && (!sum.limits || lim.ts >= sum.limits.ts)) sum.limits = lim
}

function readFile(file, st) {
  let sum = fileCache.get(file)
  // shrunk or replaced: start over
  if (!sum || st.size < sum.offset) sum = blankSummary()
  if (st.size > sum.offset) {
    const fd = fs.openSync(file, 'r')
    try {
      const buf = Buffer.alloc(st.size - sum.offset)
      fs.readSync(fd, buf, 0, buf.length, sum.offset)
      sum.offset = st.size
      const lines = (sum.rest + buf.toString('utf8')).split('\n')
      // the last piece may be a line still being written
      sum.rest = lines.pop()
      for (const line of lines) ingestLine(sum, line)
    } finally {
      fs.closeSync(fd)
    }
  }
  sum.mtime = st.mtimeMs
  fileCache.set(file, sum)
  return sum
}

// every rollout, with its stat; `since` drops the ones untouched since then
function rollouts(since) {
  const out = []
  const walk = (dir, depth) => {
    let kids
    try {
      kids = fs.readdirSync(dir, { withFileTypes: true })
    } catch {
      return
    }
    for (const k of kids) {
      const full = path.join(dir, k.name)
      if (k.isDirectory() && depth < 3) walk(full, depth + 1)
      else if (k.isFile() && k.name.endsWith('.jsonl')) {
        try {
          const st = fs.statSync(full)
          if (st.mtimeMs >= since) out.push({ file: full, st })
        } catch {}
      }
    }
  }
  walk(path.join(codexDir, 'sessions'), 0)
  return out
}

// Is there a Codex session on this machine? The folder alone is not enough.
function detectCodex() {
  const files = rollouts(0)
  if (!files.length) return { found: false, plan: null }
  files.sort((a, b) => b.st.mtimeMs - a.st.mtimeMs)
  let plan = null
  for (const { file, st } of files.slice(0, 5)) {
    const lim = readFile(file, st).limits
    if (lim?.plan) {
      plan = lim.plan
      break
    }
  }
  return { found: true, plan }
}

// A window whose reset has passed tells us nothing about today: the log is
// older than the window, so the % is unknown until Codex writes a fresh one.
function windowNow(w, now) {
  if (!w) return null
  const expired = w.resetsAt != null && w.resetsAt <= now
  return {
    pct: expired ? null : w.pct,
    resetMs: w.resetsAt != null && !expired ? w.resetsAt - now : null,
    expired,
  }
}

function ranked(map) {
  return [...map.entries()]
    .map(([label, tokens]) => ({ label, tokens }))
    .sort((a, b) => b.tokens - a.tokens)
}

function getCodexUsage(now = Date.now()) {
  if (!fs.existsSync(path.join(codexDir, 'sessions'))) return null
  if (now - lastLiveTry >= (lastLiveFailed ? LIVE_RETRY_MS : LIVE_MS)) refreshLimits(now)
  const startOfDay = new Date(now).setHours(0, 0, 0, 0)
  const start30 = startOfDay - (DAYS - 1) * DAY
  const files = rollouts(start30)
  const seen = new Set()
  let limits = null
  let latest = null
  let tokensToday = 0
  let tokens5h = 0
  let tokensWeek = 0
  let monthTokens = 0
  const days30 = new Array(DAYS).fill(0)
  const byModel = new Map()
  const byProject = new Map()
  for (const { file, st } of files) {
    seen.add(file)
    const sum = readFile(file, st)
    if (sum.limits && (!limits || sum.limits.ts > limits.ts)) limits = sum.limits
    if (!latest || sum.mtime > latest.mtime) latest = sum
    for (const e of sum.events) {
      if (e.ts < start30) continue
      monthTokens += e.tokens
      days30[Math.min(DAYS - 1, Math.floor((e.ts - start30) / DAY))] += e.tokens
      if (e.ts >= startOfDay) tokensToday += e.tokens
      if (now - e.ts < 5 * 3600000) tokens5h += e.tokens
      if (now - e.ts < WEEK) {
        tokensWeek += e.tokens
        const m = e.model || 'unknown'
        byModel.set(m, (byModel.get(m) || 0) + e.tokens)
        if (e.cwd) {
          const p = path.basename(e.cwd)
          byProject.set(p, (byProject.get(p) || 0) + e.tokens)
        }
      }
    }
  }
  // whichever is newer: a rollout written after the last live read beats it
  if (liveLimits && (!limits || liveLimits.ts >= limits.ts)) limits = liveLimits
  // forget files that aged out of the window
  for (const f of fileCache.keys()) if (!seen.has(f)) fileCache.delete(f)

  const projects = ranked(byProject)
  const byProjectArr = projects.slice(0, TOP_PROJECTS)
  const rest = projects.slice(TOP_PROJECTS)
  if (rest.length) {
    byProjectArr.push({
      label: `other · ${rest.length}`,
      tokens: rest.reduce((n, p) => n + p.tokens, 0),
    })
  }

  const weeklyOnly = !!limits?.weekly && !limits.session
  return {
    active: !!latest && now - latest.mtime < ACTIVE_MS,
    lastSeen: latest ? latest.mtime : null,
    plan: limits?.plan || null,
    // with no 5-hour window, the week is the budget that binds: it takes the
    // session's place, the way Cursor's month does
    session: limits ? windowNow(weeklyOnly ? limits.weekly : limits.session, now) : null,
    weekly: limits && !weeklyOnly ? windowNow(limits.weekly, now) : null,
    weeklyOnly,
    limitsAt: limits?.ts || null,
    tokensToday,
    tokens5h,
    tokensWeek,
    byModel: ranked(byModel),
    byProject: byProjectArr,
    days30,
    monthTokens,
  }
}

module.exports = { getCodexUsage, refreshLimits, detectCodex, setCodexDir, setCodexBin }
