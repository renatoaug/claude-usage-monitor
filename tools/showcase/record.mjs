import fs from 'node:fs'
import path from 'node:path'
import { chromium } from 'playwright'

const HERE = path.dirname(new URL(import.meta.url).pathname)
const REPO = path.resolve(HERE, '..', '..')
const URL_ = `file://${REPO}/renderer/index.html`
const LANG = process.env.LANG_ || 'en'
const script = JSON.parse(fs.readFileSync(`${HERE}/narration.${LANG}.json`, 'utf8'))
const clips = JSON.parse(fs.readFileSync(`${HERE}/clips/${LANG}/manifest.json`, 'utf8'))
const beat = Object.fromEntries(script.beats.map((b) => [b.id, b]))
const PAD = 0.7 // breathing room after each narration line
const PAD_LEAD = 180 // must match LEAD in assemble.py — when the clip starts
const W = 1920
const H = 1080

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const cuts = []

// ---------- fake data (never the user's real account) ----------
const days30 = [
  4.1, 2.2, 0, 6.8, 9.4, 3.1, 0.4, 7.7, 12.1, 5.5, 2.9, 0, 8.3, 10.9, 6.2, 3.4, 1.1, 9.8, 13.6, 7.1,
  4.4, 0.9, 6.6, 11.2, 8.8, 5.1, 2.6, 10.4, 14.2, 9.1,
].map((v) => Math.round(v * 1e6))

const usage = (o = {}) => ({
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
    { label: 'claude-usage-monitor', tokens: 74_300_000 },
    { label: 'nova-checkout', tokens: 52_100_000 },
    { label: 'orbit-api', tokens: 38_600_000 },
    { label: 'design-system', tokens: 21_400_000 },
    { label: 'infra-terraform', tokens: 14_900_000 },
    { label: 'other · 6', tokens: 13_500_000 },
  ],
  days30,
  monthTokens: 892_000_000,
  ...o,
})

const real = (sessPct, resetMin, wkPct = 41) => ({
  session: { pct: sessPct, resetMs: resetMin * 60_000 },
  week: { pct: wkPct, resetMs: 3.1 * 24 * 3600_000 },
})

// ---------- page bootstrap ----------
const INIT = `
  // stub the Electron preload bridge; every channel becomes a callback we can fire
  const H = {}
  const on = (k) => (cb) => { (H[k] ||= []).push(cb) }
  window.__fire = (k, v) => (H[k] || []).forEach((cb) => cb(v))
  window.api = {
    onUsage: on('usage'), onError: on('error'), onConfig: on('config'),
    onRealUsage: on('real'), onAuthState: on('auth'), onProfile: on('profile'),
    onAuthResult: on('authres'), onDebugState: on('debug'), onVersion: on('version'),
    onUpdateStatus: on('update'),
    saveConfig: () => {}, resize: () => {}, openUsage: () => {},
    authStart: () => {}, authCode: () => {}, authLogout: () => {},
    checkUpdates: () => {}, doUpdate: () => {}, quit: () => {},
  }
  // the burn-rate tracker needs a 5-minute trail before it speaks; drive it from outside
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

const STAGE = `
  html, body { background: #0d0a09 !important; }
  body::before {
    content: ''; position: fixed; inset: 0; z-index: 0;
    background:
      radial-gradient(900px 700px at 74% 44%, rgba(217,119,87,.20), transparent 62%),
      radial-gradient(1100px 900px at 12% 88%, rgba(126,199,125,.07), transparent 60%),
      linear-gradient(160deg, #17110e 0%, #0d0a09 60%, #080605 100%);
  }
  body::after {
    content: ''; position: fixed; inset: 0; z-index: 1; pointer-events: none;
    background-image: radial-gradient(rgba(255,255,255,.035) 1px, transparent 1px);
    background-size: 4px 4px;
  }
  #card {
    position: fixed !important; top: 50% !important; left: 1330px !important;
    transform: translate(-50%, -50%) scale(var(--vs, 1.5));
    transform-origin: center; z-index: 5;
    transition: transform .55s cubic-bezier(.4,0,.2,1),
                top .55s cubic-bezier(.4,0,.2,1), left .55s cubic-bezier(.4,0,.2,1);
  }
  #vfx { position: fixed; inset: 0; z-index: 40; pointer-events: none;
         font-family: -apple-system, "SF Pro Text", system-ui, sans-serif; }
  #ring { position: absolute; border: 2px solid #d97757; border-radius: 12px;
          box-shadow: 0 0 0 6px rgba(217,119,87,.14), 0 0 34px rgba(217,119,87,.35);
          opacity: 0; transition: all .4s cubic-bezier(.4,0,.2,1); }
  #ring.on { opacity: 1; animation: rpulse .62s cubic-bezier(.2,1.5,.4,1); }
  @keyframes rpulse {
    0%   { box-shadow: 0 0 0 0 rgba(217,119,87,.55), 0 0 0 rgba(217,119,87,0); }
    55%  { box-shadow: 0 0 0 13px rgba(217,119,87,.10), 0 0 44px rgba(217,119,87,.5); }
    100% { box-shadow: 0 0 0 6px rgba(217,119,87,.14), 0 0 34px rgba(217,119,87,.35); }
  }
  #copy { position: absolute; left: 132px; top: 50%; transform: translateY(-50%); width: 780px; }
  #kicker { font-size: 19px; letter-spacing: .2em; text-transform: uppercase;
            color: #d97757; font-weight: 700; margin-bottom: 26px; min-height: 24px;
            display: flex; align-items: center; }
  #kicker .kb { display: inline-block; width: 0; height: 3px; border-radius: 2px;
                background: #d97757; margin-right: 0;
                transition: width .5s cubic-bezier(.2,.9,.25,1),
                            margin-right .5s cubic-bezier(.2,.9,.25,1); }
  #kicker .kt { display: inline-block; opacity: 0; transform: translateX(-10px);
                transition: opacity .45s ease .14s, transform .45s ease .14s; }
  #kicker.on .kb { width: 40px; margin-right: 16px; }
  #kicker.on .kt { opacity: 1; transform: none; }
  #cap { font-size: 43px; line-height: 1.34; color: #f3ebe3; font-weight: 500;
         letter-spacing: -.015em; }
  /* one span per spoken word, delayed by that word's real timestamp */
  #cap .w { display: inline-block; opacity: 0; filter: blur(9px); will-change: transform;
            transform: translateY(30px) scale(.88) rotate(-5deg); }
  #cap.on .w { opacity: 1; filter: blur(0); transform: none;
               transition: opacity .3s ease var(--d), filter .3s ease var(--d),
                           transform .52s cubic-bezier(.18,1.65,.4,1) var(--d); }
  #cap .w.e { color: #f0a173; font-weight: 700; }
  #cap.on .w.e { animation: wpop .58s cubic-bezier(.2,1.7,.4,1) var(--d) both; }
  @keyframes wpop {
    0%   { opacity: 0; filter: blur(9px); transform: translateY(30px) scale(.88) rotate(-5deg); }
    55%  { opacity: 1; filter: blur(0);   transform: translateY(0) scale(1.16) rotate(1.5deg); }
    100% { opacity: 1; filter: blur(0);   transform: none; }
  }
  #cap.out .w { opacity: 0; filter: blur(6px); transform: translateY(-22px) scale(.94);
                transition: opacity .26s ease var(--od), filter .26s ease var(--od),
                            transform .3s cubic-bezier(.5,0,.75,0) var(--od);
                animation: none; }
  #brand { position: absolute; left: 132px; bottom: 74px; max-width: 780px;
           display: flex; gap: 14px; align-items: center;
           opacity: 0; transition: opacity .5s ease; }
  #brand.on { opacity: 1; }
  #brand code, #outro code { font: 500 17px/1.6 ui-monospace, "SF Mono", monospace;
                color: #e6a07f; background: rgba(217,119,87,.10);
                border: 1px solid rgba(217,119,87,.28);
                border-radius: 9px; padding: 14px 18px; white-space: pre; text-align: left; }
  #invite { margin-top: 20px; font-size: 21px; color: #9d9085; }
  /* A stand-in macOS menu bar. The Clauddy item is the real tray asset
     (build/trayTemplate.png) inverted the way macOS renders a template icon on a
     dark bar, and the title matches what main.js actually sets: ' 63%'. */
  #mbar { position: absolute; top: 0; left: 0; right: 0; height: 34px;
          background: rgba(22,18,15,.88); backdrop-filter: blur(20px);
          border-bottom: 1px solid rgba(255,255,255,.07);
          display: flex; align-items: center; justify-content: flex-end; gap: 20px;
          padding-right: 34px; transform: translateY(-100%); z-index: 6;
          transition: transform .45s cubic-bezier(.4,0,.2,1); }
  #mbar.on { transform: none; }
  .mb-item { display: flex; align-items: center; gap: 5px; }
  .mb-item.hot { background: rgba(255,255,255,.16); border-radius: 6px;
                 padding: 4px 8px; margin: 0 -8px; }
  .mb-icon { width: 19px; height: 17px; image-rendering: pixelated;
             filter: brightness(0) invert(1); opacity: .92; }
  .mb-pct { font: 500 14px/1 -apple-system, "SF Pro Text", system-ui; color: #f0ece8; }
  .mb-sys { width: 17px; height: 17px; fill: none; stroke: #f0ece8; stroke-width: 1.6;
            stroke-linecap: round; stroke-linejoin: round; opacity: .9; }
  .mb-clock { font: 400 14px/1 -apple-system, "SF Pro Text", system-ui; color: #f0ece8; }

  /* the popover hangs off the tray icon, notch and all */
  html.mb #card { top: 42px !important; left: var(--pop-x, 1720px) !important;
                  transform: translate(-50%, 0) scale(var(--mbs, 1.45));
                  transform-origin: top center; }

  /* title + outro cards: the real pet tile, centered, with typography under it */
  #title, #outro { position: absolute; left: 0; right: 0; top: 700px; text-align: center;
                   opacity: 0; transform: translateY(16px);
                   transition: opacity .6s ease .1s, transform .6s ease .1s; }
  #title.on, #outro.on { opacity: 1; transform: none; }
  #wordmark { font-size: 94px; font-weight: 700; letter-spacing: -.03em; color: #f3ebe3; }
  #tagline { margin-top: 26px; font-size: 31px; color: #9d9085; font-weight: 400; }
  #outro-url { font-size: 38px; font-weight: 600; color: #e6a07f; letter-spacing: -.01em; }
  #outro-cmd { margin-top: 30px; display: flex; justify-content: center; }
  html.card-tile #card { left: 960px !important; top: 38% !important;
                         transform: translate(-50%, -50%) scale(2.9); }
  html.card-tile #controls { opacity: 0; transition: opacity .3s ease; }
  #flash { position: fixed; inset: 0; background: #ff00ff; z-index: 99; display: none; }
`

const browser = await chromium.launch({
  args: ['--force-color-profile=srgb', '--font-render-hinting=none'],
})
const ctx = await browser.newContext({
  viewport: { width: W, height: H },
  deviceScaleFactor: 1,
  recordVideo: { dir: `${HERE}/takes`, size: { width: W, height: H } },
})
await ctx.addInitScript(INIT)
const page = await ctx.newPage()

async function boot() {
  await page.goto(URL_)
  await page.waitForFunction(() => !!document.body && !!document.getElementById('card'))
  await page.addStyleTag({ content: STAGE })
  await page.evaluate(
    ([tagline, url, install, invite, icon]) => {
      const v = document.createElement('div')
      v.id = 'vfx'
      v.innerHTML =
        '<div id="ring"></div>' +
        '<div id="copy"><div id="kicker"></div><div id="cap"></div></div>' +
        `<div id="brand"><code>${install}</code></div>` +
        '<div id="mbar">' +
        `<span class="mb-item hot"><img class="mb-icon" src="file://${icon}">` +
        '<span class="mb-pct">63%</span></span>' +
        '<svg class="mb-sys" viewBox="0 0 24 24"><path d="M5 12.5a10 10 0 0 1 14 0"/>' +
        '<path d="M8.5 16a5.5 5.5 0 0 1 7 0"/><circle cx="12" cy="19" r="1" fill="currentColor"/></svg>' +
        '<svg class="mb-sys" viewBox="0 0 24 24"><rect x="2" y="8" width="16" height="9" rx="2.5"/>' +
        '<path d="M20.5 11.5v3"/><rect x="4" y="10" width="10" height="5" rx="1" fill="#f0ece8" stroke="none"/></svg>' +
        '<span class="mb-clock">Fri 9:41</span></div>' +
        `<div id="title"><div id="wordmark">Clauddy</div><div id="tagline">${tagline}</div></div>` +
        `<div id="outro"><div id="outro-url">${url}</div>` +
        `<div id="outro-cmd"><code>${install}</code></div>` +
        `<div id="invite">${invite}</div></div>`
      document.body.appendChild(v)
      const f = document.createElement('div')
      f.id = 'flash'
      document.body.appendChild(f)
    },
    [
      script.tagline,
      script.outro,
      // shell line-continuation so it fits the column and still pastes correctly
      'curl -fsSL https://raw.githubusercontent.com/renatoaug/\\\n' +
        '  claude-usage-monitor/main/install.sh | bash',
      script.invite,
      // the real menu-bar asset, so the bar shows the icon the app actually ships
      `${REPO}/build/trayTemplate@2x.png`,
    ],
  )
  await page.evaluate(() => {
    window.__fire('config', { mode: 'floating', fireThreshold: 90 })
    window.__fire('auth', { connected: true })
    window.__fire('profile', { email: 'you@example.com', plan: 'max' })
  })
}

// ---------- helpers ----------
async function marker(id) {
  await page.evaluate(() => {
    document.getElementById('flash').style.display = 'block'
  })
  await sleep(90)
  await page.evaluate(() => {
    document.getElementById('flash').style.display = 'none'
  })
  cuts.push({ id })
}

async function push(d, r, proj = null) {
  await page.evaluate(
    ([d, r, proj]) => {
      window.__proj = proj
      if (r !== undefined) window.__fire('real', r)
      if (d) window.__fire('usage', d)
    },
    [d, r, proj],
  )
}

// the on-screen line is the narrated line, so it appears the way it is spoken:
// one span per word, each delayed by that word's real timestamp in the clip.
const wordsFor = (id) =>
  JSON.parse(fs.readFileSync(`${HERE}/clips/${LANG}/${id}.words.json`, 'utf8'))

async function prepare(id) {
  const b = beat[id]
  await page.evaluate(
    ([kick, words, emph, lead]) => {
      const fold = (x) =>
        x
          .toLowerCase()
          .normalize('NFKD')
          .replace(/[^a-z0-9]/g, '')
      const hot = new Set(emph.map(fold))
      document.getElementById('kicker').classList.remove('on')
      document.getElementById('kicker').innerHTML =
        `<span class="kb"></span><span class="kt">${kick}</span>`
      const cap = document.getElementById('cap')
      cap.className = ''
      cap.innerHTML = words
        .map(
          (w, i) =>
            `<span class="w${hot.has(fold(w.w)) ? ' e' : ''}" ` +
            `style="--d:${Math.round(w.t * 1000 + lead)}ms;--od:${i * 16}ms">${w.w}</span>`,
        )
        .join(' ')
    },
    [b.kicker || '', wordsFor(id), b.emph || [], PAD_LEAD],
  )
}

const revealCopy = () =>
  page.evaluate(() => {
    document.getElementById('cap').classList.add('on')
    document.getElementById('kicker').classList.add('on')
  })

const exitCopy = async () => {
  await page.evaluate(() => {
    const cap = document.getElementById('cap')
    const n = cap.children.length
    for (const [i, el] of [...cap.children].entries())
      el.style.setProperty('--od', `${(n - 1 - i) * 14}ms`)
    cap.classList.remove('on')
    cap.classList.add('out')
    document.getElementById('kicker').classList.remove('on')
  })
  await sleep(360)
}

async function ring(sel, pad = 10) {
  await page.evaluate(
    ([sel, pad]) => {
      const r = document.getElementById('ring')
      if (!sel) return r.classList.remove('on')
      const e = document.querySelector(sel)
      if (!e) return r.classList.remove('on')
      const b = e.getBoundingClientRect()
      r.style.left = `${b.left - pad}px`
      r.style.top = `${b.top - pad}px`
      r.style.width = `${b.width + pad * 2}px`
      r.style.height = `${b.height + pad * 2}px`
      r.classList.add('on')
    },
    [sel, pad],
  )
}

const show = (sel, on) =>
  page.evaluate(([sel, on]) => document.querySelector(sel).classList.toggle('on', on), [sel, on])
const cls = (name, on) =>
  page.evaluate(([name, on]) => document.documentElement.classList.toggle(name, on), [name, on])
// centre {tile, caption} as one block so the wordmark never crowds the card
const layoutTile = (sel) =>
  page.evaluate(
    ([sel]) => {
      const card = document.getElementById('card')
      const cap = document.querySelector(sel)
      const gap = 70
      const ch = card.getBoundingClientRect().height
      const th = cap.offsetHeight
      const top = (window.innerHeight - (ch + gap + th)) / 2 - 16
      card.style.setProperty('top', `${top + ch / 2}px`, 'important')
      cap.style.top = `${top + ch + gap}px`
    },
    [sel],
  )

const clipMs = (id, extra = 0) => Math.round((clips[id].duration + PAD + extra) * 1000)
const hold = (id, extra = 0) => sleep(clipMs(id, extra))

// ---------- the takes ----------
async function takeTitle() {
  await boot()
  await push(usage({ activity: 'editing' }), real(63, 74))
  await page.evaluate(() => document.body.classList.add('collapsed'))
  await cls('card-tile', true)
  await sleep(700)
  await layoutTile('#title')
  await sleep(400)
  await show('#title', true)
  await sleep(350)
  await marker('b0_title')
  await hold('b0_title', -0.35)
}

async function takeA() {
  // b1 — expand the tile into the full widget
  await show('#title', false)
  await sleep(250)
  await page.evaluate(() => document.getElementById('card').style.removeProperty('top'))
  await cls('card-tile', false)
  await page.evaluate(() => document.body.classList.remove('collapsed'))
  await sleep(450)
  await exitCopy()
  await prepare('b1_open')
  await marker('b1_open')
  await revealCopy()
  await hold('b1_open')

  // The tour opens on the number people actually came for — the session bar — then
  // walks down the card. The live status line belongs with the pet's reactions, so
  // both wait until the end, once there is something to react to.

  // b2 — session bar
  await exitCopy()
  await prepare('b2_session')
  await push(usage(), real(63, 74), null)
  await ring('#limits-meters .meter:nth-child(1)')
  await marker('b2_session')
  await revealCopy()
  await hold('b2_session')

  // b3 — burn rate
  await exitCopy()
  await prepare('b3_burn')
  await push(usage(), real(63, 74), { kind: 'eta', ms: 35 * 60_000 })
  await sleep(380)
  await ring('#session-proj', 3)
  await marker('b3_burn')
  await revealCopy()
  await hold('b3_burn')

  // b4 — weekly
  await exitCopy()
  await prepare('b4_week')
  await ring('#limits-meters .meter:nth-child(2)')
  await marker('b4_week')
  await revealCopy()
  await hold('b4_week')

  // b7 — by model
  await exitCopy()
  await prepare('b7_history')
  await ring('#bymodel')
  await marker('b7_history')
  await revealCopy()
  await hold('b7_history')

  // b7b — by project
  await exitCopy()
  await prepare('b7b_project')
  await ring('#byproject')
  await marker('b7b_project')
  await revealCopy()
  await hold('b7b_project')

  // b7c — the 30-day map, the bottom of the card
  await exitCopy()
  await prepare('b7c_heat')
  await ring('#heat')
  await marker('b7c_heat')
  await revealCopy()
  await hold('b7c_heat')

  // b5 — status line, activities cycling underneath it
  await exitCopy()
  await prepare('b5_status')
  await ring('#statusline', 8)
  await marker('b5_status')
  await revealCopy()
  const acts = ['reading', 'editing', 'running', 'planning']
  const slice = clipMs('b5_status') / acts.length
  for (const a of acts) {
    await push(
      usage({ activity: a, tokensPerMin: 1_200_000 + Math.round(Math.random() * 9e5) }),
      undefined,
      { kind: 'eta', ms: 35 * 60_000 },
    )
    await sleep(Math.round(slice))
  }

  // b6 — the three reactions: asleep, on fire, then flat out at the limit
  await exitCopy()
  await prepare('b6_states')
  await ring(null)
  await marker('b6_states')
  await revealCopy()
  const beat3 = clipMs('b6_states') / 3
  await push(usage({ active: false, sleeping: true, activity: null }), real(63, 74), null)
  await sleep(Math.round(beat3))
  await push(usage({ active: false, sleeping: false, activity: null }), real(94, 26), null)
  await sleep(Math.round(beat3))
  // 100% is what pet.js turns into `tired` — no debug override needed
  await push(usage({ active: false, sleeping: false, activity: null }), real(100, 4), null)
  await sleep(Math.round(beat3))
  // back to a calm card before the widget shrinks away
  await push(usage(), real(63, 74), { kind: 'eta', ms: 35 * 60_000 })
  await sleep(400)
}

async function takeB() {
  // b8 — fold the breakdowns, then shrink to the face
  await exitCopy()
  await prepare('b8_small')
  await ring(null)
  await marker('b8_small')
  await revealCopy()
  // paced against the clip, not on top of it: hold() sleeps a whole clip, so the
  // steps below have to add up to it rather than precede it
  const C = clipMs('b8_small')
  await sleep(Math.round(C * 0.3))
  // fold both breakdowns, one after the other, the way a user would
  for (const sec of ['byproject', 'bymodel']) {
    await page.evaluate((x) => document.querySelector(`.sec-head[data-sec="${x}"]`).click(), sec)
    await sleep(420)
  }
  await sleep(Math.round(C * 0.22))
  await page.evaluate(() => {
    document.body.classList.add('collapsed')
    document.documentElement.style.setProperty('--vs', '2.9')
  })
  await sleep(Math.round(C * 0.3) + 500)

  // b8b — into the menu bar, panel and all
  await exitCopy()
  await prepare('b8b_menubar')
  await marker('b8b_menubar')
  await revealCopy()
  await show('#mbar', true)
  await sleep(520)
  // the popover hangs off the tray icon, so anchor it where the icon actually landed
  await page.evaluate(() => {
    const r = document.querySelector('.mb-item.hot').getBoundingClientRect()
    document.documentElement.style.setProperty('--pop-x', `${Math.round(r.left + r.width / 2)}px`)
  })
  await sleep(320)
  await cls('mb', true)
  await sleep(420)
  // in menu-bar mode the widget lives behind the icon and a click opens the panel
  await page.evaluate(() => document.body.classList.remove('collapsed'))
  await sleep(Math.max(600, clipMs('b8b_menubar') - 1260 + 500))

  // b9 — install
  await show('#mbar', false)
  await cls('mb', false)
  await page.evaluate(() => document.documentElement.style.setProperty('--vs', '2.05'))
  await exitCopy()
  await prepare('b9_close')
  await show('#brand', true)
  await marker('b9_close')
  await revealCopy()
  await push(usage({ activity: 'reading' }), real(63, 74), { kind: 'eta', ms: 35 * 60_000 })
  await sleep(350)
  await page.evaluate(() => document.body.classList.remove('collapsed'))
  await hold('b9_close')
}

async function takeOutro() {
  // b10 — back to the tile, confetti, repo + install
  await show('#brand', false)
  await exitCopy()
  await page.evaluate(() => document.body.classList.add('collapsed'))
  await cls('card-tile', true)
  await sleep(600)
  await layoutTile('#outro')
  await sleep(400)
  await show('#outro', true)
  await sleep(400)
  await marker('b10_end')
  await page.evaluate(() => window.__fire('debug', { state: 'celebrate' }))
  await hold('b10_end', 0.8)
  await marker('__end')
  await sleep(300)
}

await takeTitle()
await takeA()
await takeB()
await takeOutro()

const v = page.video()
await ctx.close()
await browser.close()
const raw = await v.path()
fs.mkdirSync(`${HERE}/takes/${LANG}`, { recursive: true })
fs.renameSync(raw, `${HERE}/takes/${LANG}/main.webm`)
fs.writeFileSync(`${HERE}/takes/${LANG}/cuts.json`, JSON.stringify(cuts, null, 2))
console.log(`[${LANG}] take: takes/${LANG}/main.webm · ${cuts.length} markers`)
