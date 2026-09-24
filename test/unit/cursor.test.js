import { Database } from 'bun:sqlite'
import { afterAll, beforeEach, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const cursor = require('../../cursor.js')

const NOW = Date.parse('2026-09-24T12:00:00Z')
const END = NOW + 10 * 86400000
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'cursor-'))
const home = path.join(root, '.cursor')
const dbFile = path.join(root, 'state.vscdb')

function stateDb(values) {
  fs.rmSync(dbFile, { force: true })
  const db = new Database(dbFile)
  db.exec('CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)')
  const put = db.prepare('INSERT INTO ItemTable (key, value) VALUES (?, ?)')
  for (const [k, v] of Object.entries(values)) put.run(k, v)
  db.close()
}

// a fetch double: answers per endpoint, and remembers what it was asked
function api(answers) {
  const calls = []
  const fetch = async (url, init) => {
    calls.push({ url, init })
    const a = answers[url.split('/').pop()]
    if (a instanceof Error) throw a
    const status = typeof a === 'number' ? a : 200
    return { ok: status < 400, status, json: async () => a }
  }
  fetch.calls = calls
  return fetch
}
const USAGE = {
  billingCycleStart: String(NOW - 20 * 86400000),
  billingCycleEnd: String(END),
  planUsage: { totalPercentUsed: 42.5, autoPercentUsed: 10, apiPercentUsed: 61 },
}
const PLAN = { planInfo: { planName: 'Pro', billingCycleEnd: String(END) } }

function transcript(project, id, lines, mtime = NOW - 5000, sub = null) {
  const dir = path.join(
    home,
    'projects',
    project,
    'agent-transcripts',
    id,
    ...(sub ? ['subagents'] : []),
  )
  fs.mkdirSync(dir, { recursive: true })
  const f = path.join(dir, `${sub || id}.jsonl`)
  fs.writeFileSync(f, `${lines.map((l) => JSON.stringify(l)).join('\n')}\n`)
  fs.utimesSync(f, mtime / 1000, mtime / 1000)
  return f
}
const user = { role: 'user', message: { content: [{ type: 'text', text: 'hi' }] } }
const tool = (name) => ({
  role: 'assistant',
  message: {
    content: [
      { type: 'text', text: '…' },
      { type: 'tool_use', name, input: {} },
    ],
  },
})
const ended = { type: 'turn_ended', status: 'success' }

beforeEach(() => {
  fs.rmSync(home, { recursive: true, force: true })
  stateDb({ 'cursorAuth/accessToken': 'tok-1', 'cursorAuth/stripeMembershipType': 'free' })
  cursor.configureCursor({ home, stateDb: dbFile, fetch: api({}) })
  // a reading from the test before must not leak in
  cursor.configureCursor({ home, stateDb: dbFile })
})
afterAll(() => {
  fs.rmSync(root, { recursive: true, force: true })
  cursor.configureCursor({ home: null, stateDb: null, fetch: null })
})

describe('detectCursor', () => {
  test('finds a signed-in app, with its cached plan', () => {
    expect(cursor.detectCursor()).toEqual({ found: true, plan: 'free' })
  })
  test('an app without a token is not connectable', () => {
    stateDb({ 'cursorAuth/stripeMembershipType': 'free' })
    expect(cursor.detectCursor()).toEqual({ found: false, plan: 'free' })
  })
  test('no app, or an unreadable file, finds nothing', () => {
    cursor.configureCursor({ stateDb: path.join(root, 'missing.vscdb') })
    expect(cursor.detectCursor()).toEqual({ found: false, plan: null })
    fs.writeFileSync(path.join(root, 'junk.vscdb'), 'not sqlite')
    cursor.configureCursor({ stateDb: path.join(root, 'junk.vscdb') })
    expect(cursor.detectCursor()).toEqual({ found: false, plan: null })
  })
  test('a token stored as bytes reads the same', () => {
    stateDb({ 'cursorAuth/accessToken': Buffer.from('tok-b') })
    expect(cursor.detectCursor().found).toBe(true)
  })
})

describe('limits', () => {
  test('the billing cycle is the window, with the API pool beside it', async () => {
    const fetch = api({ GetCurrentPeriodUsage: USAGE, GetPlanInfo: PLAN })
    cursor.configureCursor({ fetch })
    await cursor.refreshLimits(NOW)
    const u = cursor.getCursorUsage(NOW + 1000)
    expect(u.plan).toBe('pro')
    expect(u.session).toEqual({ pct: 42.5, resetMs: END - NOW - 1000, expired: false })
    expect(u.api.pct).toBe(61)
    expect(u.limitsAt).toBe(NOW)
    expect(u.signedOut).toBe(false)
    const { init } = fetch.calls[0]
    expect(init.method).toBe('POST')
    expect(init.headers.Authorization).toBe('Bearer tok-1')
  })

  test('past the cycle end the old % is unknown, never zero', async () => {
    cursor.configureCursor({ fetch: api({ GetCurrentPeriodUsage: USAGE, GetPlanInfo: PLAN }) })
    await cursor.refreshLimits(NOW)
    const u = cursor.getCursorUsage(END + 1)
    expect(u.session).toEqual({ pct: null, resetMs: null, expired: true })
  })

  test('the plan falls back to the cached one, and odd fields read as unknown', async () => {
    const odd = { billingCycleEnd: 'soon', planUsage: { totalPercentUsed: 'lots' } }
    cursor.configureCursor({ fetch: api({ GetCurrentPeriodUsage: odd, GetPlanInfo: 500 }) })
    await cursor.refreshLimits(NOW)
    const u = cursor.getCursorUsage(NOW)
    expect(u.plan).toBe('free')
    expect(u.session).toEqual({ pct: null, resetMs: null, expired: false })
  })

  test('signed out: no fetch, and it says so', async () => {
    stateDb({})
    const fetch = api({ GetCurrentPeriodUsage: USAGE })
    cursor.configureCursor({ fetch })
    await cursor.refreshLimits(NOW)
    expect(fetch.calls.length).toBe(0)
    const u = cursor.getCursorUsage(NOW)
    expect(u.signedOut).toBe(true)
    expect(u.session).toBe(null)
  })

  test('a failed fetch keeps the last reading, and retries sooner', async () => {
    cursor.configureCursor({ fetch: api({ GetCurrentPeriodUsage: USAGE, GetPlanInfo: PLAN }) })
    await cursor.refreshLimits(NOW)
    const failing = api({ GetCurrentPeriodUsage: 401 })
    cursor.configureCursor({ fetch: failing })
    const err = console.error
    console.error = () => {}
    try {
      await cursor.refreshLimits(NOW + 5 * 60000)
    } finally {
      console.error = err
    }
    const u = cursor.getCursorUsage(NOW + 5 * 60000 + 1000)
    expect(u.session.pct).toBe(42.5)
    expect(u.limitsAt).toBe(NOW)
    const n = failing.calls.length
    cursor.getCursorUsage(NOW + 6 * 60000 + 1000)
    expect(failing.calls.length).toBeGreaterThan(n)
  })

  test('polls are throttled: one fetch per 5 minutes, one in flight', async () => {
    const fetch = api({ GetCurrentPeriodUsage: USAGE, GetPlanInfo: PLAN })
    cursor.configureCursor({ fetch })
    const a = cursor.refreshLimits(NOW)
    const b = cursor.refreshLimits(NOW)
    expect(a).toBe(b)
    await a
    const count = fetch.calls.length
    cursor.getCursorUsage(NOW + 60000)
    expect(fetch.calls.length).toBe(count)
    cursor.getCursorUsage(NOW + 5 * 60000)
    await cursor.refreshLimits(NOW + 5 * 60000)
    expect(fetch.calls.length).toBeGreaterThan(count)
  })

  test('errors are logged, and the next try comes after a minute', async () => {
    const err = console.error
    const logged = []
    console.error = (...a) => logged.push(a.join(' '))
    try {
      const fetch = api({ GetCurrentPeriodUsage: new Error('offline') })
      cursor.configureCursor({ fetch })
      await cursor.refreshLimits(NOW)
      expect(logged.join()).toContain('offline')
      expect(cursor.getCursorUsage(NOW).session).toBe(null)
      const n = fetch.calls.length
      cursor.getCursorUsage(NOW + 30000)
      expect(fetch.calls.length).toBe(n)
      cursor.getCursorUsage(NOW + 60000)
      await cursor.refreshLimits(NOW + 60000)
      expect(fetch.calls.length).toBeGreaterThan(n)
    } finally {
      console.error = err
    }
  })
})

describe('activity', () => {
  test('no transcripts: never seen, not active', () => {
    const u = cursor.getCursorUsage(NOW)
    expect(u).toMatchObject({ active: false, activity: null, lastSeen: null })
  })

  test('the newest tool call names the scene', () => {
    transcript('proj', 'a', [user, tool('Read')], NOW - 60 * 60000)
    transcript('proj', 'b', [user, tool('StrReplace')])
    expect(cursor.getCursorUsage(NOW)).toMatchObject({
      active: true,
      activity: 'editing',
      lastSeen: NOW - 5000,
    })
  })

  test('a subagent transcript counts, and unknown tools just work', () => {
    transcript('proj', 'c', [user, tool('Task')], NOW - 20000)
    transcript('proj', 'c', [tool('SomethingNew')], NOW - 1000, 'sub-1')
    expect(cursor.getCursorUsage(NOW).activity).toBe('working')
  })

  test('a finished turn rests, and an old file is not activity', () => {
    transcript('proj', 'd', [user, tool('Shell'), ended])
    expect(cursor.getCursorUsage(NOW)).toMatchObject({ active: false, activity: null })
    transcript('proj', 'e', [user, tool('Shell')], NOW - 120000)
    transcript('proj', 'd', [user, tool('Shell'), ended], NOW - 130000)
    const u = cursor.getCursorUsage(NOW)
    expect(u.active).toBe(false)
    expect(u.lastSeen).toBe(NOW - 120000)
  })

  test('only the tail of a long transcript is read', () => {
    const filler = {
      role: 'assistant',
      message: { content: [{ type: 'text', text: 'x'.repeat(70000) }] },
    }
    transcript('proj', 'f', [filler, user, tool('WebSearch')])
    expect(cursor.getCursorUsage(NOW).activity).toBe('researching')
  })

  test('activityOf: user turns think, broken lines are skipped', () => {
    expect(cursor.activityOf([JSON.stringify(user)])).toBe('working')
    expect(cursor.activityOf(['{broken', JSON.stringify(tool('Grep'))])).toBe('reading')
    expect(cursor.activityOf(['{broken'])).toBe('working')
    expect(cursor.activityOf([JSON.stringify({ role: 'assistant', message: {} })])).toBe('working')
  })
})
