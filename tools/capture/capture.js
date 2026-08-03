// Records each pet state + the full widget, building a GIF for each (needs ffmpeg).
//   bun run gifs   (or: electron tools/capture/capture.js)
const { app, BrowserWindow } = require('electron')
const path = require('node:path')
const fs = require('node:fs')
const os = require('node:os')
const { execSync } = require('node:child_process')

const ROOT = path.join(__dirname, '..', '..')
const DOCS = path.join(ROOT, 'docs', 'media')
const FRAMES = path.join(os.tmpdir(), 'cum-frames')

// base = resting state to set first; trigger = one-shot fired while recording
const STATES = [
  { name: 'idle', base: 'idle', ms: 4200 },
  { name: 'working', base: 'working', ms: 3200 },
  { name: 'sleeping', base: 'sleeping', ms: 3400 },
  { name: 'on-fire', base: 'fire', ms: 2200 },
  { name: 'tired', base: 'tired', ms: 3400 },
  { name: 'poke', base: 'idle', trigger: 'poke', ms: 1600 },
  { name: 'celebrate', base: 'idle', trigger: 'celebrate', ms: 2200 },
]

// activity states (what Claude Code is doing) — these need the full desk scenery
// (mug/doc/terminal/glasses), so we drive the real renderer directly and capture
// its #stage region, rather than the minimal capture.html stub.
const ACT_STATES = [
  { name: 'reading', cls: 'state-working act-reading live', ms: 5000 },
  { name: 'editing', cls: 'state-working act-editing live', ms: 5000 },
  { name: 'running', cls: 'state-working act-running live', ms: 5500 },
]

const delay = (ms) => new Promise((r) => setTimeout(r, ms))

async function grab(win, ms, rect) {
  const frames = []
  const interval = 50
  const start = Date.now()
  while (Date.now() - start < ms) {
    const t0 = Date.now()
    const img = rect ? await win.webContents.capturePage(rect) : await win.webContents.capturePage()
    frames.push(img.toPNG())
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

app.disableHardwareAcceleration()

app.whenReady().then(async () => {
  fs.mkdirSync(DOCS, { recursive: true })
  fs.rmSync(FRAMES, { recursive: true, force: true })
  fs.mkdirSync(FRAMES, { recursive: true })

  const win = new BrowserWindow({
    width: 300,
    height: 220,
    x: 80,
    y: 80,
    frame: false,
    resizable: true,
    backgroundColor: '#110c0a',
    webPreferences: { contextIsolation: true, nodeIntegration: false },
  })
  const load = async (file) => {
    for (let i = 0; i < 3; i++) {
      try {
        await win.loadFile(path.join(__dirname, file))
        return
      } catch {
        await delay(300)
      }
    }
    throw new Error(`failed to load ${file}`)
  }
  const exec = (js) => win.webContents.executeJavaScript(js)

  // --- 1) per-state GIFs (just the night-scene stage) ---
  win.setContentSize(240, 150)
  await load('capture.html')
  await delay(1600) // let the welcome "wave" finish
  await exec('window.__init()')

  for (const s of STATES) {
    await exec(`window.__set('${s.base}')`)
    await delay(s.trigger ? 400 : 300)
    await exec("document.getElementById('dropzone').innerHTML = ''") // drop stale particles
    const rec = grab(win, s.ms)
    if (s.trigger) {
      await delay(60)
      await exec(`window.__set('${s.trigger}')`)
    }
    const { frames, elapsed } = await rec
    encode(s.name, frames, elapsed, 420)
  }

  // --- 1b) activity-state GIFs: the real renderer, capturing the #stage region.
  // These need the full desk scenery, so we drive the actual index.html and set
  // the body class directly. Uses its own window sized for the full card — the
  // whole stage must sit inside the viewport or capturePage clips the scene, and
  // resizing the shared window mid-run trips capturePage (UnknownVizError). ---
  win.hide() // base captures are done; leave a single window on the compositor
  const awin = new BrowserWindow({
    width: 320,
    height: 600,
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
  const aexec = (js) => awin.webContents.executeJavaScript(js)
  for (const s of ACT_STATES) {
    // reload fresh per state so no previous scene's fade-out bleeds into the
    // opening frames, then let the entrance transitions settle before recording.
    await awin.loadFile(path.join(ROOT, 'renderer', 'index.html'))
    await delay(500)
    await aexec(`document.body.className = ${JSON.stringify(s.cls)}`)
    await delay(900)
    const rect = await aexec(
      "(()=>{const r=document.getElementById('stage').getBoundingClientRect();return {x:Math.max(0,Math.floor(r.x)),y:Math.max(0,Math.floor(r.y)),width:Math.ceil(r.width),height:Math.ceil(r.height)}})()",
    )
    const { frames, elapsed } = await grab(awin, s.ms, rect)
    encode(s.name, frames, elapsed, 420)
  }
  awin.close()
  win.show()

  // --- 2) full-widget GIF (the whole card with live-looking data) ---
  win.setContentSize(300, 220)
  await load('capture-full.html')
  await delay(900)
  await exec('window.__init()')
  await delay(400)
  const cardH = await exec("document.getElementById('card').offsetHeight")
  win.setContentSize(300, Math.round(cardH) + 44)
  await delay(500)
  const overview = await grab(win, 5000)
  encode('overview', overview.frames, overview.elapsed, 300)

  console.log('done')
  app.quit()
})
