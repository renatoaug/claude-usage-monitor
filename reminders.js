// One-shot reset reminders, independent of automatic threshold alerts.
function createReminders({ load, save, now = Date.now }) {
  let pending = []
  let delivered = {}
  const valid = (r) =>
    r &&
    ['claude', 'codex'].includes(r.provider) &&
    typeof r.key === 'string' &&
    typeof r.label === 'string' &&
    Number.isFinite(r.at) &&
    Number.isFinite(r.pct)
  try {
    const data = load()
    pending = Array.isArray(data?.pending) ? data.pending.filter(valid) : []
    delivered = data?.delivered || {}
  } catch {}
  function persist(nextPending, nextDelivered = delivered) {
    const recent = Object.fromEntries(
      Object.entries(nextDelivered).filter(([, at]) => Number.isFinite(at) && now() - at < 3600000),
    )
    save({ pending: nextPending, delivered: recent })
    pending = nextPending
    delivered = recent
  }
  return {
    list: () => pending.map((r) => ({ ...r })),
    arm(reminder) {
      if (!valid(reminder) || reminder.at <= now() || reminder.at > now() + 7 * 86400000)
        return false
      persist([...pending.filter((r) => r.key !== reminder.key), { ...reminder }])
      return true
    },
    cancel(key) {
      const found = pending.some((r) => r.key === key)
      if (found) persist(pending.filter((r) => r.key !== key))
    },
    takeDue() {
      const due = pending.filter((r) => r.at <= now())
      if (!due.length) return []
      const nextDelivered = { ...delivered }
      for (const r of due) nextDelivered[r.key] = now()
      // Consume before delivering: restart must not replay it.
      persist(
        pending.filter((r) => r.at > now()),
        nextDelivered,
      )
      return due
    },
    recentlyDelivered: (key) => Number.isFinite(delivered[key]) && now() - delivered[key] < 3600000,
  }
}

module.exports = { createReminders }
