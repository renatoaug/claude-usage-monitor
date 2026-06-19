const { app, BrowserWindow, ipcMain, screen, Notification, shell } = require('electron')
const path = require('node:path')
const fs = require('node:fs')
const os = require('node:os')
const { getUsage } = require('./usage')
const auth = require('./auth')

// data dir: kept outside the project folder so moving the repo doesn't break it
const DATA_DIR = path.join(os.homedir(), '.claude-usage-monitor')
// external config: edit without rebuilding the .app
const EXTERNAL_CONFIG = path.join(DATA_DIR, 'config.json')
// debug channel: `./pet <state>` writes here to force a state (dev only)
const DEBUG_FILE = path.join(DATA_DIR, 'debug.json')

let win
let pollTimer
let config
let doTick = null
const W = 276

function publicConfig(c) {
  return {
    plan: c.plan,
    sessionTokenBudget: c.sessionTokenBudget,
    weeklyTokenBudget: c.weeklyTokenBudget,
    weeklyAnchorIso: c.weeklyAnchorIso,
    alerts: c.alerts,
    alertThresholds: c.alertThresholds,
  }
}

function loadConfig() {
  const defaults = {
    plan: 'max5x',
    sessionTokenBudget: 630000000,
    weeklyTokenBudget: 3450000000,
    weeklyAnchorIso: null,
    alerts: true,
    alertThresholds: [80, 95],
    pollIntervalMs: 4000,
    activeThresholdMs: 8000,
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
            title: 'Claude Usage Monitor',
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
    resizable: false,
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
    watchDebug()
  })
}

// resize the window to fit the content
ipcMain.on('resize', (_e, w, h) => {
  if (!win || win.isDestroyed()) return
  const width = Math.max(100, Math.round(w))
  const height = Math.max(110, Math.round(h))
  win.setContentSize(width, height)
  const { workAreaSize } = screen.getPrimaryDisplay()
  win.setPosition(workAreaSize.width - width - 24, workAreaSize.height - height - 24)
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
    if (win && !win.isDestroyed()) win.webContents.send('real-usage', u)
  } catch (e) {
    if (e && e.status === 429) {
      usageBackoff = Math.min(usageBackoff * 2, 30 * 60 * 1000)
    } else if (e && e.status === 401) {
      auth.clear()
      if (win && !win.isDestroyed()) {
        win.webContents.send('auth-state', { connected: false })
        win.webContents.send('real-usage', null)
      }
    }
  }
  scheduleUsagePoll()
}
function startUsagePoll() {
  if (auth.isConnected()) pollUsage()
}

ipcMain.on('auth-start', () => shell.openExternal(auth.begin()))
ipcMain.on('auth-code', async (_e, code) => {
  const ok = () => {
    if (win && !win.isDestroyed()) {
      win.webContents.send('auth-state', { connected: true })
      win.webContents.send('auth-result', { ok: true })
    }
  }
  try {
    await auth.complete(code)
    usageBackoff = 5 * 60 * 1000
    try {
      const u = await auth.fetchUsage() // validate the token
      ok()
      if (win && !win.isDestroyed()) win.webContents.send('real-usage', u)
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
  if (win && !win.isDestroyed()) {
    win.webContents.send('auth-state', { connected: false })
    win.webContents.send('real-usage', null)
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
})

ipcMain.on('quit', () => app.quit())

app.whenReady().then(() => {
  createWindow()
  // open at login (packaged app only)
  if (app.isPackaged) {
    app.setLoginItemSettings({ openAtLogin: true, openAsHidden: false })
  }
})

app.on('window-all-closed', () => {
  if (pollTimer) clearInterval(pollTimer)
  fs.unwatchFile(DEBUG_FILE)
  app.quit()
})

if (process.platform === 'darwin' && app.dock) app.dock.hide()
