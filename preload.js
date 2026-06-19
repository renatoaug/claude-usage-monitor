const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('api', {
  onUsage: (cb) => ipcRenderer.on('usage', (_e, data) => cb(data)),
  onError: (cb) => ipcRenderer.on('usage-error', (_e, msg) => cb(msg)),
  onConfig: (cb) => ipcRenderer.on('config', (_e, cfg) => cb(cfg)),
  saveConfig: (patch) => ipcRenderer.send('save-config', patch),
  resize: (w, h) => ipcRenderer.send('resize', w, h),
  openUsage: () => ipcRenderer.send('open-usage'),
  onRealUsage: (cb) => ipcRenderer.on('real-usage', (_e, u) => cb(u)),
  onAuthState: (cb) => ipcRenderer.on('auth-state', (_e, s) => cb(s)),
  onAuthResult: (cb) => ipcRenderer.on('auth-result', (_e, r) => cb(r)),
  authStart: () => ipcRenderer.send('auth-start'),
  authCode: (code) => ipcRenderer.send('auth-code', code),
  authLogout: () => ipcRenderer.send('auth-logout'),
  onDebugState: (cb) => ipcRenderer.on('debug-state', (_e, s) => cb(s)),
  quit: () => ipcRenderer.send('quit'),
})
