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
const { getUsage } = require('./usage')
const auth = require('./auth')

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

// native notification when usage crosses a threshold
const armed = new Set()
function checkAlerts(config, d) {
  if (!config.alerts || !Notification.isSupported()) return
  const ths = config.alertThresholds || [80, 95]
  const scopes = [
    ['session', d.session.pct],
    ['weekly usage', d.week.pct],
  ]
  for (const [name, pct] of scopes) {
    for (const t of ths) {
      const key = `${name}:${t}`
      if (pct >= t) {
        if (!armed.has(key)) {
          armed.add(key)
          new Notification({
            title: 'Clauddy',
            body: `Your ${name} is over ${t}% — now at ${Math.round(pct)}%`,
            silent: false,
          }).show()
        }
      } else {
        armed.delete(key) // re-arm when it drops below
      }
    }
  }
}

function createWindow() {
  config = loadConfig()
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
    win.webContents.send('auth-state', { connected: auth.isConnected() })
    pollTimer = setInterval(tick, config.pollIntervalMs)
    startUsagePoll()
    sendProfile()
    watchDebug()
    applyMode(config.mode) // reveal the widget, or set up the tray + popover
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
  win.setPosition(x, y)
}

function positionFloating() {
  const { workAreaSize } = screen.getPrimaryDisplay()
  const b = win.getBounds()
  win.setPosition(workAreaSize.width - b.width - 24, workAreaSize.height - b.height - 24)
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
  sessionPct = u?.session ? u.session.pct : null
  updateTray()
  if (win && !win.isDestroyed()) win.webContents.send('real-usage', u)
}

// resize the window to fit the content
ipcMain.on('resize', (_e, w, h) => {
  if (!win || win.isDestroyed()) return
  const width = Math.max(100, Math.round(w))
  const height = Math.max(110, Math.round(h))
  win.setContentSize(width, height)
  if (currentMode === 'menubar') {
    if (win.isVisible()) positionUnderTray() // keep it anchored under the tray
  } else {
    const { workAreaSize } = screen.getPrimaryDisplay()
    win.setPosition(workAreaSize.width - width - 24, workAreaSize.height - height - 24)
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
  config = loadConfig()
  armed.clear() // re-arm alerts with new thresholds
  if (doTick) doTick()
  if (win && !win.isDestroyed()) win.webContents.send('config', publicConfig(config))
  applyMode(config.mode) // switch between floating widget and menu-bar popover live
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
