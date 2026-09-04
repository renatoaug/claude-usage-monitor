import { beforeEach, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

// accounts.js resolves its store at import time from CLAUDE_CONFIG_DIR, so the
// fixture root has to exist and be exported before it loads.
const ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-accounts-'))
process.env.CLAUDE_CONFIG_DIR = ROOT
const BASE = path.join(ROOT, 'usage-monitor')
const FILE = path.join(BASE, 'accounts.json')

function write(obj) {
  fs.mkdirSync(BASE, { recursive: true })
  fs.writeFileSync(FILE, JSON.stringify(obj))
}
function read() {
  return JSON.parse(fs.readFileSync(FILE, 'utf8'))
}
const a = await import('../../accounts.js')

beforeEach(() => {
  fs.rmSync(BASE, { recursive: true, force: true })
})

describe('accounts', () => {
  test('starts with a single default account and writes nothing', () => {
    expect(a.list()).toEqual([{ id: 'default', label: null, claudeDir: null }])
    expect(a.activeId()).toBe('default')
    // reading must not create the file — an untouched install stays untouched
    expect(fs.existsSync(FILE)).toBe(false)
  })

  test('the default account keeps the original data dir', () => {
    expect(a.dataDirOf('default')).toBe(BASE)
    expect(a.dataDirOf('acct-2')).toBe(path.join(BASE, 'accounts', 'acct-2'))
  })

  test('add() creates a logged-out account and persists it', () => {
    const id = a.add()
    expect(id).not.toBe('default')
    expect(a.list()).toHaveLength(2)
    expect(read().accounts.at(-1)).toEqual({ id, label: null, claudeDir: null })
  })

  test('two adds in the same millisecond still get distinct ids', () => {
    // the second id used to collide with the first, and a collided id reads as
    // "the account you are already on" — the switch, and the login, went nowhere
    const ids = new Set([a.add(), a.add(), a.add()])
    expect(ids.size).toBe(3)
    expect(a.list()).toHaveLength(4)
  })

  test('setActive() only accepts an account that exists', () => {
    const id = a.add()
    expect(a.setActive('nope')).toBe(null)
    expect(a.activeId()).toBe('default')
    expect(a.setActive(id).id).toBe(id)
    expect(read().active).toBe(id)
  })

  test('label() names an account', () => {
    const id = a.add()
    a.label(id, 'work@example.com')
    expect(read().accounts.at(-1).label).toBe('work@example.com')
    expect(a.list().find((x) => x.id === id).label).toBe('work@example.com')
  })

  test('remove() drops the account and its token', () => {
    const id = a.add()
    const dir = a.dataDirOf(id)
    fs.mkdirSync(dir, { recursive: true })
    fs.writeFileSync(path.join(dir, 'auth.json'), '{}')

    expect(a.remove(id)).toBe(true)
    expect(fs.existsSync(dir)).toBe(false) // a stray token nobody can revoke
    expect(a.remove('gone')).toBe(false)
  })

  test('the account in use cannot be removed out from under the widget', () => {
    const id = a.add()
    a.setActive(id)
    expect(a.remove(id)).toBe(false)
    expect(a.list()).toHaveLength(2)
  })

  test('the last account standing cannot be removed', () => {
    expect(a.remove('default')).toBe(false)
    expect(a.list()).toHaveLength(1)
  })

  test('the default account goes too, keeping the files that are not its own', () => {
    const id = a.add()
    a.setActive(id)
    fs.writeFileSync(path.join(BASE, 'auth.json'), '{}')
    fs.writeFileSync(path.join(BASE, 'config.json'), '{}')
    expect(a.remove('default')).toBe(true)
    expect(a.list().map((x) => x.id)).toEqual([id])
    expect(fs.existsSync(path.join(BASE, 'auth.json'))).toBe(false) // its token
    expect(fs.existsSync(path.join(BASE, 'config.json'))).toBe(true) // everyone's settings
  })

  test('a corrupted or half-written store falls back to a usable state', () => {
    write({ active: 'ghost', accounts: [{ id: 'work' }, null, { label: 'no id' }] })
    // junk rows are dropped, and an unknown active id can't leave the widget
    // pointing at nothing
    expect(a.list().map((x) => x.id)).toEqual(['work'])
    expect(a.activeId()).toBe('work')

    fs.writeFileSync(FILE, 'not json')
    expect(a.list()).toHaveLength(1)
  })

  test('pruneOrphans() drops folders no account owns, never one with a token', () => {
    const keep = a.add()
    const dir = (id) => path.join(BASE, 'accounts', id)
    fs.mkdirSync(dir(keep), { recursive: true })
    fs.mkdirSync(dir('acct-gone'), { recursive: true })
    fs.writeFileSync(path.join(dir('acct-gone'), 'alerts.json'), '{}')
    fs.mkdirSync(dir('acct-with-token'), { recursive: true })
    fs.writeFileSync(path.join(dir('acct-with-token'), 'auth.json'), '{}')

    expect(a.pruneOrphans()).toBe(1)
    expect(fs.existsSync(dir(keep))).toBe(true) // still listed
    expect(fs.existsSync(dir('acct-gone'))).toBe(false)
    expect(fs.existsSync(dir('acct-with-token'))).toBe(true) // credentials stay
  })

  test('pruneOrphans() is a no-op when no extra account was ever added', () => {
    expect(a.pruneOrphans()).toBe(0)
  })

  test('setClaudeDir() points an account at its own logs', () => {
    const id = a.add('/tmp/claude-work')
    expect(a.list().find((x) => x.id === id).claudeDir).toBe('/tmp/claude-work')
    a.setClaudeDir(id, null)
    expect(read().accounts.at(-1).claudeDir).toBe(null)
  })
})
