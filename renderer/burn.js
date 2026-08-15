// ---- burn-rate projection ----
// The panel knows where you are (82%) and when the window resets (1h 12m); it
// can also say where you're headed — fit a slope through recent usage and
// project the crossing of 100%. If the reset lands first there's nothing to
// worry about, which is worth saying out loud rather than leaving blank.
//
// The slope is fitted over session *tokens*, not the account %. The % is the
// number we ultimately care about, but it arrives as a whole number every ~5
// min: over a short window the whole signal is a single 16 → 17 step, which
// makes the fitted pace wrong by multiples. Local-log tokens step too — one
// jump per assistant turn, with plateaus in between — but in increments some
// 10-20x finer, so they carry a far steadier slope. The account % still anchors
// it: pct/tokens converts tokens/ms into %/ms and re-calibrates on every poll,
// so the projection stays tied to the real number.
//
// Loaded both as a plain <script> by the renderer (exposing `Burn`) and via
// require() by the tests — hence the dual export at the bottom.
;(() => {
  const WINDOW_MS = 45 * 60 * 1000 // only fit recent samples — pace changes
  const MIN_SAMPLES = 4
  const MIN_SPAN_MS = 5 * 60 * 1000 // shorter than a % fit affords, and steadier
  const SAMPLE_EVERY_MS = 30 * 1000 // usage polls every few seconds; thin it out

  // `now` is injectable so tests can drive the trail without sleeping
  function createBurnTracker() {
    let trail = []

    function note(tokens, active, now = Date.now()) {
      if (!active || !(tokens > 0)) {
        trail = [] // no session, no trend
        return
      }
      // session tokens only climb within a window — a drop means it rolled
      // over, and the fresh sample starts the new trail rather than being
      // throttled away
      let last = trail[trail.length - 1]
      if (last && tokens < last.tokens) {
        trail = []
        last = undefined
      }
      if (last && now - last.t < SAMPLE_EVERY_MS) return
      trail.push({ t: now, tokens })
      trail = trail.filter((s) => s.t >= now - WINDOW_MS)
    }

    // least-squares slope in tokens per ms, or null when there isn't enough to say
    function slope() {
      if (trail.length < MIN_SAMPLES) return null
      const span = trail[trail.length - 1].t - trail[0].t
      if (span < MIN_SPAN_MS) return null
      const n = trail.length
      const t0 = trail[0].t
      let sx = 0
      let sy = 0
      let sxy = 0
      let sxx = 0
      for (const s of trail) {
        const x = s.t - t0
        sx += x
        sy += s.tokens
        sxy += x * s.tokens
        sxx += x * x
      }
      const denom = n * sxx - sx * sx
      if (denom === 0) return null
      return (n * sxy - sx * sy) / denom
    }

    // → { kind: 'eta', ms } when you'd run out first, { kind: 'safe' } when the
    // reset beats you there, or null when we can't tell yet.
    function project(pct, resetMs, tokens) {
      if (!(pct > 0) || !(tokens > 0)) return null // no anchor to convert tokens → %
      const tokPerMs = slope()
      if (tokPerMs == null || tokPerMs <= 0) return null // idle or flat
      const pctPerMs = tokPerMs * (pct / tokens)
      const eta = (100 - pct) / pctPerMs
      if (!Number.isFinite(eta) || eta <= 0) return null
      if (resetMs != null && eta >= resetMs) return { kind: 'safe' }
      return { kind: 'eta', ms: eta }
    }

    return {
      note,
      slope,
      project,
      reset: () => {
        trail = []
      },
      get trail() {
        return trail
      },
    }
  }

  const api = { createBurnTracker, WINDOW_MS, MIN_SAMPLES, MIN_SPAN_MS, SAMPLE_EVERY_MS }
  if (typeof module === 'object' && module.exports) module.exports = api
  else globalThis.Burn = api
})()
