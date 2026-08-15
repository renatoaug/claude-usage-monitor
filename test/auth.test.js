import { afterAll, afterEach, beforeEach, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'

// auth.js resolves its data dir at import time, same as usage.js
const ROOT = fs.mkdtempSync(path.join(os.tmpdir(), 'clauddy-auth-'))
process.env.CLAUDE_CONFIG_DIR = ROOT
const TOKEN_PATH = path.join(ROOT, 'usage-monitor', 'auth.json')
const auth = await import('../auth.js')

afterAll(() => fs.rmSync(ROOT, { recursive: true, force: true }))

const realFetch = globalThis.fetch
let calls = []

// Route by URL fragment. Each handler is { status, body } or a function.
function mockFetch(routes) {
  globalThis.fetch = async (url, opts) => {
    calls.push({ url: String(url), opts })
    const hit = Object.entries(routes).find(([frag]) => String(url).includes(frag))
    if (!hit) throw new Error(`unrouted fetch: ${url}`)
    const spec = typeof hit[1] === 'function' ? hit[1]() : hit[1]
    const status = spec.status ?? 200
    return {
      ok: status >= 200 && status < 300,
      status,
      json: async () => spec.body ?? {},
      text: async () => (spec.text != null ? spec.text : JSON.stringify(spec.body ?? {})),
    }
  }
}

// clear() drops the in-memory token+profile cache and deletes the file, so
// writing afterwards gives each test a known starting state.
function seedToken(t) {
  auth.clear()
  fs.mkdirSync(path.dirname(TOKEN_PATH), { recursive: true })
  fs.writeFileSync(TOKEN_PATH, JSON.stringify(t))
}
const LIVE = { access_token: 'tok-live', refresh_token: 'ref-1', expires_at: Date.now() + 3600_000 }
const EXPIRED = { access_token: 'tok-old', refresh_token: 'ref-1', expires_at: Date.now() - 1000 }

beforeEach(() => {
  calls = []
  auth.clear()
})
afterEach(() => {
  globalThis.fetch = realFetch
})

describe('authorize URL', () => {
  test('carries the PKCE challenge and the app identity', () => {
    const url = new URL(auth.begin())
    const q = url.searchParams
    expect(url.origin + url.pathname).toBe('https://claude.ai/oauth/authorize')
    expect(q.get('response_type')).toBe('code')
    expect(q.get('code_challenge_method')).toBe('S256')
    expect(q.get('client_id')).toBeTruthy()
    expect(q.get('redirect_uri')).toContain('platform.claude.com')
    expect(q.get('scope')).toContain('user:profile')
    // base64url: no padding, no + or /
    expect(q.get('code_challenge')).toMatch(/^[A-Za-z0-9_-]+$/)
    expect(q.get('state')).toMatch(/^[A-Za-z0-9_-]+$/)
  })

  test('is different every time', () => {
    const a = new URL(auth.begin()).searchParams
    const b = new URL(auth.begin()).searchParams
    expect(a.get('code_challenge')).not.toBe(b.get('code_challenge'))
    expect(a.get('state')).not.toBe(b.get('state'))
  })
})

describe('completing the login', () => {
  test('a pasted long-lived token skips the exchange entirely', async () => {
    mockFetch({})
    await auth.complete('  sk-ant-longlived  ') // trimmed, no "#"
    expect(calls.length).toBe(0)
    expect(auth.isConnected()).toBe(true)
    expect(JSON.parse(fs.readFileSync(TOKEN_PATH, 'utf8')).access_token).toBe('sk-ant-longlived')
  })

  test('exchanges code#state for tokens', async () => {
    mockFetch({
      '/oauth/token': { body: { access_token: 'A', refresh_token: 'R', expires_in: 60 } },
    })
    auth.begin() // arms the pending verifier/state
    await auth.complete('the-code#the-state')

    const sent = JSON.parse(calls[0].opts.body)
    expect(calls[0].url).toContain('platform.claude.com/v1/oauth/token')
    expect(sent.grant_type).toBe('authorization_code')
    expect(sent.code).toBe('the-code')
    expect(sent.state).toBe('the-state')
    expect(sent.code_verifier).toMatch(/^[A-Za-z0-9_-]+$/)

    const saved = JSON.parse(fs.readFileSync(TOKEN_PATH, 'utf8'))
    expect(saved.access_token).toBe('A')
    expect(saved.refresh_token).toBe('R')
    expect(saved.expires_at).toBeGreaterThan(Date.now())
  })

  test('refuses a code once the pending login has been consumed', async () => {
    mockFetch({})
    auth.begin()
    await auth.complete('a-long-lived-token') // this path clears `pending`
    await expect(auth.complete('code#state')).rejects.toThrow('no pending auth')
  })

  test('surfaces a rejected exchange', async () => {
    mockFetch({ '/oauth/token': { status: 400, text: 'bad_verifier' } })
    auth.begin()
    await expect(auth.complete('c#s')).rejects.toThrow(/exchange 400.*bad_verifier/)
  })

  test('stores the token file with owner-only permissions', async () => {
    mockFetch({})
    await auth.complete('sk-ant-secret')
    expect(fs.statSync(TOKEN_PATH).mode & 0o777).toBe(0o600)
  })
})

describe('connection state', () => {
  test('tracks whether a token exists', () => {
    expect(auth.isConnected()).toBe(false)
    seedToken(LIVE)
    expect(auth.isConnected()).toBe(true)
    auth.clear()
    expect(auth.isConnected()).toBe(false)
    expect(fs.existsSync(TOKEN_PATH)).toBe(false)
  })

  test('a corrupt token file reads as disconnected', () => {
    auth.clear()
    fs.mkdirSync(path.dirname(TOKEN_PATH), { recursive: true })
    fs.writeFileSync(TOKEN_PATH, '{ not json')
    expect(auth.isConnected()).toBe(false)
  })
})

describe('usage mapping', () => {
  const usageBody = {
    five_hour: { utilization: 42, resets_at: new Date(Date.now() + 2 * 3600_000).toISOString() },
    seven_day: { utilization: 7, resets_at: new Date(Date.now() + 48 * 3600_000).toISOString() },
    seven_day_opus: { utilization: 3, resets_at: null },
  }

  test('maps the API windows into percentages and countdowns', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/usage': { body: usageBody } })
    const u = await auth.fetchUsage()
    expect(u.session.pct).toBe(42)
    expect(u.session.resetMs / 3600_000).toBeCloseTo(2, 1)
    expect(u.week.pct).toBe(7)
    expect(u.opus).toEqual({ pct: 3, resetMs: null })
    expect(u.sonnet).toBeNull() // absent from the payload
  })

  test('a missing window reads as zero rather than undefined', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/usage': { body: {} } })
    const u = await auth.fetchUsage()
    expect(u.session).toEqual({ pct: 0, resetMs: null })
    expect(u.week).toEqual({ pct: 0, resetMs: null })
  })

  test('a non-numeric utilization is not trusted', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/usage': { body: { five_hour: { utilization: 'lots' } } } })
    expect((await auth.fetchUsage()).session).toEqual({ pct: 0, resetMs: null })
  })

  test('sends the bearer token and the oauth beta headers', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/usage': { body: usageBody } })
    await auth.fetchUsage()
    expect(calls[0].opts.headers.Authorization).toBe('Bearer tok-live')
    expect(calls[0].opts.headers['anthropic-beta']).toBe('oauth-2025-04-20')
  })

  test('keeps the status on a throttled response so callers can back off', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/usage': { status: 429, text: 'slow down' } })
    await expect(auth.fetchUsage()).rejects.toMatchObject({ status: 429 })
  })
})

describe('token refresh', () => {
  test('refreshes an expired token before calling the API', async () => {
    seedToken(EXPIRED)
    mockFetch({
      '/oauth/token': { body: { access_token: 'fresh', expires_in: 3600 } },
      '/oauth/usage': { body: { five_hour: { utilization: 1 } } },
    })
    await auth.fetchUsage()
    expect(calls[0].url).toContain('/oauth/token')
    expect(JSON.parse(calls[0].opts.body).grant_type).toBe('refresh_token')
    expect(calls[1].opts.headers.Authorization).toBe('Bearer fresh')
  })

  test('keeps the old refresh token when the response omits one', async () => {
    seedToken(EXPIRED)
    mockFetch({
      '/oauth/token': { body: { access_token: 'fresh', expires_in: 3600 } },
      '/oauth/usage': { body: {} },
    })
    await auth.fetchUsage()
    expect(JSON.parse(fs.readFileSync(TOKEN_PATH, 'utf8')).refresh_token).toBe('ref-1')
  })

  test.each([
    [400, 401, 'a rejected grant is a dead session'],
    [403, 401, 'a forbidden grant is a dead session'],
    [429, 429, 'throttling is transient — retry, do not log out'],
    [500, 500, 'a server error is transient'],
  ])('refresh %i surfaces as %i (%s)', async (got, want) => {
    seedToken(EXPIRED)
    mockFetch({ '/oauth/token': { status: got } })
    await expect(auth.fetchUsage()).rejects.toMatchObject({ status: want })
  })

  test('an expired token with nothing to refresh is a 401', async () => {
    seedToken({ access_token: 'x', refresh_token: null, expires_at: Date.now() - 1 })
    mockFetch({})
    await expect(auth.fetchUsage()).rejects.toMatchObject({ status: 401 })
  })

  test('not being connected at all is a 401', async () => {
    mockFetch({})
    await expect(auth.fetchUsage()).rejects.toMatchObject({ status: 401 })
  })
})

describe('account profile', () => {
  const withAccount = (account) => ({ '/oauth/profile': { body: { account } } })

  test('maps the account identity', async () => {
    seedToken(LIVE)
    mockFetch(withAccount({ email: 'a@b.com', display_name: 'Ana', has_claude_max: true }))
    expect(await auth.fetchProfile()).toEqual({ email: 'a@b.com', name: 'Ana', plan: 'Max' })
  })

  test.each([
    [{ has_claude_max: true, has_claude_pro: true }, 'Max'],
    [{ has_claude_pro: true }, 'Pro'],
    [{}, null],
  ])('plan tier %o → %s', async (flags, plan) => {
    seedToken(LIVE)
    mockFetch(withAccount({ email: 'a@b.com', ...flags }))
    expect((await auth.fetchProfile()).plan).toBe(plan)
  })

  test('falls back to full_name and tolerates a nameless account', async () => {
    seedToken(LIVE)
    mockFetch(withAccount({ email: 'a@b.com', full_name: 'Ana Full' }))
    expect((await auth.fetchProfile()).name).toBe('Ana Full')
  })

  test('is cached for the session and dropped on logout', async () => {
    seedToken(LIVE)
    mockFetch(withAccount({ email: 'a@b.com' }))
    await auth.fetchProfile()
    await auth.fetchProfile()
    expect(calls.filter((c) => c.url.includes('/profile')).length).toBe(1)

    auth.clear()
    seedToken(LIVE)
    await auth.fetchProfile()
    expect(calls.filter((c) => c.url.includes('/profile')).length).toBe(2)
  })

  test('surfaces a failed profile fetch with its status', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/profile': { status: 500 } })
    await expect(auth.fetchProfile()).rejects.toMatchObject({ status: 500 })
  })

  test('an empty payload yields nulls rather than throwing', async () => {
    seedToken(LIVE)
    mockFetch({ '/oauth/profile': { body: {} } })
    expect(await auth.fetchProfile()).toEqual({ email: null, name: null, plan: null })
  })
})
