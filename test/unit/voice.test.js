import { describe, expect, test } from 'bun:test'
import {
  BASE_HZ,
  blipPlan,
  codexFireLine,
  codexMaxedLine,
  codexResetLine,
  createBlipper,
  fireLine,
  greetingLine,
  MAX_BLIPS,
  maxedLine,
  pitchOf,
  recapLine,
  recordLine,
  resetLine,
  SPREAD_HZ,
  STEP_MS,
  streakLine,
  welcomeLine,
} from '../../renderer/voice.js'

const fmt = (n) => `${n / 1e6}M`
const dur = (ms) => `${Math.round(ms / 60000)}m`

// a recording stand-in for Web Audio: just enough surface for the blipper
function fakeCtx() {
  const ctx = {
    state: 'suspended',
    currentTime: 10,
    destination: {},
    oscs: [],
    resume() {
      ctx.state = 'running'
    },
    createOscillator() {
      const o = {
        hz: [],
        frequency: { setValueAtTime: (v) => o.hz.push(v) },
        connect() {},
        start(t) {
          o.at = t
        },
        stop() {},
      }
      ctx.oscs.push(o)
      return o
    },
    createGain() {
      const ramp = () => {}
      return {
        gain: {
          setValueAtTime: ramp,
          linearRampToValueAtTime: ramp,
          exponentialRampToValueAtTime: ramp,
        },
        connect() {},
      }
    },
  }
  return ctx
}

describe('pitch', () => {
  test('letters span the range, a lowest and z highest', () => {
    expect(pitchOf('a')).toBe(BASE_HZ)
    expect(pitchOf('Z')).toBe(BASE_HZ + SPREAD_HZ)
    expect(pitchOf('0')).toBe(BASE_HZ)
    expect(pitchOf('9')).toBe(BASE_HZ + SPREAD_HZ)
  })

  test('anything else is a rest', () => {
    expect(pitchOf(' ')).toBeNull()
    expect(pitchOf('%')).toBeNull()
  })
})

describe('the blip schedule', () => {
  test('one note per letter, a step apart', () => {
    const plan = blipPlan('abc')
    expect(plan.map((b) => b.at)).toEqual([0, STEP_MS, 2 * STEP_MS])
    expect(plan[0].hz).toBe(BASE_HZ)
  })

  test('a long line is capped to a babble', () => {
    expect(blipPlan('x'.repeat(80))).toHaveLength(MAX_BLIPS)
  })

  test('punctuation and spaces breathe', () => {
    const plain = blipPlan('ab')[1].at
    expect(blipPlan('a b')[1].at).toBeGreaterThan(plain)
    expect(blipPlan('a. b')[1].at).toBeGreaterThan(blipPlan('a b')[1].at)
  })

  test('on fire it talks faster and higher; sleepy, slower and lower', () => {
    const [, n] = blipPlan('aa')
    const [, f] = blipPlan('aa', 'fire')
    const [, s] = blipPlan('aa', 'sleepy')
    expect(f.at).toBeLessThan(n.at)
    expect(f.hz).toBeGreaterThan(n.hz)
    expect(s.at).toBeGreaterThan(n.at)
    expect(s.hz).toBeLessThan(n.hz)
    expect(blipPlan('aa', 'nope')).toEqual(blipPlan('aa'))
  })
})

describe('the blipper', () => {
  test('builds the context once, wakes it, and plays one square wave per note', () => {
    let made = 0
    const ctx = fakeCtx()
    const b = createBlipper(() => {
      made++
      return ctx
    })
    expect(b.play('hi')).toBe(2)
    expect(b.play('yo')).toBe(2)
    expect(made).toBe(1)
    expect(ctx.state).toBe('running')
    expect(ctx.oscs).toHaveLength(4)
    expect(ctx.oscs.every((o) => o.type === 'square')).toBe(true)
    expect(ctx.oscs[0].at).toBeGreaterThan(ctx.currentTime) // scheduled, never "now"
  })

  test('stays quiet with no audio at all', () => {
    expect(createBlipper(() => null).play('hi')).toBe(0)
    const broken = createBlipper(() => {
      throw new Error('no device')
    })
    expect(broken.play('hi')).toBe(0)
  })
})

describe('remarks', () => {
  const week = (y, rest) => [...new Array(22).fill(0), ...rest, y, 5e6]

  test('the greeting names yesterday, and says when it was the heaviest', () => {
    const l = greetingLine(week(12e6, [1e6, 2e6, 3e6, 4e6, 5e6, 6e6]), 9, fmt)
    expect(l.text).toBe('Good morning! Yesterday was your heaviest day this week, 12M tokens.')
    expect(l.short).toBe('Morning! Big day yesterday.')
  })

  test('a quiet day is called quiet', () => {
    const l = greetingLine(week(1e6, [8e6, 8e6, 8e6, 8e6, 8e6, 8e6]), 14, fmt)
    expect(l.text).toBe('Good afternoon! Yesterday was a quiet one, just 1M tokens.')
    expect(l.short).toBe('Afternoon! 1M yesterday.')
  })

  test('an ordinary day is just reported', () => {
    const l = greetingLine(week(5e6, [4e6, 6e6, 5e6, 5e6, 5e6, 5e6]), 20, fmt)
    expect(l.text).toBe('Good evening! You spent 5M tokens yesterday.')
    expect(l.short).toBe('Evening! 5M yesterday.')
  })

  test('the small hours get their own hello', () => {
    const l = greetingLine(week(5e6, [4e6, 6e6, 5e6, 5e6, 5e6, 5e6]), 2, fmt)
    expect(l.text).toBe('Still up? You spent 5M tokens yesterday.')
    expect(l.short).toBe('Up late! 5M yesterday.')
    expect(greetingLine(week(5e6, [4e6, 6e6, 5e6, 5e6, 5e6, 5e6]), 5, fmt).short).toBe(
      'Morning! 5M yesterday.',
    )
  })

  test('nothing to say without a yesterday', () => {
    expect(greetingLine(week(0, [1, 1, 1, 1, 1, 1]), 9, fmt)).toBeNull()
    expect(greetingLine([], 9, fmt)).toBeNull()
    expect(greetingLine(undefined, 9, fmt)).toBeNull()
  })

  test('fire leads with the pace when there is one', () => {
    expect(fireLine(91.4, 40 * 60000, 3600_000, dur).text).toBe(
      "Getting warm, 91% already. At this pace you'll run out in about 40m.",
    )
    expect(fireLine(91, null, 90 * 60000, dur).text).toBe(
      'Getting warm, 91% already. It resets in 90m.',
    )
    expect(fireLine(91, null, null, dur).text).toBe('Getting warm, 91% already.')
    expect(fireLine(91, null, null, dur).short).toBe('91%, getting warm!')
  })

  test('the rest read as sentences, with a glance for the mini face', () => {
    expect(resetLine(92.6)).toEqual({
      text: 'Fresh window! The last one closed at 93%.',
      short: 'Fresh window!',
    })
    expect(maxedLine('14:35')).toEqual({
      text: "That's the limit. I'll be back at 14:35.",
      short: 'Maxed out till 14:35.',
    })
    expect(maxedLine(null).text).toBe("That's the limit. Nap time.")
    expect(welcomeLine(180 * 60000, 12, dur).text).toBe(
      "Welcome back! You were gone 180m. The session's at 12%.",
    )
    expect(welcomeLine(180 * 60000, null, dur).text).toBe('Welcome back! You were gone 180m.')
    expect(streakLine(95 * 60000, dur)).toEqual({
      text: "You've been at it for 95m straight. Stretch break?",
      short: 'Stretch break?',
    })
  })

  test('a record needs a history, a floor, and to actually beat it', () => {
    const days = [...new Array(29).fill(2e6), 3e6]
    expect(recordLine(days, fmt)).toEqual({
      text: 'New record! 3M tokens today, your biggest day in a month.',
      short: 'New record: 3M!',
    })
    expect(recordLine([...new Array(29).fill(2e6), 2e6], fmt)).toBeNull()
    expect(recordLine([...new Array(29).fill(0), 3e6], fmt)).toBeNull()
    expect(recordLine([...new Array(29).fill(1e5), 5e5], fmt)).toBeNull()
    expect(recordLine([1, 2], fmt)).toBeNull()
  })

  test('the recap is a sentence, built from what is known', () => {
    const all = { was: 92, tokens: 34e6, activeMs: 252 * 60000, top: 'editing' }
    expect(recapLine(all, fmt, dur)).toEqual({
      text: 'Fresh window! That was 34M tokens over 252m, mostly editing code.',
      short: 'Fresh window! 34M last time.',
    })
    expect(recapLine({ was: 92, tokens: 5e6, activeMs: 30000, top: 'nope' }, fmt, dur).text).toBe(
      'Fresh window! That was 5M tokens.',
    )
    expect(recapLine({ was: 92, tokens: 0 }, fmt, dur).text).toBe(
      'Fresh window! The last one closed at 92%.',
    )
    expect(recapLine({ tokens: 0 }, fmt, dur).text).toBe('Fresh window! The last one closed at 0%.')
  })

  test('Codex lines always say whose they are', () => {
    expect(codexFireLine(91.2, 3600_000, dur)).toEqual({
      text: 'Codex is at 91% now. It resets in 60m.',
      short: 'Codex at 91%!',
    })
    expect(codexFireLine(91, null, dur).text).toBe('Codex is at 91% now.')
    expect(codexMaxedLine('14:35').text).toBe("Codex is maxed out. It's back at 14:35.")
    expect(codexMaxedLine(null).text).toBe('Codex is maxed out.')
    expect(codexResetLine(88)).toEqual({
      text: 'Codex has a fresh window! The last one closed at 88%.',
      short: 'Codex: fresh window!',
    })
    // Cursor borrows them, with its name and its month
    expect(codexFireLine(95, null, dur, 'Cursor').text).toBe('Cursor is at 95% now.')
    expect(codexMaxedLine(null, 'Cursor').text).toBe('Cursor is maxed out.')
    expect(codexResetLine(90, 'Cursor', 'month').text).toBe(
      'Cursor has a fresh month! The last one closed at 90%.',
    )
  })
})
