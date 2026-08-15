import { afterAll, beforeEach, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import path from 'node:path'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

// pet.js is a plain <script>: it reads the real index.html by element id and
// talks to the preload bridge on window.api. So the test builds that world —
// the actual markup, a recording bridge, and stubbed animations — and then
// evaluates the script into it. Running against the real index.html means a
// renamed element breaks a test instead of shipping a silently dead panel.
//
// Lives in its own `bun test` process: registering happy-dom installs a DOM on
// every global, which the non-DOM suites should not inherit.
GlobalRegistrator.register()

const ROOT = path.join(import.meta.dir, '..', '..')
const html = fs.readFileSync(path.join(ROOT, 'renderer', 'index.html'), 'utf8')
const body = html
  .match(/<body[^>]*>([\s\S]*)<\/body>/i)[1]
  .replace(/<script[\s\S]*?<\/script>/gi, '')
document.body.innerHTML = body

// The Web Animations API is not implemented by happy-dom. pet.js only ever
// reads `onfinish`, so a stub that runs it immediately keeps the DOM tidy
// (particles remove themselves) without a timing dependency.
Element.prototype.animate = function animate() {
  const handle = { cancel() {}, finish() {} }
  queueMicrotask(() => handle.onfinish?.())
  return handle
}
globalThis.requestAnimationFrame = (cb) => {
  cb()
  return 0
}

// the preload bridge, recording what the renderer sends back to main
const api = { sent: [], handlers: {} }
for (const name of [
  'onUsage',
  'onError',
  'onConfig',
  'onRealUsage',
  'onAuthState',
  'onProfile',
  'onAuthResult',
  'onDebugState',
  'onVersion',
  'onUpdateStatus',
]) {
  api[name] = (cb) => {
    api.handlers[name] = cb
  }
}
api.all = [] // never cleared: fitSize only reports when the size actually changes
for (const name of [
  'saveConfig',
  'resize',
  'openUsage',
  'authStart',
  'authCode',
  'authLogout',
  'checkUpdates',
  'doUpdate',
  'quit',
]) {
  api[name] = (...args) => {
    api.sent.push({ name, args })
    api.all.push({ name, args })
  }
}
window.api = api

// Imported rather than eval'd so the coverage tool can see it. burn.js takes
// its CommonJS branch under import, so the global the widget relies on has to
// be planted before pet.js loads.
globalThis.Burn = await import('../../renderer/burn.js')
const pet = await import('../../renderer/pet.js')

afterAll(() => GlobalRegistrator.unregister())

const el = (id) => document.getElementById(id)
const usage = (over = {}) => ({
  session: { pct: 0, tokens: 1000, resetMs: 3600_000, active: true },
  week: { pct: 0, tokens: 5000, resetMs: null },
  today: { tokens: 2000 },
  byModel: [],
  days30: new Array(30).fill(0),
  monthTokens: 9000,
  tokensPerMin: 0,
  active: false,
  sleeping: false,
  activity: null,
  lastActivityMs: 0,
  ts: Date.now(),
  ...over,
})
const live = (sessionPct, weekPct = 0, resetMs = 3600_000) =>
  api.handlers.onRealUsage({
    session: { pct: sessionPct, resetMs },
    week: { pct: weekPct, resetMs: null },
  })

beforeEach(() => {
  api.sent.length = 0
  api.handlers.onAuthState({ connected: true })
  live(0)
  pet.burn.reset()
})

describe('the markup and the script agree', () => {
  test('every id pet.js reaches for exists in index.html', () => {
    // read the source off disk rather than Function.toString(): under coverage
    // instrumentation the runtime source is rewritten
    const source = fs.readFileSync(path.join(ROOT, 'renderer', 'pet.js'), 'utf8')
    const ids = [...new Set([...source.matchAll(/\bel\('([a-z0-9-]+)'\)/g)].map((m) => m[1]))]
    expect(ids.length).toBeGreaterThan(20)
    const missing = ids.filter((id) => el(id) === null)
    expect(missing).toEqual([])
  })
})

describe('number formatting', () => {
  test.each([
    [0, '0'],
    [999, '999'],
    [1500, '1.5k'],
    [2_400_000, '2.4M'],
    [3_200_000_000, '3.20B'],
  ])('%i tokens → %s', (n, s) => expect(pet.fmtTokens(n)).toBe(s))

  test.each([
    [0, 'now'],
    [-5, 'now'],
    [90_000, '1m'],
    [45 * 60_000, '45m'],
    [3 * 3600_000 + 25 * 60_000, '3h 25m'],
  ])('%i ms → %s', (ms, s) => expect(pet.fmtReset(ms)).toBe(s))
})

describe('the pet reacts to what Claude is doing', () => {
  const stateOf = () => [...document.body.classList].find((c) => c.startsWith('state-'))

  test('idles when nothing is happening', () => {
    pet.render(usage())
    expect(stateOf()).toBe('state-idle')
    expect(el('status-text').textContent).toBe('idle')
  })

  test('works, and names the activity', () => {
    pet.render(usage({ active: true, activity: 'editing' }))
    expect(stateOf()).toBe('state-working')
    expect(el('status-text').textContent).toBe('editing')
    expect(document.body.classList.contains('act-editing')).toBe(true)
  })

  test('sleeps after a long idle', () => {
    pet.render(usage({ sleeping: true }))
    expect(stateOf()).toBe('state-sleeping')
  })

  test('catches fire near the limit', () => {
    live(95)
    pet.render(usage())
    expect(stateOf()).toBe('state-stressed')
    expect(el('status-text').textContent).toBe('on fire')
  })

  test('is maxed out at 100%, even while working', () => {
    live(100)
    pet.render(usage({ active: true }))
    expect(stateOf()).toBe('state-tired')
    expect(el('status-text').textContent).toBe('maxed out')
  })

  test('working outranks being on fire — the work is what you can see', () => {
    live(95)
    pet.render(usage({ active: true, activity: 'running' }))
    expect(stateOf()).toBe('state-working')
  })

  test('shows the live rate while working, today’s total when not', () => {
    pet.render(usage({ active: true, tokensPerMin: 1500 }))
    expect(el('rate').textContent).toBe('1.5k tok/min')
    pet.render(usage({ active: false, today: { tokens: 2_400_000 } }))
    expect(el('rate').textContent).toBe('2.4M tokens today')
  })
})

describe('the usage panel', () => {
  test('fills the session bar and labels the reset', () => {
    live(42, 0, 2 * 3600_000)
    pet.render(usage({ session: { pct: 0, tokens: 1_500_000, resetMs: 0, active: true } }))
    expect(el('session-pct').textContent).toBe('42%')
    expect(el('session-fill').style.width).toBe('42%')
    expect(el('session-sub').textContent).toContain('resets in 2h 0m')
    expect(el('session-sub').textContent).toContain('1.5M tokens')
  })

  test('marks the bar as high past 80%', () => {
    live(81)
    pet.render(usage())
    expect(el('session-fill').classList.contains('high')).toBe(true)
    live(79)
    pet.render(usage())
    expect(el('session-fill').classList.contains('high')).toBe(false)
  })

  test('says so when there is no active session', () => {
    api.handlers.onRealUsage({ session: { pct: 0, resetMs: null }, week: { pct: 0 } })
    pet.render(usage())
    expect(el('session-sub').textContent).toBe('no active session')
  })

  test('falls back to a 7-day label when the week has no anchor', () => {
    live(10, 33)
    pet.render(usage({ week: { pct: 0, tokens: 2_000_000, resetMs: null } }))
    expect(el('week-pct').textContent).toBe('33%')
    expect(el('week-sub').textContent).toContain('last 7 days')
  })

  test('shows a dash for the mini % until an account is connected', () => {
    api.handlers.onAuthState({ connected: false })
    pet.render(usage())
    expect(el('mini-pct').textContent).toBe('—')
  })
})

describe('the model and month panels', () => {
  test('ranks models and renders a bar each', () => {
    pet.renderModels([
      { label: 'Opus 5', tokens: 1_000_000 },
      { label: 'Sonnet 5', tokens: 250_000 },
    ])
    const rows = el('bymodel-list').textContent
    expect(rows).toContain('Opus 5')
    expect(rows).toContain('1.0M')
    expect(rows).toContain('Sonnet 5')
  })

  test('draws one square per day of the month map', () => {
    const days = new Array(30).fill(0).map((_, i) => i * 1000)
    pet.renderHeat(days)
    expect(el('heat-row').children.length).toBe(30)
  })

  test('the busiest day is coloured hotter than the quietest', () => {
    const days = new Array(30).fill(0)
    days[0] = 1
    days[29] = 10_000_000
    pet.renderHeat(days)
    const sq = el('heat-row').children
    expect(sq[29].className).not.toBe(sq[0].className)
  })
})

describe('the account chip', () => {
  test('shows the email and plan when connected', () => {
    pet.showProfile({ email: 'ana@example.com', name: 'Ana', plan: 'Max' })
    expect(el('ac-email').textContent).toBe('ana@example.com')
    expect(el('account-chip').hidden).toBe(false)
  })

  test('hides itself when there is no account', () => {
    pet.showProfile(null)
    expect(el('account-chip').hidden).toBe(true)
  })
})

describe('talking back to main', () => {
  test('the close button quits', () => {
    el('close').click()
    expect(api.sent.some((s) => s.name === 'quit')).toBe(true)
  })

  test('the usage button opens the official page', () => {
    el('usage').click()
    expect(api.sent.some((s) => s.name === 'openUsage')).toBe(true)
  })

  test('reports its content size so main can fit the window', () => {
    pet.render(usage())
    expect(api.all.some((s) => s.name === 'resize')).toBe(true)
  })
})

describe('the burn-rate line', () => {
  test('stays hidden until there is a trail', () => {
    live(20)
    pet.render(usage())
    expect(el('session-proj').hidden).toBe(true)
  })

  test('warns in coral when the pace would run you out first', () => {
    const now = Date.now()
    const MIN = 60_000
    live(25, 0, 3 * 3600_000)
    pet.burn.reset() // live() re-renders, which samples — start clean
    // 30%/h against a 3h reset: 75 points left → 2.5h, so the reset loses
    for (let m = 20; m >= 0; m -= 0.5) {
      pet.burn.note(20e6 - 30 * (20e6 / 25) * (m / 60), true, now - m * MIN)
    }
    pet.render(usage({ session: { pct: 0, tokens: 20e6, resetMs: 0, active: true } }))
    expect(el('session-proj').hidden).toBe(false)
    expect(el('session-proj').textContent).toContain('left at this pace')
    expect(el('session-proj').classList.contains('tight')).toBe(true)
  })

  test('reassures when the reset arrives first', () => {
    const now = Date.now()
    const MIN = 60_000
    live(25, 0, 8 * 3600_000)
    pet.burn.reset()
    for (let m = 20; m >= 0; m -= 0.5) {
      pet.burn.note(20e6 - 5 * (20e6 / 25) * (m / 60), true, now - m * MIN)
    }
    pet.render(usage({ session: { pct: 0, tokens: 20e6, resetMs: 0, active: true } }))
    expect(el('session-proj').textContent).toBe('resets before you run out')
    expect(el('session-proj').classList.contains('tight')).toBe(false)
  })
})

describe('settings and updates', () => {
  test('reflects the config it is given', () => {
    api.handlers.onConfig({ mode: 'menubar', alerts: true, alertThresholds: [80, 95] })
    expect(document.body.classList.contains('is-menubar')).toBe(true)
    api.handlers.onConfig({ mode: 'floating' })
    expect(document.body.classList.contains('is-menubar')).toBe(false)
  })

  test('surfaces an update when one is available', () => {
    api.handlers.onUpdateStatus({ state: 'available', latest: 'v9.9.9' })
    expect(document.body.textContent).toContain('9.9.9')
  })

  test('shows the running version', () => {
    api.handlers.onVersion('1.2.3')
    expect(document.body.textContent).toContain('1.2.3')
  })

  test('reports a usage error without blanking the panel', () => {
    api.handlers.onError('boom')
    expect(el('status-text').textContent).toBe('error')
  })
})

describe('the debug simulator', () => {
  test('forces a state', () => {
    pet.render(usage())
    api.handlers.onDebugState({ state: 'fire' })
    expect([...document.body.classList].find((c) => c.startsWith('state-'))).toBe('state-stressed')
  })

  test('forces an activity scene', () => {
    api.handlers.onDebugState({ state: 'reading' })
    expect(document.body.classList.contains('act-reading')).toBe(true)
  })

  test('hands control back to the real usage', () => {
    api.handlers.onDebugState({ state: 'auto' })
    pet.render(usage())
    expect([...document.body.classList].find((c) => c.startsWith('state-'))).toBe('state-idle')
  })
})
