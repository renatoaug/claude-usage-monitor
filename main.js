const {
  app,
  BrowserWindow,
  ipcMain,
  screen,
  Notification,
  shell,
  Tray,
  Menu,
  nativeImage,
} = require('electron')
const path = require('node:path')
const fs = require('node:fs')
const os = require('node:os')
const { spawn } = require('node:child_process')
const { getUsage } = require('./usage')
const auth = require('./auth')

const REPO = 'renatoaug/claude-usage-monitor'

// data dir: kept outside the project folder so moving the repo doesn't break it.
// when CLAUDE_CONFIG_DIR is set (e.g. via direnv for multi-account setups), nest
// the widget's data under that account's Claude config dir so each account gets
// its own auth.json, config.json, and Electron userData.
const DATA_DIR = process.env.CLAUDE_CONFIG_DIR
  ? path.join(process.env.CLAUDE_CONFIG_DIR, 'usage-monitor')
  : path.join(os.homedir(), '.claude-usage-monitor')
// when isolating per-account, also pin Electron's userData under DATA_DIR so two
// widgets can run in parallel without fighting over cookies/cache. left untouched
// in the default case so existing installs keep their window position etc.
if (process.env.CLAUDE_CONFIG_DIR) {
  fs.mkdirSync(DATA_DIR, { recursive: true })
  app.setPath('userData', path.join(DATA_DIR, 'electron'))
}
// external config: edit without rebuilding the .app
const EXTERNAL_CONFIG = path.join(DATA_DIR, 'config.json')
// debug channel: `./pet <state>` writes here to force a state (dev only)
const DEBUG_FILE = path.join(DATA_DIR, 'debug.json')
// remembered floating-widget position: survives restarts and lets the widget
// return to the monitor the user parked it on after a display is unplugged/replugged
const WINDOW_STATE = path.join(DATA_DIR, 'window.json')

let win
let pollTimer
let config
let doTick = null
const W = 276

// menu-bar (tray) mode state
let tray = null
let currentMode = 'floating' // 'floating' widget | 'menubar' popover
let trayBounds = null // last known tray icon rect, to anchor the popover
let lastBlurHide = 0 // debounce: ignore the tray click that dismissed the popover
let sessionPct = null // authoritative session % shown in the tray title
let realUsage = null // last OAuth usage payload — the % alerts trust when logged in
let lastProgrammaticMove = 0 // ignore the 'moved' event our own setPosition triggers
let displayChanging = 0 // ignore OS window-shuffles while a display (dis)connects

function publicConfig(c) {
  return {
    plan: c.plan,
    sessionTokenBudget: c.sessionTokenBudget,
    weeklyTokenBudget: c.weeklyTokenBudget,
    weeklyAnchorIso: c.weeklyAnchorIso,
    mode: c.mode,
    alerts: c.alerts,
    alertThresholds: c.alertThresholds,
    fireThreshold: c.fireThreshold,
    zoom: c.zoom,
  }
}

function loadConfig() {
  const defaults = {
    plan: 'max5x',
    sessionTokenBudget: 630000000,
    weeklyTokenBudget: 3450000000,
    weeklyAnchorIso: null,
    mode: 'floating', // 'floating' widget or 'menubar' popover
    alerts: true,
    alertThresholds: [80, 95],
    fireThreshold: 90, // session % at which the pet catches fire (tired still fixed at 100)
    zoom: 100, // widget scale %, 100-200
    pollIntervalMs: 4000,
    activeThresholdMs: 20000,
    sleepThresholdMs: 300000,
  }
  for (const p of [EXTERNAL_CONFIG, path.join(__dirname, 'config.json')]) {
    try {
      return { ...defaults, ...JSON.parse(fs.readFileSync(p, 'utf8')) }
    } catch {}
  }
  return defaults
}

// ---- alerts ------------------------------------------------------------------
// One notification per (scope × threshold), re-armed when usage drops back below.
// `armed` is persisted so relaunching at 85% doesn't repeat the 80% alert you
// already dismissed; it also carries the last session % so a reset is detectable
// across restarts.
const ALERTS_FILE = path.join(DATA_DIR, 'alerts.json')
let armed = new Set()
let lastSessionPct = null
function loadAlertState() {
  try {
    const j = JSON.parse(fs.readFileSync(ALERTS_FILE, 'utf8'))
    armed = new Set(Array.isArray(j.armed) ? j.armed : [])
    lastSessionPct = typeof j.lastSessionPct === 'number' ? j.lastSessionPct : null
  } catch {} // first run, or a corrupted file: start from a clean slate
}
function saveAlertState() {
  try {
    fs.mkdirSync(DATA_DIR, { recursive: true })
    fs.writeFileSync(ALERTS_FILE, JSON.stringify({ armed: [...armed], lastSessionPct }))
  } catch {}
}

function fmtDuration(ms) {
  const h = Math.floor(ms / 3600000)
  const m = Math.floor((ms % 3600000) / 60000)
  return h > 0 ? `${h}h ${m}m` : `${m}m`
}
// wall-clock time the window flips, in the machine's own locale + timezone
function fmtClock(ms) {
  const at = new Date(Date.now() + ms)
  const time = at.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
  return ms >= 86400000 ? `${at.toLocaleDateString([], { weekday: 'short' })} ${time}` : time
}

function notify(title, body, { silent = true } = {}) {
  if (!Notification.isSupported()) return
  const n = new Notification({ title, body, silent })
  n.on('click', showWidget) // the obvious gesture: bring the pet to the front
  n.show()
}

// the alert lines carry the reset, since "what do I do about it" is a question
// about time left, not about the threshold that happened to fire
function resetLine(resetMs, withRemaining) {
  if (resetMs == null) return ''
  const at = `resets ${fmtClock(resetMs)}`
  return withRemaining && resetMs > 0 ? `${fmtDuration(resetMs)} left · ${at}` : at
}

// usage crossing a threshold. `scopes` are {label, pct, resetMs, session} rows;
// the real (OAuth) numbers are preferred over the local token estimate, which is
// only a budget guess and can be off by an order of magnitude.
function alertScopes(config, scopes) {
  if (!config.alerts) return
  const ths = config.alertThresholds || [80, 95]
  const top = Math.max(...ths)
  let dirty = false
  for (const { label, pct, resetMs, session } of scopes) {
    if (pct == null) continue
    for (const t of ths) {
      const key = `${label}:${t}`
      if (pct >= t) {
        if (!armed.has(key)) {
          armed.add(key)
          dirty = true
          const urgent = t === top
          notify(
            `${label} at ${Math.round(pct)}%${urgent ? ' — almost out' : ''}`,
            resetLine(resetMs, session),
            { silent: !urgent }, // 80% is a heads-up; the last threshold earns a sound
          )
        }
      } else if (armed.delete(key)) {
        dirty = true // re-arm when it drops below
      }
    }
  }
  if (dirty) saveAlertState()
}

function checkAlerts(config, d) {
  const r = realUsage // authoritative when logged in; the estimate is the fallback
  const session = r?.session ?? d.session
  const week = r?.week ?? d.week
  alertScopes(config, [
    { label: 'Session', pct: session.pct, resetMs: session.resetMs, session: true },
    { label: 'Weekly usage', pct: week.pct, resetMs: week.resetMs },
  ])
  checkWindowReset(config, session.pct)
}
// per-model weekly limits only exist on the account side, so they're checked
// off the OAuth poll rather than the local tick
function checkScopedAlerts(config, u) {
  alertScopes(
    config,
    (u?.scoped || []).map((s) => ({
      label: `${s.label} weekly`,
      pct: s.pct,
      resetMs: s.resetMs,
    })),
  )
}

// "you can work again" — only worth saying to someone who was actually near the
// ceiling, so a window flipping at 20% stays silent
const RESET_FROM = 80
const RESET_TO = 5
function checkWindowReset(config, pct) {
  if (pct == null) return
  const was = lastSessionPct
  lastSessionPct = pct
  if (config.alerts && was != null && was >= RESET_FROM && pct <= RESET_TO) {
    notify('Session window reset', 'full budget again')
  }
  if (was == null || Math.round(was) !== Math.round(pct)) saveAlertState()
}

// the token going dead is a silent failure otherwise: the widget quietly falls
// back to the local estimate and keeps showing a number the user trusts
function alertAuthLost(config) {
  if (!config.alerts) return
  notify('Clauddy lost access to your usage', 'open Settings to reconnect')
}

function createWindow() {
  config = loadConfig()
  loadAlertState()
  const { workAreaSize } = screen.getPrimaryDisplay()
  const H = 480

  win = new BrowserWindow({
    width: W,
    height: H,
    x: workAreaSize.width - W - 24,
    y: workAreaSize.height - H - 24,
    frame: false,
    transparent: true,
    backgroundColor: '#00000000', // fully transparent — Windows needs this or the window paints black
    resizable: false,
    show: false, // applyMode() reveals it (floating) or keeps it a hidden popover (menubar)
    alwaysOnTop: true,
    skipTaskbar: true,
    hasShadow: false,
    fullscreenable: false,
    icon: path.join(__dirname, 'build', 'icon.png'),
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  win.setAlwaysOnTop(true, 'floating')
  win.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true })
  win.loadFile(path.join(__dirname, 'renderer', 'index.html'))

  // in menu-bar mode the popover dismisses when it loses focus (click elsewhere)
  win.on('blur', () => {
    if (currentMode === 'menubar' && win.isVisible()) {
      win.hide()
      lastBlurHide = Date.now()
    }
  })

  // remember where the user parks the widget — but not the moves we make
  // ourselves (resize re-anchoring) nor the ones the OS forces when a display
  // (dis)connects, so a monitor going dark never overwrites the saved spot
  win.on('moved', () => {
    if (Date.now() - lastProgrammaticMove < 500) return
    if (Date.now() - displayChanging < 2000) return
    saveWindowState()
  })

  // when a monitor is unplugged/replugged (or its layout changes), put the
  // floating widget back on the display the user parked it on — the OS dumps
  // it on the primary display otherwise, and never moves it back on its own
  const onDisplayChange = () => {
    displayChanging = Date.now()
    if (currentMode === 'floating' && win && !win.isDestroyed() && win.isVisible())
      positionFloating()
  }
  screen.on('display-added', onDisplayChange)
  screen.on('display-removed', onDisplayChange)
  screen.on('display-metrics-changed', onDisplayChange)

  const tick = () => {
    if (!win || win.isDestroyed()) return
    try {
      const data = getUsage(config)
      win.webContents.send('usage', data)
      checkAlerts(config, data)
    } catch (err) {
      win.webContents.send('usage-error', String(err))
    }
  }
  doTick = tick

  win.webContents.once('did-finish-load', () => {
    tick()
    win.webContents.send('config', publicConfig(config))
    win.webContents.send('version', app.getVersion())
    win.webContents.send('auth-state', { connected: auth.isConnected() })
    pollTimer = setInterval(tick, config.pollIntervalMs)
    startUsagePoll()
    sendProfile()
    watchDebug()
    applyMode(config.mode) // reveal the widget, or set up the tray + popover
    setTimeout(autoUpdateCheck, 8000) // check once shortly after launch…
    setInterval(autoUpdateCheck, 6 * 60 * 60 * 1000) // …then every 6h
  })
}

// ---- menu-bar (tray) mode ----------------------------------------------------
// Floating mode: the widget lives bottom-right, always visible. Menu-bar mode:
// the same window becomes a popover shown under a tray icon on click. Switching
// is live — no relaunch — so the Settings toggle applies immediately.
function applyMode(mode) {
  const next = mode === 'menubar' ? 'menubar' : 'floating'
  const changed = next !== currentMode
  currentMode = next
  if (currentMode === 'menubar') {
    ensureTray()
    win.setSkipTaskbar(true)
    if (changed && win.isVisible()) win.hide() // hide only when switching in
  } else {
    destroyTray()
    if (changed || !win.isVisible()) {
      positionFloating()
      win.show()
    }
  }
  updateTray()
}

function ensureTray() {
  if (tray) return
  // macOS recolors a "template" (black+alpha) image for the light/dark menu
  // bar; Linux/Windows tray icons get no such recoloring, so they get the
  // pre-colored terracotta variant instead — a plain black icon disappears
  // on dark panels (e.g. Linux Mint's default Cinnamon taskbar).
  const isMac = process.platform === 'darwin'
  const iconFile = isMac ? 'trayTemplate.png' : 'trayColor.png'
  const img = nativeImage.createFromPath(path.join(__dirname, 'build', iconFile))
  if (isMac) img.setTemplateImage(true)
  tray = new Tray(img)
  tray.setToolTip('Clauddy')
  tray.on('click', (_e, bounds) => {
    trayBounds = bounds
    togglePopover()
  })
  tray.on('right-click', () => tray.popUpContextMenu(trayMenu()))
}

function destroyTray() {
  if (tray) {
    tray.destroy()
    tray = null
  }
}

function trayMenu() {
  return Menu.buildFromTemplate([
    { label: 'Open Clauddy', click: () => showPopover() },
    {
      label: 'Open Usage page',
      click: () => shell.openExternal('https://claude.ai/settings/usage'),
    },
    { type: 'separator' },
    { label: 'Quit Clauddy', click: () => app.quit() },
  ])
}

function togglePopover() {
  if (Date.now() - lastBlurHide < 250) return // this click just dismissed it
  if (win.isVisible()) win.hide()
  else showPopover()
}

function showPopover() {
  positionUnderTray()
  win.show()
  win.focus()
}

// bring the widget to the front from wherever it is, in either mode
function showWidget() {
  if (!win || win.isDestroyed()) return
  if (currentMode === 'menubar') showPopover()
  else {
    win.show()
    win.focus()
  }
}

// route every programmatic move through here so the 'moved' handler can tell
// our own repositioning apart from a genuine user drag (and skip saving it)
function moveWindow(x, y) {
  lastProgrammaticMove = Date.now()
  win.setPosition(Math.round(x), Math.round(y))
}

// persist the widget's anchor — its bottom-right corner, since the window's
// size changes as the pet animates — so it can be restored later
function saveWindowState() {
  if (currentMode !== 'floating' || !win || win.isDestroyed()) return
  const b = win.getBounds()
  try {
    fs.mkdirSync(DATA_DIR, { recursive: true })
    fs.writeFileSync(WINDOW_STATE, JSON.stringify({ right: b.x + b.width, bottom: b.y + b.height }))
  } catch {}
}

function loadWindowState() {
  try {
    const s = JSON.parse(fs.readFileSync(WINDOW_STATE, 'utf8'))
    return Number.isFinite(s?.right) && Number.isFinite(s?.bottom) ? s : null
  } catch {
    return null
  }
}

// is the saved bottom-right corner on a display that's currently connected?
function cornerVisible(right, bottom) {
  const x = Math.round(right - 1)
  const y = Math.round(bottom - 1)
  return screen.getAllDisplays().some((d) => {
    const b = d.bounds
    return x >= b.x && x < b.x + b.width && y >= b.y && y < b.y + b.height
  })
}

// center the popover under the tray icon, kept on-screen
function positionUnderTray() {
  const b = win.getBounds()
  const { workAreaSize } = screen.getPrimaryDisplay()
  let x = workAreaSize.width - b.width - 24
  let y = 24
  if (trayBounds?.width) {
    x = Math.round(trayBounds.x + trayBounds.width / 2 - b.width / 2)
    y = Math.round(trayBounds.y + trayBounds.height)
  }
  x = Math.max(8, Math.min(x, workAreaSize.width - b.width - 8))
  moveWindow(x, y)
}

function positionFloating() {
  const b = win.getBounds()
  // reuse the saved spot when that monitor is still connected; otherwise fall
  // back to the primary display's bottom-right corner
  const saved = loadWindowState()
  if (saved && cornerVisible(saved.right, saved.bottom)) {
    moveWindow(saved.right - b.width, saved.bottom - b.height)
    return
  }
  const { workAreaSize } = screen.getPrimaryDisplay()
  moveWindow(workAreaSize.width - b.width - 24, workAreaSize.height - b.height - 24)
}

// the tray shows the live session % (macOS title), turning 🔥 near the limit
function updateTray() {
  if (!tray) return
  if (sessionPct == null) {
    if (process.platform === 'darwin') tray.setTitle('')
    tray.setToolTip('Clauddy — connect your account for live %')
    return
  }
  const pct = Math.round(sessionPct)
  const hot = pct >= (config?.fireThreshold ?? 90)
  if (process.platform === 'darwin') tray.setTitle(hot ? ` ${pct}% 🔥` : ` ${pct}%`)
  tray.setToolTip(`Clauddy — session ${pct}%`)
}

// send real usage to the renderer and refresh the tray title in one place
function pushRealUsage(u) {
  realUsage = u
  sessionPct = u?.session ? u.session.pct : null
  updateTray()
  checkScopedAlerts(config, u)
  if (win && !win.isDestroyed()) win.webContents.send('real-usage', u)
}

// resize the window to fit the content
ipcMain.on('resize', (_e, w, h) => {
  if (!win || win.isDestroyed()) return
  const width = Math.max(100, Math.round(w))
  const height = Math.max(110, Math.round(h))
  if (currentMode === 'menubar') {
    win.setContentSize(width, height)
    if (win.isVisible()) positionUnderTray() // keep it anchored under the tray
  } else {
    // keep the widget pinned to its current bottom-right corner on whatever
    // display the user dragged it to. setContentSize grows from the top-left
    // origin, so re-anchor by the old corner instead of snapping to the primary
    // display's bottom-right (which yanked the widget back on every update).
    const before = win.getBounds()
    const right = before.x + before.width
    const bottom = before.y + before.height
    win.setContentSize(width, height)
    const after = win.getBounds()
    moveWindow(right - after.width, bottom - after.height)
  }
})

ipcMain.on('open-usage', () => shell.openExternal('https://claude.ai/settings/usage'))

// watch the debug file; forward forced states to the renderer
function watchDebug() {
  fs.watchFile(DEBUG_FILE, { interval: 400 }, () => {
    if (!win || win.isDestroyed()) return
    try {
      const txt = fs.readFileSync(DEBUG_FILE, 'utf8').trim()
      win.webContents.send('debug-state', txt ? JSON.parse(txt) : null)
    } catch {}
  })
}

// ---- real usage via OAuth (authoritative %), polled slowly with 429 backoff ----
let usageTimer = null
let usageBackoff = 5 * 60 * 1000
function scheduleUsagePoll() {
  clearTimeout(usageTimer)
  if (auth.isConnected()) usageTimer = setTimeout(pollUsage, usageBackoff)
}
async function pollUsage() {
  try {
    const u = await auth.fetchUsage()
    usageBackoff = 5 * 60 * 1000
    pushRealUsage(u)
  } catch (e) {
    if (e && e.status === 429) {
      usageBackoff = Math.min(usageBackoff * 2, 30 * 60 * 1000)
    } else if (e && e.status === 401) {
      auth.clear()
      pushRealUsage(null)
      alertAuthLost(config)
      if (win && !win.isDestroyed()) {
        win.webContents.send('auth-state', { connected: false })
        win.webContents.send('profile', null)
      }
    }
  }
  scheduleUsagePoll()
}
function startUsagePoll() {
  if (auth.isConnected()) pollUsage()
}

// push the logged-in account's identity (email + plan) to the renderer
async function sendProfile() {
  if (!auth.isConnected()) return
  try {
    const p = await auth.fetchProfile()
    if (win && !win.isDestroyed()) win.webContents.send('profile', p)
  } catch {} // non-fatal: the chip just stays hidden
}

ipcMain.on('auth-start', () => shell.openExternal(auth.begin()))
ipcMain.on('auth-code', async (_e, code) => {
  const ok = () => {
    if (win && !win.isDestroyed()) {
      win.webContents.send('auth-state', { connected: true })
      win.webContents.send('auth-result', { ok: true })
    }
    sendProfile()
  }
  try {
    await auth.complete(code)
    usageBackoff = 5 * 60 * 1000
    try {
      const u = await auth.fetchUsage() // validate the token
      ok()
      pushRealUsage(u)
    } catch (e) {
      if (e && e.status === 429) {
        // token is fine, the usage endpoint is just throttled — keep it and retry later
        ok()
      } else {
        throw e
      }
    }
    scheduleUsagePoll()
  } catch (err) {
    auth.clear() // don't keep an invalid token
    if (win && !win.isDestroyed())
      win.webContents.send('auth-result', { ok: false, error: String(err?.message || err) })
  }
})
ipcMain.on('auth-logout', () => {
  auth.clear()
  clearTimeout(usageTimer)
  pushRealUsage(null)
  if (win && !win.isDestroyed()) {
    win.webContents.send('auth-state', { connected: false })
    win.webContents.send('profile', null)
  }
})

ipcMain.on('save-config', (_e, patch) => {
  let obj = {}
  for (const p of [EXTERNAL_CONFIG, path.join(__dirname, 'config.json')]) {
    try {
      obj = JSON.parse(fs.readFileSync(p, 'utf8'))
      break
    } catch {}
  }
  Object.assign(obj, patch)
  try {
    fs.mkdirSync(path.dirname(EXTERNAL_CONFIG), { recursive: true })
    fs.writeFileSync(EXTERNAL_CONFIG, JSON.stringify(obj, null, 2))
  } catch {}
  const prevThresholds = String(config.alertThresholds)
  config = loadConfig()
  // re-arm only when the thresholds actually moved: saving an unrelated setting
  // (zoom, mode…) shouldn't replay an alert the user already dismissed
  if (String(config.alertThresholds) !== prevThresholds) {
    armed.clear()
    saveAlertState()
  }
  if (doTick) doTick()
  if (win && !win.isDestroyed()) win.webContents.send('config', publicConfig(config))
  applyMode(config.mode) // switch between floating widget and menu-bar popover live
})

// ---- self-update (checks the latest GitHub release, runs install.sh) ---------
async function fetchLatestTag() {
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { 'User-Agent': 'Clauddy', Accept: 'application/vnd.github+json' },
  })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  const j = await res.json()
  return j.tag_name // e.g. "v1.9.1"
}
// compare two "1.2.3" versions; >0 if a is newer than b
function cmpVer(a, b) {
  const pa = String(a).replace(/^v/, '').split('.').map(Number)
  const pb = String(b).replace(/^v/, '').split('.').map(Number)
  for (let i = 0; i < 3; i++) {
    const d = (pa[i] || 0) - (pb[i] || 0)
    if (d) return d > 0 ? 1 : -1
  }
  return 0
}
async function computeUpdate() {
  const latest = await fetchLatestTag()
  return { latest, available: cmpVer(latest, app.getVersion()) > 0 }
}
// manual check (from the Settings button): shows checking / result / error
ipcMain.on('check-updates', async () => {
  if (!win || win.isDestroyed()) return
  const send = (s) => !win.isDestroyed() && win.webContents.send('update-status', s)
  send({ state: 'checking' })
  try {
    const u = await computeUpdate()
    send(u.available ? { state: 'available', latest: u.latest } : { state: 'uptodate' })
  } catch (e) {
    send({ state: 'error', message: String(e?.message || e) })
  }
})
// silent background check: only speaks up when there's actually an update, so
// the renderer can badge the gear without any UI churn on the common case
async function autoUpdateCheck() {
  if (!win || win.isDestroyed()) return
  try {
    const u = await computeUpdate()
    if (u.available) win.webContents.send('update-status', { state: 'available', latest: u.latest })
  } catch {} // offline / rate-limited: stay quiet, try again on the next tick
}
ipcMain.on('do-update', () => {
  // Windows/Linux have no install.sh: send them to the releases page instead.
  if (process.platform !== 'darwin') {
    shell.openExternal(`https://github.com/${REPO}/releases/latest`)
    return
  }
  if (win && !win.isDestroyed()) win.webContents.send('update-status', { state: 'updating' })
  const cmd = `curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash`
  const child = spawn('/bin/bash', ['-lc', cmd], { detached: true, stdio: 'ignore' })
  child.unref()
  // quit so the installer can replace the running .app and relaunch the new build
  setTimeout(() => app.quit(), 1500)
})

ipcMain.on('quit', () => app.quit())

// Electron's login-item API only covers macOS (SMAppService) and Windows (the
// registry Run key) — @platform darwin,win32 in electron.d.ts, a no-op on
// Linux. The real "start with the system" mechanism there is the XDG
// Autostart spec: a .desktop file in ~/.config/autostart, read by every major
// desktop environment's session manager at login (GNOME, KDE, XFCE, Cinnamon,
// MATE) — same role as the registry key or SMAppService, just file-based.
function enableLinuxAutostart() {
  const exec = process.env.APPIMAGE || process.execPath
  const autostartDir = path.join(os.homedir(), '.config', 'autostart')
  const desktopFile = path.join(autostartDir, 'clauddy.desktop')
  // AppImage isn't registered in the system's hicolor icon theme, so a bare
  // "Icon=clauddy" name won't resolve — copy the icon to a path that outlives
  // the AppImage's temp mount and reference it absolutely instead.
  const iconFile = path.join(DATA_DIR, 'icon.png')
  fs.mkdirSync(DATA_DIR, { recursive: true })
  fs.copyFileSync(path.join(__dirname, 'build', 'icon.png'), iconFile)
  const entry = [
    '[Desktop Entry]',
    'Type=Application',
    'Name=Clauddy',
    'Comment=A cute pixel-art desktop pet that tracks your Claude Code usage',
    `Exec="${exec}"`,
    `Icon=${iconFile}`,
    'Terminal=false',
    'X-GNOME-Autostart-enabled=true',
    '',
  ].join('\n')
  if (fs.existsSync(desktopFile) && fs.readFileSync(desktopFile, 'utf8') === entry) return
  fs.mkdirSync(autostartDir, { recursive: true })
  fs.writeFileSync(desktopFile, entry)
}

app.whenReady().then(() => {
  // Windows toast notifications need an explicit AppUserModelID to show reliably
  if (process.platform === 'win32') app.setAppUserModelId('app.clauddy')
  createWindow()
  // open at login (packaged app only)
  if (app.isPackaged) {
    if (process.platform === 'linux') {
      enableLinuxAutostart()
    } else {
      app.setLoginItemSettings({ openAtLogin: true, openAsHidden: false })
    }
  }
})

app.on('window-all-closed', () => {
  if (pollTimer) clearInterval(pollTimer)
  fs.unwatchFile(DEBUG_FILE)
  destroyTray()
  app.quit()
})

if (process.platform === 'darwin' && app.dock) app.dock.hide()
