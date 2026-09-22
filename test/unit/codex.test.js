import { afterAll, beforeEach, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

const codex = require('../../codex.js')

const NOW = Date.parse('2026-09-12T20:00:00Z')
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codex-'))
const dayDir = path.join(root, 'sessions', '2026', '09', '12')

const tokenCount = (ts, total, limits) =>
  JSON.stringify({
    timestamp: ts,
    type: 'event_msg',
    payload: {
      type: 'token_count',
      info: { total_token_usage: { total_tokens: total } },
      rate_limits: limits,
    },
  })
const CODEX_LIMITS = {
  limit_id: 'codex',
  plan_type: 'plus',
  primary: { used_percent: 40, window_minutes: 300, resets_at: NOW / 1000 + 3600 },
  secondary: { used_percent: 31, window_minutes: 10080, resets_at: NOW / 1000 + 86400 },
}
const PREMIUM = { limit_id: 'premium', plan_type: 'plus', primary: null, secondary: null }

function write(name, lines, mtime = NOW - 10000) {
  const f = path.join(dayDir, name)
  fs.writeFileSync(f, `${lines.join('\n')}\n`)
  fs.utimesSync(f, mtime / 1000, mtime / 1000)
  return f
}

beforeEach(() => {
  fs.rmSync(path.join(root, 'sessions'), { recursive: true, force: true })
  fs.mkdirSync(dayDir, { recursive: true })
  codex.setCodexDir(root)
})
afterAll(() => fs.rmSync(root, { recursive: true, force: true }))

describe('getCodexUsage', () => {
  test('returns null when Codex is not installed', () => {
    codex.setCodexDir(path.join(root, 'nope'))
    expect(codex.getCodexUsage(NOW)).toBeNull()
  })

  test('reads limits, tokens, model and activity from a rollout', () => {
    write('rollout-a.jsonl', [
      JSON.stringify({ type: 'turn_context', payload: { model: 'gpt-6', cwd: '/x/proj' } }),
      tokenCount('2026-09-12T19:00:00Z', 100, CODEX_LIMITS),
      tokenCount('2026-09-12T19:00:01Z', 100, CODEX_LIMITS), // repeated count
      tokenCount('2026-09-12T19:30:00Z', 250, PREMIUM), // null windows: ignored
      'not json {"token_count"',
      '{"type":"response_item","payload":{}}',
    ])
    const u = codex.getCodexUsage(NOW)
    expect(u.active).toBe(true)
    expect(u.plan).toBe('plus')
    expect(u.session).toEqual({ pct: 40, resetMs: 3600000, expired: false })
    expect(u.weekly.pct).toBe(31)
    expect(u.tokens5h).toBe(250)
    expect(u.tokensWeek).toBe(250)
  })

  test('reads only what was appended, and survives a partial line', () => {
    const f = write('rollout-b.jsonl', [tokenCount('2026-09-12T19:00:00Z', 100, CODEX_LIMITS)])
    expect(codex.getCodexUsage(NOW).tokensWeek).toBe(100)
    const next = tokenCount('2026-09-12T19:10:00Z', 180, CODEX_LIMITS)
    fs.appendFileSync(f, next.slice(0, 20))
    expect(codex.getCodexUsage(NOW).tokensWeek).toBe(100)
    fs.appendFileSync(f, `${next.slice(20)}\n`)
    expect(codex.getCodexUsage(NOW).tokensWeek).toBe(180)
    // truncated/replaced: read again from scratch
    fs.writeFileSync(f, `${tokenCount('2026-09-12T19:20:00Z', 5, null)}\n`)
    expect(codex.getCodexUsage(NOW).tokensWeek).toBe(5)
  })

  test('an idle Codex keeps the last %, and a passed reset is unknown, not zero', () => {
    write('rollout-c.jsonl', [tokenCount('2026-09-12T10:00:00Z', 10, CODEX_LIMITS)], NOW - 3600000)
    const later = NOW + 2 * 3600000 // past the 5h reset, before the weekly one
    const u = codex.getCodexUsage(later)
    expect(u.active).toBe(false)
    expect(u.session).toEqual({ pct: null, resetMs: null, expired: true })
    expect(u.weekly.pct).toBe(31)
  })

  test('ignores rollouts older than the 30-day map', () => {
    write(
      'rollout-old.jsonl',
      [tokenCount('2026-08-01T10:00:00Z', 10, CODEX_LIMITS)],
      NOW - 40 * 86400000,
    )
    const u = codex.getCodexUsage(NOW)
    expect(u.session).toBeNull()
    expect(u.lastSeen).toBeNull()
    expect(u.monthTokens).toBe(0)
  })

  test('attributes each delta to the model and project in effect', () => {
    const ctx = (model, cwd) => JSON.stringify({ type: 'turn_context', payload: { model, cwd } })
    const more = [1, 2, 3, 4, 5, 6].flatMap((i) => [
      ctx('gpt-6', `/w/p${i}`),
      tokenCount('2026-09-12T18:00:00Z', 1000 + i * 10, null),
    ])
    write('rollout-d.jsonl', [
      ctx('gpt-6', '/w/alpha'),
      tokenCount('2026-09-12T19:00:00Z', 100, null),
      ctx('gpt-6-mini', '/w/beta'),
      tokenCount('2026-09-12T19:10:00Z', 130, null),
      tokenCount('2026-09-10T09:00:00Z', 900, null), // before today, still this week
      ...more,
    ])
    const u = codex.getCodexUsage(NOW)
    expect(u.byModel.find((m) => m.label === 'gpt-6-mini').tokens).toBe(800)
    expect(u.byProject.find((p) => p.label === 'alpha').tokens).toBe(100)
    expect(u.byProject.at(-1).label).toMatch(/^other · /)
    expect(u.byProject).toHaveLength(6)
    expect(u.days30).toHaveLength(30)
    expect(u.days30.reduce((a, b) => a + b, 0)).toBe(u.monthTokens)
    expect(u.days30[29]).toBe(u.tokensToday)
  })

  test('an event with no model still counts', () => {
    write('rollout-e.jsonl', [tokenCount('2026-09-12T19:00:00Z', 50, null)])
    expect(codex.getCodexUsage(NOW).byModel).toEqual([{ label: 'unknown', tokens: 50 }])
  })
})

describe('detectCodex', () => {
  test('finds nothing without rollouts', () => {
    expect(codex.detectCodex()).toEqual({ found: false, plan: null })
  })

  test('finds a session and its plan, however old', () => {
    write(
      'rollout-f.jsonl',
      [tokenCount('2026-06-01T10:00:00Z', 10, CODEX_LIMITS)],
      NOW - 90 * 86400000,
    )
    expect(codex.detectCodex()).toEqual({ found: true, plan: 'plus' })
  })

  test('a session without limits is found, plan unknown', () => {
    write('rollout-g.jsonl', [tokenCount('2026-09-12T10:00:00Z', 10, PREMIUM)])
    expect(codex.detectCodex()).toEqual({ found: true, plan: null })
  })
})
