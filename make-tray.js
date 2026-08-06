// Generate the menu-bar (tray) icons from the same pixel pet sprite.
// A macOS "template" image is pure black + alpha; the system recolors it to
// match the menu bar (light/dark) — macOS/win32 only (Tray#setTemplateImage
// has no effect elsewhere). Linux tray icons get no such OS-level recoloring,
// so we ship a second, pre-colored variant (same terracotta as the app icon)
// that reads on both light and dark panels. The eyes are punched out so the
// face reads even at ~18px. Writes build/trayTemplate*.png (@1x/@2x) and
// build/trayColor*.png (@1x/@2x). No dependencies — a tiny PNG encoder, same
// as make-icon.js.
const zlib = require('node:zlib')
const fs = require('node:fs')
const path = require('node:path')

const CRC = (() => {
  const t = []
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    t[n] = c >>> 0
  }
  return t
})()
function crc32(buf) {
  let c = 0xffffffff
  for (let i = 0; i < buf.length; i++) c = CRC[(c ^ buf[i]) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}
function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length)
  const t = Buffer.from(type, 'ascii')
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])))
  return Buffer.concat([len, t, data, crc])
}
function png(W, H, rgba) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(W, 0)
  ihdr.writeUInt32BE(H, 4)
  ihdr[8] = 8
  ihdr[9] = 6
  const raw = Buffer.alloc((W * 4 + 1) * H)
  for (let y = 0; y < H; y++) {
    raw[y * (W * 4 + 1)] = 0
    rgba.copy(raw, y * (W * 4 + 1) + 1, y * W * 4, (y + 1) * W * 4)
  }
  const idat = zlib.deflateSync(raw, { level: 9 })
  return Buffer.concat([
    sig,
    chunk('IHDR', ihdr),
    chunk('IDAT', idat),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

// same pet sprite as make-icon.js: '#' body, 'o' eye (a hole), '.' empty
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
const COLS = SPRITE[0].length
const ROWS = SPRITE.length

// render the sprite at `cell` px per pixel, `col` where solid, transparent
// elsewhere (eyes included, so they punch through — recolored on macOS, or
// just the panel background showing through on Linux/Windows)
function render(cell, col) {
  const W = COLS * cell
  const H = ROWS * cell
  const rgba = Buffer.alloc(W * H * 4) // zero = transparent
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      if (SPRITE[r][c] !== '#') continue // '.' and 'o' stay transparent
      for (let dy = 0; dy < cell; dy++) {
        for (let dx = 0; dx < cell; dx++) {
          const i = ((r * cell + dy) * W + (c * cell + dx)) * 4
          rgba[i] = col[0]
          rgba[i + 1] = col[1]
          rgba[i + 2] = col[2]
          rgba[i + 3] = 255
        }
      }
    }
  }
  return png(W, H, rgba)
}

// pet terracotta — same CORAL as make-icon.js / --pixel in renderer/style.css
const BLACK = [0, 0, 0]
const CORAL = [0xd5, 0x76, 0x58]

const OUT = path.join(__dirname, 'build')
fs.mkdirSync(OUT, { recursive: true })
fs.writeFileSync(path.join(OUT, 'trayTemplate.png'), render(2, BLACK)) // 20x18
fs.writeFileSync(path.join(OUT, 'trayTemplate@2x.png'), render(4, BLACK)) // 40x36
fs.writeFileSync(path.join(OUT, 'trayColor.png'), render(2, CORAL)) // 20x18
fs.writeFileSync(path.join(OUT, 'trayColor@2x.png'), render(4, CORAL)) // 40x36
console.log('built build/trayTemplate.png + @2x, build/trayColor.png + @2x')
