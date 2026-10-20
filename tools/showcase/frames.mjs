// Frame-exact capture: the page's clock is faked and every CSS/WAAPI animation
// is paused and seeked by hand, so each screenshot lands exactly 1/FPS later no
// matter how long the screenshot itself takes.
import fs from 'node:fs'

export const FPS = 30
const STEP = 1000 / FPS

// call before page.goto
export const freeze = (page) => page.clock.install()

async function advance(page) {
  await page.clock.runFor(STEP)
  await page.evaluate((step) => {
    for (const a of document.getAnimations()) {
      if (a.__t === undefined) {
        a.__t = a.currentTime || 0
        a.pause()
      }
      a.__t += step
      const t = a.effect?.getComputedTiming()
      const end = t ? t.endTime : Number.POSITIVE_INFINITY
      if (Number.isFinite(end) && a.__t >= end) {
        if (!a.__done) {
          a.__done = true
          a.finish()
        }
      } else a.currentTime = a.__t
    }
  }, STEP)
}

// runs `ms` of page time without capturing
export async function wait(page, ms) {
  for (let t = 0; t < ms; t += STEP) await advance(page)
}

// captures `ms` of page time into dir/0000.png…; `at` maps a frame index to a
// callback, for events fired mid-clip
export async function record(page, dir, ms, { clip, at = {} } = {}) {
  fs.rmSync(dir, { recursive: true, force: true })
  fs.mkdirSync(dir, { recursive: true })
  const n = Math.round((ms / 1000) * FPS)
  for (let i = 0; i < n; i++) {
    if (at[i]) await at[i]()
    await advance(page)
    await page.screenshot({
      path: `${dir}/${String(i).padStart(4, '0')}.png`,
      omitBackground: true,
      clip,
      animations: 'allow',
    })
  }
  return n
}
