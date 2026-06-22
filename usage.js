const fs = require('node:fs')
const path = require('node:path')
const os = require('node:os')

const CLAUDE_DIR = process.env.CLAUDE_CONFIG_DIR || path.join(os.homedir(), '.claude')
const PROJECTS_DIR = path.join(CLAUDE_DIR, 'projects')

function labelFor(model) {
  if (!model) return 'desconhecido'
  const m = model.toLowerCase()
  const fam = m.includes('opus')
    ? 'Opus'
    : m.includes('sonnet')
      ? 'Sonnet'
      : m.includes('haiku')
        ? 'Haiku'
        : m.includes('fable')
          ? 'Fable'
          : m.includes('mythos')
            ? 'Mythos'
            : null
  const ver = m.match(/-(\d)-(\d+)/)
  if (fam && ver) return `${fam} ${ver[1]}.${ver[2]}`
  if (fam) return fam
  return model
}

function tokensOf(entry) {
  const u = entry.usage || {}
  return (
    (u.input_tokens || 0) +
    (u.output_tokens || 0) +
    (u.cache_read_input_tokens || 0) +
    (u.cache_creation_input_tokens || 0)
  )
}

const fileCache = new Map()

function walkJsonl(dir, out, cutoffMs) {
  let items
  try {
    items = fs.readdirSync(dir, { withFileTypes: true })
  } catch {
    return
  }
  for (const it of items) {
    const full = path.join(dir, it.name)
    if (it.isDirectory()) {
      walkJsonl(full, out, cutoffMs)
    } else if (it.isFile() && it.name.endsWith('.jsonl')) {
      let st
      try {
        st = fs.statSync(full)
      } catch {
        continue
      }
      if (st.mtimeMs < cutoffMs) continue
      out.push({ full, st })
    }
  }
}

function parseFile(full, st) {
  const cached = fileCache.get(full)
  if (cached && cached.mtimeMs === st.mtimeMs && cached.size === st.size) {
    return cached.entries
  }
  const entries = []
  let raw
  try {
    raw = fs.readFileSync(full, 'utf8')
  } catch {
    return entries
  }
  for (const line of raw.split('\n')) {
    if (!line) continue
    let obj
    try {
      obj = JSON.parse(line)
    } catch {
      continue
    }
    if (obj.type !== 'assistant' || !obj.message || !obj.message.usage) continue
    if (obj.message.model === '<synthetic>') continue // Claude Code internal messages
    const ts = Date.parse(obj.timestamp || obj.message?.timestamp || 0)
    if (!ts) continue
    entries.push({
      ts,
      model: obj.message.model,
      tokens: tokensOf({ usage: obj.message.usage }),
      key: `${obj.message.id || ''}:${obj.requestId || obj.uuid || ''}`,
    })
  }
  fileCache.set(full, { mtimeMs: st.mtimeMs, size: st.size, entries })
  return entries
}

const DAYS = 30
const SESSION_MS = 5 * 3600 * 1000

// Plan presets (token budgets ~= 100%). Calibrated for Max 5x from the official
// panel (5h ~24% at 152M tokens, weekly ~62% at 2.14B); Pro/Max20x scaled by the
// plan multiplier. ESTIMATES — Anthropic doesn't publish exact numbers.
const PLAN_BUDGETS = {
  pro: { session: 126e6, week: 690e6 },
  max5x: { session: 630e6, week: 3450e6 },
  max20x: { session: 2520e6, week: 13800e6 },
}

function getUsage(config) {
  const now = Date.now()
  const dayMs = 24 * 3600 * 1000
  const startOfToday = new Date()
  startOfToday.setHours(0, 0, 0, 0)
  const todayMs = startOfToday.getTime()
  let weekStart = now - 7 * dayMs
  let weekResetMs = null
  if (config.weeklyAnchorIso) {
    const anchor = Date.parse(config.weeklyAnchorIso)
    if (!Number.isNaN(anchor)) {
      const period = 7 * dayMs
      const lastReset = anchor + Math.floor((now - anchor) / period) * period
      weekStart = lastReset
      weekResetMs = lastReset + period - now
    }
  }
  const start30 = todayMs - (DAYS - 1) * dayMs
  const fiveMinAgo = now - 5 * 60000
  const recentCutoff = now - 12 * 3600 * 1000
  const scanCutoff = start30 - dayMs

  const files = []
  walkJsonl(PROJECTS_DIR, files, scanCutoff)

  let lastMtime = 0
  for (const f of files) lastMtime = Math.max(lastMtime, f.st.mtimeMs)

  const seen = new Set()
  let todayTokens = 0
  let weekTokens = 0
  let monthTokens = 0
  const byModel = new Map() // tokens per model, 7 days
  const days30 = new Array(DAYS).fill(0) // tokens per day
  const recent = [] // last 12h, to detect the 5h session
  let last5mTokens = 0

  for (const f of files) {
    const entries = parseFile(f.full, f.st)
    for (const e of entries) {
      if (e.ts < start30) continue
      if (e.key && e.key !== ':' && seen.has(e.key)) continue
      if (e.key && e.key !== ':') seen.add(e.key)
      const t = e.tokens
      monthTokens += t

      const dayIdx = Math.min(DAYS - 1, Math.max(0, Math.floor((e.ts - start30) / dayMs)))
      days30[dayIdx] += t

      if (e.ts >= weekStart) {
        weekTokens += t
        const lbl = labelFor(e.model)
        byModel.set(lbl, (byModel.get(lbl) || 0) + t)
      }
      if (e.ts >= todayMs) todayTokens += t
      if (e.ts >= recentCutoff) recent.push({ ts: e.ts, tokens: t })
      if (e.ts >= fiveMinAgo) last5mTokens += t
    }
  }

  // 5h session window (like /usage "Current session")
  recent.sort((a, b) => a.ts - b.ts)
  let sStart = null
  let sEnd = null
  for (const e of recent) {
    if (sStart === null || e.ts >= sEnd) {
      sStart = e.ts
      sEnd = sStart + SESSION_MS
    }
  }
  const planB = PLAN_BUDGETS[config.plan]
  const sessionBudget = planB ? planB.session : config.sessionTokenBudget
  const weeklyBudget = planB ? planB.week : config.weeklyTokenBudget

  const session = { tokens: 0, pct: 0, resetMs: 0, active: false }
  if (sStart !== null && now < sEnd) {
    let tk = 0
    for (const e of recent) if (e.ts >= sStart && e.ts < sEnd) tk += e.tokens
    session.tokens = tk
    session.active = true
    session.resetMs = sEnd - now
    session.pct = sessionBudget ? Math.min(100, (tk / sessionBudget) * 100) : 0
  }

  const weekPct = weeklyBudget ? Math.min(100, (weekTokens / weeklyBudget) * 100) : 0

  const lastActivityMs = lastMtime ? now - lastMtime : Infinity
  const active = lastActivityMs <= (config.activeThresholdMs || 8000)
  const sleeping = lastActivityMs >= (config.sleepThresholdMs || 300000)

  const byModelArr = [...byModel.entries()]
    .map(([label, tokens]) => ({ label, tokens }))
    .sort((a, b) => b.tokens - a.tokens)

  return {
    session,
    week: { tokens: weekTokens, pct: weekPct, resetMs: weekResetMs },
    today: { tokens: todayTokens },
    byModel: byModelArr,
    days30,
    monthTokens,
    tokensPerMin: Math.round(last5mTokens / 5),
    active,
    sleeping,
    lastActivityMs,
    ts: now,
  }
}

module.exports = { getUsage }
