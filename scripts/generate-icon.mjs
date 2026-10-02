// Generates the M0 engineering placeholder app icon: a rounded square in the
// Weave accent color with a white "weave" X mark. No image library required —
// the PNG encoder below is ~60 lines of deterministic Node.
//
// This is a placeholder, not the brand asset (see BRAND.md): the official
// brand mark, if any, replaces `assets/icon-source.png` and this script is
// rerun via `npm run generate:icon`.
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 1024;
const ACCENT = [0x3d, 0x6f, 0xfe, 0xff]; // matches ui/src/design tokens accent
const MARK = [0xff, 0xff, 0xff, 0xff];
const OUT = resolve(dirname(fileURLToPath(import.meta.url)), "../assets/icon-source.png");

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBytes = Buffer.from(type, "ascii");
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBytes, data])));
  return Buffer.concat([len, typeBytes, data, crc]);
}

function insideRoundedSquare(x, y, size, margin, radius) {
  const min = margin;
  const max = size - margin;
  if (x < min || x > max || y < min || y > max) return false;
  const cx = Math.max(min + radius, Math.min(x, max - radius));
  const cy = Math.max(min + radius, Math.min(y, max - radius));
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= radius * radius;
}

function insideWeaveMark(x, y, size) {
  const c = size / 2;
  const dx = x - c;
  const dy = y - c;
  const half = size * 0.115; // bar half-width
  const projA = (dx + dy) / Math.SQRT2;
  const projB = (dx - dy) / Math.SQRT2;
  const span = size * 0.3; // arm length from center
  const inArmA = Math.abs(projA) <= half && Math.abs(projB) <= span;
  const inArmB = Math.abs(projB) <= half && Math.abs(projA) <= span;
  return inArmA || inArmB;
}

const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  const rowStart = y * (SIZE * 4 + 1);
  raw[rowStart] = 0; // filter: none
  for (let x = 0; x < SIZE; x++) {
    const offset = rowStart + 1 + x * 4;
    const color = insideWeaveMark(x, y, SIZE)
      ? MARK
      : insideRoundedSquare(x, y, SIZE, 64, 176)
        ? ACCENT
        : [0, 0, 0, 0];
    raw[offset] = color[0];
    raw[offset + 1] = color[1];
    raw[offset + 2] = color[2];
    raw[offset + 3] = color[3];
  }
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type: RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, png);
console.log(`wrote ${OUT} (${png.length} bytes)`);
