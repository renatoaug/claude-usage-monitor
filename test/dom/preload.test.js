import { describe, expect, mock, test } from 'bun:test'

// preload.js is the whole contract between the renderer and main: every method
// here maps to a channel main.js listens on, or one it sends. Small file, but a
// typo in a channel name silently breaks a feature with no error anywhere — so
// the mapping is worth pinning down.
const exposed = {}
const sends = []
const listeners = []

mock.module('electron', () => ({
  contextBridge: {
    exposeInMainWorld: (name, api) => {
      exposed[name] = api
    },
  },
  ipcRenderer: {
    send: (channel, ...args) => sends.push({ channel, args }),
    on: (channel, cb) => listeners.push({ channel, cb }),
  },
}))

await import('../../preload.js')
const api = exposed.api

describe('the preload bridge', () => {
  test('exposes exactly one namespace on the window', () => {
    expect(Object.keys(exposed)).toEqual(['api'])
    expect(typeof api).toBe('object')
  })

  test.each([
    ['saveConfig', 'save-config', [{ alerts: false }]],
    ['resize', 'resize', [300, 400]],
    ['openUsage', 'open-usage', []],
    ['authStart', 'auth-start', []],
    ['authCode', 'auth-code', ['abc#def']],
    ['authLogout', 'auth-logout', []],
    ['accountSwitch', 'accounts-switch', ['acct-1']],
    ['accountAdd', 'accounts-add', []],
    ['accountRemove', 'accounts-remove', ['acct-1']],
    ['checkUpdates', 'check-updates', []],
    ['doUpdate', 'do-update', []],
    ['quit', 'quit', []],
  ])('%s() sends on "%s"', (method, channel, args) => {
    sends.length = 0
    api[method](...args)
    expect(sends).toEqual([{ channel, args }])
  })

  test.each([
    ['onUsage', 'usage'],
    ['onError', 'usage-error'],
    ['onConfig', 'config'],
    ['onRealUsage', 'real-usage'],
    ['onAuthState', 'auth-state'],
    ['onProfile', 'profile'],
    ['onAuthResult', 'auth-result'],
    ['onAuthPending', 'auth-pending'],
    ['onAccounts', 'accounts'],
    ['onDebugState', 'debug-state'],
    ['onVersion', 'version'],
    ['onUpdateStatus', 'update-status'],
  ])('%s() subscribes to "%s"', (method, channel) => {
    // the on* methods register lazily, when the renderer subscribes
    listeners.length = 0
    api[method](() => {})
    expect(listeners.map((l) => l.channel)).toEqual([channel])
  })

  test('listeners hand the renderer the payload, not the IPC event', () => {
    let got = null
    api.onUsage((d) => {
      got = d
    })
    const sub = listeners.filter((l) => l.channel === 'usage').at(-1)
    sub.cb({ senderId: 1 }, { session: { pct: 42 } })
    expect(got).toEqual({ session: { pct: 42 } })
  })

  test('exposes nothing beyond the documented surface', () => {
    // a stray export here would widen the renderer's reach into Electron
    expect(Object.keys(api).sort()).toEqual(
      [
        'accountAdd',
        'accountRemove',
        'accountSwitch',
        'authCode',
        'authLogout',
        'authStart',
        'checkUpdates',
        'doUpdate',
        'onAccounts',
        'onAuthPending',
        'onAuthResult',
        'onAuthState',
        'onConfig',
        'onDebugState',
        'onError',
        'onProfile',
        'onRealUsage',
        'onUpdateStatus',
        'onUsage',
        'onVersion',
        'openUsage',
        'quit',
        'resize',
        'saveConfig',
      ].sort(),
    )
  })
})
