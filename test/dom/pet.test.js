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
  'onCodex',
  'onCodexDetected',
  'onAuthState',
  'onProfile',
  'onAuthResult',
  'onAuthPending',
  'onAccounts',
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
  'codexDetect',
  'codexEnable',
  'authStart',
  'authCode',
  'authLogout',
  'accountSwitch',
  'accountAdd',
  'accountCancelAdd',
  'accountRemove',
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
globalThis.Voice = await import('../../renderer/voice.js')
const pet = await import('../../renderer/pet.js')

afterAll(() => GlobalRegistrator.unregister())

const el = (id) => document.getElementById(id)
const usage = (over = {}) => ({
  session: { pct: 0, tokens: 1000, resetMs: 3600_000, active: true },
  week: { pct: 0, tokens: 5000, resetMs: null },
  today: { tokens: 2000 },
  byModel: [],
  byProject: [],
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
    [160 * 3600_000 + 57 * 60_000, '6d 16h'],
  ])('%i ms → %s', (ms, s) => expect(pet.fmtReset(ms)).toBe(s))

  test('the reset countdown carries the local clock time it lands on', () => {
    const now = new Date('2026-09-01T10:00:00')
    const realNow = Date.now
    Date.now = () => now.getTime()
    const at = (ms) =>
      new Date(now.getTime() + ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })

    expect(pet.fmtResetIn(2 * 3600_000)).toBe(`2h 0m (${at(2 * 3600_000)})`)
    // past a day the weekday matters as much as the hour
    expect(pet.fmtResetIn(69 * 3600_000)).toBe(`2d 21h (Fri ${at(69 * 3600_000)})`)
    expect(pet.fmtResetIn(0)).toBe('now')
    Date.now = realNow
  })
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

  test('draws one meter per scoped weekly limit, with that model family tokens', () => {
    api.handlers.onRealUsage({
      session: { pct: 0, resetMs: null },
      week: { pct: 10, resetMs: null },
      scoped: [{ label: 'Fable', pct: 38, resetMs: 5 * 3600_000 }],
    })
    pet.render(
      usage({
        byModel: [
          { label: 'Opus 5', tokens: 1_000_000 },
          { label: 'Fable 5', tokens: 300_000 },
          { label: 'Fable 4.9', tokens: 50_000 },
        ],
      }),
    )
    const meters = el('scoped-meters').querySelectorAll('.meter')
    expect(meters.length).toBe(1)
    expect(meters[0].querySelector('.meter-top').textContent).toBe('weekly · fable38%')
    expect(meters[0].querySelector('.fill').style.width).toBe('38%')
    expect(meters[0].querySelector('.fill').classList.contains('high')).toBe(false)
    expect(meters[0].querySelector('.sub').textContent).toMatch(
      /^resets in 5h 0m \(.+\) · 350\.0k tokens$/,
    )
  })

  test('scoped meters go high past 80% and vanish when the account has none', () => {
    api.handlers.onRealUsage({
      session: { pct: 0, resetMs: null },
      week: { pct: 0, resetMs: null },
      scoped: [{ label: 'Opus', pct: 91, resetMs: null }],
    })
    pet.render(usage())
    const f = el('scoped-meters').querySelector('.fill')
    expect(f.classList.contains('high')).toBe(true)
    expect(el('scoped-meters').querySelector('.sub').textContent).toBe('0 tokens')
    live(0) // no `scoped` at all
    pet.render(usage())
    expect(el('scoped-meters').children.length).toBe(0)
  })

  test('the collapsed ring drains with the session and goes high past 80%', () => {
    const LEN = 2 * Math.PI * 45
    live(25)
    pet.render(usage())
    const rf = el('ring-fill')
    expect(Number(rf.style.strokeDashoffset)).toBeCloseTo(LEN * 0.75, 3)
    expect(rf.classList.contains('high')).toBe(false)
    expect(el('ring-pct').textContent).toBe('25%')
    live(88)
    pet.render(usage())
    expect(rf.classList.contains('high')).toBe(true)
    expect(el('ring-pct').classList.contains('high')).toBe(true)
  })

  test('shows a dash for the mini % until an account is connected', () => {
    api.handlers.onAuthState({ connected: false })
    pet.render(usage())
    expect(el('mini-pct').textContent).toBe('—')
    expect(el('ring-pct').textContent).toBe('—')
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

  test('ranks projects and keeps the full name on the row', () => {
    pet.renderProjects([
      { label: 'claude-usage-monitor', tokens: 900_000 },
      { label: 'other · 3', tokens: 12_000 },
    ])
    const rows = el('byproject-list')
    expect(rows.children.length).toBe(2)
    expect(rows.textContent).toContain('claude-usage-monitor')
    expect(rows.textContent).toContain('other · 3')
    // the column is clipped by CSS, so the untruncated label lives on the title
    expect(rows.children[0].firstChild.title).toBe('claude-usage-monitor')
  })

  test('escapes a label instead of injecting it as markup', () => {
    // project labels come from directory names, which are not ours to trust
    pet.renderProjects([{ label: '<img src=x onerror=alert(1)>', tokens: 1 }])
    const name = el('byproject-list').querySelector('.mname')
    expect(name.querySelector('img')).toBe(null)
    expect(name.textContent).toBe('<img src=x onerror=alert(1)>')
  })

  test('folds a section away and back, and says so for assistive tech', () => {
    const sec = el('byproject')
    const head = sec.querySelector('.sec-head')
    expect(sec.classList.contains('folded')).toBe(false)

    head.click()
    expect(sec.classList.contains('folded')).toBe(true)
    expect(head.getAttribute('aria-expanded')).toBe('false')

    head.click()
    expect(sec.classList.contains('folded')).toBe(false)
    expect(head.getAttribute('aria-expanded')).toBe('true')
  })

  test('remembers which sections were folded', () => {
    el('bymodel').querySelector('.sec-head').click()
    expect(JSON.parse(localStorage.getItem('clauddy.folded'))).toEqual(['bymodel'])
    el('bymodel').querySelector('.sec-head').click()
    expect(JSON.parse(localStorage.getItem('clauddy.folded'))).toEqual([])
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

describe('the history disclosure', () => {
  test('Details opens every history section and remembers its state', () => {
    expect(el('details-toggle').getAttribute('aria-expanded')).toBe('false')
    expect(el('usage-details').hidden).toBe(true)
    for (const id of ['bymodel', 'byproject', 'heat']) {
      expect(el('usage-details').contains(el(id))).toBe(true)
    }
    el('details-toggle').click()
    expect(el('usage-details').hidden).toBe(false)
    expect(el('details-toggle').getAttribute('aria-expanded')).toBe('true')
    expect(localStorage.getItem('clauddy.details-open')).toBe('true')
    pet.render(usage({ byModel: [{ label: 'Opus', tokens: 1234 }] }))
    expect(el('usage-details').hidden).toBe(false)
    expect(el('bymodel-list').textContent).toContain('Opus')
    el('details-toggle').click()
    expect(el('usage-details').hidden).toBe(true)
    expect(localStorage.getItem('clauddy.details-open')).toBe('false')
    expect(el('limits').closest('#usage-details')).toBeNull()
  })

  test('profile updates keep the Claude logo and use a plan badge that hides when unknown', () => {
    api.handlers.onProfile({ email: 'ana@example.com', plan: 'Max' })
    expect(el('acc-avatar').querySelector('svg.connection-logo')).not.toBeNull()
    expect(el('acc-ok').textContent).toBe('ana@example.com')
    expect(el('acc-sub').classList.contains('plan-badge')).toBe(true)
    expect(el('acc-sub').textContent).toBe('Max')
    expect(el('acc-sub').hidden).toBe(false)
    api.handlers.onProfile({ email: 'ana@example.com' })
    expect(el('acc-sub').hidden).toBe(true)
    expect(el('acc-avatar').querySelector('svg.connection-logo')).not.toBeNull()
  })
})

describe('the account chip', () => {
  test('shows the email and plan when connected', () => {
    pet.showProfile({ email: 'ana@example.com', name: 'Ana', plan: 'Max' })
    expect(el('ac-email').textContent).toBe('ana@example.com')
    expect(el('account-chip').hidden).toBe(false)
  })

  test('hides itself when there is no account', () => {
    api.handlers.onAccounts({ active: 'default', accounts: [] })
    pet.showProfile(null)
    expect(el('account-chip').hidden).toBe(true)
  })

  test('falls back to the account label when the profile never arrives', () => {
    pet.showProfile(null)
    api.handlers.onAccounts({
      active: 'default',
      accounts: [{ id: 'default', label: 'ana@example.com', connected: true }],
    })
    expect(el('account-chip').hidden).toBe(false)
    expect(el('ac-email').textContent).toBe('ana@example.com')
    expect(el('mini-acct-name').textContent).toBe('ana')
  })

  test('a listed but disconnected account gets no chip', () => {
    pet.showProfile(null)
    api.handlers.onAccounts({
      active: 'default',
      accounts: [{ id: 'default', label: 'ana@example.com', connected: false }],
    })
    expect(el('account-chip').hidden).toBe(true)
  })
})

describe('the account menu', () => {
  const menu = () => [...el('acc-menu').children]
  const rows = () => menu().slice(0, -1) // the last entry adds an account
  const open = (a) => {
    pet.renderAccounts(a)
    if (el('acc-menu').hidden) el('account-chip').click()
    else pet.renderAccountMenu()
  }
  const two = () =>
    open({
      active: 'default',
      accounts: [
        { id: 'default', label: 'me@example.com', connected: true },
        { id: 'w', label: null, connected: false },
      ],
    })

  test('opens from the chip, even with a single account', () => {
    open({ active: 'default', accounts: [{ id: 'default', connected: true }] })
    expect(el('acc-menu').hidden).toBe(false)
    expect(rows()).toHaveLength(1)
    expect(document.body.classList.contains('one-account')).toBe(true)
  })

  test('shows one row per account, marking the active and connected ones', () => {
    two()
    expect(document.body.classList.contains('one-account')).toBe(false)
    expect(rows()).toHaveLength(2)
    expect(rows()[0].querySelector('.who').textContent).toBe('me@example.com')
    expect(rows()[0].classList.contains('connected')).toBe(true)
    expect(rows()[0].classList.contains('active')).toBe(true)
    expect(rows()[1].querySelector('.who').textContent).toBe('Not connected')
  })

  test('clicking an inactive row switches to it, and the row goes pending', () => {
    two()
    api.sent.length = 0
    rows()[0].click() // already active
    expect(api.sent).toEqual([])
    rows()[1].click()
    expect(api.sent).toEqual([{ name: 'accountSwitch', args: ['w'] }])
    // the menu stays up, showing which account is being loaded…
    expect(el('acc-menu').hidden).toBe(false)
    expect(rows()[1].classList.contains('loading')).toBe(true)
    // …until main answers with the new active account
    api.handlers.onAccounts({
      active: 'w',
      accounts: [
        { id: 'default', label: 'me@example.com', connected: true },
        { id: 'w', label: 'work@example.com', connected: true },
      ],
    })
    expect(el('acc-menu').hidden).toBe(true)
  })

  test('a second switch is ignored while one is still loading', () => {
    open({
      active: 'default',
      busy: 'w',
      accounts: [
        { id: 'default', label: 'me@example.com', connected: true },
        { id: 'w', label: null, connected: false },
      ],
    })
    api.sent.length = 0
    // the clicked row goes pending immediately, without waiting for main
    expect(rows()[1].classList.contains('loading')).toBe(true)
    rows()[1].click()
    expect(api.sent).toEqual([])
  })

  test('every row but the active one can be removed', () => {
    two()
    expect(rows()[0].querySelector('.drop')).toBeNull() // the active one
    expect(rows()[1].querySelector('.drop')).not.toBeNull()
  })

  test('a lone account has no × at all', () => {
    open({ active: 'w', accounts: [{ id: 'w', label: 'me@example.com', connected: true }] })
    expect(rows()[0].querySelector('.drop')).toBeNull()
  })

  test('removing takes two clicks, and does not switch accounts', () => {
    two()
    api.sent.length = 0
    rows()[1].querySelector('.drop').click()
    expect(api.sent).toEqual([]) // armed, not done
    expect(rows()[1].querySelector('.drop').textContent).toBe('remove?')
    rows()[1].querySelector('.drop').click()
    expect(api.sent).toEqual([{ name: 'accountRemove', args: ['w'] }])
  })

  test('the add entry asks main for a fresh account and closes the menu', () => {
    two()
    api.sent.length = 0
    menu().at(-1).click()
    expect(api.sent).toEqual([{ name: 'accountAdd', args: [] }])
    expect(el('acc-menu').hidden).toBe(true)
  })

  test('a click elsewhere puts the menu away', () => {
    two()
    document.body.click()
    expect(el('acc-menu').hidden).toBe(true)
    expect(el('acc-backdrop').hidden).toBe(true)
  })

  test('the backdrop catches clicks the drag region would swallow', () => {
    two()
    expect(el('acc-backdrop').hidden).toBe(false)
    el('acc-backdrop').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))
    expect(el('acc-menu').hidden).toBe(true)
  })
})

describe('giving up on a login', () => {
  test('closing settings while a code is pending abandons the new account', () => {
    api.handlers.onAuthPending()
    expect(document.body.classList.contains('awaiting')).toBe(true)
    api.sent.length = 0
    el('set-cancel').click()
    expect(api.sent).toEqual([{ name: 'accountCancelAdd', args: [] }])
    expect(document.body.classList.contains('awaiting')).toBe(false)
    // and nothing is sent when there was no login to abandon
    api.sent.length = 0
    el('set-cancel').click()
    expect(api.sent).toEqual([])
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

  test('reflects the zoom it is given', () => {
    api.handlers.onConfig({ mode: 'floating', zoom: 150 })
    expect(document.body.style.zoom).toBe('1.5')
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

describe('codex', () => {
  const codex = (over = {}) => ({
    active: false,
    lastSeen: Date.now() - 60_000,
    limitsAt: Date.now(),
    model: 'gpt-6-astra',
    plan: 'plus',
    session: { pct: 72.4, resetMs: 3600000 },
    weekly: { pct: 31, resetMs: 5 * 86400000 },
    tokensToday: 900,
    tokens5h: 1500,
    tokensWeek: 2_000_000,
    byModel: [{ label: 'gpt-6-astra', tokens: 2_000_000 }],
    byProject: [{ label: 'clauddy', tokens: 2_000_000 }],
    days30: new Array(30).fill(0).map((_, i) => (i === 29 ? 900 : 0)),
    monthTokens: 900,
    ...over,
  })
  const claudeLive = () => {
    api.handlers.onAuthState({ connected: true })
    api.handlers.onRealUsage({
      session: { pct: 40, resetMs: 3600000 },
      week: { pct: 20, resetMs: 86400000 },
      scoped: [],
    })
  }
  const setConfig = (on) => api.handlers.onConfig({ alertThresholds: [80, 95], codex: on })
  beforeEach(claudeLive)

  test('without Codex enabled the panel is the classic one', () => {
    claudeLive()
    setConfig(false)
    pet.render(usage())
    api.handlers.onCodex(codex())
    expect(document.body.classList.contains('dual')).toBe(false)
    expect(document.body.classList.contains('view-codex')).toBe(false)
    expect(el('session-pct').textContent).toBe('40%')
  })

  test('both services: tabs with each session %, Claude on screen', () => {
    pet.pickProvider('claude')
    setConfig(true)
    api.handlers.onCodex(codex())
    expect(document.body.classList.contains('dual')).toBe(true)
    expect(el('tab-claude-value').textContent).toBe('40%')
    expect(el('tab-codex-value').textContent).toBe('72%')
    expect(el('tab-claude').getAttribute('aria-selected')).toBe('true')
    expect(el('session-pct').textContent).toBe('40%')
    expect(el('mini-codex-value').textContent).toBe('72%')
  })

  test('a Codex window near the limit flags its tab without taking focus', () => {
    api.handlers.onCodex(codex({ weekly: { pct: 90, resetMs: 1000 } }))
    expect(el('tab-codex').classList.contains('urgent')).toBe(true)
    expect(el('tab-codex').querySelector('.tab-alert').hidden).toBe(false)
    expect(el('tab-claude').getAttribute('aria-selected')).toBe('true')
  })

  test('the Codex tab swaps the whole panel', () => {
    api.handlers.onCodex(codex())
    el('tab-codex').click()
    expect(document.body.classList.contains('view-codex')).toBe(true)
    expect(el('session-pct').textContent).toBe('72%')
    expect(el('session-sub').textContent).toContain('1.5k tokens')
    expect(el('week-pct').textContent).toBe('31%')
    expect(el('ac-email').textContent).toBe('Codex')
    expect(el('ac-plan').textContent).toBe('plus')
    expect(el('bymodel-list').textContent).toContain('gpt-6-astra')
    expect(el('byproject-list').textContent).toContain('clauddy')
    expect(el('status-text').textContent).toBe('idle')
    expect(el('rate').textContent).toBe('900 tokens today')
    expect(el('session-proj').hidden).toBe(true)
  })

  test('the Usage arrow follows the tab', () => {
    api.sent.length = 0
    el('usage').click()
    expect(api.sent.find((s) => s.name === 'openUsage').args).toEqual(['codex'])
  })

  test('the chip leads to Settings instead of the Claude account menu', () => {
    el('account-chip').click()
    expect(document.body.classList.contains('settings-open')).toBe(true)
    expect(el('acc-menu').hidden).toBe(true)
    el('gear').click()
  })

  test('an active Codex puts the pet to work', () => {
    api.handlers.onCodex(codex({ active: true }))
    expect(el('status-text').textContent).toBe('working')
    expect(el('tab-codex').classList.contains('active')).toBe(true)
  })

  test('a long-idle Codex sleeps; no data never does', () => {
    api.handlers.onCodex(codex({ lastSeen: Date.now() - 3600_000 }))
    expect(el('status-text').textContent).toBe('sleeping')
    api.handlers.onCodex(codex({ lastSeen: null }))
    expect(el('status-text').textContent).toBe('idle')
  })

  test('unknown and stale windows read as such, never as 0%', () => {
    api.handlers.onCodex(codex({ session: null, weekly: null }))
    expect(el('session-pct').textContent).toBe('—')
    expect(el('session-sub').textContent).toContain('limits not recorded yet')
    api.handlers.onCodex(
      codex({ session: { pct: null, expired: true }, limitsAt: Date.now() - 3600_000 }),
    )
    expect(el('session-sub').textContent).toContain('waiting for a fresh reading')
    expect(el('week-sub').textContent).toMatch(/read 1h 0m ago$/)
    expect(el('mini-reset').hidden).toBe(true) // no reset to tell
    api.handlers.onCodex(codex({ monthTokens: undefined }))
    expect(el('mini-reset').textContent).toMatch(/^resets /)
    el('tab-claude').click()
    expect(el('mini-reset').hidden).toBe(false)
    el('tab-codex').click()
    expect(el('month-total').textContent).toBe('—')
  })

  test('subtle tabs switch directly and support roving keyboard focus', () => {
    setConfig(true)
    pet.pickProvider('claude')
    api.handlers.onCodex(codex())
    expect(el('tab-claude').tabIndex).toBe(0)
    expect(el('tab-codex').tabIndex).toBe(-1)
    el('tab-claude').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }),
    )
    expect(document.activeElement).toBe(el('tab-codex'))
    expect(el('tab-codex').getAttribute('aria-selected')).toBe('true')
    expect(el('tab-claude').tabIndex).toBe(-1)
    expect(el('service-panel').getAttribute('aria-labelledby')).toBe('tab-codex')
    expect(el('session-pct').textContent).toBe('72%')
    el('tab-codex').dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }))
    expect(el('session-pct').textContent).toBe('40%')
    el('tab-claude').dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    expect(document.activeElement).toBe(el('tab-codex'))
    el('tab-codex').dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }))
    expect(document.activeElement).toBe(el('tab-claude'))
  })

  test('the inactive tab and dock keep weekly alerts visible without stealing focus', () => {
    pet.pickProvider('claude')
    api.handlers.onCodex(codex({ weekly: { pct: 90, resetMs: 1000 } }))
    expect(el('tab-codex-value').textContent).toBe('72%')
    expect(el('tab-codex').querySelector('.tab-alert').hidden).toBe(false)
    expect(
      document.querySelector('.mini-source[data-provider="codex"]').classList.contains('urgent'),
    ).toBe(true)
    expect(el('tab-claude').getAttribute('aria-selected')).toBe('true')
    el('tab-codex').click()
    expect(el('week-pct').textContent).toBe('90%')
    api.handlers.onCodex(codex({ session: null, limitsAt: Date.now() - 3600000 }))
    expect(el('tab-codex-value').textContent).toBe('—')
    api.handlers.onCodex(codex())
  })

  test('compact sizing and single-source labels survive a disconnect', () => {
    el('min').click()
    expect(api.all.filter((s) => s.name === 'resize').at(-1).args[0]).toBe(240)
    el('min').click()
    expect(api.all.filter((s) => s.name === 'resize').at(-1).args[0]).toBe(304)
    setConfig(false)
    expect(el('service-panel').getAttribute('role')).toBe('region')
    expect(el('service-panel').getAttribute('aria-label')).toBe('Claude usage')
    expect(el('service-panel').hasAttribute('aria-labelledby')).toBe(false)
    setConfig(true)
    api.handlers.onCodex(codex())
  })

  test('collapsed with both: a line per service, and a click picks one', () => {
    api.handlers.onCodex(codex())
    document.body.classList.add('collapsed')
    document.querySelector('.mini-source[data-provider="claude"]').click()
    expect(el('tab-claude').getAttribute('aria-selected')).toBe('true')
    expect(el('mini-claude-value').textContent).toBe('40%')
    document.body.classList.remove('collapsed')
  })

  test('with Claude signed out, Codex is the only service', () => {
    api.handlers.onAuthState({ connected: false })
    expect(document.body.classList.contains('dual')).toBe(false)
    expect(document.body.classList.contains('view-codex')).toBe(true)
    expect(el('session-pct').textContent).toBe('72%')
    claudeLive()
  })

  test('Settings: connect turns Codex on when a session is found', () => {
    setConfig(false)
    api.sent.length = 0
    el('codex-connect').click()
    expect(api.sent.map((s) => s.name)).toContain('codexDetect')
    api.handlers.onCodexDetected({ found: false, plan: null })
    expect(el('codex-hint').textContent).toContain('No session')
    expect(el('codex-setup').hidden).toBe(false)
    expect(api.sent.some((s) => s.name === 'codexEnable')).toBe(false)
    el('codex-setup-cancel').click()
    expect(el('codex-setup').hidden).toBe(true)
    el('codex-connect').click()
    el('codex-retry').click()
    api.handlers.onCodexDetected({ found: true, plan: 'plus' })
    expect(api.sent.find((s) => s.name === 'codexEnable').args).toEqual([true])
    expect(el('codex-setup').hidden).toBe(true)
  })

  test('Settings: disconnect stops monitoring', () => {
    setConfig(true)
    expect(document.body.classList.contains('codex-on')).toBe(true)
    api.sent.length = 0
    el('codex-disconnect').click()
    expect(api.sent.find((s) => s.name === 'codexEnable').args).toEqual([false])
    setConfig(false)
    expect(document.body.classList.contains('view-codex')).toBe(false)
  })
})

describe('the voice', () => {
  // the bubble keeps a 10-minute gap between remarks, so every test starts
  // well past the last one
  const realNow = Date.now
  let clock = realNow() + 3600_000
  const tick = (ms) => {
    clock += ms
  }
  const bubble = () => (el('bubble').hidden ? null : el('bubble-text').textContent)
  const floating = (over = {}) =>
    api.handlers.onConfig({ mode: 'floating', alertThresholds: [80, 95], ...over })

  // a recording Web Audio, installed before the blipper first reaches for one
  const made = []
  globalThis.AudioContext = class {
    constructor() {
      this.state = 'running'
      this.currentTime = 0
      this.destination = {}
      this.oscs = 0
      made.push(this)
    }
    createOscillator() {
      this.oscs++
      return { frequency: { setValueAtTime() {} }, connect() {}, start() {}, stop() {} }
    }
    createGain() {
      const f = () => {}
      return {
        gain: { setValueAtTime: f, linearRampToValueAtTime: f, exponentialRampToValueAtTime: f },
        connect() {},
      }
    }
  }
  const blips = () => made.reduce((n, c) => n + c.oscs, 0)

  beforeEach(() => {
    tick(11 * 60000)
    Date.now = () => clock
    document.body.classList.remove('collapsed', 'settings-open')
    el('bubble').hidden = true
    floating()
  })
  afterAll(() => {
    Date.now = realNow
  })

  test('a remark shows in the bubble, and a click puts it away', async () => {
    expect(pet.say('hello')).toBe(true)
    expect(bubble()).toBe('hello')
    el('bubble').click()
    expect(el('bubble').classList.contains('leaving')).toBe(true)
    await new Promise((r) => setTimeout(r, 200))
    expect(el('bubble').hidden).toBe(true)
  })

  test('keeps a gap between remarks — unless the moment is a headline', () => {
    expect(pet.say('one')).toBe(true)
    tick(60000)
    expect(pet.say('two')).toBe(false)
    expect(pet.say('three', { headline: true })).toBe(true)
    expect(bubble()).toBe('three')
  })

  test('stays quiet when talk is off, or in settings', () => {
    floating({ talk: false })
    expect(pet.say('no')).toBe(false)
    floating()
    document.body.classList.add('settings-open')
    expect(pet.say('no')).toBe(false)
    expect(pet.say('')).toBe(false)
  })

  test('sound is opt-in, and silent in the menu bar, collapsed, or muted', () => {
    expect(pet.soundOn()).toBe(false)
    floating({ sound: true })
    expect(pet.soundOn()).toBe(true)
    api.handlers.onConfig({ mode: 'menubar', sound: true })
    expect(pet.soundOn()).toBe(false)
    floating({ sound: true })
    document.body.classList.add('collapsed')
    expect(pet.soundOn()).toBe(false)
    document.body.classList.remove('collapsed')
    floating({ sound: true, soundMutedUntil: clock + 60000 })
    expect(pet.soundOn()).toBe(false)
  })

  test('collapsed, the bubble moves above the pet — and back when expanded', () => {
    document.body.classList.add('collapsed')
    expect(pet.say({ text: 'a whole sentence', short: 'up here' })).toBe(true)
    expect(bubble()).toBe('up here') // the mini face gets the glance
    expect(el('bubble').nextElementSibling).toBe(el('stage'))
    document.body.classList.remove('collapsed')
    tick(11 * 60000)
    pet.say('in the scene')
    expect(el('bubble').parentElement).toBe(el('stage'))
  })

  test('only a headline blips', () => {
    floating({ sound: true })
    const before = blips()
    pet.say('plain remark')
    expect(blips()).toBe(before)
    tick(11 * 60000)
    pet.say('big news', { headline: true })
    expect(blips()).toBeGreaterThan(before)
  })

  test('the mute button: shown with the voice on, one click for an hour', () => {
    floating()
    expect(el('mute').hidden).toBe(true)
    floating({ sound: true })
    expect(el('mute').hidden).toBe(false)
    expect(document.body.classList.contains('has-mute')).toBe(true) // the chip makes room
    api.handlers.onConfig({ mode: 'menubar', sound: true })
    expect(el('mute').hidden).toBe(true) // always silent there
    expect(document.body.classList.contains('has-mute')).toBe(false)
    floating({ sound: true })
    api.sent.length = 0
    el('mute').click()
    const muted = api.sent.find((s) => s.name === 'saveConfig').args[0]
    expect(muted.soundMutedUntil).toBe(clock + 3600_000)
    expect(el('mute').classList.contains('muted')).toBe(true)
    expect(el('mute').title).toContain('Muted until')
    expect(pet.soundOn()).toBe(false)
    el('mute').click()
    expect(api.sent.at(-1).args[0].soundMutedUntil).toBe(0)
    expect(el('mute').classList.contains('muted')).toBe(false)
  })

  test('catching fire is announced once, on the crossing', () => {
    live(50)
    pet.render(usage())
    live(92)
    pet.render(usage())
    expect(bubble()).toContain('92% already')
    el('bubble').hidden = true
    tick(11 * 60000)
    pet.render(usage())
    expect(bubble()).toBeNull()
  })

  test('hitting the ceiling says when it comes back', () => {
    live(97)
    pet.render(usage())
    tick(11 * 60000) // 97% just caught fire, and that remark holds the gap
    live(100)
    pet.render(usage())
    expect(bubble()).toContain("That's the limit")
  })

  test('a fresh window recaps the one that closed', () => {
    live(90)
    pet.render(usage({ session: { tokens: 34e6, resetMs: 60000 } }))
    for (let i = 0; i < 20; i++) {
      tick(10000)
      pet.render(usage({ active: true, activity: i < 15 ? 'editing' : 'reading' }))
    }
    tick(10000)
    live(2)
    pet.render(usage())
    expect(bubble()).toBe('Fresh window! That was 34.0M tokens over 3m, mostly editing code.')
  })

  test('welcomes you back after hours away', () => {
    live(0)
    api.handlers.onRealUsage(null)
    pet.render(usage({ sleeping: true, lastActivityMs: 3 * 3600_000 }))
    tick(11 * 60000)
    pet.render(usage({ active: true }))
    expect(bubble()).toContain('Welcome back! You were gone 3h 0m.')
  })

  test('notices a long unbroken streak, once', () => {
    for (let m = 0; m <= 95; m += 5) {
      pet.render(usage({ active: true }))
      tick(5 * 60000)
    }
    expect(bubble()).toContain('Stretch break?')
  })

  test('calls a record day, once a day', () => {
    localStorage.removeItem('clauddy.record')
    const days = [...new Array(29).fill(2e6), 9e6]
    pet.render(usage({ days30: days }))
    expect(bubble()).toContain('New record! 9.0M tokens today')
    el('bubble').hidden = true
    tick(11 * 60000)
    pet.render(usage({ days30: days }))
    expect(bubble()).toBeNull()
  })

  test('Codex speaks for itself: fire, ceiling and a fresh window', () => {
    const cx = (pct) =>
      api.handlers.onCodex({
        session: { pct, resetMs: 3600_000 },
        weekly: { pct: 1 },
        limitsAt: clock,
      })
    api.handlers.onConfig({ mode: 'floating', alertThresholds: [80, 95], codex: true })
    cx(40)
    cx(92)
    expect(bubble()).toBe('Codex is at 92% now. It resets in 1h 0m.')
    tick(11 * 60000)
    cx(100)
    expect(bubble()).toContain('Codex is maxed out')
    cx(3)
    expect(bubble()).toBe('Codex has a fresh window! The last one closed at 100%.')
    api.handlers.onConfig({ mode: 'floating', alertThresholds: [80, 95], codex: false })
  })

  test('the simulator can make it talk', () => {
    api.handlers.onDebugState({ state: 'say' })
    expect(bubble()).toMatch(/yesterday/i)
  })

  test.each([
    ['fire', 'At this pace'],
    ['reset', 'Fresh window'],
    ['maxed', "That's the limit"],
    ['welcome', 'Welcome back'],
    ['streak', 'Stretch break?'],
    ['record', 'New record!'],
    ['greeting', 'yesterday'],
    ['codex', 'Codex is at 91%'],
  ])('the simulator previews the %s remark', (kind, text) => {
    api.handlers.onDebugState({ state: 'say', kind })
    expect(bubble()?.toLowerCase()).toContain(text.toLowerCase())
  })

  test('collapsing or opening settings puts the bubble away', () => {
    pet.say('hi', { headline: true })
    el('min').click()
    expect(el('bubble').classList.contains('leaving')).toBe(true)
    el('min').click()
  })

  test('Settings: talk and sound round-trip, and turning sound on plays a sample', () => {
    floating({ talk: false, sound: false })
    el('gear').click()
    expect(el('set-talk').checked).toBe(false)
    expect(el('set-rows').classList.contains('talk-off')).toBe(true)
    el('set-talk').checked = true
    el('set-talk').dispatchEvent(new Event('change'))
    expect(el('set-rows').classList.contains('talk-off')).toBe(false)
    const before = blips()
    el('set-sound').checked = true
    el('set-sound').dispatchEvent(new Event('change'))
    expect(blips()).toBeGreaterThan(before)
    api.sent.length = 0
    el('set-save').click()
    const saved = api.sent.find((s) => s.name === 'saveConfig').args[0]
    expect(saved.talk).toBe(true)
    expect(saved.sound).toBe(true)
  })
})
