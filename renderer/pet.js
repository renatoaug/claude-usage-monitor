const SVGNS = 'http://www.w3.org/2000/svg'
const el = (id) => document.getElementById(id)

// Claude pixel-art sprite
const SPRITE = [
  '.########.',
  '.########.',
  '##########',
  '###o##o###',
  '##########',
  '.########.',
  '.########.',
  '.#.#..#.#.',
  '.#.#..#.#.',
]

;(function buildPixel() {
  const body = el('body')
  const eyes = el('eyes')
  const C = 10
  // Each pixel is its own <rect>, so neighbours share an edge. Under the mood
  // animations the sprite is scaled by fractions, that edge lands between device
  // pixels, and the card shows through as a hairline grid — crispEdges cannot
  // help, it rounds in local space, before the transform. So a cell is grown to
  // overlap the neighbour it actually has: same fill, invisible seam, and the
  // silhouette stays exact because edge cells are left alone.
  const SEAM = 0.5
  const filled = (r, c) => {
    const ch = SPRITE[r]?.[c]
    return ch !== undefined && ch !== '.'
  }
  SPRITE.forEach((row, r) => {
    for (let c = 0; c < row.length; c++) {
      const ch = row[c]
      if (ch === '.') continue
      const rect = document.createElementNS(SVGNS, 'rect')
      rect.setAttribute('x', c * C)
      rect.setAttribute('y', r * C)
      rect.setAttribute('width', C + (filled(r, c + 1) ? SEAM : 0))
      rect.setAttribute('height', C + (filled(r + 1, c) ? SEAM : 0))
      body.appendChild(rect)
      if (ch === 'o') {
        const eye = document.createElementNS(SVGNS, 'rect')
        eye.setAttribute('x', c * C)
        eye.setAttribute('y', r * C)
        eye.setAttribute('width', C)
        eye.setAttribute('height', C)
        eyes.appendChild(eye)
      }
    }
  })
})()

// night scene backdrop (clouds, crescent moon, stars, dotted ground/sky)
;(function buildScene() {
  const s = el('scene')
  if (!s) return
  const add = (tag, attrs) => {
    const e = document.createElementNS(SVGNS, tag)
    for (const k in attrs) e.setAttribute(k, attrs[k])
    s.appendChild(e)
  }
  // dotted top + bottom (sky + ground)
  for (let x = 4; x <= 212; x += 9) {
    add('circle', { cx: x, cy: 6, r: 1.3, fill: '#fff', 'fill-opacity': 0.85 })
    add('circle', { cx: x, cy: 130, r: 1.3, fill: '#fff', 'fill-opacity': 0.85 })
  }
  // clouds (blocky, dark gray)
  const cloud = (x, y, b, m, t) => {
    add('rect', { x: x + 12, y: y, width: t, height: 9, fill: '#3a3a3a', class: 'cloud' })
    add('rect', { x: x + 5, y: y + 7, width: m, height: 9, fill: '#3a3a3a', class: 'cloud' })
    add('rect', { x: x, y: y + 14, width: b, height: 10, fill: '#3a3a3a', class: 'cloud' })
  }
  cloud(16, 10, 58, 42, 22)
  cloud(120, 50, 40, 28, 15)
  // stars (small plus)
  const star = (x, y) => {
    add('rect', { x: x - 3, y: y - 0.7, width: 6, height: 1.4, fill: '#d8d8d8' })
    add('rect', { x: x - 0.7, y: y - 3, width: 1.4, height: 6, fill: '#d8d8d8' })
  }
  ;[
    [100, 16],
    [60, 22],
    [150, 100],
    [196, 56],
    [205, 98],
    [128, 26],
    [182, 22],
    [202, 14],
  ].forEach(([x, y]) => {
    star(x, y)
  })
})()

// reading mode: the doc's filename slowly cycles through project docs, since
// it's a Claude app "reading" different files (a gentle fade between names)
;(function cycleDocName() {
  const node = el('dc-fname')
  if (!node) return
  const names = [
    'README.md',
    'ARCHITECTURE.md',
    'api.md',
    'AUTH.md',
    'MIGRATION.md',
    'CONTRIBUTING.md',
    'usage.md',
    'config.md',
  ]
  let i = 0
  setInterval(() => {
    node.style.opacity = '0'
    setTimeout(() => {
      i = (i + 1) % names.length
      node.textContent = names[i]
      node.style.opacity = '1'
    }, 260)
  }, 3800)
})()

// helpers
// labels come from log fields and directory names — neither is ours to trust
function esc(s) {
  return String(s).replace(
    /[&<>"']/g,
    (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c],
  )
}

function fmtTokens(t) {
  t = t || 0
  if (t >= 1e9) return `${(t / 1e9).toFixed(2)}B`
  if (t >= 1e6) return `${(t / 1e6).toFixed(1)}M`
  if (t >= 1e3) return `${(t / 1e3).toFixed(1)}k`
  return String(t)
}
function fmtReset(ms) {
  if (!ms || ms <= 0) return 'now'
  // past a day, hours are the useful grain: "6d 16h", not "160h 57m"
  if (ms >= 86400000)
    return `${Math.floor(ms / 86400000)}d ${Math.floor((ms % 86400000) / 3600000)}h`
  const h = Math.floor(ms / 3600000)
  const m = Math.floor((ms % 3600000) / 60000)
  return h > 0 ? `${h}h ${m}m` : `${m}m`
}
// wall-clock time of the reset, in the machine's own locale + timezone.
// Past 24h the day matters too, so prefix the weekday.
function fmtResetClock(ms) {
  if (!ms || ms <= 0) return null
  const at = new Date(Date.now() + ms)
  const time = at.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
  return ms >= 86400000 ? `${at.toLocaleDateString([], { weekday: 'short' })} ${time}` : time
}
// "2h 22m (14:35)" — countdown plus the clock time it lands on
function fmtResetIn(ms) {
  const at = fmtResetClock(ms)
  return at ? `${fmtReset(ms)} (${at})` : fmtReset(ms)
}
// burn-rate projection lives in burn.js (shared with the tests)
const burn = Burn.createBurnTracker()

function setState(name) {
  const b = document.body
  ;[...b.classList].forEach((c) => {
    if (c.startsWith('state-')) b.classList.remove(c)
  })
  b.classList.add(`state-${name}`)
}

// coins (Claude "eating" tokens) — arc in, spin, get gulped with a crumb pop
const W = 86
const H = 77
const MOUTH_X = 43
const MOUTH_Y = 46

function spawnCoin() {
  const zone = el('dropzone')
  const coin = document.createElement('div')
  coin.className = 'coin'
  const sz = 6 + Math.random() * 3
  coin.style.width = coin.style.height = `${sz.toFixed(1)}px`

  const side = Math.floor(Math.random() * 4)
  let x, y
  if (side === 0) {
    x = Math.random() * W
    y = -10
  } else if (side === 1) {
    x = W + 10
    y = Math.random() * H * 0.7
  } else if (side === 2) {
    x = Math.random() * W
    y = H + 10
  } else {
    x = -10
    y = Math.random() * H * 0.7
  }
  coin.style.left = `${x}px`
  coin.style.top = `${y}px`
  zone.appendChild(coin)

  const dx = MOUTH_X - x
  const dy = MOUTH_Y - y
  // perpendicular offset -> curved arc toward the mouth
  const mxo = dx * 0.5 - dy * 0.18
  const myo = dy * 0.5 + dx * 0.18
  const rot = (Math.random() * 2 - 1) * 320
  const anim = coin.animate(
    [
      { transform: 'translate(0,0) scale(0.7) rotate(0deg)', opacity: 0.95 },
      {
        transform: `translate(${mxo}px,${myo}px) scale(1) rotate(${(rot * 0.6).toFixed(0)}deg)`,
        opacity: 1,
        offset: 0.55,
      },
      {
        transform: `translate(${dx}px,${dy}px) scale(0.2) rotate(${rot.toFixed(0)}deg)`,
        opacity: 0,
      },
    ],
    { duration: 780 + Math.random() * 320, easing: 'cubic-bezier(0.45,0,0.55,1)' },
  )
  anim.onfinish = () => {
    coin.remove()
    popCrumbs()
    chomp() // mouth opens to eat it
    nibble() // tiny gulp reaction
  }
}

// the mouth opens and snaps shut on each token
function chomp() {
  el('mouth').animate(
    [
      { transform: 'scaleY(0.1)' },
      { transform: 'scaleY(1)', offset: 0.4 },
      { transform: 'scaleY(0.1)' },
    ],
    { duration: 240, easing: 'ease-in-out' },
  )
}

// quick squash of the whole pet on each gulp (doesn't fight the hop on #claude)
function nibble() {
  el('pet').animate(
    [
      { transform: 'scale(1, 1)' },
      { transform: 'scale(1.06, 0.94)', offset: 0.5 },
      { transform: 'scale(1, 1)' },
    ],
    { duration: 200, easing: 'ease' },
  )
}

// poke reaction: bouncy squish + little hearts floating up
function popHearts() {
  const zone = el('dropzone')
  const n = 5
  for (let i = 0; i < n; i++) {
    const h = document.createElement('div')
    h.className = 'heart'
    h.textContent = '♥'
    // spread across lanes along the width + a little jitter
    const baseX = 14 + i * 15 + (Math.random() * 6 - 3)
    h.style.left = `${baseX.toFixed(0)}px`
    h.style.top = `${(14 + Math.random() * 12).toFixed(0)}px`
    zone.appendChild(h)
    // fan outward from the center
    const dx = (baseX - 43) * 0.55 + (Math.random() * 8 - 4)
    const a = h.animate(
      [
        { transform: 'translate(0, 8px) scale(0.4)', opacity: 0 },
        {
          transform: `translate(${(dx * 0.5).toFixed(1)}px, -12px) scale(1.3)`,
          opacity: 1,
          offset: 0.3,
        },
        { transform: `translate(${dx.toFixed(1)}px, -48px) scale(0.85)`, opacity: 0 },
      ],
      { duration: 1100 + Math.random() * 350, easing: 'ease-out', delay: i * 120 },
    )
    a.onfinish = () => h.remove()
  }
}
function pokePet() {
  oneShot('poke', 850)
  popHearts()
}

function popCrumbs() {
  const zone = el('dropzone')
  for (let i = 0; i < 3; i++) {
    const c = document.createElement('div')
    c.className = 'crumb'
    c.style.left = `${MOUTH_X}px`
    c.style.top = `${MOUTH_Y}px`
    zone.appendChild(c)
    const ang = Math.random() * Math.PI * 2
    const d = 5 + Math.random() * 8
    const a = c.animate(
      [
        { transform: 'translate(0,0) scale(1)', opacity: 0.9 },
        {
          transform: `translate(${(Math.cos(ang) * d).toFixed(1)}px, ${(Math.sin(ang) * d - 4).toFixed(1)}px) scale(0.2)`,
          opacity: 0,
        },
      ],
      { duration: 240 + Math.random() * 160, easing: 'ease-out' },
    )
    a.onfinish = () => c.remove()
  }
}

// continuous stream while working; faster when burning more tokens/min
let eatTimer = null
let eating = false
let currentRate = 0
function eatInterval() {
  return Math.max(750, 1700 - Math.min(850, currentRate / 2600))
}
function startEating() {
  stopEating()
  const loop = () => {
    spawnCoin()
    eatTimer = setTimeout(loop, eatInterval())
  }
  loop()
}
function stopEating() {
  if (eatTimer) clearTimeout(eatTimer)
  eatTimer = null
}

// 30-day map
function renderHeat(days) {
  const row = el('heat-row')
  row.innerHTML = ''
  const max = Math.max(1, ...days)
  days.forEach((v, i) => {
    const sq = document.createElement('div')
    let lv = 0
    if (v > 0) {
      const r = v / max
      lv = r < 0.3 ? 1 : r < 0.6 ? 2 : r < 0.85 ? 3 : 4
    }
    sq.className = `sq${lv ? ` lv${lv}` : ''}`
    const daysAgo = days.length - 1 - i
    sq.title = `${daysAgo === 0 ? 'today' : `${daysAgo}d ago`} · ${fmtTokens(v)} tokens`
    row.appendChild(sq)
  })
}

// ranked bar list — shared by the model and project panels
//
// The name column is one width for the whole list, never per row: the bars are
// only comparable if every track starts and ends at the same x. So it is sized
// to the widest label actually present, clamped so a long path cannot squeeze
// the bars into stubs, and anything past the clamp is clipped with an ellipsis.

function renderBars(boxId, list, limit) {
  const box = el(boxId)
  box.innerHTML = ''
  const top = list.slice(0, limit)
  const max = Math.max(1, ...top.map((m) => m.tokens))
  for (const m of top) {
    const row = document.createElement('div')
    row.className = 'mrow'
    row.innerHTML =
      `<span class="mname">${esc(m.label)}</span>` +
      `<span class="mbar"><i style="width:${(m.tokens / max) * 100}%"></i></span>` +
      `<span class="mval">${fmtTokens(m.tokens)}</span>`
    // the column can clip, so keep the full label reachable
    row.firstChild.title = m.label
    box.appendChild(row)
  }
  if (!top.length) {
    box.innerHTML = '<div class="mrow" style="opacity:.5">no activity</div>'
    return
  }
}

// measure the labels unconstrained, then lock the column to the widest one

// per-model weekly limits (e.g. "Fable" on Max) — one meter each, in the same
// shape as the all-models one. The token count pairs the limit with the local
// log totals for that model family (a "Fable" limit covers every Fable row).
function renderScoped(list, byModel) {
  const box = el('scoped-meters')
  box.innerHTML = ''
  for (const s of list) {
    const key = s.label.toLowerCase()
    const tokens = byModel
      .filter((m) => m.label.toLowerCase().startsWith(key))
      .reduce((n, m) => n + m.tokens, 0)
    const m = document.createElement('div')
    m.className = 'meter'
    m.innerHTML =
      `<div class="meter-top"><span>weekly · ${esc(key)}</span><span class="${levelOf(s.pct)}">${Math.round(s.pct)}%</span></div>` +
      `<div class="track"><div class="fill week${s.pct >= 80 ? ' high' : ''} ${levelOf(s.pct)}" style="width:${s.pct}%"></div></div>` +
      `<div class="sub">${s.resetMs != null ? `resets in ${fmtResetIn(s.resetMs)} · ` : ''}${fmtTokens(tokens)} tokens</div>`
    box.appendChild(m)
  }
}

// One ramp for every meter: plain below 60, warm to 85, hot past it. The number
// and its bar always read the same level.
const levelOf = (pct) => (pct >= 85 ? 'hot' : pct >= 60 ? 'mid' : '')
function setLevel(node, pct) {
  node.classList.toggle('mid', levelOf(pct) === 'mid')
  node.classList.toggle('hot', levelOf(pct) === 'hot')
}

// by model (7 days)
function renderModels(list) {
  renderBars('bymodel-list', list, 4)
}

// by project (7 days) — usage.js already folds everything past the top few
// into a single `other` row, so whatever arrives here is meant to be drawn
function renderProjects(list) {
  renderBars('byproject-list', list, 6)
}

// one-shot reaction (adds a class, removes after ms)
function oneShot(cls, ms) {
  document.body.classList.add(cls)
  setTimeout(() => document.body.classList.remove(cls), ms)
}

const RING_LEN = 2 * Math.PI * 45 // the collapsed ring's circumference (r=45)

// session-reset celebration: jump + a colorful confetti burst
const CONFETTI_COLORS = ['#ffd23f', '#ff5d86', '#7ec77d', '#6db3f2', '#e0805a', '#c89bff']
function spawnConfetti(i) {
  const zone = el('dropzone')
  const p = document.createElement('div')
  p.className = 'confetti'
  p.style.background = CONFETTI_COLORS[i % CONFETTI_COLORS.length]
  const w = 4 + Math.random() * 4
  p.style.width = `${w.toFixed(1)}px`
  p.style.height = `${(w + 2 + Math.random() * 4).toFixed(1)}px`
  p.style.left = '43px'
  p.style.top = '38px'
  zone.appendChild(p)
  // launch up + outward, then fall back down with a tumble
  const ang = -Math.PI / 2 + (Math.random() * 2 - 1) * 1.15
  const speed = 34 + Math.random() * 36
  const ux = Math.cos(ang) * speed
  const uy = Math.sin(ang) * speed // negative = upward
  const fallY = 50 + Math.random() * 40
  const rot = (Math.random() * 2 - 1) * 600
  const a = p.animate(
    [
      { transform: 'translate(0,0) rotate(0) scale(0.5)', opacity: 1 },
      {
        transform: `translate(${ux.toFixed(0)}px, ${uy.toFixed(0)}px) rotate(${(rot * 0.4).toFixed(0)}deg) scale(1)`,
        opacity: 1,
        offset: 0.4,
      },
      {
        transform: `translate(${(ux * 1.4).toFixed(0)}px, ${fallY.toFixed(0)}px) rotate(${rot.toFixed(0)}deg) scale(0.9)`,
        opacity: 0,
      },
    ],
    { duration: 1150 + Math.random() * 550, easing: 'cubic-bezier(0.25, 0.7, 0.4, 1)' },
  )
  a.onfinish = () => p.remove()
}
function celebrate() {
  oneShot('celebrate', 1300)
  for (let i = 0; i < 24; i++) spawnConfetti(i)
}

// ---- voice ------------------------------------------------------------------
// The bubble speaks on transitions only, at most once per REMARK_GAP_MS. The
// headline moments (the day's first hello, a fresh window, catching fire) may
// jump that queue, and only they blip: a noise on every remark wears out far
// faster than the remark itself.
const REMARK_GAP_MS = 10 * 60000
const MUTE_MS = 60 * 60000
const blipper = Voice.createBlipper(() => {
  const AC = globalThis.AudioContext || globalThis.webkitAudioContext
  return AC ? new AC() : null
})
let lastRemarkAt = 0
let bubbleTimer = null

const talkOn = () => currentConfig.talk !== false
const mutedNow = () => currentConfig.soundMutedUntil > Date.now()
// opt-in, and never where it would surprise someone: the menu-bar popover, the
// mini face, or the hour after a mute
function soundOn() {
  if (!currentConfig.sound || currentConfig.mode === 'menubar') return false
  if (document.body.classList.contains('collapsed')) return false
  return !mutedNow()
}

function moodOf(st) {
  return st === 'stressed' ? 'fire' : st === 'sleeping' || st === 'tired' ? 'sleepy' : 'normal'
}

// `line` is a remark from voice.js ({ text, short }) or a plain string. The
// mini face has room for a glance, not a sentence, so it gets the short one.
// `force` is the simulator's: it skips the gates but still respects the mute
function say(line, { headline = false, mood = 'normal', force = false } = {}) {
  if (!line || document.body.classList.contains('pet-only')) return false
  const collapsed = document.body.classList.contains('collapsed')
  const text = typeof line === 'string' ? line : collapsed ? line.short : line.text
  if (!force) {
    const b = document.body.classList
    if (!talkOn() || b.contains('settings-open')) return false
    if (!headline && Date.now() - lastRemarkAt < REMARK_GAP_MS) return false
  }
  lastRemarkAt = Date.now()
  const bubble = el('bubble')
  el('bubble-text').textContent = text
  bubble.classList.remove('leaving')
  placeBubble()
  bubble.hidden = false
  fitSize() // collapsed, the bubble is in the flow: the window grows to fit it
  clearTimeout(bubbleTimer)
  bubbleTimer = setTimeout(hideBubble, Math.max(4500, text.length * 70 + 2500))
  // the mouth moves along, whether or not there's sound
  for (let i = 0; i < Math.min(4, Math.ceil(text.length / 18)); i++) setTimeout(chomp, i * 260)
  if ((headline || force) && soundOn()) blipper.play(text, mood)
  return true
}

// expanded, the bubble floats over the scene beside the pet; collapsed there's
// no room beside it, so it moves above the pet, into the card's flow
function placeBubble() {
  const bubble = el('bubble')
  const stage = el('stage')
  if (document.body.classList.contains('collapsed')) {
    if (bubble.nextElementSibling !== stage) stage.before(bubble)
  } else if (bubble.parentElement !== stage) stage.appendChild(bubble)
}

function hideBubble() {
  clearTimeout(bubbleTimer)
  const bubble = el('bubble')
  if (bubble.hidden) return
  bubble.classList.add('leaving')
  bubbleTimer = setTimeout(() => {
    bubble.hidden = true
    bubble.classList.remove('leaving')
    fitSize()
  }, 180)
}
el('bubble').addEventListener('click', (e) => {
  e.stopPropagation()
  hideBubble()
})

// once-a-day remarks, keyed by local date; memory is the fallback when storage
// isn't there, so a broken profile can't make the pet repeat itself
const toldMemo = {}
const dayKey = () => new Date().toDateString()
function toldToday(key) {
  try {
    if (localStorage.getItem(key) === dayKey()) return true
  } catch {}
  return toldMemo[key] === dayKey()
}
function markToday(key) {
  toldMemo[key] = dayKey()
  try {
    localStorage.setItem(key, dayKey())
  } catch {}
}

// what the pet has noticed so far — the transitions are all relative to it
const AWAY_MS = 2 * 3600000
const STREAK_MS = 90 * 60000
const STREAK_GAP_MS = 10 * 60000
let greeted = false
let heardConfig = false
let lastLivePct = null // Claude's session %, null until a live reading lands
let awayMs = 0
let streakFrom = 0
let lastWorkAt = 0
let streakTold = false

// what the current Claude window has been like, for the recap when it closes
// (#31). The peak is the session's tokens just before the rollover — after it
// the counter starts over. Only the time the pet actually watched counts.
const TALLY_GAP_MS = 15000 // a longer gap is the machine asleep, not work
let tally = { peak: 0, activeMs: 0, acts: {}, at: 0 }
function noteSession(d) {
  const now = Date.now()
  const dt = tally.at ? Math.min(now - tally.at, TALLY_GAP_MS) : 0
  tally.at = now
  if (d.active) {
    tally.activeMs += dt
    if (d.activity) tally.acts[d.activity] = (tally.acts[d.activity] || 0) + dt
  }
  tally.peak = Math.max(tally.peak, d.session?.tokens || 0)
}
function takeRecap(was) {
  const top = Object.entries(tally.acts).sort((a, b) => b[1] - a[1])[0]?.[0] || null
  const r = { was, tokens: tally.peak, activeMs: tally.activeMs, top }
  tally = { peak: 0, activeMs: 0, acts: {}, at: tally.at }
  return r
}

// Codex, followed whichever tab is on screen. Its logs only move while it
// runs, so a stale reading keeps the baseline: the next fresh one is compared
// against the last number actually seen.
let lastCodexPct = null
function listenCodex() {
  const c = codexData
  if (!codexOn() || !c) {
    lastCodexPct = null
    return
  }
  const pct = c.session?.pct
  if (pct == null || codexStale(c)) return
  const was = lastCodexPct
  lastCodexPct = pct
  if (was == null) return
  const fireAt = currentConfig.fireThreshold ?? 90
  const reset = c.session.resetMs
  if (was - pct > 25) say(Voice.codexResetLine(was), { headline: true })
  else if (was < 100 && pct >= 100)
    say(Voice.codexMaxedLine(fmtResetClock(reset)), { mood: 'sleepy' })
  else if (was < fireAt && pct >= fireAt && pct < 100)
    say(Voice.codexFireLine(pct, reset, fmtReset), { headline: true, mood: 'fire' })
}

// called by paint() with the state it just drew. The Claude remarks follow the
// Claude view; Codex has its own, above.
function listen(before, st, { isCodex, liveOn, sp, sessReset, proj }) {
  // the saved config decides whether it talks at all, and it lands just after
  // the first usage tick: speaking before it would ignore a "talk: false"
  if (!heardConfig) return
  const d = lastData
  const mood = moodOf(st)
  if (!greeted) {
    greeted = true
    if (!toldToday('clauddy.greeted')) {
      markToday('clauddy.greeted')
      const line = Voice.greetingLine(d.days30, new Date().getHours(), fmtTokens)
      if (say(line, { headline: true, mood })) return
    }
  }
  listenCodex()
  if (isCodex) return
  const now = Date.now()

  if (liveOn) {
    const fireAt = currentConfig.fireThreshold ?? 90
    const was = lastLivePct
    lastLivePct = sp
    // a first reading is a baseline: launching at 95% is not "catching fire"
    if (was != null && was < 100 && sp >= 100) {
      say(Voice.maxedLine(fmtResetClock(sessReset)), { mood: 'sleepy' })
    } else if (was != null && was < fireAt && sp >= fireAt && sp < 100) {
      const eta = proj?.kind === 'eta' ? proj.ms : null
      say(Voice.fireLine(sp, eta, sessReset, fmtReset), { headline: true, mood: 'fire' })
    }
  } else {
    lastLivePct = null // logged out, or switching: the next reading starts over
  }

  if (st === 'sleeping' && Number.isFinite(d.lastActivityMs))
    awayMs = Math.max(awayMs, d.lastActivityMs)
  else if (before === 'sleeping') {
    if (awayMs >= AWAY_MS) say(Voice.welcomeLine(awayMs, liveOn ? sp : null, fmtReset))
    awayMs = 0
  }

  if (st === 'working') {
    if (!streakFrom || now - lastWorkAt > STREAK_GAP_MS) {
      streakFrom = now
      streakTold = false
    }
    lastWorkAt = now
    if (!streakTold && now - streakFrom >= STREAK_MS)
      streakTold = say(Voice.streakLine(now - streakFrom, fmtReset))
  }

  const record = Voice.recordLine(d.days30, fmtTokens)
  if (record && !toldToday('clauddy.record') && say(record, { mood })) markToday('clauddy.record')
}

// `./pet say <kind>` previews one remark — real numbers where there are some,
// plausible ones where the moment hasn't happened
function sampleLine(kind) {
  const days = lastData?.days30?.length ? [...lastData.days30] : new Array(30).fill(0)
  const pct = realUsage?.session?.pct
  const reset = realUsage?.session?.resetMs ?? 72 * 60000
  if (kind === 'fire') return Voice.fireLine(Math.max(pct ?? 0, 91), 40 * 60000, reset, fmtReset)
  if (kind === 'reset') {
    const t = tally.peak ? tally : { peak: 34e6, activeMs: 4 * 3600000 + 12 * 60000, acts: {} }
    const top = Object.entries(t.acts).sort((a, b) => b[1] - a[1])[0]?.[0] || 'editing'
    return Voice.recapLine(
      { was: 92, tokens: t.peak, activeMs: t.activeMs, top },
      fmtTokens,
      fmtReset,
    )
  }
  if (kind === 'codex')
    return Voice.codexFireLine(91, codexData?.session?.resetMs ?? reset, fmtReset)
  if (kind === 'maxed') return Voice.maxedLine(fmtResetClock(reset))
  if (kind === 'welcome') return Voice.welcomeLine(3 * 3600000 + 12 * 60000, pct ?? null, fmtReset)
  if (kind === 'streak') return Voice.streakLine(95 * 60000, fmtReset)
  if (kind === 'record') {
    days[days.length - 1] = Math.max(2e6, Math.max(...days.slice(0, -1)) * 1.15)
    return Voice.recordLine(days, fmtTokens)
  }
  if (!days[days.length - 2]) days[days.length - 2] = 12e6 // no yesterday yet: pretend one
  return Voice.greetingLine(days, new Date().getHours(), fmtTokens)
}
function sayDebug(kind) {
  const st = [...document.body.classList].find((c) => c.startsWith('state-'))?.slice(6)
  const mood =
    kind === 'fire' || kind === 'codex' ? 'fire' : kind === 'maxed' ? 'sleepy' : moodOf(st)
  return say(sampleLine(kind), { force: true, mood })
}

// mute sits in the title bar while the voice is on: one click before a call
let unmuteTimer = null
function paintMute() {
  const btn = el('mute')
  // menu-bar mode is always silent, so there is nothing there to mute
  btn.hidden = !(currentConfig.sound && talkOn()) || currentConfig.mode === 'menubar'
  document.body.classList.toggle('has-mute', !btn.hidden) // the chip makes room for it
  const muted = mutedNow()
  btn.classList.toggle('muted', muted)
  btn.title = muted
    ? `Muted until ${new Date(currentConfig.soundMutedUntil).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })} — click to unmute`
    : 'Mute the voice for 1 hour'
  clearTimeout(unmuteTimer)
  if (muted) unmuteTimer = setTimeout(paintMute, currentConfig.soundMutedUntil - Date.now() + 50)
}
el('mute').addEventListener('click', (e) => {
  e.stopPropagation()
  const until = mutedNow() ? 0 : Date.now() + MUTE_MS
  currentConfig = { ...currentConfig, soundMutedUntil: until } // repaint now, main confirms
  paintMute()
  window.api.saveConfig({ soundMutedUntil: until })
})

// ---- services ---------------------------------------------------------------
// Claude and Codex are monitored side by side, but the panel shows one at a
// time: tabs own the chip, meters, breakdowns and Usage arrow. The expanded
// scene follows that view; compact pets follow activity across both services.
const PROVIDER_KEY = 'clauddy.provider'
let codexData = null // last Codex payload; null while Codex isn't enabled
let pickedProvider = (() => {
  try {
    return localStorage.getItem(PROVIDER_KEY) === 'codex' ? 'codex' : 'claude'
  } catch {
    return 'claude'
  }
})()

const claudeOn = () => document.body.classList.contains('auth-on')
const codexOn = () => !!currentConfig.codex
const bothOn = () => claudeOn() && codexOn()
// the service on screen: the pick, unless that one isn't there any more
function viewProvider() {
  if (!codexOn()) return 'claude'
  if (!claudeOn()) return 'codex'
  return pickedProvider
}

function pickProvider(p) {
  if (p === pickedProvider) return
  pickedProvider = p
  try {
    localStorage.setItem(PROVIDER_KEY, p)
  } catch {}
  paint()
}

const STALE_MS = 15 * 60000
const CODEX_SLEEP_MS = 5 * 60000 // main's default sleepThresholdMs
// Codex logs are only written while it runs, so a reading can be hours old
const codexStale = (c) =>
  !!c && (c.session?.expired || (c.limitsAt != null && Date.now() - c.limitsAt > STALE_MS))

// Codex's numbers in the shape the panel draws; its logs have no activity or tok/min
function codexView(c) {
  const idleFor = c.lastSeen ? Date.now() - c.lastSeen : null
  return {
    d: {
      active: c.active,
      activity: null,
      // no data is not the same as asleep
      sleeping: !c.active && idleFor != null && idleFor >= CODEX_SLEEP_MS,
      tokensPerMin: 0,
      today: { tokens: c.tokensToday || 0 },
      session: { tokens: c.tokens5h || 0 },
      week: { tokens: c.tokensWeek || 0 },
      byModel: c.byModel || [],
      byProject: c.byProject || [],
      days30: c.days30 || [],
      monthTokens: c.monthTokens ?? null,
    },
    sess: c.session,
    week: c.weekly,
    stale: c.limitsAt != null && Date.now() - c.limitsAt > STALE_MS ? c.limitsAt : null,
  }
}

// a Codex meter caption: its reset, or how old the reading is when that's what matters
function codexSub(w, tokens, stale) {
  const t = `${fmtTokens(tokens)} tokens`
  if (!w) return `limits not recorded yet · ${t}`
  if (w.pct == null) return `waiting for a fresh reading · ${t}`
  if (stale) {
    const at = w.resetMs != null ? `resets ${fmtResetClock(w.resetMs)} · ` : ''
    return `${at}read ${fmtReset(Date.now() - stale)} ago`
  }
  const reset = w.resetMs != null ? `resets in ${fmtResetIn(w.resetMs)} · ` : ''
  return `${reset}${t}`
}

const pctText = (v) => (v == null ? '—' : `${Math.round(v)}%`)
const warnAt = () => Math.min(...(currentConfig.alertThresholds || [80, 95]))

// tabs + the collapsed pair: both services' session %, whichever is on screen
function paintServices(view) {
  const dual = bothOn()
  document.body.classList.toggle('dual', dual)
  document.body.classList.toggle('view-codex', view === 'codex')
  // collapsed with both: when the selected service's session resets
  const resetMs = view === 'codex' ? codexData?.session?.resetMs : realUsage?.session?.resetMs
  el('mini-reset').hidden = !dual || resetMs == null
  el('mini-reset').textContent = resetMs == null ? '' : `resets ${fmtResetClock(resetMs)}`
  el('service-panel').setAttribute('role', dual ? 'tabpanel' : 'region')
  if (dual) {
    el('service-panel').setAttribute('aria-labelledby', `tab-${view}`)
    el('service-panel').removeAttribute('aria-label')
  } else {
    el('service-panel').removeAttribute('aria-labelledby')
    el('service-panel').setAttribute('aria-label', `${view === 'codex' ? 'Codex' : 'Claude'} usage`)
    return
  }
  const cl = realUsage
  const cx = codexData
  const rows = {
    claude: {
      pct: cl?.session?.pct ?? null,
      week: cl?.week?.pct ?? null,
      active: !!lastData?.active,
    },
    codex: {
      pct: cx?.session?.pct ?? null,
      week: cx?.weekly?.pct ?? null,
      active: !!cx?.active,
      stale: codexStale(cx),
    },
  }
  for (const [p, r] of Object.entries(rows)) {
    const on = p === view
    const urgent = Math.max(r.pct ?? 0, r.week ?? 0) >= warnAt()
    const tab = el(`tab-${p}`)
    tab.setAttribute('aria-selected', String(on))
    tab.tabIndex = on ? 0 : -1
    tab.classList.toggle('active', r.active)
    tab.classList.toggle('urgent', urgent)
    tab.querySelector('.tab-alert').hidden = !urgent
    el(`tab-${p}-value`).textContent = pctText(r.pct)
    const name = p === 'claude' ? 'Claude' : 'Codex'
    tab.title = `${name} session ${pctText(r.pct)} · weekly ${pctText(r.week)}${r.active ? ' · working' : ''}${r.stale ? ' · not updated recently' : ''}`
    const line = document.querySelector(`.mini-source[data-provider="${p}"]`)
    line.setAttribute('aria-pressed', String(on))
    line.title = tab.title
    line.setAttribute('aria-label', `Show ${tab.title}`)
    line.classList.toggle('urgent', urgent)
    tab.setAttribute('aria-label', `Show ${tab.title}`)
    el(`mini-${p}-value`).textContent = pctText(r.pct)
    const bar = el(`mini-${p}-bar`)
    bar.style.width = `${Math.min(r.pct ?? 0, 100)}%`
    bar.classList.toggle('hot', (r.pct ?? 0) >= 80)
  }
}

for (const b of document.querySelectorAll('#harness-tabs button, .mini-source')) {
  b.addEventListener('click', (e) => {
    e.stopPropagation()
    pickProvider(b.dataset.provider)
  })
}
el('harness-tabs').addEventListener('keydown', (e) => {
  const providers = ['claude', 'codex']
  const current = providers.indexOf(e.target.dataset.provider)
  if (current < 0) return
  let next
  if (e.key === 'ArrowRight') next = (current + 1) % providers.length
  else if (e.key === 'ArrowLeft') next = (current + providers.length - 1) % providers.length
  else if (e.key === 'Home') next = 0
  else if (e.key === 'End') next = providers.length - 1
  else return
  e.preventDefault()
  pickProvider(providers[next])
  el(`tab-${providers[next]}`).focus()
})

// ---- companion: pet-only size, reset reminders, and provider reactions -------
const SIZE_KEY = 'clauddy.size'
let sizeLoaded = false
let peekTimer = null
let nativePetInside = null
let noticeTimer = null
let reminders = { claude: null, codex: null }
const activityTracker = Companion.createActivityTracker()

function setPetPeek(on) {
  if (!document.body.classList.contains('pet-only')) return
  clearTimeout(peekTimer)
  document.body.classList.toggle('pet-peek', on)
}
function setDisplaySize(size, remember = true) {
  if (!['expanded', 'compact', 'pet'].includes(size)) size = 'expanded'
  if (currentConfig.mode === 'menubar' && size === 'pet') size = 'compact'
  hideBubble()
  clearTimeout(peekTimer)
  document.body.classList.remove('pet-peek')
  document.body.classList.toggle('collapsed', size !== 'expanded')
  document.body.classList.toggle('pet-only', size === 'pet')
  nativePetInside = null
  window.api.watchPetPointer(size === 'pet')
  el('pet').tabIndex = size === 'pet' ? 0 : -1
  if (size === 'pet') {
    el('pet').setAttribute('role', 'button')
    el('pet').setAttribute('aria-label', 'Drag to move; press Enter to open compact monitor')
  } else {
    el('pet').removeAttribute('role')
    el('pet').removeAttribute('aria-label')
  }
  if (remember) {
    try {
      localStorage.setItem(SIZE_KEY, size)
    } catch {}
  }
  paint()
  fitSize()
}
el('pet-only-toggle').addEventListener('click', () => setDisplaySize('pet'))
el('pet-restore').addEventListener('click', () => setDisplaySize('compact'))
window.api.onPetPointer((point) => {
  if (!document.body.classList.contains('pet-only')) return
  const contains = (rect) =>
    point &&
    point.x >= rect.left &&
    point.x <= rect.right &&
    point.y >= rect.top &&
    point.y <= rect.bottom
  // Keep the preview open across the small gap to its controls. It opens from
  // the sprite, not from the otherwise empty space below it.
  const inside = !!(
    contains(el('pet').getBoundingClientRect()) ||
    (document.body.classList.contains('pet-peek') && contains(el('card').getBoundingClientRect()))
  )
  if (inside === nativePetInside) return
  nativePetInside = inside
  clearTimeout(peekTimer)
  if (inside) setPetPeek(true)
  else peekTimer = setTimeout(() => setPetPeek(false), 180)
})
window.addEventListener('blur', () => setPetPeek(false))
el('card').addEventListener('focusin', () => setPetPeek(true))
el('card').addEventListener('focusout', (e) => {
  if (!el('card').contains(e.relatedTarget)) setPetPeek(false)
})
el('pet').addEventListener('keydown', (e) => {
  if (!document.body.classList.contains('pet-only')) return
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    setDisplaySize('compact')
  } else if (e.key === 'Escape') {
    el('pet').blur()
    setPetPeek(false)
  }
})
for (const button of document.querySelectorAll('#pet-glance button[data-provider]')) {
  button.addEventListener('click', () => pickProvider(button.dataset.provider))
}
function paintCompanion(view) {
  for (const provider of ['claude', 'codex']) {
    const connected = provider === 'claude' ? claudeOn() : codexOn()
    const session = provider === 'claude' ? realUsage?.session : codexData?.session
    const button = el(`glance-${provider}`)
    button.hidden = !connected
    button.setAttribute('aria-pressed', String(provider === view))
    button.classList.toggle('urgent', session?.pct >= warnAt())
    el(`glance-${provider}-value`).textContent = pctText(session?.pct)
    const stale = provider === 'codex' && codexStale(codexData)
    button.title = stale
      ? 'Last recorded usage · waiting for a fresh reading'
      : `Show ${provider} usage`
    button.classList.toggle('stale-reading', !!stale)
  }
  el('glance-empty').hidden = claudeOn() || codexOn()
  const session = view === 'claude' ? realUsage?.session : codexData?.session
  const reminder = reminders[view]
  const stale =
    view === 'codex'
      ? !codexData?.limitsAt || codexStale(codexData)
      : !realUsageReadAt || Date.now() - realUsageReadAt > STALE_MS
  const available = session?.pct != null && session.resetMs > 0 && !stale
  const name = view === 'codex' ? 'Codex' : 'Claude'
  for (const id of ['reminder-toggle', 'mini-reminder']) {
    const button = el(id)
    button.hidden = !reminder && (!available || session.pct < warnAt())
    button.setAttribute('aria-pressed', String(!!reminder))
    button.textContent = reminder ? '✓ Reminder on' : 'Notify at reset'
    button.title = reminder
      ? `Cancel ${name} reminder for ${new Date(reminder.at).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`
      : `Remind me when ${name}'s session reset is due`
  }
}
function companionNotice(text, provider = null) {
  clearTimeout(noticeTimer)
  const notice = el('companion-notice')
  notice.textContent = text
  notice.title = text
  notice.dataset.provider = provider || ''
  notice.hidden = false
  fitSize()
  noticeTimer = setTimeout(() => {
    notice.hidden = true
    fitSize()
  }, 5000)
}
for (const id of ['reminder-toggle', 'mini-reminder']) {
  el(id).addEventListener('click', () => {
    const provider = viewProvider()
    window.api.setReminder(provider, !reminders[provider])
  })
}
window.api.onReminders((state) => {
  reminders = state || { claude: null, codex: null }
  if (state?.error) companionNotice(state.error)
  paintCompanion(viewProvider())
  fitSize()
})
window.api.onReminderDue((event) => {
  if (!event.isCurrent) return
  const name = event.provider === 'codex' ? 'Codex' : 'Claude'
  companionNotice(
    event.confirmed ? `${name} · budget is back!` : `${name} · reset time reached`,
    event.provider,
  )
  if (event.confirmed) celebrate()
})
function reactToProvider(provider, data) {
  // no caption: the eyes glance toward whichever service changed
  const cue = activityTracker.observe(provider, data)
  if (!cue || document.body.classList.contains('settings-open')) return
  if (!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
    const x = provider === 'codex' ? 3 : -3
    el('eyes').animate(
      [
        { transform: 'translateX(0)' },
        { transform: `translateX(${x}px)`, offset: 0.3 },
        { transform: `translateX(${x}px)`, offset: 0.7 },
        { transform: 'translateX(0)' },
      ],
      { duration: 1100, easing: 'ease-in-out' },
    )
  }
}

function paintActivity() {
  const activity = Companion.currentActivity(
    claudeOn() ? claudeActivityData : null,
    codexOn() ? codexData : null,
  )
  const names = activity.providers.map((p) => (p === 'claude' ? 'Claude' : 'Codex')).join(' + ')
  const label = names ? `${names} · ${activity.activity}` : ''
  for (const id of ['pet-activity', 'mini-workers']) {
    const badge = el(id)
    badge.hidden = !names
    badge.setAttribute('aria-label', label)
    for (const logo of badge.querySelectorAll('[data-provider]')) {
      logo.hidden = !activity.providers.includes(logo.dataset.provider)
    }
  }
  el('mini-dot').hidden =
    activity.providers.length > 0 && document.body.classList.contains('collapsed')
  return activity
}

// main render
let prevState = null
const prevPct = {} // per service: a switch must not read as a reset
let lastData = null
// Claude's activity, kept apart from lastData: an account switch or a logout
// must drop the old account's worker at once, not a tick later
let claudeActivityData = null
function render(d) {
  if (claudeOn()) reactToProvider('claude', d)
  lastData = d
  claudeActivityData = claudeOn() ? d : null
  noteSession(d)
  // the burn trail is Claude's, fed on every Claude tick whatever is on screen
  const cl = realUsage
  burn.note(d.session.tokens, !!cl && cl.session.resetMs != null)
  paint()
}

function paint() {
  setPlan(el('codex-plan'), codexData?.plan)
  const d0 = lastData
  if (!d0) return
  const view = viewProvider()
  paintServices(view)
  paintCompanion(view)
  const isCodex = view === 'codex' && !!codexData
  const cv = isCodex ? codexView(codexData) : null
  const d = isCodex ? cv.d : d0
  // % comes only from the connected account (or Codex's own logs) — no estimates
  const liveOn = isCodex || !!realUsage
  document.body.classList.toggle('live', liveOn)
  const sessPct = isCodex ? (cv.sess?.pct ?? null) : liveOn ? realUsage.session.pct : 0
  const sessReset = isCodex ? (cv.sess?.resetMs ?? null) : liveOn ? realUsage.session.resetMs : null
  const sessActive = liveOn && sessReset != null
  const wkPct = isCodex ? (cv.week?.pct ?? null) : liveOn ? realUsage.week.pct : 0
  const wkReset = isCodex ? (cv.week?.resetMs ?? null) : liveOn ? realUsage.week.resetMs : null
  const sp = sessPct ?? 0
  const wp = wkPct ?? 0

  const fireAt = currentConfig.fireThreshold ?? 90
  let st =
    liveOn && sp >= 100
      ? 'tired'
      : d.active
        ? 'working'
        : liveOn && sp >= fireAt
          ? 'stressed'
          : d.sleeping
            ? 'sleeping'
            : 'idle'
  const acts = ['editing', 'reading', 'planning', 'running', 'researching', 'delegating', 'waiting']
  let curActivity = d.activity
  const activity = paintActivity()
  const collapsedNow = document.body.classList.contains('collapsed')
  if (collapsedNow) {
    st = activity.providers.length ? 'working' : activity.sleeping ? 'sleeping' : 'idle'
    curActivity = activity.providers.length ? activity.activity : null
  }
  // dev override from `./pet <state>` (base states or an activity name)
  if (debugState) {
    const map = {
      working: 'working',
      sleeping: 'sleeping',
      fire: 'stressed',
      tired: 'tired',
      idle: 'idle',
    }
    if (map[debugState]) {
      st = map[debugState]
      curActivity = null
    } else if (acts.includes(debugState)) {
      st = 'working'
      curActivity = debugState
    }
  }
  setState(st)
  // expose the current activity as a body class so per-action animations can
  // hook it (e.g. body.act-reading). Only while actively working.
  for (const a of acts) {
    document.body.classList.toggle(`act-${a}`, st === 'working' && curActivity === a)
  }
  const before = prevState
  if (prevState && prevState !== st) {
    if (st === 'sleeping') oneShot('drowse', 700)
    else if (prevState === 'sleeping') oneShot('wake', 700)
  }
  prevState = st

  const word =
    st === 'working'
      ? curActivity || 'working' // show what Claude's doing (editing/reading/…)
      : st === 'sleeping'
        ? 'sleeping'
        : st === 'stressed'
          ? 'on fire'
          : st === 'tired'
            ? 'maxed out'
            : 'idle'
  el('status-text').textContent = word
  el('mini-text').textContent = word
  el('rate').textContent =
    d.active && d.tokensPerMin > 0
      ? `${fmtTokens(d.tokensPerMin)} tok/min`
      : `${fmtTokens(d.today.tokens)} tokens today`

  const was = prevPct[view]
  if (liveOn && was != null && sessActive && sessPct != null && was - sessPct > 25) {
    celebrate()
    // Codex's own reset is called out by listenCodex, on screen or not
    if (!isCodex) say(Voice.recapLine(takeRecap(was), fmtTokens, fmtReset), { headline: true })
  }
  prevPct[view] = sessPct
  el('session-pct').textContent = pctText(liveOn ? sessPct : 0)
  setLevel(el('session-pct'), liveOn ? sp : 0)
  const mini = el('mini-pct')
  mini.textContent = liveOn ? pctText(sessPct) : '—'
  mini.classList.toggle('high', liveOn && sp >= 80)
  // the collapsed ring: how much of the session is already spent
  const rf = el('ring-fill')
  rf.style.strokeDashoffset = `${RING_LEN * (1 - Math.min(sp, 100) / 100)}`
  rf.classList.toggle('high', sp >= 80)
  const rp = el('ring-pct')
  rp.textContent = mini.textContent
  rp.classList.toggle('high', liveOn && sp >= 80)
  const sf = el('session-fill')
  sf.style.width = `${sp}%`
  sf.classList.toggle('high', sp >= 80)
  setLevel(sf, sp)
  el('session-sub').textContent = isCodex
    ? codexSub(cv.sess, d.session.tokens, cv.stale)
    : sessActive
      ? `resets in ${fmtResetIn(sessReset)} · ${fmtTokens(d.session.tokens)} tokens`
      : 'no active session'

  // where this pace is taking you — Claude's only: Codex has no trail to read
  const proj =
    !isCodex && sessActive && sp < 100 ? burn.project(sp, sessReset, d.session.tokens) : null
  const pe = el('session-proj')
  pe.hidden = !proj
  pe.classList.toggle('tight', proj?.kind === 'eta')
  if (proj) {
    pe.textContent =
      proj.kind === 'eta' ? `~${fmtReset(proj.ms)} left at this pace` : 'resets before you run out'
  }

  el('week-pct').textContent = pctText(liveOn ? wkPct : 0)
  setLevel(el('week-pct'), wp)
  const wf = el('week-fill')
  wf.style.width = `${wp}%`
  wf.classList.toggle('high', wp >= 80)
  setLevel(wf, wp)
  el('week-sub').textContent = isCodex
    ? codexSub(cv.week, d.week.tokens, cv.stale)
    : wkReset != null
      ? `resets in ${fmtResetIn(wkReset)} · ${fmtTokens(d.week.tokens)} tokens`
      : `${fmtTokens(d.week.tokens)} tokens · last 7 days`

  renderScoped(!isCodex && liveOn ? realUsage.scoped || [] : [], d.byModel || [])
  renderModels(d.byModel || [])
  renderProjects(d.byProject || [])
  renderHeat(d.days30 || [])
  el('month-total').textContent = d.monthTokens == null ? '—' : `${fmtTokens(d.monthTokens)} tokens`
  paintChip()

  // compact eats at Claude's pace only while Claude is one of the workers
  const claudeRate = !collapsedNow || activity.providers.includes('claude')
  currentRate = claudeRate ? (collapsedNow ? d0 : d).tokensPerMin || 0 : 0
  if (st === 'working') {
    if (!eating) {
      eating = true
      startEating()
    }
  } else if (eating) {
    eating = false
    stopEating()
  }

  listen(before, st, { isCodex, liveOn, sp, sessReset, proj })
  fitSize()
}

// fit the window to the content (no leftover border)
let lastH = 0
let lastW = 0
function fitSize() {
  requestAnimationFrame(() => {
    const collapsed = document.body.classList.contains('collapsed')
    const zoom = Number.parseFloat(document.body.style.zoom) || 1
    // The dual-service dock trades height for a little horizontal room.
    const dual = document.body.classList.contains('dual')
    const petOnly = document.body.classList.contains('pet-only')
    const w = (petOnly ? 168 : collapsed ? (dual ? 240 : 192) : 304) * zoom
    // offsetHeight never reflects a CSS zoom applied to an ancestor (confirmed
    // empirically against this Electron build) — so measure the unzoomed content
    // height, then scale the whole thing (content + margin) ourselves
    const h = (el('card').offsetHeight + 24) * zoom // 12px margin top + bottom
    if (Math.abs(h - lastH) > 2 || w !== lastW) {
      lastH = h
      lastW = w
      window.api.resize(w, h)
    }
  })
}

// widget scale, applied as a CSS zoom (not transform) so offsetWidth/Height
// keep reflecting it — offsetHeight is a layout px, so it must be measured as one
function applyZoom(z) {
  document.body.style.zoom = (z || 100) / 100
}

let currentConfig = {}
let realUsage = null
let realUsageReadAt = 0
let debugState = null
window.api.onDebugState((o) => {
  const s = o?.state
  if (s === 'poke') return pokePet()
  if (s === 'celebrate') return celebrate()
  if (s === 'say') return sayDebug(o.kind)
  debugState = s === 'auto' || s === 'clear' || !s ? null : s
  if (lastData) render(lastData)
})
window.api.onUsage(render)
window.api.onError((msg) => {
  el('status-text').textContent = 'error'
  console.error(msg)
})
window.api.onConfig((cfg) => {
  currentConfig = cfg || {}
  document.body.classList.toggle('is-menubar', currentConfig.mode === 'menubar')
  document.body.classList.toggle('codex-on', !!currentConfig.codex)
  if (!currentConfig.codex) codexData = null
  if (!currentConfig.codex) activityTracker.forget('codex')
  if (!sizeLoaded) {
    sizeLoaded = true
    let size = 'expanded'
    try {
      size = localStorage.getItem(SIZE_KEY) || size
    } catch {}
    setDisplaySize(size, false)
  } else if (currentConfig.mode === 'menubar' && document.body.classList.contains('pet-only')) {
    setDisplaySize('compact', false)
  }
  heardConfig = true
  paintMute()
  paint()
  applyZoom(currentConfig.zoom)
  fitSize()
})
window.api.onCodex((c) => {
  codexData = c || null
  reactToProvider('codex', codexOn() ? c : null)
  paint()
})
window.api.onRealUsage((u) => {
  realUsage = u || null
  realUsageReadAt = u ? Date.now() : 0
  if (lastData) render(lastData)
})
window.api.onAuthState((s) => {
  const on = !!s?.connected
  document.body.classList.toggle('auth-on', on)
  if (!on) {
    activityTracker.forget('claude')
    claudeActivityData = null
    realUsage = null
    document.body.classList.remove('live')
    // main sends this before it opens the browser for a new login, so the
    // pending state that comes next is not undone by it
    endLogin()
    showProfile(null)
  }
  paint() // losing Claude can leave Codex as the only service
  if (document.body.classList.contains('settings-open')) fitSize()
})

// logged-in account chip (email + plan) shown top-left when connected
let lastProfile = null
function showProfile(p) {
  lastProfile = p
  paintChip()
}

// The chip is also the account switcher, so it must not vanish just because the
// profile fetch failed — that would strand the user on one account until a
// restart. The label main stored for the active account is the same email, so
// it stands in until the profile lands.
function paintChip() {
  const p = lastProfile
  const a = lastAccounts
  const active = (a?.accounts || []).find((x) => x.id === a?.active)
  const email = p?.email || (active?.connected ? active.label : null)
  // the settings block says who is connected, not just that someone is: the
  // logo identifies the service, followed by the same plan badge and email.
  el('acc-ok').textContent = email || 'Connected'
  setPlan(el('acc-sub'), p?.plan)
  el('acc-ok').title = email || 'Connected'
  const chip = el('account-chip')
  const mini = el('mini-acct')
  // the Codex view never borrows Claude's identity: its logs carry no email
  if (viewProvider() === 'codex') {
    const plan = codexData?.plan || null
    el('ac-email').textContent = 'Codex'
    chip.title = 'Codex on this computer'
    setPlan(el('ac-plan'), plan)
    chip.hidden = false
    el('mini-acct-name').textContent = 'Codex'
    setPlan(el('mini-acct-plan'), plan)
    mini.hidden = false
    return
  }
  if (!email) {
    chip.hidden = true
    mini.hidden = true
    return
  }
  // expanded: full email chip in the top bar
  el('ac-email').textContent = email
  chip.title = p?.name ? `${p.name} · ${email}` : email
  setPlan(el('ac-plan'), p?.plan)
  chip.hidden = false
  // collapsed: short name + plan in the mini block (email won't fit at 116px)
  el('mini-acct-name').textContent = p?.name || email.split('@')[0]
  setPlan(el('mini-acct-plan'), p?.plan)
  mini.hidden = false
}
function setPlan(node, plan) {
  if (plan) {
    node.textContent = plan
    node.hidden = false
  } else {
    node.hidden = true
  }
}
window.api.onProfile(showProfile)

// ---- accounts ---------------------------------------------------------------
// The account chip doubles as the switcher: click it for the list, with the
// active one marked. Settings only keeps the login flow itself.
let pendingRemove = null // id whose × is armed, so removal takes two clicks
let lastAccounts = null
let switching = false // a switch is in flight, so the menu waits for its answer

// one row of the chip dropdown
function accountRow(acc, a) {
  const active = acc.id === a.active
  const row = document.createElement('div')
  row.className = 'acc-item'
  if (active) row.classList.add('active')
  if (acc.connected) row.classList.add('connected')
  // the account being switched to owns the spinner: everything on screen
  // still belongs to the one we're leaving
  if (a.busy === acc.id) row.classList.add('loading')

  const dot = document.createElement('span')
  dot.className = 'dot'
  const who = document.createElement('span')
  who.className = 'who'
  who.textContent =
    acc.label || (acc.connected ? 'Connected' : active ? 'Waiting for login…' : 'Not connected')
  row.append(dot, who)
  return row
}

// Everything about accounts lives in the chip dropdown: the chip already says
// which one you are on, so switching, adding and removing all happen there.
function renderAccounts(a) {
  lastAccounts = a
  document.body.classList.toggle('one-account', (a?.accounts || []).length < 2)
  paintChip() // the label may be all the chip has to go on

  if (el('acc-menu').hidden) return
  // the switch is done: the chip now names the account the menu was pointing at
  if (switching && !a?.busy) closeAccountMenu()
  else renderAccountMenu()
}

function renderAccountMenu() {
  const a = lastAccounts
  const box = el('acc-menu')
  box.textContent = ''
  box.classList.toggle('busy', !!a?.busy)
  for (const acc of a?.accounts || []) {
    const active = acc.id === a.active
    const row = accountRow(acc, a)

    // the active account is what the widget is showing: it can't be removed
    // from under you, and the last one standing can't be removed at all
    if (!active && (a.accounts || []).length > 1) {
      const x = document.createElement('button')
      const armed = pendingRemove === acc.id
      x.className = armed ? 'drop armed' : 'drop'
      x.textContent = armed ? 'remove?' : '\u00d7'
      x.title = 'Remove this account'
      x.addEventListener('click', (e) => {
        e.stopPropagation() // the row itself switches accounts
        if (armed) {
          pendingRemove = null
          window.api.accountRemove(acc.id)
        } else {
          pendingRemove = acc.id // a click deletes a token: ask once
          renderAccountMenu()
        }
      })
      row.appendChild(x)
    }

    // stopPropagation: re-rendering detaches this row, and the document-level
    // "clicked outside" handler would then read that as a click off the menu
    if (!active)
      row.addEventListener('click', (e) => {
        e.stopPropagation()
        switchAccount(acc.id)
      })
    box.appendChild(row)
  }

  const add = document.createElement('div')
  add.className = 'acc-item acc-add'
  add.textContent = '+ Add another account'
  add.addEventListener('click', () => {
    closeAccountMenu()
    el('acc-msg').textContent = ''
    window.api.accountAdd() // switches to a fresh slot and opens the browser
  })
  box.appendChild(add)
}

function closeAccountMenu() {
  el('acc-menu').hidden = true
  el('acc-backdrop').hidden = true
  pendingRemove = null
  switching = false
}

function toggleAccountMenu() {
  const box = el('acc-menu')
  if (!box.hidden) {
    closeAccountMenu()
    return
  }
  renderAccountMenu()
  box.hidden = false
  el('acc-backdrop').hidden = false
}

el('account-chip').addEventListener('click', (e) => {
  e.stopPropagation()
  // Codex has no account switcher: its chip leads to its Settings block
  if (viewProvider() === 'codex') return openSettings()
  toggleAccountMenu()
})
// clicking anywhere else — the pet, the gear, another app — puts it away
el('acc-backdrop').addEventListener('mousedown', closeAccountMenu)
document.addEventListener('click', (e) => {
  if (!el('acc-menu').hidden && !el('acc-menu').contains(e.target)) closeAccountMenu()
})
window.addEventListener('blur', closeAccountMenu)
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closeAccountMenu()
})

// paint the pending state from the click itself rather than waiting for main to
// answer — the answer is exactly what takes a moment
function switchAccount(id) {
  if (lastAccounts?.busy) return
  endLogin() // the code from the account we're leaving is no good here
  pendingRemove = null
  switching = true // the row stays on screen, pending, until main answers
  renderAccounts({ ...lastAccounts, busy: id })
  window.api.accountSwitch(id)
}

window.api.onAccounts((a) => {
  if (lastAccounts?.active !== a?.active) {
    activityTracker.forget('claude')
    claudeActivityData = null
  }
  pendingRemove = null
  renderAccounts(a)
  paint()
})

// closing the panel while a login is pending gives up on it: main puts us back
// on the account we were using, and the half-made one goes away
function abandonLogin() {
  if (!document.body.classList.contains('awaiting')) return
  endLogin()
  window.api.accountCancelAdd()
}

function endLogin() {
  document.body.classList.remove('awaiting')
  el('acc-paste').classList.remove('show')
  el('acc-code').value = ''
  el('acc-msg').textContent = ''
}
let successTimer = null
window.api.onAuthResult((r) => {
  if (r?.ok) {
    endLogin()
    // logging in is done: land back on the home panel, where the fresh live
    // meters show up under a short-lived cheer — and the pet throws confetti
    document.body.classList.remove('settings-open')
    clearSaveDirty()
    el('home-success').hidden = false
    clearTimeout(successTimer)
    successTimer = setTimeout(() => {
      el('home-success').hidden = true
      fitSize()
    }, 6000)
    celebrate()
  } else {
    // failed: restore the connection card so a retry is one click away
    document.body.classList.remove('awaiting')
    const e = r?.error || ''
    el('acc-msg').textContent = /429|rate_limit/i.test(e)
      ? 'Rate limited by Anthropic — wait a few minutes, then try once with a fresh code.'
      : `Failed: ${e || 'check the code and try again'}`
  }
  fitSize()
})

// Keep the main view focused on limits; history is an explicit, remembered choice.
const DETAILS_KEY = 'clauddy.details-open'
function setDetailsOpen(open) {
  el('usage-details').hidden = !open
  el('details-toggle').setAttribute('aria-expanded', String(open))
  fitSize()
}
try {
  setDetailsOpen(localStorage.getItem(DETAILS_KEY) === 'true')
} catch {
  setDetailsOpen(false)
}
el('details-toggle').addEventListener('click', () => {
  const open = el('usage-details').hidden
  setDetailsOpen(open)
  try {
    localStorage.setItem(DETAILS_KEY, String(open))
  } catch {}
})

// collapsible panel sections — the widget is a desktop pet, not a dashboard, so
// each breakdown can be folded away and the choice is remembered per machine
const SEC_KEY = 'clauddy.folded'

function readFolded() {
  try {
    return new Set(JSON.parse(localStorage.getItem(SEC_KEY) || '[]'))
  } catch {
    return new Set()
  }
}

function toggleSection(id, force) {
  const sec = el(id)
  if (!sec) return
  // `folded`, not `collapsed` — the body already uses that word for the pet
  const folded = force !== undefined ? force : !sec.classList.contains('folded')
  sec.classList.toggle('folded', folded)
  const head = sec.querySelector('.sec-head')
  if (head) head.setAttribute('aria-expanded', String(!folded))
  // the card just changed height, and the next usage poll is seconds away —
  // without this the window keeps its old size and clips the content. The body
  // slides now rather than snapping, so size it again once that settles.
  fitSize()
  // ...and again when the slide lands. The listener has to check what finished:
  // the bar widths inside the section transition too, and those events bubble up
  // here — resizing on one of them catches the card mid-slide.
  const body = sec.querySelector('.sec-body')
  if (!body) return
  const settle = (e) => {
    if (e.target !== body || e.propertyName !== 'grid-template-rows') return
    body.removeEventListener('transitionend', settle)
    fitSize()
  }
  body.addEventListener('transitionend', settle)
}

for (const head of document.querySelectorAll('.sec-head')) {
  head.addEventListener('click', () => {
    const id = head.dataset.sec
    toggleSection(id)
    const open = readFolded()
    if (el(id).classList.contains('folded')) open.add(id)
    else open.delete(id)
    try {
      localStorage.setItem(SEC_KEY, JSON.stringify([...open]))
    } catch {
      // private mode or a wiped profile — the panel just forgets, which is fine
    }
  })
}

for (const id of readFolded()) toggleSection(id, true)

el('close').addEventListener('click', () => window.api.quit())
el('usage').addEventListener('click', () => window.api.openUsage(viewProvider()))

// Settings → Codex: Connect looks for a local session and turns monitoring on if found
function endCodexSetup() {
  document.body.classList.remove('codex-setup')
  el('codex-setup').hidden = true
  el('codex-hint').textContent = 'Track Codex alongside Claude, in this pet.'
}
el('codex-connect').addEventListener('click', () => {
  el('codex-hint').textContent = 'Looking…'
  window.api.codexDetect()
})
el('codex-retry').addEventListener('click', () => window.api.codexDetect())
// Connect is the consent: found → monitoring starts, not found → say why
window.api.onCodexDetected((r) => {
  if (r?.found) {
    endCodexSetup()
    window.api.codexEnable(true)
    return
  }
  document.body.classList.add('codex-setup')
  el('codex-setup').hidden = false
  el('codex-hint').textContent = 'No session — sign in to Codex'
  fitSize()
})
el('codex-setup-cancel').addEventListener('click', () => {
  endCodexSetup()
  fitSize()
})
el('codex-disconnect').addEventListener('click', () => window.api.codexEnable(false))

// account login (browser flow)
el('acc-connect').addEventListener('click', () => window.api.authStart())
// main opened the browser — the only thing left to do is paste the code back,
// so the panel narrows to exactly that
window.api.onAuthPending(() => {
  // the login can start from the account menu, with the panel closed: the code
  // field is where the flow continues, so bring it up
  openSettings()
  document.body.classList.add('awaiting')
  el('acc-paste').classList.add('show')
  el('acc-code').classList.remove('filled')
  el('acc-code').focus()
  fitSize()
})
// the Connect button only lights up once there's a code to submit, and the
// field stops asking for attention once it has one
el('acc-code').addEventListener('input', () => {
  const code = el('acc-code').value.trim()
  el('acc-confirm').classList.toggle('ready', !!code)
  el('acc-code').classList.toggle('filled', !!code)
})
el('acc-confirm').addEventListener('click', () => {
  const code = el('acc-code').value.trim()
  if (!code) return
  el('acc-msg').textContent = 'Checking…'
  window.api.authCode(code)
  const b = el('acc-confirm')
  b.disabled = true
  setTimeout(() => (b.disabled = false), 5000) // avoid hammering the rate-limited endpoint
})
el('acc-logout').addEventListener('click', () => window.api.authLogout())

// self-update: show the version, check the latest release, one-click update
window.api.onVersion((v) => {
  if (v) el('app-version').textContent = v
})
window.api.onUpdateStatus((s) => {
  const status = el('upd-status')
  const now = el('upd-now')
  const check = el('upd-check')
  // reset to the idle look, then layer each state on top
  now.hidden = true
  check.hidden = false
  status.textContent = ''
  status.className = ''
  const state = s?.state
  if (state === 'checking') {
    status.textContent = 'Checking…'
    check.hidden = true
  } else if (state === 'uptodate') {
    status.textContent = "You're up to date"
    status.className = 'ok'
  } else if (state === 'available') {
    // the button carries the version, so no separate status text is needed
    now.textContent = `Update to ${s.latest}`
    now.hidden = false
    check.hidden = true
  } else if (state === 'updating') {
    status.textContent = 'Updating… the app will restart'
    check.hidden = true
  } else if (state === 'error') {
    status.textContent = 'Check failed'
    status.className = 'err'
  }
  // pulse a dot on the gear whenever an update is waiting (cleared once it's
  // up to date or actively updating) — so it's noticeable without opening Settings
  if (state === 'available') document.body.classList.add('has-update')
  else if (state === 'uptodate' || state === 'updating')
    document.body.classList.remove('has-update')
  if (document.body.classList.contains('settings-open')) fitSize()
})
el('upd-check').addEventListener('click', () => window.api.checkUpdates())
el('upd-now').addEventListener('click', () => {
  el('upd-now').disabled = true
  window.api.doUpdate()
})

// settings panel
// snapshot of the editable fields, to tell whether there are unsaved changes
let settingsBaseline = null
let selectedMode = 'floating'
function setModeUI(mode) {
  selectedMode = mode === 'menubar' ? 'menubar' : 'floating'
  for (const b of document.querySelectorAll('#set-mode .seg-btn')) {
    b.classList.toggle('on', b.dataset.mode === selectedMode)
  }
}
function snapshotSettings() {
  return JSON.stringify({
    mode: selectedMode,
    alerts: el('set-alerts').checked,
    t1: el('set-t1').value,
    t2: el('set-t2').value,
    fire: el('set-fire').value,
    zoom: el('set-zoom').value,
    talk: el('set-talk').checked,
    sound: el('set-sound').checked,
  })
}
function refreshSaveDirty() {
  if (settingsBaseline == null) return
  const dirty = snapshotSettings() !== settingsBaseline
  const b = el('set-save')
  b.classList.toggle('dirty', dirty)
  b.disabled = !dirty // nothing to save, nothing to click
}
function clearSaveDirty() {
  settingsBaseline = snapshotSettings()
  const b = el('set-save')
  b.classList.remove('dirty')
  b.disabled = true
}
function reflectAlertsOn() {
  el('set-rows').classList.toggle('alerts-off', !el('set-alerts').checked)
}
function reflectTalkOn() {
  el('set-rows').classList.toggle('talk-off', !el('set-talk').checked)
}
// zoom applies as you step it, so the widget shows the size right away;
// Cancel puts the saved value back
function previewZoom() {
  const v = Number.parseFloat(el('set-zoom').value)
  if (v >= 100 && v <= 200) {
    applyZoom(v)
    fitSize()
  }
}
function populateSettings() {
  const c = currentConfig || {}
  setModeUI(c.mode)
  el('set-alerts').checked = c.alerts !== false
  reflectAlertsOn()
  const th = c.alertThresholds || [80, 95]
  el('set-t1').value = th[0] != null ? th[0] : 80
  el('set-t2').value = th[1] != null ? th[1] : 95
  el('set-fire').value = c.fireThreshold != null ? c.fireThreshold : 90
  el('set-zoom').value = c.zoom != null ? c.zoom : 100
  el('set-talk').checked = c.talk !== false
  el('set-sound').checked = !!c.sound
  reflectTalkOn()
  clearSaveDirty() // fields now match the saved config
}
function openSettings() {
  hideBubble()
  setDisplaySize('expanded', false)
  populateSettings()
  document.body.classList.add('settings-open')
  fitSize()
}

// Leaving settings plays the panel out to the right while home slides back in.
// The panel is lifted out of the flow for those few frames (see .settings-closing
// in the stylesheet), so the card can already be measured at its home height.
let closingTimer = null
function closeSettings() {
  if (!document.body.classList.contains('settings-open')) return
  abandonLogin()
  endCodexSetup()
  document.body.classList.remove('settings-open')
  document.body.classList.add('settings-closing')
  clearTimeout(closingTimer)
  closingTimer = setTimeout(() => document.body.classList.remove('settings-closing'), 600)
  clearSaveDirty()
  applyZoom(currentConfig?.zoom != null ? currentConfig.zoom : 100) // undo the preview
  fitSize()
}
el('gear').addEventListener('click', () => {
  if (document.body.classList.contains('settings-open')) closeSettings()
  else openSettings()
})
// the "connect" placeholder jumps straight to settings
el('limits-connect').addEventListener('click', openSettings)
// custom number steppers (▲ / ▼)
for (const b of document.querySelectorAll('.num-btn')) {
  b.addEventListener('click', () => {
    const input = el(b.dataset.for)
    const min = Number(input.min) || 1
    const max = Number(input.max) || 100
    const next = (Number.parseInt(input.value, 10) || 0) + Number(b.dataset.step)
    input.value = Math.min(max, Math.max(min, next))
    refreshSaveDirty() // steppers change the value without an 'input' event
    if (input.id === 'set-zoom') previewZoom()
  })
}
// light up Save whenever an editable field changes
for (const id of [
  'set-alerts',
  'set-t1',
  'set-t2',
  'set-fire',
  'set-zoom',
  'set-talk',
  'set-sound',
]) {
  el(id).addEventListener('input', refreshSaveDirty)
  el(id).addEventListener('change', refreshSaveDirty)
}
el('set-alerts').addEventListener('change', reflectAlertsOn)
el('set-talk').addEventListener('change', reflectTalkOn)
// turning the voice on plays a sample, so you hear what you just agreed to
el('set-sound').addEventListener('change', () => {
  if (el('set-sound').checked) blipper.play('Hello there!')
})
el('set-zoom').addEventListener('input', previewZoom)
// display-mode segmented control (floating pet | menu bar)
for (const b of document.querySelectorAll('#set-mode .seg-btn')) {
  b.addEventListener('click', () => {
    setModeUI(b.dataset.mode)
    refreshSaveDirty()
  })
}
el('set-cancel').addEventListener('click', closeSettings)
el('set-save').addEventListener('click', () => {
  abandonLogin() // leaving the panel gives up on a login waiting for its code
  const num = (id) => parseFloat(el(id).value)
  const fire = num('set-fire')
  const zoomRaw = num('set-zoom')
  const zoom = zoomRaw >= 100 && zoomRaw <= 200 ? Math.round(zoomRaw) : 100
  applyZoom(zoom) // apply immediately so the fitSize() below measures the new size
  window.api.saveConfig({
    mode: selectedMode,
    alerts: el('set-alerts').checked,
    alertThresholds: [num('set-t1'), num('set-t2')]
      .filter((n) => n >= 1 && n <= 100)
      .sort((a, b) => a - b),
    fireThreshold: fire >= 1 && fire <= 99 ? fire : 90,
    zoom,
    talk: el('set-talk').checked,
    sound: el('set-sound').checked,
  })
  clearSaveDirty()
  document.body.classList.remove('settings-open')
  fitSize()
})
el('min').addEventListener('click', () => {
  hideBubble()
  setDisplaySize(document.body.classList.contains('collapsed') ? 'expanded' : 'compact')
})
// double-click the pet to collapse / expand
el('pet').addEventListener('dblclick', () => {
  if (document.body.classList.contains('pet-only')) return
  hideBubble()
  setDisplaySize(document.body.classList.contains('collapsed') ? 'expanded' : 'compact')
})

// poke the pet -> bouncy squish + hearts
el('pet').addEventListener('click', () => {
  if (!document.body.classList.contains('pet-only')) pokePet()
})

// eyes follow the cursor
const eyesG = el('eyes')
window.addEventListener('mousemove', (e) => {
  const b = document.body.classList
  if (b.contains('state-sleeping') || b.contains('state-tired')) return
  const r = el('pet').getBoundingClientRect()
  const dx = e.clientX - (r.left + r.width / 2)
  const dy = e.clientY - (r.top + r.height / 2)
  const len = Math.hypot(dx, dy) || 1
  eyesG.setAttribute(
    'transform',
    `translate(${((dx / len) * 3).toFixed(2)} ${((dy / len) * 2).toFixed(2)})`,
  )
})
window.addEventListener('mouseout', (e) => {
  if (!e.relatedTarget) eyesG.setAttribute('transform', 'translate(0 0)')
})

// welcome wave
document.body.classList.add('greet')
setTimeout(() => document.body.classList.remove('greet'), 1200)

// Loaded as a plain <script> by the widget, where `module` doesn't exist. The
// tests import it instead, against a happy-dom document and a stub bridge —
// same dual export as burn.js.
if (typeof module === 'object' && module.exports) {
  module.exports = {
    render,
    fmtTokens,
    fmtReset,
    fmtResetIn,
    setState,
    renderModels,
    renderProjects,
    renderHeat,
    showProfile,
    renderAccounts,
    renderAccountMenu,
    paint,
    pickProvider,
    burn,
    say,
    hideBubble,
    soundOn,
    setDisplaySize,
    setPetPeek,
    paintCompanion,
    activityTracker,
    blipper,
  }
}
