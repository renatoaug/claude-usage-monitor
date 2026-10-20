// Records every widget clip the showcase uses into assets/clips/<name>.webm
// (VP9 with alpha, so the clips float on the composition's own background).
//   node capture.mjs [name…]
import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { record, wait } from './frames.mjs'
import { codex, cursor, open, real, usage } from './stage.mjs'

const HERE = path.dirname(new URL(import.meta.url).pathname)
const OUT = `${HERE}/assets/clips`
const TMP = `${HERE}/tmp`
const only = process.argv.slice(2)

// the element's box plus room for its shadow
async function boxOf(page, sel, pad = 14) {
  const r = await page.evaluate((s) => {
    const b = document.querySelector(s).getBoundingClientRect()
    return { x: b.x, y: b.y, width: b.width, height: b.height }
  }, sel)
  return { x: r.x - pad, y: r.y - pad, width: r.width + pad * 2, height: r.height + pad * 2 }
}

function encode(name) {
  fs.mkdirSync(OUT, { recursive: true })
  execFileSync('ffmpeg', [
    '-v',
    'error',
    '-y',
    '-framerate',
    '30',
    '-i',
    `${TMP}/${name}/%04d.png`,
    '-c:v',
    'libvpx-vp9',
    '-pix_fmt',
    'yuva420p',
    '-b:v',
    '6M',
    '-auto-alt-ref',
    '0',
    `${OUT}/${name}.webm`,
  ])
  // a still of the first frame, handy for layout checks
  fs.copyFileSync(`${TMP}/${name}/0000.png`, `${OUT}/${name}.png`)
}

// a pet-state scene inside the full widget's stage
const stageClip =
  (state, { r = real(63, 134), d = usage({ activity: null }), ms = 7500, at } = {}) =>
  async () => {
    const s = await open({ frozen: true, scale: 3 })
    await s.push(d, r)
    if (state) await s.debug(state)
    await wait(s.page, 700)
    const clip = await boxOf(s.page, '#stage', 0)
    return { s, clip, ms, at: at?.(s) }
  }

// pet-only, the sprite's center sits at x=84: a box symmetric around it
const PET_BOX = { x: 0, y: 8, width: 168, height: 150 }
const PEEK_BOX = { x: 0, y: 8, width: 168, height: 186 }

// the same widget in each of its sizes, Claude alone
const modeClip =
  (size, ms, config = {}) =>
  async () => {
    const s = await open({ size, frozen: true, scale: 3, config })
    await s.push(usage(), real(63, 134))
    await s.debug('editing')
    await wait(s.page, 600)
    return { s, clip: await boxOf(s.page, '#card'), ms }
  }

const CLIPS = {
  // the hook: a pet that has hit the wall
  maxed: async () => {
    const s = await open({
      size: 'pet',
      frozen: true,
      scale: 4,
      viewport: { width: 260, height: 220 },
    })
    await s.push(usage({ activity: null, active: false }), real(100, 72))
    await s.debug('tired')
    await wait(s.page, 600)
    return { s, clip: PET_BOX, ms: 5500 }
  },
  // the reveal: idle, then a poke
  hello: async () => {
    const s = await open({
      size: 'pet',
      frozen: true,
      scale: 4,
      viewport: { width: 260, height: 220 },
    })
    await s.push(usage({ activity: null, active: false }), real(24, 260))
    await s.debug('idle')
    await wait(s.page, 600)
    return {
      s,
      clip: PET_BOX,
      ms: 10000,
      at: { 40: () => s.debug('poke'), 170: () => s.debug('poke') },
    }
  },
  // each stays on screen from its entrance to the end of the lineup scene
  expanded: modeClip('expanded', 22000),
  compact: modeClip('compact', 18000),
  menubar: modeClip('compact', 10000, { mode: 'menubar' }),
  // pet-only: the cursor comes over the sprite and the usage glance fades in
  petonly: async () => {
    const s = await open({
      size: 'pet',
      frozen: true,
      scale: 4,
      viewport: { width: 260, height: 220 },
    })
    await s.push(usage(), real(63, 134))
    await s.debug('editing')
    await wait(s.page, 600)
    const hover = (on) => () =>
      s.page.evaluate((on) => window.__fire('onPetPointer', on ? { x: 84, y: 88 } : null), on)
    // once while it's in the spotlight, again once all four are lined up
    const at = { 25: hover(true), 100: hover(false), 255: hover(true), 360: hover(false) }
    return { s, clip: PEEK_BOX, ms: 14000, at }
  },
  reading: stageClip('reading'),
  editing: stageClip('editing'),
  running: stageClip('running'),
  fire: stageClip('fire', { r: real(94, 48) }),
  sleeping: stageClip('sleeping', { d: usage({ activity: null, active: false, sleeping: true }) }),
  celebrate: stageClip('idle', {
    r: real(3, 299),
    at: (s) => ({
      15: () => s.debug('celebrate'),
      90: () => s.debug('celebrate'),
      165: () => s.debug('celebrate'),
    }),
  }),
  // the voice: a few remarks, one after another, in the scene's speech bubble
  talk: async () => {
    const s = await open({ frozen: true, scale: 3, config: { talk: true } })
    await s.push(usage(), real(63, 134))
    await s.debug('editing')
    await wait(s.page, 700)
    const say = (kind) => () =>
      s.page.evaluate((k) => window.__fire('onDebugState', { state: 'say', kind: k }), kind)
    return {
      s,
      clip: await boxOf(s.page, '#stage', 0),
      ms: 13000,
      at: { 10: say('greeting'), 105: say('fire'), 200: say('reset'), 295: say('streak') },
    }
  },
  // the real numbers climb while the pace projection appears
  numbers: async () => {
    const s = await open({ frozen: true, scale: 2.5 })
    await s.push(usage(), real(58, 70))
    await s.debug('editing')
    await wait(s.page, 600)
    const step = (pct, tok, proj) => () =>
      s.push(usage({ session: { tokens: tok } }), real(pct, 70), { proj })
    return {
      s,
      clip: await boxOf(s.page, '#card'),
      ms: 10000,
      at: {
        60: step(61, 6_900_000, { kind: 'eta', ms: 41 * 60_000 }),
        120: step(63, 7_300_000, { kind: 'eta', ms: 35 * 60_000 }),
      },
    }
  },
  // Claude, Codex and Cursor side by side
  trio: async () => {
    const s = await open({
      size: 'compact',
      frozen: true,
      scale: 3,
      config: { codex: true, cursor: true },
    })
    await s.push(usage(), real(63, 134), { codex: codex(), cursor: cursor() })
    await s.debug('editing')
    await wait(s.page, 600)
    return { s, clip: await boxOf(s.page, '#card'), ms: 6500 }
  },
  // the breakdowns: by model, by project, 30 days
  details: async () => {
    const s = await open({ frozen: true, scale: 2.5, viewport: { width: 520, height: 1400 } })
    await s.push(usage(), real(63, 134))
    await s.debug('editing')
    await wait(s.page, 300)
    await s.page.click('#details-toggle')
    await wait(s.page, 900)
    return { s, clip: await boxOf(s.page, '#card'), ms: 7500 }
  },
}

for (const [name, setup] of Object.entries(CLIPS)) {
  if (only.length && !only.includes(name)) continue
  const t = Date.now()
  const { s, clip, ms, at } = await setup()
  const n = await record(s.page, `${TMP}/${name}`, ms, { clip, at })
  await s.browser.close()
  encode(name)
  console.log(
    `${name}: ${n} frames, ${Math.round(clip.width)}x${Math.round(clip.height)}, ${Date.now() - t}ms`,
  )
}
