'use strict';
// Generate a 1024x1024 icon PNG (pixel pet on a dark card), no dependencies.
const zlib = require('zlib');
const fs = require('fs');

const CRC = (() => {
  const t = [];
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}
function png(W, H, rgba) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(W, 0);
  ihdr.writeUInt32BE(H, 4);
  ihdr[8] = 8; ihdr[9] = 6;
  const raw = Buffer.alloc((W * 4 + 1) * H);
  for (let y = 0; y < H; y++) {
    raw[y * (W * 4 + 1)] = 0;
    rgba.copy(raw, y * (W * 4 + 1) + 1, y * W * 4, (y + 1) * W * 4);
  }
  const idat = zlib.deflateSync(raw, { level: 9 });
  return Buffer.concat([sig, chunk('IHDR', ihdr), chunk('IDAT', idat), chunk('IEND', Buffer.alloc(0))]);
}

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
];

const S = 1024;
const buf = Buffer.alloc(S * S * 4);
const set = (x, y, r, g, b, a) => {
  const i = (y * S + x) * 4;
  buf[i] = r; buf[i + 1] = g; buf[i + 2] = b; buf[i + 3] = a;
};

const m = Math.round(S * 0.085);
const x0 = m, y0 = m, x1 = S - m, y1 = S - m;
const rad = Math.round((x1 - x0) * 0.235);
function insideRR(x, y) {
  if (x < x0 || x >= x1 || y < y0 || y >= y1) return false;
  let cx = null, cy = null;
  if (x < x0 + rad && y < y0 + rad) { cx = x0 + rad; cy = y0 + rad; }
  else if (x >= x1 - rad && y < y0 + rad) { cx = x1 - rad; cy = y0 + rad; }
  else if (x < x0 + rad && y >= y1 - rad) { cx = x0 + rad; cy = y1 - rad; }
  else if (x >= x1 - rad && y >= y1 - rad) { cx = x1 - rad; cy = y1 - rad; }
  if (cx !== null) { const dx = x - cx, dy = y - cy; return dx * dx + dy * dy <= rad * rad; }
  return true;
}
function bg(x, y) {
  const t = (y - y0) / (y1 - y0);
  let r = 0x3a + (0x14 - 0x3a) * t;
  let g = 0x2a + (0x0d - 0x2a) * t;
  let b = 0x20 + (0x0a - 0x20) * t;
  const gx = (x0 + x1) / 2, gy = y0 + (y1 - y0) * 0.16;
  const dx = (x - gx) / ((x1 - x0) * 0.62), dy = (y - gy) / ((y1 - y0) * 0.5);
  let gl = Math.max(0, 1 - Math.sqrt(dx * dx + dy * dy)) * 0.6;
  r += (0x8f - r) * gl; g += (0x56 - g) * gl; b += (0x36 - b) * gl;
  return [r | 0, g | 0, b | 0];
}

for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    if (insideRR(x, y)) { const [r, g, b] = bg(x, y); set(x, y, r, g, b, 255); }
    else set(x, y, 0, 0, 0, 0);
  }
}

const cell = Math.round((x1 - x0) * 0.62 / 10);
const cw = cell * 10, ch = cell * SPRITE.length;
const ox = Math.round((S - cw) / 2);
const oy = Math.round((S - ch) / 2 + S * 0.01);
const CORAL = [0xbd, 0x78, 0x53], EYE = [0x22, 0x1b, 0x16];
function fillCell(c, r, col) {
  for (let yy = 0; yy < cell; yy++)
    for (let xx = 0; xx < cell; xx++) set(ox + c * cell + xx, oy + r * cell + yy, col[0], col[1], col[2], 255);
}
SPRITE.forEach((row, r) => {
  for (let c = 0; c < row.length; c++) {
    const ch2 = row[c];
    if (ch2 === '.') continue;
    fillCell(c, r, CORAL);
    if (ch2 === 'o') fillCell(c, r, EYE);
  }
});

fs.writeFileSync('/tmp/cum-icon-1024.png', png(S, S, buf));
console.log('ok /tmp/cum-icon-1024.png');
