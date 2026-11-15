#!/usr/bin/env python3
"""Build assets/icon.ico (the Windows exe's icon, 16 to 256 px) from
assets/icon.png, a 1024 x 1024 8-bit RGBA PNG. Standard library only.

    python3 native/scripts/ico.py
"""
import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "..", "assets", "icon.png")
OUT = os.path.join(HERE, "..", "assets", "icon.ico")
SIZES = [256, 128, 64, 48, 32, 24, 16]


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat = 8, b""
    while pos < len(data):
        (n,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + n]
        if kind == b"IHDR":
            w, h, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert (depth, color, interlace) == (8, 6, 0), "expects 8-bit RGBA, not interlaced"
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * 4
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b = prev[i]
            c = prev[i - 4] if i >= 4 else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line)
        prev = line
    return w, h, rows


def resize(w, h, rows, size):
    """Area average with premultiplied alpha, so transparent edges don't darken."""
    out = []
    for y in range(size):
        y0, y1 = y * h // size, (y + 1) * h // size
        line = bytearray()
        for x in range(size):
            x0, x1 = x * w // size, (x + 1) * w // size
            r = g = b = a = 0
            for yy in range(y0, y1):
                row = rows[yy]
                for xx in range(x0, x1):
                    i = xx * 4
                    al = row[i + 3]
                    r += row[i] * al
                    g += row[i + 1] * al
                    b += row[i + 2] * al
                    a += al
            n = (y1 - y0) * (x1 - x0)
            if a:
                line += bytes((round(r / a), round(g / a), round(b / a), round(a / n)))
            else:
                line += b"\0\0\0\0"
        out.append(line)
    return out


def write_png(size, rows):
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    raw = b"".join(b"\0" + bytes(r) for r in rows)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def main():
    w, h, rows = read_png(SRC)
    images = [write_png(s, resize(w, h, rows, s)) for s in SIZES]
    head = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = b""
    for s, png in zip(SIZES, images):
        dim = 0 if s >= 256 else s  # 0 means 256
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(png), offset)
        offset += len(png)
    with open(OUT, "wb") as f:
        f.write(head + entries + b"".join(images))
    print("built assets/icon.ico")


if __name__ == "__main__":
    main()
