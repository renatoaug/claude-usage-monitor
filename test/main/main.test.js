import { afterAll, beforeEach, describe, expect, mock, test } from 'bun:test'
import childProcess from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

// main.js has no exports — it wires Electron up at import time. So we hand it a
// fake `electron` and assert on what it *does*: the windows it opens, the
// notifications it fires, the messages it sends the renderer.
//
// This lives in test/main/ and runs as its own `bun test` process: mocking
// `electron`, `./usage` and `./auth` replaces those modules for the whole
// runtime, and bun loads every test file before running any of them — so these
// mocks would otherwise reach the suites that test the real modules.
const ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-main-'))
process.env.CLAUDE_CONFIG_DIR = ROOT
const DATA_DIR = path.join(ROOT, 'usage-monitor')
const CONFIG_PATH = path.join(DATA_DIR, 'config.json')
const WINDOW_STATE = path.join(DATA_DIR, 'window.json')
const ALERTS_PATH = path.join(DATA_DIR, 'alerts.json')

// ---- the fake Electron -------------------------------------------------------
const sent = [] // every webContents.send(channel, payload)
const ipc = new Map() // channel → handler registered by main.js
const notifications = []
const opened = [] // shell.openExternal
const screenHandlers = new Map()
let loadedFile = null
let winOptions = null
let quitCount = 0

const trayState = { instance: null, title: null, tooltip: null, handlers: new Map(), menu: null }

const winMock = {
  bounds: { x: 100, y: 100, width: 276, height: 480 },
  visible: false,
  destroyed: false,
  handlers: new Map(),
  getBounds() {
    return { ...this.bounds }
  },
  setPosition(x, y) {
    this.bounds.x = x
    this.bounds.y = y
  },
  setContentSize(w, h) {
    this.bounds.width = w
    this.bounds.height = h
  },
  isVisible() {
    return this.visible
  },
  isDestroyed() {
    return this.destroyed
  },
  show() {
    this.visible = true
  },
  hide() {
    this.visible = false
  },
  focus() {},
  setAlwaysOnTop() {},
  setVisibleOnAllWorkspaces() {},
  setSkipTaskbar() {},
  loadFile(f) {
    loadedFile = f
  },
  on(ev, cb) {
    this.handlers.set(ev, cb)
  },
  webContents: {
    once(ev, cb) {
      winMock.handlers.set(`wc:${ev}`, cb)
    },
    send(channel, payload) {
      sent.push({ channel, payload })
    },
  },
}

let displays = [{ bounds: { x: 0, y: 0, width: 1920, height: 1080 } }]

class FakeNotification {
  constructor(opts) {
    this.opts = opts
    this.handlers = new Map()
  }
  on(ev, cb) {
    this.handlers.set(ev, cb)
  }
  get title() {
    return this.opts.title
  }
  get body() {
    return this.opts.body
  }
  get silent() {
    return this.opts.silent
  }
  show() {
    notifications.push(this)
  }
  static isSupported() {
    return true
  }
}

const electronMock = {
  app: {
    whenReady: () => Promise.resolve(),
    getVersion: () => '1.5.0',
    quit: () => {
      quitCount++
    },
    isPackaged: false,
    setPath: () => {},
    setAppUserModelId: () => {},
    setLoginItemSettings: () => {},
    on: () => {},
    dock: null,
  },
  BrowserWindow: function BrowserWindow(opts) {
    winOptions = opts
    return winMock
  },
  ipcMain: {
    on: (channel, handler) => ipc.set(channel, handler),
  },
  screen: {
    getPrimaryDisplay: () => ({ workAreaSize: { width: 1920, height: 1080 } }),
    getAllDisplays: () => displays,
    on: (ev, cb) => screenHandlers.set(ev, cb),
  },
  Notification: FakeNotification,
  shell: { openExternal: (u) => opened.push(u) },
  Tray: function Tray() {
    trayState.instance = {
      setTitle: (t) => {
        trayState.title = t
      },
      setToolTip: (t) => {
        trayState.tooltip = t
      },
      on: (ev, cb) => trayState.handlers.set(ev, cb),
      popUpContextMenu: (m) => {
        trayState.menu = m
      },
      destroy: () => {
        trayState.instance = null
      },
    }
    return trayState.instance
  },
  Menu: { buildFromTemplate: (tpl) => tpl },
  nativeImage: { createFromPath: () => ({ setTemplateImage: () => {} }) },
}

// ---- fakes for the app's own modules ----------------------------------------
let usageData = {
  session: { pct: 10, tokens: 100, resetMs: 3600_000, active: true },
  week: { pct: 5, tokens: 500, resetMs: null },
  today: { tokens: 100 },
  byModel: [],
  days30: new Array(30).fill(0),
  monthTokens: 100,
  tokensPerMin: 10,
  active: true,
  sleeping: false,
  activity: 'editing',
  lastActivityMs: 0,
  ts: Date.now(),
}
let usageThrows = null
const DEFAULT_USAGE = { session: { pct: 40, resetMs: 1000 }, week: { pct: 4, resetMs: null } }
const authState = {
  connected: true,
  usage: DEFAULT_USAGE,
  usageError: null,
  profile: { email: 'a@b.com', name: 'Ana', plan: 'Max' },
  completeError: null,
  cleared: 0,
}

mock.module('../../usage.js', () => ({
  getUsage: () => {
    if (usageThrows) throw usageThrows
    return usageData
  },
}))
mock.module('../../auth.js', () => ({
  isConnected: () => authState.connected,
  begin: () => 'https://claude.ai/oauth/authorize?x=1',
  complete: async () => {
    if (authState.completeError) throw authState.completeError
    authState.connected = true
  },
  fetchUsage: async () => {
    if (authState.usageError) throw authState.usageError
    return authState.usage
  },
  fetchProfile: async () => authState.profile,
  clear: () => {
    authState.cleared++
    authState.connected = false
  },
}))
mock.module('electron', () => electronMock)

// MUST be stubbed: the macOS update path spawns `curl … | bash`, which would
// download and run the real installer and replace the app on this machine.
// main.js destructures `spawn` at import time, so the real module object is
// patched here — before that import — rather than via mock.module, which does
// not intercept node: builtins.
const spawned = []
childProcess.spawn = (cmd, args, opts) => {
  spawned.push({ cmd, args, opts })
  return { unref: () => {} }
}

// keep every timer main.js starts so the test process can exit
const realSetInterval = globalThis.setInterval
const realSetTimeout = globalThis.setTimeout
const timers = { intervals: [], timeouts: [] }
globalThis.setInterval = (fn, ms) => {
  const id = realSetInterval(fn, ms)
  timers.intervals.push({ id, fn, ms })
  return id
}
globalThis.setTimeout = (fn, ms) => {
  const id = realSetTimeout(fn, ms)
  timers.timeouts.push({ id, fn })
  return id
}

await import('../../main.js')
await Promise.resolve() // let app.whenReady().then(createWindow) run
await new Promise((r) => realSetTimeout(r, 0))

// main.js only finishes wiring after the renderer loads
const finishLoad = winMock.handlers.get('wc:did-finish-load')
finishLoad()
await new Promise((r) => realSetTimeout(r, 10)) // the async usage/profile pushes

// Everything main.js pushed during startup. Snapshotted because re-running
// did-finish-load would re-register the debug-file watcher every time.
const startup = [...sent]
const startupOf = (channel) => [...startup].reverse().find((m) => m.channel === channel)?.payload
// the recurring usage poll main.js installed — the tick, without the startup
const tick = () => timers.intervals.find((t) => t.ms === 4000).fn()

afterAll(() => {
  for (const t of timers.intervals) clearInterval(t.id)
  for (const t of timers.timeouts) clearTimeout(t.id)
  globalThis.setInterval = realSetInterval
  globalThis.setTimeout = realSetTimeout
  fs.rmSync(ROOT, { recursive: true, force: true })
})

const lastOf = (channel) => [...sent].reverse().find((m) => m.channel === channel)?.payload
const fire = (channel, ...args) => ipc.get(channel)(...[{}, ...args])

beforeEach(() => {
  sent.length = 0
  notifications.length = 0
  opened.length = 0
})

describe('startup', () => {
  test('opens a frameless, transparent, always-on-top widget', () => {
    expect(winOptions.frame).toBe(false)
    expect(winOptions.transparent).toBe(true)
    expect(winOptions.alwaysOnTop).toBe(true)
    expect(winOptions.show).toBe(false) // applyMode reveals it
    expect(winOptions.webPreferences.contextIsolation).toBe(true)
    expect(winOptions.webPreferences.nodeIntegration).toBe(false)
    expect(loadedFile).toContain(path.join('renderer', 'index.html'))
  })

  test('sets a window icon so unpackaged Linux/Windows runs are not blank', () => {
    expect(winOptions.icon).toContain(path.join('build', 'icon.png'))
  })

  test('registers every IPC channel the renderer talks on', () => {
    for (const c of [
      'resize',
      'open-usage',
      'auth-start',
      'auth-code',
      'auth-logout',
      'save-config',
      'check-updates',
      'do-update',
      'quit',
    ]) {
      expect(ipc.has(c)).toBe(true)
    }
  })

  test('pushes usage, config, version and auth state once loaded', () => {
    expect(startupOf('usage').session.pct).toBe(10)
    expect(startupOf('version')).toBe('1.5.0')
    expect(startupOf('auth-state')).toEqual({ connected: true })
    expect(startupOf('real-usage').session.pct).toBe(40) // the OAuth poll
    expect(startupOf('profile').email).toBe('a@b.com')
  })

  test('only exposes the safe subset of config to the renderer', () => {
    const cfg = startupOf('config')
    expect(cfg).toHaveProperty('alertThresholds')
    expect(cfg).toHaveProperty('fireThreshold')
    expect(cfg).not.toHaveProperty('pollIntervalMs') // internal only
    expect(cfg).not.toHaveProperty('sleepThresholdMs')
  })

  test('reports a usage failure instead of crashing the tick', () => {
    usageThrows = new Error('disk gone')
    tick()
    expect(lastOf('usage-error')).toContain('disk gone')
    usageThrows = null
  })
})

describe('threshold alerts', () => {
  // the real (OAuth) numbers are what the alerts trust, so drive those: a poll
  // refreshes them, the local tick is what evaluates the thresholds
  const at = async (sessionPct, weekPct) => {
    authState.usage = {
      session: { pct: sessionPct, resetMs: 2 * 3600_000 },
      week: { pct: weekPct, resetMs: 40 * 3600_000 },
    }
    await fire('auth-code', 'code#state')
    await new Promise((r) => realSetTimeout(r, 5))
    tick()
  }

  test('fires once when a threshold is crossed, not on every tick', async () => {
    await at(85, 0)
    expect(notifications.length).toBe(1)
    expect(notifications[0].title).toBe('Session at 85%')
    notifications.length = 0
    await at(86, 0)
    expect(notifications.length).toBe(0) // still above: stays quiet
    await at(0, 0)
  })

  test('re-arms after usage drops back below', async () => {
    await at(85, 0)
    notifications.length = 0
    await at(10, 0) // reset
    await at(85, 0)
    expect(notifications.length).toBe(1)
    await at(0, 0)
  })

  test('tracks session and weekly independently, at both levels', async () => {
    await at(0, 0)
    notifications.length = 0
    await at(96, 81)
    const titles = notifications.map((n) => n.title)
    expect(titles).toContain('Session at 96%')
    expect(titles).toContain('Session at 96% — almost out') // the top threshold
    expect(titles).toContain('Weekly usage at 81%')
    expect(titles.some((t) => t.startsWith('Weekly usage at 81% —'))).toBe(false)
    await at(0, 0)
  })

  test('only the top threshold makes a sound, and the reset is in the body', async () => {
    await at(0, 0)
    notifications.length = 0
    await at(96, 0)
    const [first, second] = notifications
    expect(first.silent).toBe(true) // 80%: a heads-up
    expect(second.silent).toBe(false) // 95%: urgency
    expect(first.body).toMatch(/^2h 0m left · resets /) // the session line carries time left
    await at(0, 0)
  })

  test('the weekly line carries only the reset, with the weekday', async () => {
    await at(0, 0)
    notifications.length = 0
    await at(0, 85)
    expect(notifications[0].body).toMatch(/^resets \w{3} /)
    await at(0, 0)
  })

  test('clicking a notification brings the widget to the front', async () => {
    await at(0, 0)
    notifications.length = 0
    await at(85, 0)
    winMock.visible = false
    notifications[0].handlers.get('click')()
    expect(winMock.visible).toBe(true)
    await at(0, 0)
  })

  test('announces a window reset only to someone who was near the ceiling', async () => {
    await at(85, 0)
    notifications.length = 0
    await at(0, 0) // the window flipped
    expect(notifications.map((n) => n.title)).toContain('Session window reset')
    notifications.length = 0
    await at(20, 0) // nowhere near the ceiling…
    await at(0, 0) // …so its reset says nothing
    expect(notifications.length).toBe(0)
  })

  test('the armed set survives a restart, so an alert is not repeated', async () => {
    await at(85, 0)
    expect(JSON.parse(fs.readFileSync(ALERTS_PATH, 'utf8')).armed).toContain('Session:80')
    await at(0, 0)
    expect(JSON.parse(fs.readFileSync(ALERTS_PATH, 'utf8')).armed).not.toContain('Session:80')
  })

  test('falls back to the local estimate when nobody is logged in', () => {
    fire('auth-logout')
    notifications.length = 0
    usageData = { ...usageData, session: { ...usageData.session, pct: 85 } }
    tick()
    expect(notifications[0].title).toBe('Session at 85%')
    usageData = { ...usageData, session: { ...usageData.session, pct: 0 } }
    tick()
  })

  test('losing the token says so, instead of failing silently', async () => {
    await fire('auth-code', 'code#state') // connected, and a poll is scheduled
    await new Promise((r) => realSetTimeout(r, 5))
    notifications.length = 0
    authState.usageError = Object.assign(new Error('dead'), { status: 401 })
    await timers.timeouts.at(-1).fn() // the scheduled poll, now rejected
    await new Promise((r) => realSetTimeout(r, 5))
    expect(notifications.map((n) => n.title)).toContain('Clauddy lost access to your usage')
    authState.usageError = null
    authState.connected = true
  })

  test('editing the thresholds re-arms the alerts, on disk too', async () => {
    await at(85, 0)
    expect(JSON.parse(fs.readFileSync(ALERTS_PATH, 'utf8')).armed).toContain('Session:80')
    fire('save-config', { alertThresholds: [70, 95] })
    expect(JSON.parse(fs.readFileSync(ALERTS_PATH, 'utf8')).armed).not.toContain('Session:80')
    fire('save-config', { alertThresholds: [80, 95] })
    await at(0, 0)
  })

  test('saving an unrelated setting does not replay a dismissed alert', async () => {
    await at(85, 0)
    notifications.length = 0
    fire('save-config', { zoom: 125 })
    expect(notifications.length).toBe(0)
    fire('save-config', { zoom: 100 })
    await at(0, 0)
  })

  test('turning alerts off silences every kind', async () => {
    await at(0, 0)
    fire('save-config', { alerts: false })
    notifications.length = 0
    await at(96, 96)
    await at(0, 0) // would also be a window reset
    expect(notifications.length).toBe(0)
    fire('save-config', { alerts: true })
  })

  test('per-model weekly limits alert off the OAuth poll', async () => {
    const poll = async () => {
      await fire('auth-code', 'code#state') // completes login → pushRealUsage
      await new Promise((r) => realSetTimeout(r, 5))
    }
    const base = { session: { pct: 0, resetMs: 1000 }, week: { pct: 0, resetMs: null } }
    notifications.length = 0
    authState.usage = { ...base, scoped: [{ label: 'Fable', pct: 82, resetMs: null }] }
    await poll()
    expect(notifications.length).toBe(1)
    expect(notifications[0].title).toBe('Fable weekly at 82%')
    await poll() // still above: stays quiet
    expect(notifications.length).toBe(1)
    authState.usage = base // no scoped limits at all: nothing to check
    await poll()
    expect(notifications.length).toBe(1)
    authState.usage = DEFAULT_USAGE // leave the shared fixture as we found it
    await poll()
  })
})

describe('window geometry', () => {
  test('resizing keeps the widget pinned by its bottom-right corner', () => {
    winMock.bounds = { x: 1000, y: 500, width: 276, height: 400 }
    const right = 1276
    const bottom = 900
    fire('resize', 276, 300) // shorter content
    expect(winMock.bounds.x + winMock.bounds.width).toBe(right)
    expect(winMock.bounds.y + winMock.bounds.height).toBe(bottom)
  })

  test('enforces a minimum size', () => {
    fire('resize', 10, 10)
    expect(winMock.bounds.width).toBe(100)
    expect(winMock.bounds.height).toBe(110)
  })

  test('restores the saved corner when that display is still connected', () => {
    fs.mkdirSync(DATA_DIR, { recursive: true })
    fs.writeFileSync(WINDOW_STATE, JSON.stringify({ right: 800, bottom: 600 }))
    winMock.bounds = { x: 0, y: 0, width: 276, height: 400 }
    winMock.visible = false
    fire('save-config', { mode: 'floating' }) // triggers applyMode → positionFloating
    expect(winMock.bounds.x).toBe(800 - 276)
    expect(winMock.bounds.y).toBe(600 - 400)
  })

  test('falls back to the primary display when the saved monitor is gone', () => {
    fs.writeFileSync(WINDOW_STATE, JSON.stringify({ right: 5000, bottom: 3000 })) // off-screen
    winMock.bounds = { x: 0, y: 0, width: 276, height: 400 }
    winMock.visible = false
    fire('save-config', { mode: 'floating' })
    expect(winMock.bounds.x).toBe(1920 - 276 - 24)
    expect(winMock.bounds.y).toBe(1080 - 400 - 24)
  })

  test('honours a second monitor for the saved corner', () => {
    displays = [
      { bounds: { x: 0, y: 0, width: 1920, height: 1080 } },
      { bounds: { x: 1920, y: 0, width: 1280, height: 1024 } },
    ]
    fs.writeFileSync(WINDOW_STATE, JSON.stringify({ right: 3000, bottom: 900 }))
    winMock.bounds = { x: 0, y: 0, width: 276, height: 400 }
    winMock.visible = false
    fire('save-config', { mode: 'floating' })
    expect(winMock.bounds.x).toBe(3000 - 276)
    displays = [{ bounds: { x: 0, y: 0, width: 1920, height: 1080 } }]
  })

  test('a corrupt window state is ignored rather than fatal', () => {
    fs.writeFileSync(WINDOW_STATE, 'not json')
    winMock.bounds = { x: 0, y: 0, width: 276, height: 400 }
    winMock.visible = false
    fire('save-config', { mode: 'floating' })
    expect(winMock.bounds.x).toBe(1920 - 276 - 24)
  })

  test('a user drag is saved, a programmatic move is not', () => {
    fs.rmSync(WINDOW_STATE, { force: true })
    winMock.bounds = { x: 300, y: 200, width: 276, height: 400 }
    winMock.handlers.get('moved')() // straight after a programmatic move → ignored
    expect(fs.existsSync(WINDOW_STATE)).toBe(false)
  })
})

describe('menu-bar mode', () => {
  test('switching to menubar creates the tray and hides the widget', () => {
    winMock.visible = true
    fire('save-config', { mode: 'menubar' })
    expect(trayState.instance).not.toBeNull()
    expect(winMock.visible).toBe(false)
  })

  test('the tray tooltip shows the live session %', () => {
    // sessionPct comes from the OAuth poll, pushed at startup (40%)
    expect(trayState.tooltip).toContain('40%')
  })

  test('the macOS tray title shows the %, turning to fire near the limit', () => {
    // only macOS renders text beside the icon, so pin the platform rather than
    // letting the result depend on which OS runs the suite
    const real = process.platform
    Object.defineProperty(process, 'platform', { value: 'darwin', configurable: true })
    try {
      fire('save-config', { mode: 'menubar', fireThreshold: 90 })
      expect(trayState.title).toBe(' 40%')
      fire('save-config', { mode: 'menubar', fireThreshold: 10 }) // 40% is now "hot"
      expect(trayState.title).toBe(' 40% 🔥')
    } finally {
      Object.defineProperty(process, 'platform', { value: real, configurable: true })
      fire('save-config', { mode: 'menubar', fireThreshold: 90 })
    }
  })

  test('clicking the tray toggles the popover under the icon', () => {
    winMock.visible = false
    trayState.handlers.get('click')({}, { x: 500, y: 0, width: 24, height: 22 })
    expect(winMock.visible).toBe(true)
    // centred under the icon
    expect(winMock.bounds.x).toBe(Math.round(500 + 12 - winMock.bounds.width / 2))
    trayState.handlers.get('click')({}, { x: 500, y: 0, width: 24, height: 22 })
    expect(winMock.visible).toBe(false)
  })

  test('the popover is kept on-screen when the icon sits at the edge', () => {
    winMock.visible = false
    trayState.handlers.get('click')({}, { x: 1910, y: 0, width: 24, height: 22 })
    expect(winMock.bounds.x).toBeLessThanOrEqual(1920 - winMock.bounds.width - 8)
  })

  test('losing focus dismisses the popover', () => {
    winMock.visible = true
    winMock.handlers.get('blur')()
    expect(winMock.visible).toBe(false)
  })

  test('the right-click menu offers the usage page and quit', () => {
    trayState.handlers.get('right-click')()
    const labels = trayState.menu.filter((i) => i.label).map((i) => i.label)
    expect(labels).toEqual(['Open Clauddy', 'Open Usage page', 'Quit Clauddy'])
    trayState.menu.find((i) => i.label === 'Open Usage page').click()
    expect(opened[0]).toContain('claude.ai/settings/usage')
    const before = quitCount
    trayState.menu.find((i) => i.label === 'Quit Clauddy').click()
    expect(quitCount).toBe(before + 1)
  })

  test('switching back to floating destroys the tray', () => {
    fire('save-config', { mode: 'floating' })
    expect(trayState.instance).toBeNull()
  })
})

describe('settings', () => {
  test('persists a patch and re-broadcasts the merged config', () => {
    fire('save-config', { fireThreshold: 75 })
    expect(JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8')).fireThreshold).toBe(75)
    expect(lastOf('config').fireThreshold).toBe(75)
  })

  test('keeps values that were saved earlier', () => {
    fire('save-config', { alerts: false })
    const saved = JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8'))
    expect(saved.fireThreshold).toBe(75) // from the previous test
    expect(saved.alerts).toBe(false)
    fire('save-config', { alerts: true, fireThreshold: 90 }) // restore defaults
  })

  test('opens the usage page on request', () => {
    fire('open-usage')
    expect(opened[0]).toContain('claude.ai/settings/usage')
  })

  test('persists zoom and re-broadcasts it', () => {
    fire('save-config', { zoom: 150 })
    expect(JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf8')).zoom).toBe(150)
    expect(lastOf('config').zoom).toBe(150)
    fire('save-config', { zoom: 100 }) // restore default for later tests
  })
})

describe('login', () => {
  test('opens the browser at the authorize URL', () => {
    fire('auth-start')
    expect(opened[0]).toContain('claude.ai/oauth/authorize')
  })

  test('a good code connects and pushes usage plus profile', async () => {
    authState.usageError = null
    await fire('auth-code', 'code#state')
    await new Promise((r) => realSetTimeout(r, 5))
    expect(lastOf('auth-result')).toEqual({ ok: true })
    expect(lastOf('auth-state')).toEqual({ connected: true })
    expect(lastOf('real-usage').session.pct).toBe(40)
    expect(lastOf('profile').email).toBe('a@b.com')
  })

  test('a throttled first fetch still counts as connected', async () => {
    authState.usageError = Object.assign(new Error('slow'), { status: 429 })
    await fire('auth-code', 'code#state')
    await new Promise((r) => realSetTimeout(r, 5))
    expect(lastOf('auth-result')).toEqual({ ok: true })
    authState.usageError = null
  })

  test('a bad code is reported and the token is discarded', async () => {
    authState.completeError = new Error('exchange 400')
    const before = authState.cleared
    await fire('auth-code', 'nope')
    await new Promise((r) => realSetTimeout(r, 5))
    expect(lastOf('auth-result').ok).toBe(false)
    expect(lastOf('auth-result').error).toContain('exchange 400')
    expect(authState.cleared).toBe(before + 1)
    authState.completeError = null
  })

  test('logging out clears the token and blanks the renderer', () => {
    const before = authState.cleared
    fire('auth-logout')
    expect(authState.cleared).toBe(before + 1)
    expect(lastOf('auth-state')).toEqual({ connected: false })
    expect(lastOf('profile')).toBeNull()
    expect(lastOf('real-usage')).toBeNull()
    authState.connected = true
  })
})

describe('update check', () => {
  const realFetch = globalThis.fetch
  const withTag = (tag, status = 200) => {
    globalThis.fetch = async () => ({
      ok: status === 200,
      status,
      json: async () => ({ tag_name: tag }),
    })
  }

  test('reports an available update', async () => {
    withTag('v1.6.0')
    await fire('check-updates')
    await new Promise((r) => realSetTimeout(r, 5))
    expect(sent.map((m) => m.payload?.state)).toContain('checking')
    expect(lastOf('update-status')).toEqual({ state: 'available', latest: 'v1.6.0' })
    globalThis.fetch = realFetch
  })

  test.each([
    ['v1.5.0', 'uptodate'],
    ['v1.4.9', 'uptodate'],
    ['v1.5.1', 'available'],
    ['v2.0.0', 'available'],
    ['v1.10.0', 'available'], // 10 > 5, not string-compared
  ])('%s against 1.5.0 → %s', async (tag, state) => {
    withTag(tag)
    await fire('check-updates')
    await new Promise((r) => realSetTimeout(r, 5))
    expect(lastOf('update-status').state).toBe(state)
    globalThis.fetch = realFetch
  })

  test('surfaces a failed check rather than hanging on "checking"', async () => {
    withTag('x', 500)
    await fire('check-updates')
    await new Promise((r) => realSetTimeout(r, 5))
    expect(lastOf('update-status').state).toBe('error')
    globalThis.fetch = realFetch
  })

  test('macOS runs the installer script and steps aside for it', () => {
    const real = process.platform
    Object.defineProperty(process, 'platform', { value: 'darwin', configurable: true })
    try {
      fire('do-update')
      expect(lastOf('update-status')).toEqual({ state: 'updating' })
      expect(spawned.at(-1).cmd).toBe('/bin/bash')
      expect(spawned.at(-1).args[1]).toContain('install.sh')
      expect(spawned.at(-1).opts.detached).toBe(true)
      // it quits ~1.5s later so the installer can replace the running .app
      expect(timers.timeouts.some((t) => t.fn)).toBe(true)
    } finally {
      Object.defineProperty(process, 'platform', { value: real, configurable: true })
    }
  })

  test('non-macOS is sent to the releases page instead', () => {
    const real = process.platform
    Object.defineProperty(process, 'platform', { value: 'win32', configurable: true })
    try {
      const before = spawned.length
      fire('do-update')
      expect(opened[0]).toContain('releases/latest')
      expect(spawned.length).toBe(before) // never spawns a shell off macOS
    } finally {
      Object.defineProperty(process, 'platform', { value: real, configurable: true })
    }
  })
})

describe('shutdown', () => {
  test('quits on request', () => {
    const before = quitCount
    fire('quit')
    expect(quitCount).toBe(before + 1)
  })
})
