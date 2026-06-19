// Records each pet state and builds a GIF per state (needs ffmpeg on PATH).
//   bun run gifs   (or: electron tools/capture/capture.js)
const { app, BrowserWindow } = require('electron')
const path = require('node:path')
const fs = require('node:fs')
const os = require('node:os')
const { execSync } = require('node:child_process')

const ROOT = path.join(__dirname, '..', '..')
const DOCS = path.join(ROOT, 'docs', 'media')
const FRAMES = path.join(os.tmpdir(), 'cum-frames')
const W = 240
const H = 150

// base = resting state to set first; trigger = one-shot fired while recording
const STATES = [
  { name: 'idle', base: 'idle', ms: 4200 },
  { name: 'working', base: 'working', ms: 3200 },
  { name: 'sleeping', base: 'sleeping', ms: 3400 },
  { name: 'on-fire', base: 'fire', ms: 2200 },
  { name: 'poke', base: 'idle', trigger: 'poke', ms: 1600 },
  { name: 'celebrate', base: 'idle', trigger: 'celebrate', ms: 2200 },
]

const delay = (ms) => new Promise((r) => setTimeout(r, ms))

app.disableHardwareAcceleration()

app.whenReady().then(async () => {
  fs.mkdirSync(DOCS, { recursive: true })
  fs.rmSync(FRAMES, { recursive: true, force: true })
  fs.mkdirSync(FRAMES, { recursive: true })

  const win = new BrowserWindow({
    width: W,
    height: H,
    x: 80,
    y: 80,
    frame: false,
    resizable: false,
    backgroundColor: '#110c0a',
    webPreferences: { contextIsolation: true, nodeIntegration: false },
  })
  await win.loadFile(path.join(__dirname, 'capture.html'))
  await delay(1600) // let the welcome "wave" finish
  await win.webContents.executeJavaScript('window.__init()')

  const exec = (js) => win.webContents.executeJavaScript(js)

  async function grab(ms) {
    const frames = []
    const interval = 50
    const start = Date.now()
    while (Date.now() - start < ms) {
      const t0 = Date.now()
      const img = await win.webContents.capturePage()
      frames.push(img.toPNG())
      const dt = Date.now() - t0
      if (dt < interval) await delay(interval - dt)
    }
    return { frames, elapsed: Date.now() - start }
  }

  for (const s of STATES) {
    await exec(`window.__set('${s.base}')`)
    await delay(s.trigger ? 400 : 300)
    const rec = grab(s.ms)
    if (s.trigger) {
      await delay(60)
      await exec(`window.__set('${s.trigger}')`)
    }
    const { frames, elapsed } = await rec
    const dir = path.join(FRAMES, s.name)
    fs.mkdirSync(dir, { recursive: true })
    frames.forEach((png, i) => {
      fs.writeFileSync(path.join(dir, `f${String(i).padStart(4, '0')}.png`), png)
    })
    const fps = Math.max(1, Math.round(frames.length / (elapsed / 1000)))
    const out = path.join(DOCS, `${s.name}.gif`)
    const vf =
      'scale=420:-1:flags=lanczos,split[s0][s1];[s0]palettegen=stats_mode=full[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3'
    execSync(`ffmpeg -y -framerate ${fps} -i "${dir}/f%04d.png" -vf "${vf}" -loop 0 "${out}"`, {
      stdio: 'ignore',
    })
    console.log(`${s.name}: ${frames.length} frames @ ${fps}fps -> ${out}`)
  }

  console.log('done')
  app.quit()
})
