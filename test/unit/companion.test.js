import { describe, expect, test } from 'bun:test'
import { createReminders } from '../../reminders'
import { createActivityTracker, currentActivity } from '../../renderer/companion'

describe('one-shot reset reminders', () => {
  function setup(initial = null) {
    let clock = 100000
    let disk = initial
    const deps = {
      now: () => clock,
      load: () => disk,
      save: (data) => {
        disk = structuredClone(data)
      },
    }
    return {
      store: createReminders(deps),
      advance: (ms) => {
        clock += ms
      },
      restart: () => createReminders(deps),
      now: () => clock,
    }
  }
  const reminder = (key, at) => ({
    key,
    provider: key === 'codex' ? 'codex' : 'claude',
    label: key,
    at,
    pct: 95,
  })
  test('survives restart and fires once after sleep, isolated by account', () => {
    const s = setup()
    s.store.arm(reminder('claude:a', s.now() + 1000))
    s.store.arm(reminder('claude:b', s.now() + 5000))
    s.store.arm(reminder('codex', s.now() + 5000))
    expect(s.store.takeDue()).toEqual([])
    s.advance(3000)
    const reboot = s.restart()
    expect(reboot.takeDue().map((r) => r.key)).toEqual(['claude:a'])
    expect(s.restart().takeDue()).toEqual([])
    expect(
      s
        .restart()
        .list()
        .map((r) => r.key),
    ).toEqual(['claude:b', 'codex'])
    expect(reboot.recentlyDelivered('claude:a')).toBe(true)
    expect(reboot.recentlyDelivered('claude:b')).toBe(false)
    s.advance(3600001)
    expect(reboot.recentlyDelivered('claude:a')).toBe(false)
  })
  test('rearming replaces only that account; cancellation survives restart', () => {
    const s = setup()
    s.store.arm(reminder('codex', s.now() + 1000))
    s.store.arm(reminder('codex', s.now() + 2000))
    s.store.cancel('missing')
    expect(s.store.list()).toHaveLength(1)
    s.store.cancel('codex')
    s.advance(3000)
    expect(s.restart().takeDue()).toEqual([])
  })
  test('rejects invalid deadlines and corrupted records', () => {
    const s = setup({ pending: [null, { at: 'tomorrow' }], delivered: { bad: 'oops' } })
    expect(s.store.list()).toEqual([])
    for (const at of [NaN, Infinity, s.now(), s.now() + 8 * 86400000]) {
      expect(s.store.arm(reminder('codex', at))).toBe(false)
    }
    const broken = createReminders({
      load: () => {
        throw new Error('bad JSON')
      },
      save: () => {},
    })
    expect(broken.list()).toEqual([])
  })
  test('failed persistence never silently arms or consumes a reminder', () => {
    let fail = false
    let clock = 1
    const s = createReminders({
      load: () => null,
      now: () => clock,
      save: () => {
        if (fail) throw new Error('disk full')
      },
    })
    fail = true
    expect(() => s.arm(reminder('codex', 100))).toThrow('disk full')
    expect(s.list()).toEqual([])
    fail = false
    s.arm(reminder('codex', 100))
    fail = true
    clock = 101
    expect(() => s.takeDue()).toThrow('disk full')
    expect(s.list()).toHaveLength(1)
    fail = false
    expect(s.takeDue()).toHaveLength(1)
  })
})

describe('provider reactions', () => {
  test('baselines, simultaneous activity, cooldown and disconnects', () => {
    let now = 0
    const t = createActivityTracker({ now: () => now, cooldown: 30 })
    expect(t.observe('claude', { active: true, activity: 'reading' })).toBeNull()
    expect(t.observe('codex', { active: false })).toBeNull()
    expect(t.observe('codex', { active: true })).toEqual({
      provider: 'codex',
      activity: 'working',
    })
    expect(t.observe('claude', { active: true, activity: 'editing' })).toBeNull()
    now = 31
    expect(t.observe('claude', { active: true, activity: 'editing' })).toBeNull()
    expect(t.observe('codex', { active: false })).toEqual({
      provider: 'codex',
      activity: 'paused',
    })
    t.forget('codex')
    now = 100
    expect(t.observe('codex', { active: true })).toBeNull()
    expect(t.observe('claude', null)).toBeNull()
    expect(t.observe('claude', { active: true })).toBeNull()
  })
})

describe('current workers', () => {
  test('uses activity timestamps independently of old quota readings', () => {
    const now = 1_000_000
    const claude = { active: true, ts: now, activity: 'editing' }
    const codex = { active: true, lastSeen: now, limitsAt: 1 }
    expect(currentActivity(claude, null, now).activity).toBe('editing')
    expect(currentActivity(claude, codex, now)).toMatchObject({
      providers: ['claude', 'codex'],
      activity: 'working',
    })
    expect(currentActivity(claude, codex, now + 60001).providers).toEqual([])
    expect(currentActivity({ active: true }, { active: true }, now).providers).toEqual([])
    expect(currentActivity(null, codex, now).providers).toEqual(['codex'])
    expect(currentActivity({ active: true, ts: now }, null, now).activity).toBe('working')
  })
  test('sleeps only when connected sources have evidence of rest', () => {
    const now = 1_000_000
    expect(currentActivity(null, null, now).sleeping).toBe(false)
    expect(currentActivity({ sleeping: true }, { lastSeen: null }, now).sleeping).toBe(false)
    expect(currentActivity({ sleeping: true }, { lastSeen: 1 }, now).sleeping).toBe(true)
    expect(currentActivity({ sleeping: false }, null, now).sleeping).toBe(false)
  })
})
