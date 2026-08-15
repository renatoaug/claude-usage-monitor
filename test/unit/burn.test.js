import { describe, expect, test } from 'bun:test'
import { createBurnTracker } from '../../renderer/burn.js'

const MIN = 60_000
const HOUR = 3_600_000
const T0 = 1_700_000_000_000 // fixed clock — the tracker takes `now` explicitly

// Build a trail ending at `tokens`, burning `tokPerHour`, spanning `mins`.
function trailOf(tracker, { tokens, tokPerHour, mins, everyMin = 0.5 }) {
  for (let m = mins; m >= 0; m -= everyMin) {
    tracker.note(tokens - tokPerHour * (m / 60), true, T0 - m * MIN)
  }
}

describe('slope fitting', () => {
  test('recovers a known burn rate', () => {
    const b = createBurnTracker()
    trailOf(b, { tokens: 30e6, tokPerHour: 24e6, mins: 20 })
    const tokPerHour = b.slope() * HOUR
    expect(tokPerHour).toBeGreaterThan(23.9e6)
    expect(tokPerHour).toBeLessThan(24.1e6)
  })

  test('is flat when no tokens are being written', () => {
    const b = createBurnTracker()
    trailOf(b, { tokens: 30e6, tokPerHour: 0, mins: 20 })
    expect(b.slope()).toBe(0)
  })
})

describe('projection branches', () => {
  // 25% used, 20M tokens => 800k tokens per 1%, 75 points left to burn
  const seed = (tokPerHour, mins = 20) => {
    const b = createBurnTracker()
    trailOf(b, { tokens: 20e6, tokPerHour, mins })
    return b
  }
  const PER_PCT = 20e6 / 25

  test('warns when you would run out before the reset', () => {
    const b = seed(30 * PER_PCT) // 30%/h => 75/30 = 2.5h
    const p = b.project(25, 4 * HOUR, 20e6)
    expect(p.kind).toBe('eta')
    expect(p.ms / HOUR).toBeCloseTo(2.5, 1)
  })

  test('reassures when the reset arrives first', () => {
    const b = seed(10 * PER_PCT) // 10%/h => 7.5h, well past a 4h reset
    expect(b.project(25, 4 * HOUR, 20e6)).toEqual({ kind: 'safe' })
  })

  test('treats the boundary as safe', () => {
    // exactly 75 points over 3h => eta == reset. Ties go to "safe": we only
    // warn when we're confident you'd actually run out first.
    const b = seed(25 * PER_PCT)
    expect(b.project(25, 3 * HOUR, 20e6).kind).toBe('safe')
  })
})

describe('stays quiet when it cannot speak honestly', () => {
  const full = () => {
    const b = createBurnTracker()
    trailOf(b, { tokens: 20e6, tokPerHour: 24e6, mins: 20 })
    return b
  }

  test('too few samples', () => {
    const b = createBurnTracker()
    b.note(10e6, true, T0 - 10 * MIN)
    b.note(12e6, true, T0 - 5 * MIN)
    b.note(14e6, true, T0)
    expect(b.trail.length).toBe(3)
    expect(b.project(25, 4 * HOUR, 20e6)).toBeNull()
  })

  test('span too short even with enough samples', () => {
    const b = createBurnTracker()
    trailOf(b, { tokens: 20e6, tokPerHour: 24e6, mins: 3, everyMin: 0.5 })
    expect(b.trail.length).toBeGreaterThanOrEqual(4)
    expect(b.project(25, 4 * HOUR, 20e6)).toBeNull()
  })

  test('flat pace', () => {
    const b = createBurnTracker()
    trailOf(b, { tokens: 20e6, tokPerHour: 0, mins: 20 })
    expect(b.project(25, 4 * HOUR, 20e6)).toBeNull()
  })

  test('no anchor to convert tokens into %', () => {
    expect(full().project(0, 4 * HOUR, 20e6)).toBeNull()
    expect(full().project(25, 4 * HOUR, 0)).toBeNull()
  })

  test('going inactive clears the trail', () => {
    const b = full()
    expect(b.trail.length).toBeGreaterThan(0)
    b.note(20e6, false, T0)
    expect(b.trail.length).toBe(0)
    expect(b.project(25, 4 * HOUR, 20e6)).toBeNull()
  })

  test('zero tokens clears the trail', () => {
    const b = full()
    b.note(0, true, T0)
    expect(b.trail.length).toBe(0)
  })
})

describe('sampling hygiene', () => {
  test('thins rapid polls down to one sample per interval', () => {
    const b = createBurnTracker()
    for (let i = 0; i < 50; i++) b.note(10e6 + i * 1000, true, T0 + i * 1000) // 1s apart
    expect(b.trail.length).toBe(2) // the first, then one 30s later
  })

  test('a rollover starts a fresh trail with the new sample', () => {
    const b = createBurnTracker()
    b.note(20e6, true, T0)
    b.note(21e6, true, T0 + 40_000)
    b.note(1e6, true, T0 + 50_000) // window rolled over: tokens dropped
    // the post-rollover sample must survive the 30s throttle, or the trail
    // stays empty for an extra cycle
    expect(b.trail).toEqual([{ t: T0 + 50_000, tokens: 1e6 }])
  })

  test('drops samples older than the fit window', () => {
    const b = createBurnTracker()
    b.note(1e6, true, T0 - 90 * MIN) // older than the 45min window
    b.note(2e6, true, T0 - 40 * MIN)
    b.note(3e6, true, T0 - 20 * MIN)
    b.note(4e6, true, T0)
    expect(b.trail.map((s) => s.tokens)).toEqual([2e6, 3e6, 4e6])
  })
})
