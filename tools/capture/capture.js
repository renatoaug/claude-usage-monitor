// Records each pet state + the full widget, building a GIF for each (needs ffmpeg).
//   bun run gifs   (or: electron tools/capture/capture.js)
//
// Everything is captured from renderer/index.html itself. This file is the main
// process, so it can feed the real preload bridge over the very channels main.js
// uses — no HTML stub, and so no second copy of the markup to drift out of date.
const { app, BrowserWindow } = require('electron')
const path = require('node:path')
const fs = require('node:fs')
const os = require('node:os')
const { execSync } = require('node:child_process')

const ROOT = path.join(__dirname, '..', '..')
const DOCS = path.join(ROOT, 'docs', 'media')
const FRAMES = path.join(os.tmpdir(), 'cum-frames')

// `state` is what ./pet would write; `trigger` is a one-shot fired mid-recording
const STATES = [
  { name: 'idle', state: 'idle', ms: 4200 },
  { name: 'working', state: 'working', ms: 3200 },
  { name: 'sleeping', state: 'sleeping', ms: 3400 },
  { name: 'on-fire', state: 'fire', ms: 2200 },
  { name: 'tired', state: 'tired', ms: 3400 },
  { name: 'poke', state: 'idle', trigger: 'poke', ms: 1600 },
  { name: 'celebrate', state: 'idle', trigger: 'celebrate', ms: 2200 },
  { name: 'reading', state: 'reading', ms: 5000 },
  { name: 'editing', state: 'editing', ms: 5000 },
  { name: 'running', state: 'running', ms: 5500 },
]

// plausible numbers, so the meters and lists in the overview look inhabited
const FAKE_USAGE = {
  session: { pct: 42, tokens: 5_240_000, active: true },
  week: { pct: 68, tokens: 1_120_000_000 },
  today: { tokens: 37_800_000 },
  byModel: [
    { label: 'Opus 5', tokens: 915_500_000 },
    { label: 'Fable 5.1', tokens: 200_400_000 },
  ],
  byProject: [
    { label: 'clauddy', tokens: 577_800_000 },
    { label: 'personal-site', tokens: 366_200_000 },
    { label: 'little-experiments', tokens: 70_100_000 },
  ],
  days30: Array.from({ length: 30 }, (_, i) => 20_000 + ((i * 37) % 148) * 1000),
  monthTokens: 8_070_000_000,
  tokensPerMin: 12_400,
  active: true,
  sleeping: false,
  ts: Date.now(),
}
const FAKE_REAL = {
  session: { pct: 42, resetMs: 8_000_000 },
  week: { pct: 68, resetMs: 90_000_000 },
}

const delay = (ms) => new Promise((r) => setTimeout(r, ms))

// capturePage rejects with UnknownVizError when the compositor has not produced
// a frame yet — right after a load, or while the window is not being composed.
// Unhandled, that rejection leaves the run hanging forever, so a frame is worth
// a few retries and, failing that, worth skipping.
async function shoot(win, rect) {
  for (let attempt = 0; attempt < 6; attempt++) {
    try {
      return rect ? await win.webContents.capturePage(rect) : await win.webContents.capturePage()
    } catch {
      await delay(150)
    }
  }
  return null
}

async function grab(win, ms, rect) {
  const frames = []
  const interval = 50
  const start = Date.now()
  while (Date.now() - start < ms) {
    const t0 = Date.now()
    const img = await shoot(win, rect)
    if (img) frames.push(img.toPNG())
    const dt = Date.now() - t0
    if (dt < interval) await delay(interval - dt)
  }
  return { frames, elapsed: Date.now() - start }
}

function encode(name, frames, elapsed, width) {
  const dir = path.join(FRAMES, name)
  fs.mkdirSync(dir, { recursive: true })
  frames.forEach((png, i) => {
    fs.writeFileSync(path.join(dir, `f${String(i).padStart(4, '0')}.png`), png)
  })
  const fps = Math.max(1, Math.round(frames.length / (elapsed / 1000)))
  const out = path.join(DOCS, `${name}.gif`)
  const vf = `scale=${width}:-1:flags=lanczos,split[s0][s1];[s0]palettegen=stats_mode=full[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3`
  execSync(`ffmpeg -y -framerate ${fps} -i "${dir}/f%04d.png" -vf "${vf}" -loop 0 "${out}"`, {
    stdio: 'ignore',
  })
  console.log(`${name}: ${frames.length} frames @ ${fps}fps -> ${out}`)
}

app.whenReady().then(async () => {
  fs.mkdirSync(DOCS, { recursive: true })
  fs.rmSync(FRAMES, { recursive: true, force: true })

  const win = new BrowserWindow({
    width: 320,
    height: 760,
    x: 60,
    y: 60,
    show: true,
    backgroundColor: '#110c0a',
    webPreferences: {
      preload: path.join(ROOT, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  })
  const send = (channel, payload) => win.webContents.send(channel, payload)
  const exec = (js) => win.webContents.executeJavaScript(js)
  const rectOf = (sel) =>
    exec(
      `(()=>{const r=document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect();` +
        'return {x:Math.max(0,Math.floor(r.x)),y:Math.max(0,Math.floor(r.y)),' +
        'width:Math.ceil(r.width),height:Math.ceil(r.height)}})()',
    )

  // Everything main.js would push on startup, so the card looks connected and
  // inhabited rather than empty.
  async function seed() {
    send('config', { mode: 'floating', zoom: 100, alerts: true, fireThreshold: 90 })
    send('accounts', {
      active: 'a1',
      accounts: [{ id: 'a1', label: 'you@example.com', connected: true }],
    })
    send('auth-state', { connected: true })
    send('profile', { email: 'you@example.com', plan: 'Max' })
    send('real-usage', FAKE_REAL)
    send('usage', FAKE_USAGE)
    send('version', '1.0.0')
    await delay(400)
  }

  await win.loadFile(path.join(ROOT, 'renderer', 'index.html'))
  await delay(600)
  await seed()
  await delay(1600) // let the welcome wave finish

  // --- 1) per-state GIFs: just the stage, so the pet fills the frame ---
  for (const s of STATES) {
    send('debug-state', { state: s.state })
    await delay(s.trigger ? 500 : 900) // scene props fade in on their own curve
    const rect = await rectOf('#stage')
    const rec = grab(win, s.ms, rect)
    if (s.trigger) {
      await delay(60)
      send('debug-state', { state: s.trigger })
    }
    const { frames, elapsed } = await rec
    encode(s.name, frames, elapsed, 420)
  }

  // --- 2) full-widget GIF: the whole card ---
  send('debug-state', { state: 'auto' })
  send('usage', { ...FAKE_USAGE, active: false, sleeping: false })
  await delay(900)
  const card = await rectOf('#card')
  const overview = await grab(win, 5000, card)
  encode('overview', overview.frames, overview.elapsed, 300)

  console.log('done')
  app.quit()
})
