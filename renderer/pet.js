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
  SPRITE.forEach((row, r) => {
    for (let c = 0; c < row.length; c++) {
      const ch = row[c]
      if (ch === '.') continue
      const rect = document.createElementNS(SVGNS, 'rect')
      rect.setAttribute('x', c * C)
      rect.setAttribute('y', r * C)
      rect.setAttribute('width', C)
      rect.setAttribute('height', C)
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
    add('rect', { x: x + 12, y: y, width: t, height: 9, fill: '#3a3a3a' })
    add('rect', { x: x + 5, y: y + 7, width: m, height: 9, fill: '#3a3a3a' })
    add('rect', { x: x, y: y + 14, width: b, height: 10, fill: '#3a3a3a' })
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

// helpers
function fmtTokens(t) {
  t = t || 0
  if (t >= 1e9) return `${(t / 1e9).toFixed(2)}B`
  if (t >= 1e6) return `${(t / 1e6).toFixed(1)}M`
  if (t >= 1e3) return `${(t / 1e3).toFixed(1)}k`
  return String(t)
}
function fmtReset(ms) {
  if (!ms || ms <= 0) return 'now'
  const h = Math.floor(ms / 3600000)
  const m = Math.floor((ms % 3600000) / 60000)
  return h > 0 ? `${h}h ${m}m` : `${m}m`
}
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

// by model (7 days)
function renderModels(list) {
  const box = el('bymodel-list')
  box.innerHTML = ''
  const top = list.slice(0, 4)
  const max = Math.max(1, ...top.map((m) => m.tokens))
  for (const m of top) {
    const row = document.createElement('div')
    row.className = 'mrow'
    row.innerHTML =
      `<span class="mname">${m.label}</span>` +
      `<span class="mbar"><i style="width:${(m.tokens / max) * 100}%"></i></span>` +
      `<span class="mval">${fmtTokens(m.tokens)}</span>`
    box.appendChild(row)
  }
  if (!top.length) {
    box.innerHTML = '<div class="mrow" style="opacity:.5">no activity</div>'
  }
}

// one-shot reaction (adds a class, removes after ms)
function oneShot(cls, ms) {
  document.body.classList.add(cls)
  setTimeout(() => document.body.classList.remove(cls), ms)
}

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

// main render
let prevState = null
let prevPct = null
let lastData = null
function render(d) {
  lastData = d
  // % comes only from the connected account — no estimates
  const liveOn = !!realUsage
  document.body.classList.toggle('live', liveOn)
  const sessPct = liveOn ? realUsage.session.pct : 0
  const sessReset = liveOn ? realUsage.session.resetMs : null
  const sessActive = liveOn && sessReset != null
  const wkPct = liveOn ? realUsage.week.pct : 0
  const wkReset = liveOn ? realUsage.week.resetMs : null

  let st = d.active
    ? 'working'
    : liveOn && sessPct >= 90
      ? 'stressed'
      : d.sleeping
        ? 'sleeping'
        : 'idle'
  // dev override from `./pet <state>`
  if (debugState) {
    const map = { working: 'working', sleeping: 'sleeping', fire: 'stressed', idle: 'idle' }
    if (map[debugState]) st = map[debugState]
  }
  setState(st)
  if (prevState && prevState !== st) {
    if (st === 'sleeping') oneShot('drowse', 700)
    else if (prevState === 'sleeping') oneShot('wake', 700)
  }
  prevState = st

  const word =
    st === 'working'
      ? 'working'
      : st === 'sleeping'
        ? 'sleeping'
        : st === 'stressed'
          ? 'on fire'
          : 'idle'
  el('status-text').textContent = word
  el('mini-text').textContent = word
  el('rate').textContent =
    d.active && d.tokensPerMin > 0
      ? `${fmtTokens(d.tokensPerMin)} tok/min`
      : `${fmtTokens(d.today.tokens)} tokens today`

  if (liveOn && prevPct != null && sessActive && prevPct - sessPct > 25) celebrate()
  prevPct = sessPct
  el('session-pct').textContent = `${Math.round(sessPct)}%`
  const mini = el('mini-pct')
  mini.textContent = liveOn ? `${Math.round(sessPct)}%` : '—'
  mini.classList.toggle('high', liveOn && sessPct >= 80)
  const sf = el('session-fill')
  sf.style.width = `${sessPct}%`
  sf.classList.toggle('high', sessPct >= 80)
  el('session-sub').textContent = sessActive
    ? `resets in ${fmtReset(sessReset)} · ${fmtTokens(d.session.tokens)} tokens`
    : 'no active session'

  el('week-pct').textContent = `${Math.round(wkPct)}%`
  const wf = el('week-fill')
  wf.style.width = `${wkPct}%`
  wf.classList.toggle('high', wkPct >= 80)
  el('week-sub').textContent =
    wkReset != null
      ? `resets in ${fmtReset(wkReset)} · ${fmtTokens(d.week.tokens)} tokens`
      : `${fmtTokens(d.week.tokens)} tokens · last 7 days`

  renderModels(d.byModel || [])
  renderHeat(d.days30 || [])
  el('month-total').textContent = `${fmtTokens(d.monthTokens)} tokens`

  currentRate = d.tokensPerMin || 0
  if (st === 'working') {
    if (!eating) {
      eating = true
      startEating()
    }
  } else if (eating) {
    eating = false
    stopEating()
  }

  fitSize()
}

// fit the window to the content (no leftover border)
let lastH = 0
let lastW = 0
function fitSize() {
  requestAnimationFrame(() => {
    const collapsed = document.body.classList.contains('collapsed')
    const w = collapsed ? 140 : 276
    const h = el('card').offsetHeight + 24 // 12px margin top + bottom
    if (Math.abs(h - lastH) > 2 || w !== lastW) {
      lastH = h
      lastW = w
      window.api.resize(w, h)
    }
  })
}

let currentConfig = {}
let realUsage = null
let debugState = null
window.api.onDebugState((o) => {
  const s = o?.state
  if (s === 'poke') return pokePet()
  if (s === 'celebrate') return celebrate()
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
})
window.api.onRealUsage((u) => {
  realUsage = u || null
  if (lastData) render(lastData)
})
window.api.onAuthState((s) => {
  const on = !!s?.connected
  document.body.classList.toggle('auth-on', on)
  if (!on) {
    realUsage = null
    document.body.classList.remove('live')
    el('acc-paste').classList.remove('show')
  }
  if (document.body.classList.contains('settings-open')) fitSize()
})
window.api.onAuthResult((r) => {
  if (r?.ok) {
    el('acc-msg').textContent = ''
    el('acc-code').value = ''
    el('acc-paste').classList.remove('show')
  } else {
    const e = r?.error || ''
    el('acc-msg').textContent = /429|rate_limit/i.test(e)
      ? 'Rate limited by Anthropic — wait a few minutes, then try once with a fresh code.'
      : `Failed: ${e || 'check the code and try again'}`
  }
  fitSize()
})

el('close').addEventListener('click', () => window.api.quit())
el('usage').addEventListener('click', () => window.api.openUsage())

// account login (browser flow)
el('acc-connect').addEventListener('click', () => {
  window.api.authStart() // opens the browser to log in
  el('acc-paste').classList.add('show') // reveal the code field
  fitSize()
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

// settings panel
function populateSettings() {
  const c = currentConfig || {}
  el('set-alerts').checked = c.alerts !== false
  const th = c.alertThresholds || [80, 95]
  el('set-t1').value = th[0] != null ? th[0] : 80
  el('set-t2').value = th[1] != null ? th[1] : 95
}
function openSettings() {
  document.body.classList.remove('collapsed')
  populateSettings()
  document.body.classList.add('settings-open')
  fitSize()
}
el('gear').addEventListener('click', openSettings)
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
  })
}
el('set-cancel').addEventListener('click', () => {
  document.body.classList.remove('settings-open')
  fitSize()
})
el('set-save').addEventListener('click', () => {
  const num = (id) => parseFloat(el(id).value)
  window.api.saveConfig({
    alerts: el('set-alerts').checked,
    alertThresholds: [num('set-t1'), num('set-t2')]
      .filter((n) => n >= 1 && n <= 100)
      .sort((a, b) => a - b),
  })
  document.body.classList.remove('settings-open')
  fitSize()
})
el('min').addEventListener('click', () => {
  document.body.classList.toggle('collapsed')
  fitSize()
})
// double-click the pet to collapse / expand
el('pet').addEventListener('dblclick', () => {
  document.body.classList.toggle('collapsed')
  fitSize()
})

// poke the pet -> bouncy squish + hearts
el('pet').addEventListener('click', () => pokePet())

// eyes follow the cursor
const eyesG = el('eyes')
window.addEventListener('mousemove', (e) => {
  if (document.body.classList.contains('state-sleeping')) return
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
