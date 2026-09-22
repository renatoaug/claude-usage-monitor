const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')

// Codex (CLI and the desktop app alike) writes one `rollout-*.jsonl` per
// session under ~/.codex/sessions/YYYY/MM/DD. Its `token_count` events carry
// both the token totals and the account's rate limits, so everything here comes
// from local files — no login needed.
let codexDir = process.env.CODEX_HOME || path.join(os.homedir(), '.codex')

function setCodexDir(dir) {
  codexDir = dir || process.env.CODEX_HOME || path.join(os.homedir(), '.codex')
  fileCache.clear()
}

const DAY = 86400000
const DAYS = 30
const WEEK = 7 * DAY
const ACTIVE_MS = 60000 // a rollout touched within the last minute = Codex working
const TOP_PROJECTS = 5 // same fold as Claude's panel

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
  return { ts, plan: rl.plan_type || null, primary: win(rl.primary), secondary: win(rl.secondary) }
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

  return {
    active: !!latest && now - latest.mtime < ACTIVE_MS,
    lastSeen: latest ? latest.mtime : null,
    plan: limits?.plan || null,
    session: limits ? windowNow(limits.primary, now) : null,
    weekly: limits ? windowNow(limits.secondary, now) : null,
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

module.exports = { getCodexUsage, detectCodex, setCodexDir }
