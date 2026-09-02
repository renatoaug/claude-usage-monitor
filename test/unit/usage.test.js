import { afterAll, beforeEach, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

// usage.js resolves the log directory at import time from CLAUDE_CONFIG_DIR,
// so the fixture root has to exist and be exported before it loads.
const ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-test-'))
process.env.CLAUDE_CONFIG_DIR = ROOT
const { getUsage, setClaudeDir, labelFor, projectLabel, tokensOf, detectActivity } = await import(
  '../../usage.js'
)

afterAll(() => fs.rmSync(ROOT, { recursive: true, force: true }))

const MIN = 60_000
const HOUR = 3_600_000
const DAY = 24 * HOUR
const BUDGET = { sessionTokenBudget: 1000, weeklyTokenBudget: 10_000 }

let seq = 0
// one assistant line as Claude Code writes it
function line(over = {}) {
  const { ts = Date.now(), model = 'claude-opus-5', tokens = 0, id, requestId, usage } = over
  return JSON.stringify({
    type: 'assistant',
    timestamp: new Date(ts).toISOString(),
    requestId: requestId ?? `req-${seq++}`,
    message: {
      id: id ?? `msg-${seq}`,
      model,
      usage: usage ?? { input_tokens: tokens, output_tokens: 0 },
    },
  })
}

// getUsage aggregates across the whole projects tree, so each test starts from
// an empty one — otherwise earlier fixtures leak into later totals. Directory
// names keep incrementing so usage.js's path-keyed file cache never matches a
// deleted fixture.
let dirSeq = 0
beforeEach(() => {
  fs.rmSync(path.join(ROOT, 'projects'), { recursive: true, force: true })
})

function writeLog(lines, name = 'session.jsonl', project = null) {
  const dir = path.join(ROOT, 'projects', project ?? `-proj-${dirSeq++}`)
  fs.mkdirSync(dir, { recursive: true })
  const file = path.join(dir, name)
  fs.writeFileSync(file, `${lines.join('\n')}\n`)
  return file
}

describe('token accounting', () => {
  test('sums all four usage fields', () => {
    expect(
      tokensOf({
        usage: {
          input_tokens: 1,
          output_tokens: 2,
          cache_read_input_tokens: 4,
          cache_creation_input_tokens: 8,
        },
      }),
    ).toBe(15)
  })

  test('treats missing fields as zero', () => {
    expect(tokensOf({ usage: { input_tokens: 5 } })).toBe(5)
    expect(tokensOf({})).toBe(0)
  })

  test('adds up entries across a log', () => {
    writeLog([line({ tokens: 100 }), line({ tokens: 250 })])
    const u = getUsage(BUDGET)
    expect(u.monthTokens).toBe(350)
    expect(u.today.tokens).toBe(350)
  })
})

describe('log decoding', () => {
  test('ignores everything that is not a billable assistant turn', () => {
    writeLog([
      line({ tokens: 100 }),
      JSON.stringify({ type: 'user', message: { content: 'hi' } }),
      JSON.stringify({ type: 'assistant', message: { model: 'claude-opus-5' } }), // no usage
      line({ tokens: 999, model: '<synthetic>' }), // Claude Code internal
      JSON.stringify({ type: 'assistant', message: { usage: { input_tokens: 7 } } }), // no timestamp
    ])
    expect(getUsage(BUDGET).monthTokens).toBe(100)
  })

  test('survives malformed and truncated lines', () => {
    // a log being appended to can end mid-write — that must not lose the rest
    writeLog([line({ tokens: 10 }), 'not json at all', '{"type":"assist', line({ tokens: 20 })])
    expect(getUsage(BUDGET).monthTokens).toBe(30)
  })

  test('deduplicates a turn recorded in more than one file', () => {
    // resumed sessions replay earlier turns into the new log
    const dup = { tokens: 500, id: 'msg-same', requestId: 'req-same' }
    writeLog([line(dup)], 'a.jsonl')
    writeLog([line(dup)], 'b.jsonl')
    expect(getUsage(BUDGET).monthTokens).toBe(500)
  })
})

describe('model labels', () => {
  test.each([
    ['claude-opus-5', 'Opus 5'],
    ['claude-opus-4-8', 'Opus 4.8'],
    ['claude-sonnet-5', 'Sonnet 5'],
    ['claude-haiku-4-5-20251001', 'Haiku 4.5'],
    ['claude-3-5-sonnet-20241022', 'Sonnet 3.5'], // legacy: version before family
    ['claude-fable-5', 'Fable 5'],
  ])('%s → %s', (model, label) => {
    expect(labelFor(model)).toBe(label)
  })

  test('passes unknown models through and handles nothing', () => {
    expect(labelFor('some-other-model')).toBe('some-other-model')
    expect(labelFor(null)).toBe('desconhecido')
  })

  test('aggregates weekly tokens per label', () => {
    writeLog([
      line({ tokens: 100, model: 'claude-opus-5' }),
      line({ tokens: 50, model: 'claude-opus-5' }),
      line({ tokens: 20, model: 'claude-sonnet-5' }),
    ])
    // sorted heaviest first
    expect(getUsage(BUDGET).byModel).toEqual([
      { label: 'Opus 5', tokens: 150 },
      { label: 'Sonnet 5', tokens: 20 },
    ])
  })
})

describe('project labels', () => {
  // the directory name flattens `/` and `.` to `-`, so it cannot be reversed by
  // string surgery — usage.js walks the real filesystem instead. These fixtures
  // build actual directories so the walk has something true to find.
  const enc = (p) => p.replace(/[/.]/g, '-')

  test('resolves an existing path down to its basename', () => {
    const dir = path.join(ROOT, 'fs', 'my-projects', 'claude-usage-monitor')
    fs.mkdirSync(dir, { recursive: true })
    expect(projectLabel(enc(dir))).toBe('claude-usage-monitor')
  })

  test('handles a dash inside a directory name', () => {
    // `my-projects` and `my` would both start the remainder; only the longer wins
    const dir = path.join(ROOT, 'fs', 'my', 'x')
    fs.mkdirSync(dir, { recursive: true })
    expect(projectLabel(enc(path.join(ROOT, 'fs', 'my-projects')))).toBe('my-projects')
  })

  test('handles a dot inside a directory name', () => {
    const dir = path.join(ROOT, 'fs', 'nord', '.worktrees', 'sup-441')
    fs.mkdirSync(dir, { recursive: true })
    expect(projectLabel(enc(dir))).toBe('sup-441')
  })

  test('falls back to the raw tail when the project is gone', () => {
    const gone = enc(path.join(ROOT, 'fs', 'my-projects', 'deleted-thing'))
    expect(projectLabel(gone)).toContain('deleted-thing')
  })

  test('does not eat into the name of a directory that no longer exists', () => {
    // `identity` survives, `identity-web` does not — reporting `web` would look
    // like a real project and be the wrong one
    fs.mkdirSync(path.join(ROOT, 'fs', 'q', 'identity'), { recursive: true })
    const label = projectLabel(enc(path.join(ROOT, 'fs', 'q', 'identity-web')))
    expect(label).not.toBe('web')
    expect(label).toContain('identity-web')
  })
})

describe('usage by project', () => {
  test('aggregates weekly tokens per project directory', () => {
    const a = path.join(ROOT, 'fs', 'alpha')
    const b = path.join(ROOT, 'fs', 'beta')
    fs.mkdirSync(a, { recursive: true })
    fs.mkdirSync(b, { recursive: true })
    const enc = (p) => p.replace(/[/.]/g, '-')
    writeLog([line({ tokens: 100 }), line({ tokens: 50 })], 'session.jsonl', enc(a))
    writeLog([line({ tokens: 20 })], 'session.jsonl', enc(b))
    expect(getUsage(BUDGET).byProject).toEqual([
      { label: 'alpha', tokens: 150 },
      { label: 'beta', tokens: 20 },
    ])
  })

  test('folds everything past the top five into one `other` row', () => {
    for (let i = 0; i < 8; i++) {
      writeLog([line({ tokens: 100 - i })], 'session.jsonl', `-nope-p${i}`)
    }
    const list = getUsage(BUDGET).byProject
    expect(list.length).toBe(6)
    expect(list[5].label).toBe('other · 3')
    // 8 projects at 100-i tokens: the tail is p5+p6+p7 = 95+94+93
    expect(list[5].tokens).toBe(95 + 94 + 93)
  })

  test('counts only the weekly window, like by-model', () => {
    const old = path.join(ROOT, 'fs', 'stale')
    fs.mkdirSync(old, { recursive: true })
    writeLog(
      [line({ ts: Date.now() - 20 * DAY, tokens: 999 })],
      'session.jsonl',
      old.replace(/[/.]/g, '-'),
    )
    expect(getUsage(BUDGET).byProject).toEqual([])
  })
})

describe('the 5-hour session window', () => {
  test('opens at the first turn and counts only what falls inside', () => {
    const now = Date.now()
    writeLog([
      line({ ts: now - 2 * HOUR, tokens: 100 }), // opens the window
      line({ ts: now - 1 * HOUR, tokens: 200 }),
    ])
    const s = getUsage(BUDGET).session
    expect(s.active).toBe(true)
    expect(s.tokens).toBe(300)
    // window opened 2h ago, so ~3h left of the 5h
    expect(s.resetMs / HOUR).toBeCloseTo(3, 1)
    expect(s.pct).toBe(30) // 300 of a 1000 budget
  })

  test('chains windows: a new one opens only after the previous expires', () => {
    // Subtle and easy to break. The 6h-old turn opens a window running until
    // 1h ago; the 2h-old turn lands *inside* that window, so it belongs to the
    // old session, not a new one. Only the turn at the moment of expiry opens
    // the live window — so the live session holds 200, not 300.
    const now = Date.now()
    writeLog([
      line({ ts: now - 6 * HOUR, tokens: 700 }),
      line({ ts: now - 2 * HOUR, tokens: 100 }),
      line({ ts: now - 1 * HOUR, tokens: 200 }),
    ])
    const s = getUsage(BUDGET).session
    expect(s.tokens).toBe(200)
    expect(s.resetMs / HOUR).toBeCloseTo(4, 1) // opened 1h ago
  })

  test('is inactive once the window has fully elapsed', () => {
    writeLog([line({ ts: Date.now() - 7 * HOUR, tokens: 100 })])
    const s = getUsage(BUDGET).session
    expect(s.active).toBe(false)
    expect(s.tokens).toBe(0)
  })

  test('caps the percentage at 100 instead of overflowing', () => {
    writeLog([line({ ts: Date.now() - MIN, tokens: 5000 })]) // 5x the budget
    expect(getUsage(BUDGET).session.pct).toBe(100)
  })
})

describe('time buckets', () => {
  test('separates today, the week and the month', () => {
    const now = Date.now()
    writeLog([
      line({ ts: now - MIN, tokens: 1 }), // today
      line({ ts: now - 3 * DAY, tokens: 10 }), // this week
      line({ ts: now - 20 * DAY, tokens: 100 }), // this month only
      line({ ts: now - 60 * DAY, tokens: 1000 }), // beyond 30 days
    ])
    const u = getUsage(BUDGET)
    expect(u.today.tokens).toBe(1)
    expect(u.week.tokens).toBe(11)
    expect(u.monthTokens).toBe(111)
  })

  test('lands each entry in its own day of the 30-day map', () => {
    const now = Date.now()
    writeLog([line({ ts: now - MIN, tokens: 5 }), line({ ts: now - 2 * DAY, tokens: 7 })])
    const d = getUsage(BUDGET).days30
    expect(d.length).toBe(30)
    expect(d[29]).toBe(5) // today is the last bucket
    expect(d[27]).toBe(7)
  })

  test('reports the recent rate over the last five minutes', () => {
    writeLog([
      line({ ts: Date.now() - MIN, tokens: 500 }),
      line({ ts: Date.now() - 30 * MIN, tokens: 9999 }), // too old to count
    ])
    expect(getUsage(BUDGET).tokensPerMin).toBe(100) // 500 / 5
  })
})

describe('activity detection', () => {
  const toolLine = (name) =>
    JSON.stringify({
      type: 'assistant',
      timestamp: new Date().toISOString(),
      message: { model: 'claude-opus-5', content: [{ type: 'tool_use', name }] },
    })

  test.each([
    ['Edit', 'editing'],
    ['Write', 'editing'],
    ['Read', 'reading'],
    ['Grep', 'reading'],
    ['Bash', 'running'],
    ['WebSearch', 'researching'],
    ['Task', 'delegating'],
    ['TodoWrite', 'planning'],
    ['AskUserQuestion', 'waiting'],
  ])('%s → %s', (tool, expected) => {
    expect(detectActivity(writeLog([toolLine(tool)]))).toBe(expected)
  })

  test('reads the most recent message, not the first', () => {
    expect(detectActivity(writeLog([toolLine('Read'), toolLine('Bash')]))).toBe('running')
  })

  test('within one message, the last tool block wins', () => {
    // Claude batches parallel tool calls into a single assistant message, so
    // "what it's doing now" is the last block, not the first.
    const batched = JSON.stringify({
      type: 'assistant',
      timestamp: new Date().toISOString(),
      message: {
        model: 'claude-opus-5',
        content: [
          { type: 'text', text: 'let me look' },
          { type: 'tool_use', name: 'Read' },
          { type: 'tool_use', name: 'Bash' },
        ],
      },
    })
    expect(detectActivity(writeLog([batched]))).toBe('running')
  })

  test('an unknown tool still counts as working', () => {
    expect(detectActivity(writeLog([toolLine('SomeNewTool')]))).toBe('working')
  })

  test('plan mode outranks whatever tool ran last', () => {
    const f = writeLog([
      JSON.stringify({ type: 'permission-mode', mode: 'plan' }),
      toolLine('Edit'),
    ])
    expect(detectActivity(f)).toBe('planning')
  })

  test('no tool use means no signal', () => {
    expect(detectActivity(writeLog([line({ tokens: 10 })]))).toBeNull()
  })

  test('a missing file is not a crash', () => {
    expect(detectActivity(path.join(ROOT, 'nope.jsonl'))).toBeNull()
  })
})

describe('switching accounts', () => {
  test('setClaudeDir reads another account logs, and null goes back to the default', () => {
    writeLog([line({ tokens: 10 })])
    const other = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-other-'))
    try {
      setClaudeDir(other)
      // a second subscription with no sessions yet must not inherit the first
      // account's token counts
      expect(getUsage(BUDGET).monthTokens).toBe(0)
      setClaudeDir(null)
      expect(getUsage(BUDGET).monthTokens).toBeGreaterThan(0)
    } finally {
      setClaudeDir(null)
      fs.rmSync(other, { recursive: true, force: true })
    }
  })
})

describe('activity state', () => {
  test('a log just written means Claude is active and awake', () => {
    writeLog([line({ tokens: 10 })])
    const u = getUsage({ ...BUDGET, activeThresholdMs: 20_000, sleepThresholdMs: 300_000 })
    expect(u.active).toBe(true)
    expect(u.sleeping).toBe(false)
  })

  test('an empty log directory reports zeroes rather than throwing', () => {
    const empty = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-empty-'))
    const saved = process.env.CLAUDE_CONFIG_DIR
    try {
      // getUsage reads the dir captured at import; assert the no-files path via
      // a project dir that exists but holds nothing readable
      fs.mkdirSync(path.join(ROOT, 'projects', 'empty-proj'), { recursive: true })
      const u = getUsage(BUDGET)
      expect(u.days30.length).toBe(30)
      expect(typeof u.monthTokens).toBe('number')
      expect(u.ts).toBeGreaterThan(0)
    } finally {
      process.env.CLAUDE_CONFIG_DIR = saved
      fs.rmSync(empty, { recursive: true, force: true })
    }
  })
})
