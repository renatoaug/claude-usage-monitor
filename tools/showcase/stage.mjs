// Loads the real renderer/ UI in Playwright with a stubbed window.api and fake
// usage, so the showcase shows the actual widget without any real account data.
import path from 'node:path'
import { chromium } from 'playwright'
import { freeze } from './frames.mjs'

const HERE = path.dirname(new URL(import.meta.url).pathname)
export const REPO = path.resolve(HERE, '..', '..')
export const BG = '#0d0a09'

// every preload channel becomes a callback the recorder can fire
const INIT = `
  const H = {}
  window.__fire = (k, v) => (H[k] || []).forEach((cb) => cb(v))
  window.api = new Proxy({}, {
    get: (_, k) => (typeof k === 'string' && k.startsWith('on'))
      ? (cb) => { (H[k] ||= []).push(cb) }
      : () => {},
  })
  // the burn-rate tracker needs a 5-minute trail; let the recorder dictate it
  let _B
  Object.defineProperty(globalThis, 'Burn', {
    configurable: true,
    get: () => _B,
    set: (v) => {
      _B = Object.assign({}, v, {
        createBurnTracker: () => ({
          note() {}, slope: () => null, reset() {}, trail: [],
          project: () => window.__proj || null,
        }),
      })
    },
  })
`

const days30 = [
  4.1, 2.2, 0, 6.8, 9.4, 3.1, 0.4, 7.7, 12.1, 5.5, 2.9, 0, 8.3, 10.9, 6.2, 3.4, 1.1, 9.8, 13.6, 7.1,
  4.4, 0.9, 6.6, 11.2, 8.8, 5.1, 2.6, 10.4, 14.2, 9.1,
].map((v) => Math.round(v * 1e6))

export const usage = (o = {}) => ({
  active: true,
  sleeping: false,
  activity: 'editing',
  tokensPerMin: 1_640_000,
  today: { tokens: 9_120_000 },
  session: { tokens: 6_430_000 },
  week: { tokens: 214_800_000 },
  byModel: [
    { label: 'Opus', tokens: 128_400_000 },
    { label: 'Sonnet', tokens: 61_900_000 },
    { label: 'Haiku', tokens: 18_300_000 },
    { label: 'Fable', tokens: 6_200_000 },
  ],
  byProject: [
    { label: 'my-saas-app', tokens: 74_300_000 },
    { label: 'nova-checkout', tokens: 52_100_000 },
    { label: 'orbit-api', tokens: 38_600_000 },
    { label: 'design-system', tokens: 21_400_000 },
    { label: 'infra-terraform', tokens: 14_900_000 },
    { label: 'other · 6', tokens: 13_500_000 },
  ],
  days30,
  monthTokens: 892_000_000,
  ts: Date.now(),
  ...o,
})

export const real = (sess, resetMin, wk = 41) => ({
  session: { pct: sess, resetMs: resetMin * 60_000 },
  week: { pct: wk, resetMs: 3.1 * 24 * 3600_000 },
  scoped: [],
})

export const codex = (o = {}) => ({
  active: false,
  lastSeen: Date.now() - 60_000,
  limitsAt: Date.now(),
  model: 'gpt-5-codex',
  plan: 'plus',
  session: { pct: 38, resetMs: 2.2 * 3600_000 },
  weekly: { pct: 22, resetMs: 5 * 86400_000 },
  tokensToday: 3_400_000,
  tokens5h: 2_100_000,
  tokensWeek: 41_000_000,
  byModel: [{ label: 'gpt-5-codex', tokens: 41_000_000 }],
  byProject: [{ label: 'orbit-api', tokens: 41_000_000 }],
  days30: days30.map((v) => Math.round(v / 5)),
  monthTokens: 120_000_000,
  ...o,
})

export const cursor = (o = {}) => ({
  active: false,
  activity: null,
  lastSeen: Date.now() - 3 * 60_000,
  limitsAt: Date.now(),
  plan: 'pro',
  session: { pct: 27, resetMs: 17 * 86400_000 },
  api: { pct: 9, resetMs: 17 * 86400_000 },
  signedOut: false,
  ...o,
})

// size: 'expanded' | 'compact' | 'pet'
export async function open({
  size = 'expanded',
  scale = 2,
  viewport,
  config = {},
  frozen = false,
} = {}) {
  const browser = await chromium.launch({
    args: ['--force-color-profile=srgb', '--font-render-hinting=none'],
  })
  const ctx = await browser.newContext({
    viewport: viewport || { width: 520, height: 900 },
    deviceScaleFactor: scale,
  })
  await ctx.addInitScript(INIT)
  await ctx.addInitScript(
    `try { localStorage.setItem('clauddy.size', ${JSON.stringify(size)}) } catch {}`,
  )
  const page = await ctx.newPage()
  if (frozen) await freeze(page)
  await page.goto(`file://${REPO}/renderer/index.html`)
  await page.waitForFunction(() => !!document.getElementById('card'))
  await page.addStyleTag({
    content: frozen
      ? 'html, body { background: transparent !important; }'
      : `html, body { background: ${BG} !important; }`,
  })
  await page.evaluate((cfg) => {
    window.__fire('onConfig', { mode: 'floating', fireThreshold: 90, talk: false, ...cfg })
    window.__fire('onAuthState', { connected: true })
    window.__fire('onProfile', { email: 'you@example.com', plan: 'max' })
  }, config)
  const push = (d, r, extra = {}) =>
    page.evaluate(
      ([d, r, extra]) => {
        window.__proj = extra.proj || null
        if (r) window.__fire('onRealUsage', r)
        if (extra.codex) window.__fire('onCodex', extra.codex)
        if (extra.cursor) window.__fire('onCursor', extra.cursor)
        if (d) window.__fire('onUsage', d)
      },
      [d, r, extra],
    )
  const debug = (state) => page.evaluate((s) => window.__fire('onDebugState', { state: s }), state)
  return { browser, page, push, debug }
}
