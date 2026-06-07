// Generates all platform icon formats from branding/icon.svg.
// Run once locally:  node branding/generate-icons.mjs
// Requires (npm install --no-save): sharp, png-to-ico.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { Buffer } from 'node:buffer';
import sharp from 'sharp';
import pngToIco from 'png-to-ico';

// Assemble a PNG-based .icns container (works on any OS, no sips/iconutil).
function buildIcns(entries) {
  // entries: [{ type: 'ic08', png: Buffer }]
  const chunks = [];
  for (const { type, png } of entries) {
    const header = Buffer.alloc(8);
    header.write(type, 0, 'ascii');
    header.writeUInt32BE(png.length + 8, 4);
    chunks.push(header, png);
  }
  const body = Buffer.concat(chunks);
  const head = Buffer.alloc(8);
  head.write('icns', 0, 'ascii');
  head.writeUInt32BE(body.length + 8, 4);
  return Buffer.concat([head, body]);
}

const dir = dirname(fileURLToPath(import.meta.url));
const svg = readFileSync(join(dir, 'icon.svg'));
const out = (n) => join(dir, n);
mkdirSync(dir, { recursive: true });

const sizes = [16, 32, 48, 64, 128, 256, 512, 1024];
const pngBufs = {};
for (const s of sizes) {
  const buf = await sharp(svg, { density: 384 }).resize(s, s).png().toBuffer();
  pngBufs[s] = buf;
  writeFileSync(out(`icon-${s}.png`), buf);
}
// Canonical PNG for Linux .desktop / docs.
writeFileSync(out('icon.png'), pngBufs[512]);

// Windows multi-resolution .ico.
const ico = await pngToIco([16, 32, 48, 64, 128, 256].map((s) => out(`icon-${s}.png`)));
writeFileSync(out('icon.ico'), ico);

// macOS .icns (PNG-based entries; map sizes to ICNS OSTypes).
const icnsBuf = buildIcns([
  { type: 'icp4', png: pngBufs[16] },
  { type: 'icp5', png: pngBufs[32] },
  { type: 'icp6', png: pngBufs[64] },
  { type: 'ic07', png: pngBufs[128] },
  { type: 'ic08', png: pngBufs[256] },
  { type: 'ic09', png: pngBufs[512] },
  { type: 'ic10', png: pngBufs[1024] },
]);
writeFileSync(out('icon.icns'), icnsBuf);

console.log('Icons generated:', sizes.length, 'PNGs + icon.ico + icon.icns');
