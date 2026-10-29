// The selectable pets beyond the classic Clauddy sprite: vector art drawn in
// code, so a pet costs a few KB in the app and nothing is fetched at runtime.
// Each builder takes a prefix that keeps its gradient ids unique, since the
// widget and the Settings thumbnails can draw the same pet at once.

;(() => {
  // Seasonal packs take over on their days: one pet per pack and year, the same
  // one all day and across restarts, whatever the user picked.
  const PACKS = [
    { id: 'everyday', name: 'Everyday' },
    { id: 'halloween', name: 'Halloween', days: [[10, 31]] },
    {
      id: 'christmas',
      name: 'Christmas',
      days: [
        [12, 24],
        [12, 25],
      ],
    },
  ]

  const star = (cx, cy, r, cls = '') => {
    const pts = Array.from({ length: 10 }, (_, i) => {
      const a = (Math.PI / 5) * i - Math.PI / 2
      const rr = i % 2 ? r * 0.45 : r
      return `${(cx + rr * Math.cos(a)).toFixed(1)},${(cy + rr * Math.sin(a)).toFixed(1)}`
    }).join(' ')
    return `<polygon class="${cls}" points="${pts}"/>`
  }
  const PETS = [
    {
      id: 'pumpkin',
      name: 'Pumpkin',
      pack: 'halloween',
      move: 'rock',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 180" aria-hidden="true">
  <defs>
    <radialGradient id="${p}m" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#ffbb66"/><stop offset=".5" stop-color="#f08426"/><stop offset="1" stop-color="#b24e0d"/></radialGradient>
    <radialGradient id="${p}s" cx=".45" cy=".3" r=".85"><stop offset="0" stop-color="#f69a45"/><stop offset=".55" stop-color="#d4681a"/><stop offset="1" stop-color="#8a380a"/></radialGradient>
    <linearGradient id="${p}st" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#8fae4a"/><stop offset="1" stop-color="#43601d"/></linearGradient>
    <linearGradient id="${p}lf" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#a6d05a"/><stop offset="1" stop-color="#4f7d24"/></linearGradient>
    <radialGradient id="${p}g" cx=".5" cy=".55" r=".7"><stop offset="0" stop-color="#fff6c2"/><stop offset=".45" stop-color="#ffd14d"/><stop offset="1" stop-color="#ff8a1a"/></radialGradient>
    <radialGradient id="${p}h" cx=".5" cy=".5" r=".5"><stop offset="0" stop-color="#ffcf5a" stop-opacity=".55"/><stop offset="1" stop-color="#ff8a1a" stop-opacity="0"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
    <g id="${p}face">
      <path d="M60 98 L90 98 L80 74 Z"/><path d="M110 98 L140 98 L120 74 Z"/><path d="M93 112 L107 112 L100 102 Z"/>
      <path d="M52 116 L66 124 L76 115 L88 126 L100 117 L112 126 L124 115 L134 124 L148 116 Q136 154 100 154 Q64 154 52 116 Z"/>
    </g>
  </defs>
  <g class="p-body" style="transform-origin: 100px 160px">
    <ellipse class="p-halo" cx="100" cy="116" rx="70" ry="48" fill="url(#${p}h)"/>
    <ellipse cx="58" cy="112" rx="46" ry="52" fill="url(#${p}s)"/>
    <ellipse cx="142" cy="112" rx="46" ry="52" fill="url(#${p}s)"/>
    <ellipse cx="80" cy="110" rx="40" ry="57" fill="url(#${p}m)" stroke="#8a3a0a" stroke-opacity=".35" stroke-width="1.5"/>
    <ellipse cx="120" cy="110" rx="40" ry="57" fill="url(#${p}m)" stroke="#8a3a0a" stroke-opacity=".35" stroke-width="1.5"/>
    <ellipse cx="100" cy="108" rx="33" ry="59" fill="url(#${p}m)" stroke="#8a3a0a" stroke-opacity=".35" stroke-width="1.5"/>
    <ellipse cx="78" cy="68" rx="16" ry="7" fill="#fff" opacity=".4" filter="url(#${p}f)" transform="rotate(-20 78 68)"/>
    <path d="M93 56 C 91 40, 96 30, 108 22 L 116 28 C 107 35, 104 46, 107 58 Z" fill="url(#${p}st)"/>
    <path d="M108 30 c 10 -12 4 -20 14 -24" fill="none" stroke="#5d7f2a" stroke-width="2.5" stroke-linecap="round"/>
    <g class="p-leaf" style="transform-origin: 106px 42px">
      <path d="M106 42 C 120 26, 146 28, 154 40 C 140 52, 120 52, 106 42 Z" fill="url(#${p}lf)"/>
      <path d="M108 42 C 124 38, 138 38, 150 40" fill="none" stroke="#3f6a1a" stroke-width="1.5" stroke-opacity=".6"/>
    </g>
    <g class="blink" style="transform-origin: 100px 90px">
      <use href="#${p}face" fill="#4a1806" transform="translate(0 -3)"/>
      <use href="#${p}face" fill="url(#${p}g)"/>
      <use class="p-flame" href="#${p}face" fill="#fff4b8" opacity=".35"/>
    </g>
  </g>
</svg>`,
    },
    {
      id: 'skull',
      name: 'Skull',
      pack: 'halloween',
      move: 'dance',
      svg: (p) => {
        const bone = (x1, y1, x2, y2, w) => {
          const dx = x2 - x1
          const dy = y2 - y1
          const l = Math.hypot(dx, dy)
          const nx = (-dy / l) * w * 0.42
          const ny = (dx / l) * w * 0.42
          const r = w * 0.52
          return (
            `<path d="M${x1} ${y1} L${x2} ${y2}" stroke="url(#${p}n)" stroke-width="${w}" stroke-linecap="round"/>` +
            [
              [x1, y1],
              [x2, y2],
            ]
              .map(
                ([x, y]) =>
                  `<circle cx="${x + nx}" cy="${y + ny}" r="${r}" fill="url(#${p}n)"/><circle cx="${x - nx}" cy="${y - ny}" r="${r}" fill="url(#${p}n)"/>`,
              )
              .join('')
          )
        }
        return `
<svg ROOT viewBox="0 0 200 215" aria-hidden="true">
  <defs>
    <radialGradient id="${p}b" cx=".38" cy=".28" r=".85"><stop offset="0" stop-color="#ffffff"/><stop offset=".5" stop-color="#efe6d2"/><stop offset="1" stop-color="#a8987a"/></radialGradient>
    <radialGradient id="${p}n" cx=".4" cy=".3" r=".9"><stop offset="0" stop-color="#fffaf0"/><stop offset="1" stop-color="#c9bb9e"/></radialGradient>
    <radialGradient id="${p}s" cx=".5" cy=".4" r=".7"><stop offset="0" stop-color="#3a2f44"/><stop offset="1" stop-color="#120d18"/></radialGradient>
    <radialGradient id="${p}g" cx=".5" cy=".5" r=".5"><stop offset="0" stop-color="#eaffd8"/><stop offset=".4" stop-color="#8dff9e"/><stop offset="1" stop-color="#2bd66a" stop-opacity="0"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
  </defs>
  <g class="k-leg-a">${bone(90, 168, 86, 192, 11)}<ellipse cx="82" cy="199" rx="14" ry="7" fill="url(#${p}n)"/></g>
  <g class="k-leg-b">${bone(110, 168, 114, 192, 11)}<ellipse cx="118" cy="199" rx="14" ry="7" fill="url(#${p}n)"/></g>
  <g class="k-arm-l" style="transform-origin: 80px 128px">${bone(80, 128, 62, 152, 9)}<circle cx="58" cy="158" r="8" fill="url(#${p}n)"/></g>
  <g class="k-arm-r" style="transform-origin: 120px 128px">${bone(120, 128, 138, 152, 9)}<circle cx="142" cy="158" r="8" fill="url(#${p}n)"/></g>
  <rect x="94" y="146" width="12" height="18" rx="5" fill="url(#${p}n)"/>
  <path d="M80 162 C 80 154, 120 154, 120 162 C 120 174, 108 176, 100 170 C 92 176, 80 174, 80 162 Z" fill="url(#${p}b)"/>
  <path d="M74 128 C 74 116, 126 116, 126 128 C 126 146, 114 156, 100 156 C 86 156, 74 146, 74 128 Z" fill="url(#${p}b)"/>
  <g stroke="#3a2f44" stroke-opacity=".75" stroke-width="3.6" stroke-linecap="round" fill="none">
    <path d="M82 129 C 88 132, 93 132, 96 131"/><path d="M118 129 C 112 132, 107 132, 104 131"/>
    <path d="M84 138 C 89 141, 93 141, 96 140"/><path d="M116 138 C 111 141, 107 141, 104 140"/>
    <path d="M88 147 C 91 149, 94 149, 96 148"/><path d="M112 147 C 109 149, 106 149, 104 148"/>
  </g>
  <g class="k-head">
    <g class="k-jaw"><path d="M76 104 L 124 104 C 124 120, 114 128, 100 128 C 86 128, 76 120, 76 104 Z" fill="url(#${p}b)"/>
      <g fill="#fffdf6" stroke="#9a8a6c" stroke-width="1"><rect x="83" y="106" width="8" height="8" rx="2.5"/><rect x="92" y="107" width="8" height="8" rx="2.5"/><rect x="101" y="107" width="8" height="8" rx="2.5"/><rect x="110" y="106" width="8" height="8" rx="2.5"/></g></g>
    <path d="M100 22 C 140 22, 152 50, 150 76 C 149 90, 142 98, 132 102 L 128 110 L 72 110 L 68 102 C 58 98, 51 90, 50 76 C 48 50, 60 22, 100 22 Z" fill="url(#${p}b)"/>
    <g fill="#fffdf6" stroke="#9a8a6c" stroke-width="1"><rect x="83" y="102" width="8" height="9" rx="2.5"/><rect x="92" y="102" width="8" height="10" rx="2.5"/><rect x="101" y="102" width="8" height="10" rx="2.5"/><rect x="110" y="102" width="8" height="9" rx="2.5"/></g>
    <ellipse cx="78" cy="42" rx="18" ry="8" fill="#fff" opacity=".8" filter="url(#${p}f)" transform="rotate(-25 78 42)"/>
    <path d="M122 28 l -5 9 l 6 4 l -4 8" fill="none" stroke="#8a7a5c" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
    <path d="M66 74 C 66 62, 76 58, 84 60 C 94 62, 98 72, 96 80 C 94 90, 84 92, 76 90 C 68 88, 66 82, 66 74 Z" fill="url(#${p}s)"/>
    <path d="M134 74 C 134 62, 124 58, 116 60 C 106 62, 102 72, 104 80 C 106 90, 116 92, 124 90 C 132 88, 134 82, 134 74 Z" fill="url(#${p}s)"/>
    <g class="blink" style="transform-origin: 100px 76px"><g class="k-glow">
      <circle cx="82" cy="76" r="12" fill="url(#${p}g)"/><circle cx="118" cy="76" r="12" fill="url(#${p}g)"/>
      <circle cx="82" cy="76" r="4" fill="#f2fff0"/><circle cx="118" cy="76" r="4" fill="#f2fff0"/>
    </g></g>
    <path d="M100 86 C 96 90, 94 95, 97 97 L 100 95 L 103 97 C 106 95, 104 90, 100 86 Z" fill="url(#${p}s)"/>
    <ellipse cx="64" cy="94" rx="7" ry="4" fill="#ff9aa8" opacity=".35" filter="url(#${p}f)"/><ellipse cx="136" cy="94" rx="7" ry="4" fill="#ff9aa8" opacity=".35" filter="url(#${p}f)"/>
  </g>
</svg>`
      },
    },
    {
      id: 'ghost',
      name: 'Ghost',
      pack: 'halloween',
      move: 'drift',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 200" aria-hidden="true">
  <defs>
    <radialGradient id="${p}b" cx=".36" cy=".24" r=".9"><stop offset="0" stop-color="#ffffff"/><stop offset=".55" stop-color="#ece8ff"/><stop offset="1" stop-color="#a99fd6"/></radialGradient>
    <radialGradient id="${p}e" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#3b3450"/><stop offset="1" stop-color="#0f0c18"/></radialGradient>
    <radialGradient id="${p}h" cx=".5" cy=".5" r=".5"><stop offset="0" stop-color="#d9d2ff" stop-opacity=".55"/><stop offset="1" stop-color="#b3a6ff" stop-opacity="0"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
  </defs>
  <g class="g-float">
    <ellipse cx="100" cy="106" rx="78" ry="84" fill="url(#${p}h)"/>
    <g class="g-arm-l" style="transform-origin: 50px 112px"><ellipse cx="40" cy="120" rx="10" ry="15" fill="url(#${p}b)" transform="rotate(35 40 120)"/></g>
    <g class="g-arm-r" style="transform-origin: 150px 112px"><ellipse cx="160" cy="120" rx="10" ry="15" fill="url(#${p}b)" transform="rotate(-35 160 120)"/></g>
    <path class="g-sheet" d="M100 30 C 140 30, 156 64, 156 104 L 156 166 Q 147 180 138 166 Q 129 152 119 166 Q 110 180 100 166 Q 90 152 81 166 Q 72 180 62 166 Q 53 152 44 166 L 44 104 C 44 64, 60 30, 100 30 Z" fill="url(#${p}b)" opacity=".96"/>
    <ellipse cx="78" cy="56" rx="18" ry="9" fill="#fff" opacity=".8" filter="url(#${p}f)" transform="rotate(-30 78 56)"/>
    <g class="blink" style="transform-origin: 100px 92px">
      <ellipse cx="84" cy="92" rx="9" ry="12" fill="url(#${p}e)"/><ellipse cx="116" cy="92" rx="9" ry="12" fill="url(#${p}e)"/>
      <circle cx="80.5" cy="87" r="3.4" fill="#fff"/><circle cx="112.5" cy="87" r="3.4" fill="#fff"/>
    </g>
    <ellipse cx="68" cy="110" rx="9" ry="5" fill="#ff9fc0" opacity=".6" filter="url(#${p}f)"/>
    <ellipse cx="132" cy="110" rx="9" ry="5" fill="#ff9fc0" opacity=".6" filter="url(#${p}f)"/>
    <ellipse class="g-mouth" cx="100" cy="114" rx="5" ry="6" fill="url(#${p}e)"/>
  </g>
</svg>`,
    },
    {
      id: 'bat',
      name: 'Bat',
      pack: 'halloween',
      move: 'drift',
      svg: (p) => {
        const wing = `<path d="M76 98 C 62 72, 34 60, 6 68 C 13 80, 13 93, 6 108 C 19 103, 30 107, 34 118 C 42 109, 52 109, 58 118 C 62 109, 70 107, 78 112 Z" fill="url(#${p}w)"/>
        <path d="M76 98 C 62 72, 34 60, 6 68" fill="none" stroke="#c4a6ff" stroke-opacity=".6" stroke-width="1.8" stroke-linecap="round"/>
        <path d="M74 100 L 8 70 M73 104 L 8 106 M74 108 L 34 116 M76 110 L 58 116" stroke="#2a1650" stroke-opacity=".6" stroke-width="2.2" stroke-linecap="round"/>`
        return `
<svg ROOT viewBox="0 0 200 190" aria-hidden="true">
  <defs>
    <radialGradient id="${p}b" cx=".38" cy=".28" r=".85"><stop offset="0" stop-color="#9580cc"/><stop offset=".5" stop-color="#5a4596"/><stop offset="1" stop-color="#271a4a"/></radialGradient>
    <radialGradient id="${p}w" cx=".95" cy=".45" r="1"><stop offset="0" stop-color="#9a72e0"/><stop offset=".55" stop-color="#5b36a0"/><stop offset="1" stop-color="#2c1658"/></radialGradient>
    <linearGradient id="${p}i" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#f0a6d0"/><stop offset="1" stop-color="#b06aa8"/></linearGradient>
    <radialGradient id="${p}e" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#3b3450"/><stop offset="1" stop-color="#07050c"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
  </defs>
  <g class="b-fly">
    <g class="b-wing-l" style="transform-origin: 76px 104px">${wing}</g>
    <g class="b-wing-r" style="transform-origin: 124px 104px"><g transform="translate(200 0) scale(-1 1)">${wing}</g></g>
    <path d="M90 148 q-4 6 -8 6 M110 148 q4 6 8 6" fill="none" stroke="#1d1238" stroke-width="3.5" stroke-linecap="round"/>
    <path d="M68 94 C 60 72, 62 52, 72 42 C 86 52, 94 66, 94 82 Z" fill="url(#${p}b)"/>
    <path d="M132 94 C 140 72, 138 52, 128 42 C 114 52, 106 66, 106 82 Z" fill="url(#${p}b)"/>
    <path d="M72 86 C 67 72, 68 60, 73 53 C 82 60, 87 70, 87 80 Z" fill="url(#${p}i)" opacity=".8"/>
    <path d="M128 86 C 133 72, 132 60, 127 53 C 118 60, 113 70, 113 80 Z" fill="url(#${p}i)" opacity=".8"/>
    <ellipse cx="100" cy="112" rx="36" ry="37" fill="url(#${p}b)" stroke="#b89cff" stroke-opacity=".35" stroke-width="1.8"/>
    <path d="M88 80 L 92 66 L 98 76 L 104 63 L 108 76 L 114 68 L 113 82 Z" fill="url(#${p}b)"/>
    <ellipse cx="100" cy="130" rx="17" ry="13" fill="#b4a2e0" opacity=".35" filter="url(#${p}f)"/>
    <ellipse cx="82" cy="88" rx="12" ry="6" fill="#fff" opacity=".35" filter="url(#${p}f)" transform="rotate(-25 82 88)"/>
    <g class="blink" style="transform-origin: 100px 106px">
      <ellipse cx="85" cy="106" rx="11" ry="12.5" fill="#fff8e6"/><ellipse cx="115" cy="106" rx="11" ry="12.5" fill="#fff8e6"/>
      <ellipse cx="87" cy="108" rx="7.5" ry="9" fill="url(#${p}e)"/><ellipse cx="113" cy="108" rx="7.5" ry="9" fill="url(#${p}e)"/>
      <circle cx="84.5" cy="103.5" r="2.8" fill="#fff"/><circle cx="110.5" cy="103.5" r="2.8" fill="#fff"/>
      <circle cx="89.5" cy="112" r="1.3" fill="#fff" opacity=".8"/><circle cx="115.5" cy="112" r="1.3" fill="#fff" opacity=".8"/>
    </g>
    <ellipse cx="72" cy="122" rx="7" ry="4" fill="#ff8fc0" opacity=".55" filter="url(#${p}f)"/>
    <ellipse cx="128" cy="122" rx="7" ry="4" fill="#ff8fc0" opacity=".55" filter="url(#${p}f)"/>
    <ellipse cx="100" cy="119" rx="3.2" ry="2.2" fill="#e88ab8"/>
    <path d="M92 123 q4 4 8 0 q4 4 8 0" fill="none" stroke="#140c26" stroke-width="2.4" stroke-linecap="round"/>
    <path d="M94.5 124.5 l1.6 5 l1.8 -4 Z M105.5 124.5 l-1.6 5 l-1.8 -4 Z" fill="#fff"/>
  </g>
</svg>`
      },
    },
    {
      id: 'cat',
      name: 'Black cat',
      pack: 'halloween',
      move: 'wiggle',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 215" aria-hidden="true">
  <defs>
    <radialGradient id="${p}b" cx=".36" cy=".24" r=".9"><stop offset="0" stop-color="#66607e"/><stop offset=".5" stop-color="#2d293d"/><stop offset="1" stop-color="#0e0c15"/></radialGradient>
    <linearGradient id="${p}l" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#4a4462"/><stop offset="1" stop-color="#1a1724"/></linearGradient>
    <linearGradient id="${p}t" x1="0" y1="1" x2="1" y2="0"><stop offset="0" stop-color="#2a2638"/><stop offset="1" stop-color="#4c4666"/></linearGradient>
    <radialGradient id="${p}e" cx=".4" cy=".35" r=".7"><stop offset="0" stop-color="#f6ffa8"/><stop offset=".6" stop-color="#b6e43c"/><stop offset="1" stop-color="#5e8a10"/></radialGradient>
    <radialGradient id="${p}h" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#7a58b0"/><stop offset=".55" stop-color="#3f2868"/><stop offset="1" stop-color="#1a0d32"/></radialGradient>
    <linearGradient id="${p}o" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffb24a"/><stop offset="1" stop-color="#e2711d"/></linearGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
  </defs>
  <g class="c-tail" style="transform-origin: 124px 184px">
    <path d="M120 184 C 162 188, 174 154, 160 128 C 154 117, 160 106, 170 109" fill="none" stroke="url(#${p}t)" stroke-width="15" stroke-linecap="round"/>
    <path d="M160 128 C 154 117, 160 106, 170 109" fill="none" stroke="#8f84c8" stroke-opacity=".35" stroke-width="3" stroke-linecap="round" transform="translate(-3 -2)"/>
  </g>
  <ellipse cx="72" cy="178" rx="19" ry="21" fill="url(#${p}b)"/><ellipse cx="128" cy="178" rx="19" ry="21" fill="url(#${p}b)"/>
  <path d="M100 112 C 74 112, 62 146, 64 176 C 65 196, 135 196, 136 176 C 138 146, 126 112, 100 112 Z" fill="url(#${p}b)" stroke="#8f84c8" stroke-opacity=".3" stroke-width="2"/>
  <path d="M92 120 C 96 130, 104 130, 108 120 C 106 134, 100 142, 100 142 C 100 142, 94 134, 92 120 Z" fill="#ece8f4" opacity=".92"/>
  <rect x="82" y="150" width="15" height="42" rx="7.5" fill="url(#${p}l)"/><rect x="103" y="150" width="15" height="42" rx="7.5" fill="url(#${p}l)"/>
  <ellipse cx="89" cy="193" rx="11" ry="6.5" fill="url(#${p}l)"/><ellipse cx="111" cy="193" rx="11" ry="6.5" fill="url(#${p}l)"/>
  <path d="M85 195 v-3 M89 196 v-3 M93 195 v-3 M107 195 v-3 M111 196 v-3 M115 195 v-3" stroke="#0e0c15" stroke-opacity=".6" stroke-width="1.4" stroke-linecap="round"/>
  <g class="c-ear-l" style="transform-origin: 76px 72px"><path d="M60 84 L 62 36 L 96 62 Z" fill="url(#${p}b)" stroke="#8f84c8" stroke-opacity=".3" stroke-width="2" stroke-linejoin="round"/><path d="M67 76 L 68 49 L 88 64 Z" fill="#e08cb0" opacity=".65"/></g>
  <g class="c-ear-r" style="transform-origin: 124px 72px"><path d="M140 84 L 138 36 L 104 62 Z" fill="url(#${p}b)" stroke="#8f84c8" stroke-opacity=".3" stroke-width="2" stroke-linejoin="round"/><path d="M133 76 L 132 49 L 112 64 Z" fill="#e08cb0" opacity=".65"/></g>
  <path d="M60 96 L 50 104 L 62 106 Z M140 96 L 150 104 L 138 106 Z" fill="#2d293d"/>
  <ellipse cx="100" cy="94" rx="42" ry="34" fill="url(#${p}b)" stroke="#8f84c8" stroke-opacity=".3" stroke-width="2"/>
  <ellipse cx="80" cy="74" rx="13" ry="5" fill="#fff" opacity=".25" filter="url(#${p}f)" transform="rotate(-20 80 74)"/>
  <g class="blink" style="transform-origin: 100px 94px">
    <ellipse cx="84" cy="94" rx="11.5" ry="12.5" fill="url(#${p}e)"/><ellipse cx="116" cy="94" rx="11.5" ry="12.5" fill="url(#${p}e)"/>
    <ellipse class="c-pupil" cx="84" cy="95" rx="2.8" ry="9" fill="#0b0a10"/><ellipse class="c-pupil" cx="116" cy="95" rx="2.8" ry="9" fill="#0b0a10"/>
    <circle cx="80" cy="88.5" r="2.8" fill="#fff"/><circle cx="112" cy="88.5" r="2.8" fill="#fff"/>
    <circle cx="87.5" cy="100" r="1.2" fill="#fff" opacity=".8"/><circle cx="119.5" cy="100" r="1.2" fill="#fff" opacity=".8"/>
  </g>
  <ellipse cx="72" cy="108" rx="7" ry="3.8" fill="#ff8fb8" opacity=".45" filter="url(#${p}f)"/><ellipse cx="128" cy="108" rx="7" ry="3.8" fill="#ff8fb8" opacity=".45" filter="url(#${p}f)"/>
  <path d="M95.5 106 L 104.5 106 L 100 111.5 Z" fill="#f29ab4"/>
  <path d="M100 111.5 q-4 5 -8 2 M100 111.5 q4 5 8 2" fill="none" stroke="#a89cc8" stroke-width="1.8" stroke-linecap="round"/>
  <path d="M72 104 L 46 99 M72 109 L 47 113 M128 104 L 154 99 M128 109 L 153 113" stroke="#c0b8dc" stroke-opacity=".75" stroke-width="1.4" stroke-linecap="round"/>
  <g class="c-hat" style="transform-origin: 104px 62px">
    <g transform="rotate(-12 104 62)">
      <ellipse cx="104" cy="62" rx="38" ry="8" fill="url(#${p}h)"/>
      <path d="M82 62 C 88 42, 95 24, 112 8 C 118 3, 128 5, 126 12 C 118 19, 119 40, 126 62 Z" fill="url(#${p}h)"/>
      <path d="M83 58 C 97 63, 112 63, 125 58 L 124 50 C 112 55, 97 55, 85 50 Z" fill="url(#${p}o)"/>
      <rect x="98" y="51" width="11" height="10" rx="1.5" fill="none" stroke="#ffe27a" stroke-width="2.2"/>
      <ellipse cx="96" cy="34" rx="3.5" ry="10" fill="#fff" opacity=".22" filter="url(#${p}f)" transform="rotate(25 96 34)"/>
    </g>
  </g>
</svg>`,
    },
    {
      id: 'snowman',
      name: 'Snowman',
      pack: 'christmas',
      move: 'sway',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 220" aria-hidden="true">
  <defs>
    <radialGradient id="${p}b" cx=".38" cy=".3" r=".8"><stop offset="0" stop-color="#ffffff"/><stop offset=".55" stop-color="#e8f0f8"/><stop offset="1" stop-color="#a6b9d1"/></radialGradient>
    <radialGradient id="${p}r" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#ff6a5c"/><stop offset=".5" stop-color="#d8333a"/><stop offset="1" stop-color="#8e1a24"/></radialGradient>
    <radialGradient id="${p}w" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#d3dbe4"/></radialGradient>
    <radialGradient id="${p}e" cx=".35" cy=".3" r=".8"><stop offset="0" stop-color="#4b4f5c"/><stop offset="1" stop-color="#111217"/></radialGradient>
    <linearGradient id="${p}n" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffab55"/><stop offset="1" stop-color="#d85f12"/></linearGradient>
    <linearGradient id="${p}s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3fb27a"/><stop offset="1" stop-color="#1c6d47"/></linearGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
  </defs>
  <g class="only-work" fill="#fff">
    <circle class="flake" cx="28" cy="30" r="3"/><circle class="flake" cx="172" cy="20" r="2.5" style="animation-delay:.5s"/><circle class="flake" cx="56" cy="8" r="2" style="animation-delay:1s"/><circle class="flake" cx="150" cy="60" r="2.2" style="animation-delay:1.4s"/><circle class="flake" cx="182" cy="96" r="2" style="animation-delay:.8s"/><circle class="flake" cx="18" cy="90" r="2.4" style="animation-delay:1.8s"/>
  </g>
  <g stroke="#6b4423" stroke-width="4.5" stroke-linecap="round" fill="none">
    <g class="s-arm-l" style="transform-origin: 66px 150px"><path d="M66 150 L 26 128 M 40 136 L 33 120 M 47 140 L 35 143"/></g>
    <g class="s-arm-r" style="transform-origin: 134px 150px"><path d="M134 150 L 174 128 M 160 136 L 167 120 M 153 140 L 165 143"/></g>
  </g>
  <circle cx="100" cy="168" r="44" fill="url(#${p}b)"/>
  <ellipse cx="82" cy="150" rx="14" ry="8" fill="#fff" opacity=".7" filter="url(#${p}f)" transform="rotate(-25 82 150)"/>
  <g fill="url(#${p}e)"><circle cx="100" cy="158" r="4.5"/><circle cx="100" cy="174" r="4.5"/><circle cx="100" cy="190" r="4.5"/></g>
  <ellipse cx="100" cy="134" rx="34" ry="6" fill="#5f7392" opacity=".4" filter="url(#${p}f)"/>
  <circle cx="100" cy="100" r="36" fill="url(#${p}b)"/>
  <ellipse cx="84" cy="84" rx="11" ry="6" fill="#fff" opacity=".7" filter="url(#${p}f)" transform="rotate(-25 84 84)"/>
  <g class="s-scarf" style="transform-origin: 120px 134px">
    <path d="M110 132 L 130 132 L 134 168 L 116 171 Z" fill="url(#${p}s)"/>
    <path d="M113 146 L 132 144 M115 158 L 133 156" stroke="#fff" stroke-opacity=".75" stroke-width="4"/>
    <path d="M118 171 v6 M123 170 v6 M128 170 v6 M133 169 v6" stroke="#1c6d47" stroke-width="2.5" stroke-linecap="round"/>
  </g>
  <path d="M64 124 C 80 134, 120 134, 136 124 L 136 137 C 120 147, 80 147, 64 137 Z" fill="url(#${p}s)"/>
  <path d="M80 129 v12 M100 132 v12 M120 129 v12" stroke="#fff" stroke-opacity=".75" stroke-width="4"/>
  <g class="only-idle"><g class="blink" style="transform-origin: 100px 96px">
    <ellipse cx="88" cy="96" rx="5" ry="6" fill="url(#${p}e)"/><ellipse cx="112" cy="96" rx="5" ry="6" fill="url(#${p}e)"/>
    <circle cx="86.3" cy="93.5" r="1.8" fill="#fff"/><circle cx="110.3" cy="93.5" r="1.8" fill="#fff"/>
  </g></g>
  <g class="only-work" fill="none" stroke="#17181e" stroke-width="3.6" stroke-linecap="round"><path d="M82 98 Q88 91 94 98"/><path d="M106 98 Q112 91 118 98"/></g>
  <path d="M98 104 Q101 102 104 104 L 128 111 Q 104 118 99 113 Q 96 109 98 104 Z" fill="url(#${p}n)"/>
  <ellipse cx="76" cy="109" rx="7" ry="4" fill="#ff8fa8" opacity=".6" filter="url(#${p}f)"/>
  <ellipse cx="124" cy="109" rx="7" ry="4" fill="#ff8fa8" opacity=".6" filter="url(#${p}f)"/>
  <g class="only-idle" fill="url(#${p}e)"><circle cx="89" cy="116" r="2.4"/><circle cx="94.5" cy="119.5" r="2.4"/><circle cx="100" cy="121" r="2.4"/><circle cx="105.5" cy="119.5" r="2.4"/><circle cx="111" cy="116" r="2.4"/></g>
  <g class="only-work"><path d="M91 116 h18 q0 9 -9 9 q-9 0 -9 -9 z" fill="#2a1c22"/></g>
  <g transform="translate(100 64) scale(.62) translate(-100 -60)">
    <ellipse cx="100" cy="99" rx="54" ry="7" fill="#5f7392" opacity=".35" filter="url(#${p}f)"/>
    <path class="s-hat" d="M42 84 C 42 48, 72 24, 108 24 C 140 24, 164 40, 172 70 L 160 74 C 156 60, 146 52, 134 50 C 146 58, 154 70, 158 84 Z" fill="url(#${p}r)"/>
    <path d="M134 50 C 146 58, 154 70, 158 84" fill="none" stroke="#7d1420" stroke-opacity=".45" stroke-width="3" stroke-linecap="round"/>
    <g class="s-pom"><circle cx="167" cy="75" r="14" fill="url(#${p}w)"/></g>
    <g fill="url(#${p}w)">
      <rect x="38" y="72" width="124" height="22" rx="11"/>
      <circle cx="46" cy="83" r="11"/><circle cx="62" cy="80" r="11"/><circle cx="78" cy="79" r="11"/><circle cx="94" cy="78" r="11"/><circle cx="110" cy="78" r="11"/><circle cx="126" cy="79" r="11"/><circle cx="142" cy="80" r="11"/><circle cx="156" cy="83" r="11"/>
    </g>
  </g>
</svg>`,
    },
    {
      id: 'santa',
      name: 'Santa',
      pack: 'christmas',
      move: 'jolly',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 215" aria-hidden="true">
  <defs>
    <radialGradient id="${p}k" cx=".38" cy=".3" r=".8"><stop offset="0" stop-color="#ffe3c8"/><stop offset=".55" stop-color="#f2b98c"/><stop offset="1" stop-color="#c98558"/></radialGradient>
    <radialGradient id="${p}r" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#ff6a5c"/><stop offset=".5" stop-color="#d8333a"/><stop offset="1" stop-color="#8e1a24"/></radialGradient>
    <radialGradient id="${p}w" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#d3dbe4"/></radialGradient>
    <radialGradient id="${p}e" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#3d2c24"/><stop offset="1" stop-color="#0f0a08"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
    <radialGradient id="${p}s" cx=".38" cy=".3" r=".85"><stop offset="0" stop-color="#d8a26a"/><stop offset=".55" stop-color="#a06c3a"/><stop offset="1" stop-color="#5e3c1c"/></radialGradient>
    <linearGradient id="${p}g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffe680"/><stop offset="1" stop-color="#e0a020"/></linearGradient>
  </defs>
  <g class="n-sack" style="transform-origin: 152px 180px">
    <rect x="132" y="86" width="17" height="17" rx="2.5" fill="#2f9e5f" transform="rotate(-12 140 94)"/><path d="M140 86 v17 M132 94 h17" stroke="#ffd45a" stroke-width="3" transform="rotate(-12 140 94)"/>
    <rect x="151" y="90" width="15" height="15" rx="2.5" fill="#d8333a" transform="rotate(10 158 97)"/><path d="M158 90 v15 M151 97 h15" stroke="#fff4d0" stroke-width="2.6" transform="rotate(10 158 97)"/>
    <path d="M167 104 v-14 q0 -7 6 -7 q6 0 6 6" fill="none" stroke="#fff" stroke-width="5" stroke-linecap="round"/>
    <path d="M167 104 v-14 q0 -7 6 -7 q6 0 6 6" fill="none" stroke="#e0353d" stroke-width="5" stroke-linecap="round" stroke-dasharray="4 5"/>
    <path d="M140 114 C 126 124, 120 146, 124 166 C 128 186, 178 186, 182 166 C 186 146, 178 124, 164 114 Z" fill="url(#${p}s)"/>
    <path d="M132 104 C 140 98, 164 98, 172 104 L 166 116 C 158 112, 146 112, 138 116 Z" fill="url(#${p}s)"/>
    <path d="M140 108 l3 6 M150 106 v8 M160 106 l-2 8" stroke="#5e3c1c" stroke-opacity=".5" stroke-width="1.8" stroke-linecap="round"/>
    <path d="M136 116 C 146 120, 158 120, 168 116" fill="none" stroke="#e8c27a" stroke-width="4.5" stroke-linecap="round"/>
    <path d="M152 119 C 148 128, 146 132, 142 136 M152 119 C 156 128, 160 130, 164 134" fill="none" stroke="#e8c27a" stroke-width="3" stroke-linecap="round"/>
    <path d="M136 140 C 140 156, 146 168, 150 178 M168 132 C 172 148, 172 162, 168 176" fill="none" stroke="#5e3c1c" stroke-opacity=".35" stroke-width="2.2" stroke-linecap="round"/>
    <rect x="158" y="150" width="14" height="13" rx="2" fill="#c99a62" stroke="#7a5030" stroke-width="1.2" stroke-dasharray="2.4 2"/>
    <ellipse cx="138" cy="132" rx="7" ry="12" fill="#fff" opacity=".22" filter="url(#${p}f)" transform="rotate(-15 138 132)"/>
  </g>
  <ellipse cx="84" cy="203" rx="15" ry="8" fill="#1e1a1a"/><ellipse cx="116" cy="203" rx="15" ry="8" fill="#1e1a1a"/>
  <rect x="78" y="176" width="20" height="26" rx="8" fill="url(#${p}r)"/><rect x="102" y="176" width="20" height="26" rx="8" fill="url(#${p}r)"/>
  <g class="n-arm-l" style="transform-origin: 66px 128px"><rect x="56" y="122" width="20" height="40" rx="10" fill="url(#${p}r)" transform="rotate(24 66 128)"/><circle cx="50" cy="164" r="10" fill="#1f6b43"/><rect x="44" y="152" width="20" height="7" rx="3.5" fill="url(#${p}w)" transform="rotate(24 54 155)"/></g>
  <g><rect x="124" y="122" width="20" height="40" rx="10" fill="url(#${p}r)" transform="rotate(-24 134 128)"/><circle cx="150" cy="164" r="10" fill="#1f6b43"/><rect x="136" y="152" width="20" height="7" rx="3.5" fill="url(#${p}w)" transform="rotate(-24 146 155)"/></g>
  <ellipse cx="100" cy="150" rx="45" ry="38" fill="url(#${p}r)"/>
  <rect x="94" y="116" width="12" height="70" rx="6" fill="url(#${p}w)"/>
  <rect x="56" y="178" width="88" height="11" rx="5.5" fill="url(#${p}w)"/>
  <rect x="56" y="148" width="88" height="12" rx="4" fill="#1e1a1a"/>
  <rect x="89" y="146" width="22" height="16" rx="3" fill="none" stroke="url(#${p}g)" stroke-width="3.4"/>
  <ellipse cx="80" cy="132" rx="14" ry="7" fill="#fff" opacity=".25" filter="url(#${p}f)" transform="rotate(-25 80 132)"/>
  <circle cx="100" cy="84" r="28" fill="url(#${p}k)"/>
  <path d="M70 86 C 64 114, 82 136, 100 136 C 118 136, 136 114, 130 86 C 122 99, 112 102, 100 102 C 88 102, 78 99, 70 86 Z" fill="url(#${p}w)"/>
  <g fill="url(#${p}w)"><circle cx="74" cy="108" r="8"/><circle cx="84" cy="124" r="9"/><circle cx="100" cy="130" r="9"/><circle cx="116" cy="124" r="9"/><circle cx="126" cy="108" r="8"/></g>
  <g class="only-work"><ellipse cx="100" cy="108" rx="6" ry="5" fill="#5a1a14"/></g>
  <ellipse cx="90" cy="100" rx="11" ry="5.5" fill="url(#${p}w)" transform="rotate(-10 90 100)"/><ellipse cx="110" cy="100" rx="11" ry="5.5" fill="url(#${p}w)" transform="rotate(10 110 100)"/>
  <circle cx="100" cy="94" r="5.5" fill="#e0605a"/><circle cx="98.5" cy="92.5" r="1.6" fill="#fff" opacity=".7"/>
  <g class="blink" style="transform-origin: 100px 82px">
    <ellipse cx="90" cy="83" rx="3.6" ry="4.6" fill="url(#${p}e)"/><ellipse cx="110" cy="83" rx="3.6" ry="4.6" fill="url(#${p}e)"/>
    <circle cx="89" cy="81.5" r="1.3" fill="#fff"/><circle cx="109" cy="81.5" r="1.3" fill="#fff"/>
  </g>
  <path d="M84 76 q6 -4 11 0 M105 76 q5 -4 11 0" fill="none" stroke="#fff" stroke-width="3.4" stroke-linecap="round"/>
  <ellipse cx="80" cy="92" rx="6" ry="3.5" fill="#ff8f8f" opacity=".6" filter="url(#${p}f)"/><ellipse cx="120" cy="92" rx="6" ry="3.5" fill="#ff8f8f" opacity=".6" filter="url(#${p}f)"/>
  <g transform="translate(100 54) scale(0.5) translate(-100 -60)">
    <path class="s-hat" d="M42 84 C 42 48, 72 24, 108 24 C 140 24, 164 40, 172 70 L 160 74 C 156 60, 146 52, 134 50 C 146 58, 154 70, 158 84 Z" fill="url(#${p}r)"/>
    <path d="M134 50 C 146 58, 154 70, 158 84" fill="none" stroke="#7d1420" stroke-opacity=".45" stroke-width="3" stroke-linecap="round"/>
    <g class="s-pom"><circle cx="167" cy="75" r="14" fill="url(#${p}w)"/></g>
    <g fill="url(#${p}w)">
      <rect x="38" y="72" width="124" height="22" rx="11"/>
      <circle cx="46" cy="83" r="11"/><circle cx="62" cy="80" r="11"/><circle cx="78" cy="79" r="11"/><circle cx="94" cy="78" r="11"/><circle cx="110" cy="78" r="11"/><circle cx="126" cy="79" r="11"/><circle cx="142" cy="80" r="11"/><circle cx="156" cy="83" r="11"/>
    </g>
  </g>
</svg>`,
    },
    {
      id: 'reindeer',
      name: 'Reindeer',
      pack: 'christmas',
      move: 'prance',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 220" aria-hidden="true">
  <defs>
    <radialGradient id="${p}k" cx=".38" cy=".3" r=".8"><stop offset="0" stop-color="#ffe3c8"/><stop offset=".55" stop-color="#f2b98c"/><stop offset="1" stop-color="#c98558"/></radialGradient>
    <radialGradient id="${p}r" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#ff6a5c"/><stop offset=".5" stop-color="#d8333a"/><stop offset="1" stop-color="#8e1a24"/></radialGradient>
    <radialGradient id="${p}w" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#d3dbe4"/></radialGradient>
    <radialGradient id="${p}e" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#3d2c24"/><stop offset="1" stop-color="#0f0a08"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
    <radialGradient id="${p}b" cx=".38" cy=".28" r=".85"><stop offset="0" stop-color="#d9a06a"/><stop offset=".55" stop-color="#a86c3c"/><stop offset="1" stop-color="#5c3618"/></radialGradient>
    <radialGradient id="${p}m" cx=".45" cy=".35" r=".75"><stop offset="0" stop-color="#fff0dc"/><stop offset="1" stop-color="#d9b48a"/></radialGradient>
    <linearGradient id="${p}a" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#e8c79a"/><stop offset="1" stop-color="#9c7448"/></linearGradient>
    <radialGradient id="${p}g" cx=".38" cy=".3" r=".8"><stop offset="0" stop-color="#fff2a0"/><stop offset=".55" stop-color="#f2b92a"/><stop offset="1" stop-color="#a8700c"/></radialGradient>
  </defs>
  <g stroke="url(#${p}a)" stroke-width="6" stroke-linecap="round" fill="none">
    <path d="M80 62 C 72 46, 66 34, 60 18 M70 40 L 54 34 M64 28 L 72 14"/>
    <path d="M120 62 C 128 46, 134 34, 140 18 M130 40 L 146 34 M136 28 L 128 14"/>
  </g>
  <g class="r-leg-b"><rect x="68" y="160" width="14" height="36" rx="7" fill="#7a4a24"/><rect x="118" y="160" width="14" height="36" rx="7" fill="#7a4a24"/></g>
  <g class="r-leg-a"><rect x="84" y="164" width="14" height="38" rx="7" fill="url(#${p}b)"/><rect x="102" y="164" width="14" height="38" rx="7" fill="url(#${p}b)"/>
    <rect x="84" y="196" width="14" height="8" rx="3" fill="#2e1c10"/><rect x="102" y="196" width="14" height="8" rx="3" fill="#2e1c10"/></g>
  <ellipse cx="100" cy="152" rx="36" ry="32" fill="url(#${p}b)"/>
  <ellipse cx="100" cy="162" rx="20" ry="18" fill="url(#${p}m)" opacity=".75"/>
  <g fill="#f0d2aa" opacity=".6"><circle cx="76" cy="140" r="3"/><circle cx="124" cy="142" r="3.4"/><circle cx="118" cy="132" r="2.4"/></g>
  <g class="r-ear-l" style="transform-origin: 72px 76px"><ellipse cx="60" cy="74" rx="16" ry="8" fill="url(#${p}b)" transform="rotate(-25 60 74)"/><ellipse cx="61" cy="74" rx="9" ry="4" fill="#f0a6a0" opacity=".7" transform="rotate(-25 61 74)"/></g>
  <g class="r-ear-r" style="transform-origin: 128px 76px"><ellipse cx="140" cy="74" rx="16" ry="8" fill="url(#${p}b)" transform="rotate(25 140 74)"/><ellipse cx="139" cy="74" rx="9" ry="4" fill="#f0a6a0" opacity=".7" transform="rotate(25 139 74)"/></g>
  <ellipse cx="100" cy="88" rx="32" ry="30" fill="url(#${p}b)"/>
  <path d="M94 60 L 98 70 L 102 60 L 106 70 L 110 62 L 108 74 L 92 74 Z" fill="#7a4a24"/>
  <ellipse cx="86" cy="68" rx="10" ry="5" fill="#fff" opacity=".3" filter="url(#${p}f)" transform="rotate(-20 86 68)"/>
  <ellipse cx="100" cy="104" rx="19" ry="14" fill="url(#${p}m)"/>
  <ellipse cx="100" cy="98" rx="7.5" ry="5.5" fill="#3a2416"/><ellipse cx="98" cy="96.5" rx="2.4" ry="1.4" fill="#fff" opacity=".6"/>
  <path d="M94 108 q6 5 12 0" fill="none" stroke="#5a3420" stroke-width="2.6" stroke-linecap="round"/>
  <g class="blink" style="transform-origin: 100px 86px">
    <ellipse cx="86" cy="86" rx="6" ry="7.5" fill="url(#${p}e)"/><ellipse cx="114" cy="86" rx="6" ry="7.5" fill="url(#${p}e)"/>
    <circle cx="84" cy="83" r="2.3" fill="#fff"/><circle cx="112" cy="83" r="2.3" fill="#fff"/>
  </g>
  <ellipse cx="74" cy="98" rx="6" ry="3.5" fill="#ff8f8f" opacity=".55" filter="url(#${p}f)"/><ellipse cx="126" cy="98" rx="6" ry="3.5" fill="#ff8f8f" opacity=".55" filter="url(#${p}f)"/>
  <path d="M70 120 C 86 128, 114 128, 130 120 L 130 128 C 114 136, 86 136, 70 128 Z" fill="url(#${p}r)"/>
  <g class="r-bell" style="transform-origin: 100px 128px">
    <path d="M100 128 v4" stroke="#a8700c" stroke-width="2"/>
    <circle cx="100" cy="139" r="8" fill="url(#${p}g)"/><path d="M95 141 h10" stroke="#7a5008" stroke-width="1.6" stroke-linecap="round"/><circle cx="100" cy="144" r="1.6" fill="#7a5008"/>
  </g>
</svg>`,
    },
    {
      id: 'gingerbread',
      name: 'Gingerbread',
      pack: 'christmas',
      move: 'jump',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 210" aria-hidden="true">
  <defs>
    <radialGradient id="${p}k" cx=".38" cy=".3" r=".8"><stop offset="0" stop-color="#ffe3c8"/><stop offset=".55" stop-color="#f2b98c"/><stop offset="1" stop-color="#c98558"/></radialGradient>
    <radialGradient id="${p}r" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#ff6a5c"/><stop offset=".5" stop-color="#d8333a"/><stop offset="1" stop-color="#8e1a24"/></radialGradient>
    <radialGradient id="${p}w" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#d3dbe4"/></radialGradient>
    <radialGradient id="${p}e" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#3d2c24"/><stop offset="1" stop-color="#0f0a08"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
    <radialGradient id="${p}c" cx=".38" cy=".28" r=".9"><stop offset="0" stop-color="#e8a868"/><stop offset=".5" stop-color="#bf7536"/><stop offset="1" stop-color="#7d4416"/></radialGradient>
  </defs>
  <g class="gb-leg-l" style="transform-origin: 88px 140px"><rect x="74" y="132" width="28" height="58" rx="14" fill="url(#${p}c)" transform="rotate(12 88 140)"/><path d="M77 177 l3.6 -4 l3.6 4 l3.6 -4 l3.6 4 l3.6 -4 l3.6 4" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" transform="rotate(12 88 140)"/></g>
  <g class="gb-leg-r" style="transform-origin: 112px 140px"><rect x="98" y="132" width="28" height="58" rx="14" fill="url(#${p}c)" transform="rotate(-12 112 140)"/><path d="M101 177 l3.6 -4 l3.6 4 l3.6 -4 l3.6 4 l3.6 -4 l3.6 4" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" transform="rotate(-12 112 140)"/></g>
  <g class="gb-arm-l" style="transform-origin: 78px 106px"><rect x="28" y="93" width="56" height="27" rx="13.5" fill="url(#${p}c)" transform="rotate(18 78 106)"/><path d="M40 98 l4 4 l-4 4 l4 4 l-4 4" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" transform="rotate(18 78 106)"/></g>
  <g class="gb-arm-r" style="transform-origin: 122px 106px"><rect x="116" y="93" width="56" height="27" rx="13.5" fill="url(#${p}c)" transform="rotate(-18 122 106)"/><path d="M160 98 l-4 4 l4 4 l-4 4 l4 4" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" transform="rotate(-18 122 106)"/></g>
  <rect x="70" y="86" width="60" height="66" rx="26" fill="url(#${p}c)"/>
  <circle cx="100" cy="60" r="32" fill="url(#${p}c)"/>
  <ellipse cx="86" cy="42" rx="12" ry="6" fill="#fff" opacity=".3" filter="url(#${p}f)" transform="rotate(-25 86 42)"/>
  <g fill="#6a3410" opacity=".35"><circle cx="78" cy="58" r="1.2"/><circle cx="122" cy="48" r="1.2"/><circle cx="84" cy="132" r="1.2"/><circle cx="118" cy="122" r="1.2"/><circle cx="112" cy="70" r="1"/></g>
  <path d="M90 92 L 100 98 L 90 104 Z M110 92 L 100 98 L 110 104 Z" fill="url(#${p}r)"/><circle cx="100" cy="98" r="3.4" fill="#b8202a"/>
  <circle cx="100" cy="114" r="5.5" fill="#e8434a"/><circle cx="100" cy="129" r="5.5" fill="#2fae6a"/><circle cx="100" cy="144" r="5.5" fill="#fff"/>
  <g fill="#fff" opacity=".6"><circle cx="98.4" cy="112.4" r="1.6"/><circle cx="98.4" cy="127.4" r="1.6"/></g>
  <g class="blink" style="transform-origin: 100px 56px">
    <ellipse cx="88" cy="56" rx="5.5" ry="6.5" fill="#3a1a08"/><ellipse cx="112" cy="56" rx="5.5" ry="6.5" fill="#3a1a08"/>
    <circle cx="86.3" cy="53.6" r="2" fill="#fff"/><circle cx="110.3" cy="53.6" r="2" fill="#fff"/>
  </g>
  <circle cx="78" cy="68" r="5" fill="#ff9aa8" opacity=".7"/><circle cx="122" cy="68" r="5" fill="#ff9aa8" opacity=".7"/>
  <path class="only-idle" d="M88 70 q12 10 24 0" fill="none" stroke="#fff" stroke-width="3.4" stroke-linecap="round"/>
  <g class="only-work"><path d="M88 69 h24 q0 13 -12 13 q-12 0 -12 -13z" fill="#4a1a0a" stroke="#fff" stroke-width="2.4" stroke-linejoin="round"/></g>
</svg>`,
    },
    {
      id: 'wizard',
      name: 'Wizard',
      pack: 'everyday',
      move: 'levitate',
      svg: (p) => `
<svg ROOT viewBox="0 0 200 220" aria-hidden="true">
  <defs>
    <radialGradient id="${p}m" cx=".38" cy=".25" r=".9"><stop offset="0" stop-color="#55556a"/><stop offset=".5" stop-color="#25252f"/><stop offset="1" stop-color="#0b0b10"/></radialGradient>
    <radialGradient id="${p}t" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#4a4a5c"/><stop offset=".55" stop-color="#1f1f28"/><stop offset="1" stop-color="#09090d"/></radialGradient>
    <linearGradient id="${p}l" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#c0303a"/><stop offset="1" stop-color="#6e1218"/></linearGradient>
    <radialGradient id="${p}k" cx=".38" cy=".3" r=".8"><stop offset="0" stop-color="#ffe3c8"/><stop offset=".55" stop-color="#f2b98c"/><stop offset="1" stop-color="#c98558"/></radialGradient>
    <linearGradient id="${p}d" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#a0683e"/><stop offset="1" stop-color="#4a2a12"/></linearGradient>
    <radialGradient id="${p}e" cx=".4" cy=".3" r=".8"><stop offset="0" stop-color="#3d2c24"/><stop offset="1" stop-color="#0f0a08"/></radialGradient>
    <radialGradient id="${p}g" cx=".5" cy=".5" r=".5"><stop offset="0" stop-color="#ffffff" stop-opacity="1"/><stop offset=".35" stop-color="#fff2b0" stop-opacity=".8"/><stop offset="1" stop-color="#ffcf3a" stop-opacity="0"/></radialGradient>
    <pattern id="${p}s" width="12" height="12" patternUnits="userSpaceOnUse"><rect width="12" height="12" fill="#8e1b22"/><rect width="6" height="12" fill="#e0ac32"/></pattern>
    <radialGradient id="${p}o" cx=".4" cy=".3" r=".85"><stop offset="0" stop-color="#d9b07a"/><stop offset=".6" stop-color="#9a6b3c"/><stop offset="1" stop-color="#5e3d1e"/></radialGradient>
    <filter id="${p}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
  </defs>
  <ellipse cx="86" cy="205" rx="12" ry="6" fill="#1a1a20"/><ellipse cx="114" cy="205" rx="12" ry="6" fill="#1a1a20"/>
  <path d="M100 112 C 80 112, 70 124, 66 150 L 56 198 C 56 208, 144 208, 144 198 L 134 150 C 130 124, 120 112, 100 112 Z" fill="url(#${p}m)"/>
  <path d="M92 124 L 100 120 L 108 124 L 114 200 C 106 203, 94 203, 86 200 Z" fill="url(#${p}l)"/>
  <path d="M93 126 L 88 200 M107 126 L 112 200" stroke="#0b0b10" stroke-opacity=".55" stroke-width="2"/>
  <ellipse cx="82" cy="134" rx="10" ry="5" fill="#fff" opacity=".18" filter="url(#${p}f)" transform="rotate(-30 82 134)"/>
  <g><path d="M76 124 C 62 130, 54 146, 52 162 L 68 165 C 70 150, 76 140, 82 134 Z" fill="url(#${p}t)"/><path d="M52 160 L 68 163" stroke="#8e1b22" stroke-width="3" stroke-linecap="round"/><circle cx="60" cy="167" r="8" fill="url(#${p}k)"/></g>
  <g class="m-arm" style="transform-origin: 124px 128px">
    <g class="only-work" fill="#fff6b0">${star(178, 112, 4, 'spark')}${star(188, 128, 3, 'spark s2')}${star(166, 102, 3, 'spark s3')}${star(190, 104, 2.5, 'spark s4')}</g>
    <circle class="m-lumos" cx="171" cy="126" r="16" fill="url(#${p}g)"/>
    <path d="M141 167 L 171 126" stroke="url(#${p}d)" stroke-width="4.5" stroke-linecap="round"/>
    <circle cx="146" cy="160" r="3.4" fill="#5a3518"/>
    <path d="M124 124 C 138 130, 146 146, 148 162 L 132 165 C 130 150, 124 140, 118 134 Z" fill="url(#${p}t)"/>
    <path d="M132 163 L 148 160" stroke="#8e1b22" stroke-width="3" stroke-linecap="round"/>
    <circle cx="141" cy="167" r="8" fill="url(#${p}k)"/>
  </g>
  <g class="m-scarf" style="transform-origin: 88px 122px">
    <path d="M80 120 L 96 122 L 92 160 L 77 157 Z" fill="url(#${p}s)" transform="rotate(4 88 140)"/>
    <path d="M78 158 v6 M83 159 v6 M88 160 v6" stroke="#8e1b22" stroke-width="2.5" stroke-linecap="round"/>
  </g>
  <path d="M70 116 C 86 126, 114 126, 130 116 L 130 128 C 114 138, 86 138, 70 128 Z" fill="url(#${p}s)"/>
  <ellipse cx="72" cy="94" rx="6" ry="11" fill="#2a1a12"/><ellipse cx="128" cy="94" rx="6" ry="11" fill="#2a1a12"/>
  <circle cx="100" cy="92" r="30" fill="url(#${p}k)"/>
  <ellipse cx="100" cy="74" rx="30" ry="5" fill="#0b0b10" opacity=".35" filter="url(#${p}f)"/>
  <g class="blink" style="transform-origin: 100px 96px">
    <ellipse cx="90" cy="96" rx="5.5" ry="7" fill="url(#${p}e)"/><ellipse cx="110" cy="96" rx="5.5" ry="7" fill="url(#${p}e)"/>
    <circle cx="88.2" cy="93" r="2.1" fill="#fff"/><circle cx="108.2" cy="93" r="2.1" fill="#fff"/>
  </g>
  <ellipse cx="80" cy="106" rx="6.5" ry="3.8" fill="#ff8fa8" opacity=".55" filter="url(#${p}f)"/>
  <ellipse cx="120" cy="106" rx="6.5" ry="3.8" fill="#ff8fa8" opacity=".55" filter="url(#${p}f)"/>
  <path class="only-idle" d="M95 108 q5 4.5 10 0" fill="none" stroke="#5a2a18" stroke-width="3" stroke-linecap="round"/>
  <g class="only-work"><path d="M94 107 h12 q0 8 -6 8 q-6 0 -6 -8z" fill="#4a1712"/></g>
  <ellipse cx="100" cy="70" rx="48" ry="10" fill="url(#${p}t)"/>
  <path d="M66 70 C 76 46, 86 26, 100 12 C 108 4, 122 2, 130 10 C 120 9, 113 16, 113 28 C 114 44, 126 58, 134 70 Z" fill="url(#${p}m)"/>
  <path d="M68 66 C 90 72, 110 72, 132 66 L 130 60 C 110 66, 90 66, 70 60 Z" fill="url(#${p}l)"/>
  <ellipse cx="86" cy="40" rx="5" ry="12" fill="#fff" opacity=".16" filter="url(#${p}f)" transform="rotate(25 86 40)"/>
  <g class="only-work"><g class="m-owl"><g class="m-owl-bob">
    <g class="m-wing-l" style="transform-origin: -7px -2px"><ellipse cx="-16" cy="0" rx="11" ry="6" fill="url(#${p}o)" transform="rotate(-15 -16 0)"/></g>
    <g class="m-wing-r" style="transform-origin: 7px -2px"><ellipse cx="16" cy="0" rx="11" ry="6" fill="url(#${p}o)" transform="rotate(15 16 0)"/></g>
    <ellipse cx="0" cy="0" rx="10" ry="12" fill="url(#${p}o)"/>
    <path d="M-7 -11 L -9 -16 L -3 -12 M7 -11 L 9 -16 L 3 -12" fill="#7a5028"/>
    <ellipse cx="0" cy="-3" rx="8" ry="6.5" fill="#f1dcb6"/>
    <circle cx="-3.6" cy="-3.5" r="3.2" fill="#ffb21f"/><circle cx="3.6" cy="-3.5" r="3.2" fill="#ffb21f"/>
    <circle cx="-3.6" cy="-3.5" r="1.7" fill="#111"/><circle cx="3.6" cy="-3.5" r="1.7" fill="#111"/>
    <path d="M-1.4 -0.5 L 1.4 -0.5 L 0 2.4 Z" fill="#e0861e"/>
    <path d="M-4 5 q2 2 4 0 q2 2 4 0" fill="none" stroke="#7a5028" stroke-width="1" opacity=".7"/>
  </g></g></g>
</svg>`,
    },
  ]

  const DEFAULT_PET = 'clauddy'

  // FNV-1a: a stable pick per year without storing anything
  function hash(text) {
    let h = 0x811c9dc5
    for (let i = 0; i < text.length; i++) {
      h ^= text.charCodeAt(i)
      h = Math.imul(h, 0x01000193)
    }
    return h >>> 0
  }

  function seasonalPet(date = new Date()) {
    const m = date.getMonth() + 1
    const d = date.getDate()
    const pack = PACKS.find((p) => p.days?.some(([pm, pd]) => pm === m && pd === d))
    if (!pack) return null
    const pool = PETS.filter((p) => p.pack === pack.id)
    return pool[hash(`${date.getFullYear()}:${pack.id}`) % pool.length].id
  }

  // what the widget shows: the season's pet on its days, else the saved choice
  function activePet(chosen, date = new Date()) {
    return seasonalPet(date) || (findPet(chosen) ? chosen : DEFAULT_PET)
  }

  function findPet(id) {
    return id === DEFAULT_PET
      ? { id, name: 'Clauddy', pack: 'everyday' }
      : PETS.find((p) => p.id === id) || null
  }

  // the pet's own <svg>, with `attrs` on its root (size, position, id)
  function petMarkup(id, prefix, attrs = '') {
    const pet = PETS.find((p) => p.id === id)
    return pet ? pet.svg(prefix).trim().replace('<svg ROOT ', `<svg ${attrs} `) : ''
  }

  const api = { PACKS, PETS, DEFAULT_PET, seasonalPet, activePet, findPet, petMarkup }
  if (typeof module === 'object' && module.exports) module.exports = api
  else globalThis.Pets = api
})()
