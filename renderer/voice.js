// ---- the pet's voice ----
// Two channels. The bubble carries the meaning: a short remark, only on a
// transition, and only when there's something specific to say — a pet that
// talks on a timer is Clippy. The sound carries the personality: chiptune
// blips, one square-wave note per letter, Animal Crossing style. Oscillators
// only, so no assets, no dependency, and no language baked into the sound.
;(() => {
  const BASE_HZ = 400
  const SPREAD_HZ = 250
  const STEP_MS = 115
  const MAX_BLIPS = 14 // a sentence is a babble, not a recital
  const GAIN = 0.05

  // the tone doubles as state: hot is faster and higher, sleepy slower and lower
  const MOODS = {
    normal: { speed: 1, pitch: 1 },
    fire: { speed: 0.72, pitch: 1.25 },
    sleepy: { speed: 1.35, pitch: 0.8 },
  }

  // letters and digits each get a fixed pitch, so the same word always sounds
  // the same; anything else is a rest
  function pitchOf(ch) {
    const c = ch.toLowerCase()
    if (c >= 'a' && c <= 'z') return BASE_HZ + ((c.charCodeAt(0) - 97) / 25) * SPREAD_HZ
    if (c >= '0' && c <= '9') return BASE_HZ + ((c.charCodeAt(0) - 48) / 9) * SPREAD_HZ
    return null
  }

  // the schedule for a line: [{ at, hz, dur }] in ms, pure so it can be tested
  function blipPlan(text, mood = 'normal') {
    const m = MOODS[mood] || MOODS.normal
    const step = STEP_MS * m.speed
    const plan = []
    let t = 0
    for (const ch of String(text)) {
      if (plan.length >= MAX_BLIPS) break
      const hz = pitchOf(ch)
      if (hz != null) {
        plan.push({ at: Math.round(t), hz: Math.round(hz * m.pitch), dur: Math.round(step * 0.6) })
        t += step
      } else if (/[.,!?—:;]/.test(ch)) {
        t += step * 1.5 // a breath on punctuation
      } else if (ch === ' ') {
        t += step * 0.4
      }
    }
    return plan
  }

  // `makeCtx` builds the AudioContext lazily: creating one up front would hold
  // an audio device open for a pet that, by default, never makes a sound
  function createBlipper(makeCtx) {
    let ctx = null
    return {
      play(text, mood) {
        try {
          ctx = ctx || makeCtx()
          if (!ctx) return 0
          if (ctx.state === 'suspended') ctx.resume?.()
          const plan = blipPlan(text, mood)
          const t0 = ctx.currentTime + 0.02
          for (const b of plan) {
            const start = t0 + b.at / 1000
            const end = start + b.dur / 1000
            const osc = ctx.createOscillator()
            const gain = ctx.createGain()
            osc.type = 'square'
            osc.frequency.setValueAtTime(b.hz, start)
            // a quick attack and decay, or every note clicks
            gain.gain.setValueAtTime(0, start)
            gain.gain.linearRampToValueAtTime(GAIN, start + 0.008)
            gain.gain.exponentialRampToValueAtTime(0.0001, end)
            osc.connect(gain)
            gain.connect(ctx.destination)
            osc.start(start)
            osc.stop(end + 0.01)
          }
          return plan.length
        } catch {
          return 0 // no audio device, or a context the OS took away: stay quiet
        }
      },
    }
  }

  // ---- remarks ----
  // Each returns { text, short }, or null when there's nothing worth saying.
  // `text` is a full, spoken sentence for the scene; `short` is the glance the
  // mini face has room for. Numbers arrive preformatted through `fmt` / `fmtDur`
  // so this file stays free of the widget.
  const line = (text, short) => ({ text, short: short || text })

  // the small hours get their own hello: "Good morning!" at 2am rings false
  const LATE_UNTIL = 5
  function salutation(hour) {
    if (hour < LATE_UNTIL) return 'Still up?'
    return hour < 12 ? 'Good morning!' : hour < 18 ? 'Good afternoon!' : 'Good evening!'
  }
  function hello(hour) {
    if (hour < LATE_UNTIL) return 'Up late!'
    return hour < 12 ? 'Morning!' : hour < 18 ? 'Afternoon!' : 'Evening!'
  }

  // `days` is the 30-day series, oldest first, today last
  function greetingLine(days, hour, fmt) {
    if (!Array.isArray(days) || days.length < 2) return null
    const y = days[days.length - 2]
    if (!y) return null
    const week = days.slice(-8, -1) // the seven days before today, yesterday included
    const worked = week.filter((v) => v > 0)
    const hi = salutation(hour)
    const short = `${hello(hour)} ${fmt(y)} yesterday.`
    if (worked.length > 1 && y >= Math.max(...week))
      return line(
        `${hi} Yesterday was your heaviest day this week, ${fmt(y)} tokens.`,
        `${hello(hour)} Big day yesterday.`,
      )
    const avg = worked.reduce((n, v) => n + v, 0) / (worked.length || 1)
    if (worked.length > 2 && y < avg * 0.5)
      return line(`${hi} Yesterday was a quiet one, just ${fmt(y)} tokens.`, short)
    return line(`${hi} You spent ${fmt(y)} tokens yesterday.`, short)
  }

  function fireLine(pct, etaMs, resetMs, fmtDur) {
    const p = Math.round(pct)
    if (etaMs != null)
      return line(
        `Getting warm, ${p}% already. At this pace you'll run out in about ${fmtDur(etaMs)}.`,
        `${p}%, getting warm!`,
      )
    if (resetMs != null)
      return line(
        `Getting warm, ${p}% already. It resets in ${fmtDur(resetMs)}.`,
        `${p}%, getting warm!`,
      )
    return line(`Getting warm, ${p}% already.`, `${p}%, getting warm!`)
  }

  function resetLine(was) {
    return line(`Fresh window! The last one closed at ${Math.round(was)}%.`, 'Fresh window!')
  }

  // what each activity was, said the way you'd say it
  const DOING = {
    editing: 'mostly editing code',
    reading: 'mostly reading',
    planning: 'mostly planning',
    running: 'mostly running commands',
    researching: 'mostly researching',
    delegating: 'mostly delegating to agents',
    waiting: 'mostly waiting on you',
  }

  // the recap of the window that just closed (#31), built from whatever is
  // known, so a pet launched mid-session still says something true
  function recapLine({ was, tokens, activeMs, top }, fmt, fmtDur) {
    if (!(tokens > 0)) return resetLine(was ?? 0)
    let text = `Fresh window! That was ${fmt(tokens)} tokens`
    if (activeMs >= 60000) text += ` over ${fmtDur(activeMs)}`
    if (DOING[top]) text += `, ${DOING[top]}`
    return line(`${text}.`, `Fresh window! ${fmt(tokens)} last time.`)
  }

  // Codex and Cursor speak through the same pet, so their lines always say whose they are
  function codexFireLine(pct, resetMs, fmtDur, name = 'Codex') {
    const p = Math.round(pct)
    const text =
      resetMs != null
        ? `${name} is at ${p}% now. It resets in ${fmtDur(resetMs)}.`
        : `${name} is at ${p}% now.`
    return line(text, `${name} at ${p}%!`)
  }
  function codexMaxedLine(resetAt, name = 'Codex') {
    return line(
      resetAt ? `${name} is maxed out. It's back at ${resetAt}.` : `${name} is maxed out.`,
      `${name} is maxed out.`,
    )
  }
  // Cursor's window is its billing month
  function codexResetLine(was, name = 'Codex', span = 'window') {
    return line(
      `${name} has a fresh ${span}! The last one closed at ${Math.round(was)}%.`,
      `${name}: fresh ${span}!`,
    )
  }

  function maxedLine(resetAt) {
    return resetAt
      ? line(`That's the limit. I'll be back at ${resetAt}.`, `Maxed out till ${resetAt}.`)
      : line("That's the limit. Nap time.", 'Maxed out.')
  }

  function welcomeLine(awayMs, pct, fmtDur) {
    const away = fmtDur(awayMs)
    const text =
      pct != null
        ? `Welcome back! You were gone ${away}. The session's at ${Math.round(pct)}%.`
        : `Welcome back! You were gone ${away}.`
    return line(text, 'Welcome back!')
  }

  function streakLine(ms, fmtDur) {
    return line(`You've been at it for ${fmtDur(ms)} straight. Stretch break?`, 'Stretch break?')
  }

  // a record only counts against a real history, and past a floor that makes
  // it more than noise
  const RECORD_FLOOR = 1e6
  function recordLine(days, fmt) {
    if (!Array.isArray(days) || days.length < 8) return null
    const today = days[days.length - 1]
    const before = days.slice(0, -1)
    const best = Math.max(...before)
    if (best <= 0 || today < RECORD_FLOOR || today <= best) return null
    return line(
      `New record! ${fmt(today)} tokens today, your biggest day in a month.`,
      `New record: ${fmt(today)}!`,
    )
  }

  const api = {
    BASE_HZ,
    SPREAD_HZ,
    STEP_MS,
    MAX_BLIPS,
    pitchOf,
    blipPlan,
    createBlipper,
    greetingLine,
    fireLine,
    resetLine,
    recapLine,
    codexFireLine,
    codexMaxedLine,
    codexResetLine,
    maxedLine,
    welcomeLine,
    streakLine,
    recordLine,
  }
  if (typeof module === 'object' && module.exports) module.exports = api
  else globalThis.Voice = api
})()
