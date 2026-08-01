// Pack a set of square PNGs into a Windows .ico (PNG-embedded entries, which
// Windows Vista+ reads natively). Usage: node make-ico.js <pngDir> <out.ico>
// where <pngDir> holds `<size>.png` files for the sizes listed below.
const fs = require('node:fs')
const path = require('node:path')

const SIZES = [16, 32, 48, 64, 128, 256]

const dir = process.argv[2]
const out = process.argv[3]
if (!dir || !out) {
  console.error('usage: node make-ico.js <pngDir> <out.ico>')
  process.exit(1)
}

const images = SIZES.map((s) => ({ size: s, data: fs.readFileSync(path.join(dir, `${s}.png`)) }))

const HEADER = 6
const ENTRY = 16
let offset = HEADER + ENTRY * images.length

const dirEntries = []
const blobs = []
for (const { size, data } of images) {
  const e = Buffer.alloc(ENTRY)
  e[0] = size >= 256 ? 0 : size // width  (0 means 256)
  e[1] = size >= 256 ? 0 : size // height (0 means 256)
  e[2] = 0 // color palette
  e[3] = 0 // reserved
  e.writeUInt16LE(1, 4) // color planes
  e.writeUInt16LE(32, 6) // bits per pixel
  e.writeUInt32LE(data.length, 8) // size of the PNG blob
  e.writeUInt32LE(offset, 12) // offset of the blob
  offset += data.length
  dirEntries.push(e)
  blobs.push(data)
}

const header = Buffer.alloc(HEADER)
header.writeUInt16LE(0, 0) // reserved
header.writeUInt16LE(1, 2) // type: 1 = icon
header.writeUInt16LE(images.length, 4) // image count

fs.writeFileSync(out, Buffer.concat([header, ...dirEntries, ...blobs]))
console.log(`built ${out} (${images.length} sizes: ${SIZES.join(', ')})`)
