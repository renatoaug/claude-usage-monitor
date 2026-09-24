// A quiet transition detector. Startup and reconnects establish a baseline;
// inactivity is a pause, never proof that an agent finished its task.
;((root) => {
  function createActivityTracker({ now = Date.now, cooldown = 30000 } = {}) {
    const seen = new Map()
    let lastCue = -Infinity
    return {
      forget(provider) {
        seen.delete(provider)
      },
      observe(provider, data) {
        if (!data) {
          seen.delete(provider)
          return null
        }
        const next = { active: !!data.active, activity: data.activity || 'working' }
        const previous = seen.get(provider)
        seen.set(provider, next)
        if (!previous) return null
        const changed =
          next.active !== previous.active || (next.active && next.activity !== previous.activity)
        if (!changed || now() - lastCue < cooldown) return null
        lastCue = now()
        return { provider, activity: next.active ? next.activity : 'paused' }
      },
    }
  }
  // Activity freshness is separate from quota freshness: Codex can work while
  // its last rate-limit reading is old. Ignore abandoned activity snapshots.
  const FRESH_MS = 60000
  const SLEEP_MS = 300000 // main's default sleepThresholdMs; Codex and Cursor have no flag of their own
  // each source is its last payload, or null while it isn't connected
  function currentActivity({ claude = null, codex = null, cursor = null } = {}, now = Date.now()) {
    const fresh = (at) => Number.isFinite(at) && now - at <= FRESH_MS
    const providers = []
    if (claude?.active && fresh(claude.ts)) providers.push('claude')
    if (codex?.active && fresh(codex.lastSeen)) providers.push('codex')
    if (cursor?.active && fresh(cursor.lastSeen)) providers.push('cursor')
    // Claude's and Cursor's logs say *what* they're doing; Codex's don't, and
    // two workers share one scene
    const solo = providers.length === 1 ? { claude, cursor }[providers[0]] : null
    const activity = solo?.activity || 'working'
    const idle = (s) => s && !!s.lastSeen && now - s.lastSeen >= SLEEP_MS
    const asleep = [claude && !!claude.sleeping, idle(codex), idle(cursor)].filter(
      (v) => v !== null && v !== undefined,
    )
    return { providers, activity, sleeping: asleep.length > 0 && asleep.every(Boolean) }
  }
  const api = { createActivityTracker, currentActivity }
  if (typeof module === 'object' && module.exports) module.exports = api
  else root.Companion = api
})(globalThis)
