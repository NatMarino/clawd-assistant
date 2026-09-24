// Turns Nat's animation pack into engine clips, for both skins:
//
//   node tools/import-sheets.js "<pack>/clawd-pet-sprites"
//
// (the folder holding cli/ and app/, each with manifest.json and sheets/)
// and writes src/sprites-office.js. Re-run it whenever the pack changes; the
// output is generated, so edit the art (or tools/letter-small.js), not that.
//
// A sheet is a horizontal strip of frames, one every FRAME units along x,
// drawn as plain <rect>s. Each skin's art grid lines up exactly with that
// skin's own grid, which is why the frames keep the art's resolution:
//
//   cli  24x24 art, 1 art px = 4 CLI cells (head 12 px = 48 cells), so the
//        clips carry res 0.25 and each pixel draws as a 6-unit square. Art
//        x=4 (the left arm nub) is CLI column 0; y=22 (the empty row under
//        the feet) is the same ground headroom every CLI frame has.
//   app  16x16 art with half-pixel details (thin happy eyes), rasterised at
//        2x: 1 art px = 2 half-cells, i.e. the app skin's own res-2 staged
//        clips, 4 units per cell. Art x=2 (the left arm) is the app grid's
//        column 0; y=14 (under the feet) is its ground row.
//
// Either way the clips stand exactly where his existing ones do.
'use strict';

const fs = require('fs');
const path = require('path');
const { smallLetter } = require('./letter-small');

const root = process.argv[2];
if (!root) {
  console.error('usage: node tools/import-sheets.js <pack>/clawd-pet-sprites');
  process.exit(1);
}

// colour -> palette char. '#', '%' and 'E' are the engine's body, shade and
// eyes (so the talk squash and the chirp squint work on this art too); the
// body and shade take each skin's own tones, so switching between old and
// new clips never flickers.
const CHARS = {
  '#D97757': '#', // body
  '#B25A42': '%', // body shade (as in the typing clip)
  '#1A1A1C': 'E', // eyes
  '#D6CCBC': 'p', // paper, shade
  '#FBF8F1': 'P', // paper, light
  '#F3EEE4': 'q', // highlight
  '#8E877C': 'g', // ink lines
  '#C4913A': 'o', // gold, dark (watch)
  '#F2C66D': 'O', // gold, light
  '#9C6533': 'b', // box, dark
  '#C98B4E': 'B', // box
  '#EBD3AE': 't', // tape
  '#D8452C': 'r', // wax seal, confetti
};

const SKINS = {
  cli: { frame: 24, scale: 1, artX0: 4, unitsPerCell: 6, feetRow: 22, res: 0.25, palette: 'office', name: 'OFFICE_CLI' },
  app: { frame: 16, scale: 2, artX0: 2, unitsPerCell: 4, feetRow: 14, res: 2, palette: 'office_app', name: 'OFFICE_APP' },
};

// engine name <- pack animation(s). `intro` plays once, `loop` repeats.
const CLIPS = {
  reading: { intro: 'letter-arrive', loop: 'letter-read' },
  clock: { loop: 'pocket-watch' },
  package: { loop: 'parcel' },
  happy_wiggle: { loop: 'happy-wiggle' },
  celebrate: { loop: 'celebrate' },
  peek: { loop: 'peek', fromBottomEdge: true },
};

function readFrames(file, count, skin) {
  const N = skin.frame * skin.scale;
  const svg = fs.readFileSync(file, 'utf8').replace(/<metadata>[\s\S]*?<\/metadata>/, '');
  const frames = Array.from({ length: count }, () => Array.from({ length: N }, () => Array(N).fill('.')));
  const num = '(-?\\d+(?:\\.\\d+)?)';
  const re = new RegExp(`<rect x="${num}" y="${num}" width="${num}" height="${num}" fill="(#[0-9A-Fa-f]{6})"`, 'g');
  const grid = (v) => {
    const g = v * skin.scale;
    if (Math.abs(g - Math.round(g)) > 1e-6) throw new Error(`${path.basename(file)}: ${v} is finer than the ${skin.scale}x grid`);
    return Math.round(g);
  };
  let m;
  let rects = 0;
  while ((m = re.exec(svg))) {
    rects++;
    const [x, y, w, h] = m.slice(1, 5).map(Number).map(grid);
    const ch = CHARS[m[5].toUpperCase()];
    if (!ch) throw new Error(`${path.basename(file)}: unmapped colour ${m[5]}`);
    const f = Math.floor(x / N);
    if (f < 0 || f >= count) continue;
    // later rects paint over earlier ones, as in the SVG
    for (let r = y; r < y + h; r++) {
      for (let c = x; c < x + w; c++) {
        const lc = c - f * N;
        if (r >= 0 && r < N && lc >= 0 && lc < N) frames[f][r][lc] = ch;
      }
    }
  }
  // every rect has to have been understood: a format change must not
  // silently drop pixels
  const all = (svg.match(/<rect\b/g) || []).length;
  if (rects !== all) throw new Error(`${path.basename(file)}: read ${rects} of ${all} rects`);
  return frames;
}

// the tightest box holding every frame of a clip, with the bottom kept at
// the ground line (or the frame's bottom edge, for art that rises from it)
function crop(allFrames, bottomRow) {
  let x0 = Infinity, x1 = -1, y0 = Infinity;
  for (const f of allFrames) {
    f.forEach((row, r) => row.forEach((ch, c) => {
      if (ch === '.') return;
      x0 = Math.min(x0, c); x1 = Math.max(x1, c); y0 = Math.min(y0, r);
    }));
  }
  return { x0, x1, y0, y1: bottomRow };
}

function cut(frame, b) {
  const out = [];
  for (let r = b.y0; r <= b.y1; r++) out.push((frame[r] || []).slice(b.x0, b.x1 + 1).join('').padEnd(b.x1 - b.x0 + 1, '.'));
  return out;
}

const out = [];
out.push('// GENERATED by tools/import-sheets.js from the Clawd character animation');
out.push('// series. Do not edit by hand: change the art and re-run it.');
out.push("'use strict';");
out.push('');
const exported = [];
for (const [skinName, skin] of Object.entries(SKINS)) {
  const dir = path.join(root, skinName);
  const manifest = JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf8'));
  const names = [];
  for (const [name, spec] of Object.entries(CLIPS)) {
    const parts = ['intro', 'loop'].filter((k) => spec[k]);
    if (!parts.every((k) => manifest.animations[spec[k]])) {
      console.warn(`${skinName}: no ${parts.map((k) => spec[k]).join(' + ')} in this pack; ${name} keeps its placeholder`);
      continue;
    }
    const loaded = {};
    for (const k of parts) {
      const a = manifest.animations[spec[k]];
      loaded[k] = { frames: readFrames(path.join(dir, a.sheet), a.frames, skin), ms: Math.round(1000 / (a.fps || manifest.fps)) };
    }
    const bottom = spec.fromBottomEdge ? skin.frame * skin.scale - 1 : (skin.feetRow + 1) * skin.scale - 1;
    const box = crop(parts.flatMap((k) => loaded[k].frames), bottom);
    const dx = (box.x0 - skin.artX0 * skin.scale) * skin.unitsPerCell;
    // identical frames share one table entry
    const table = [];
    const index = new Map();
    const ref = (f) => {
      const key = cut(f, box).join('\n');
      if (!index.has(key)) { index.set(key, table.length); table.push(cut(f, box)); }
      return index.get(key);
    };
    const steps = {};
    for (const k of parts) steps[k] = loaded[k].frames.map(ref);
    // Nat's edit on top of the pack: a smaller letter (tools/letter-small.js)
    if (name === 'reading') table.splice(0, table.length, ...smallLetter(table, skinName));
    const id = `${skinName.toUpperCase()}_${name.toUpperCase()}`;
    out.push(`const ${id}_F = ${JSON.stringify(table).replace(/\],\[/g, '],\n  [')};`);
    const stage = (k) => (steps[k] ? `${id}_STEPS(${JSON.stringify(steps[k])}, ${loaded[k].ms})` : '[]');
    out.push(`const ${id}_STEPS = (ix, ms) => ix.map((i) => ({ frame: ${id}_F[i], dx: ${dx}, dy: 0, ms }));`);
    out.push(`const ${id} = { palette: '${skin.palette}', motion: 'none', overlay: null, res: ${skin.res}, stages: { intro: ${stage('intro')}, loop: ${stage('loop')}, outro: [] } };`);
    out.push('');
    names.push(`${name}: ${id}`);
  }
  out.push(`const ${skin.name} = { ${names.join(', ')} };`);
  out.push('');
  exported.push(skin.name);
  console.log(`${skinName}: ${names.map((n) => n.split(':')[0]).join(', ')}`);
}
out.push(`export { ${exported.join(', ')} };`);
out.push('');

const dest = path.join(__dirname, '..', 'src', 'sprites-office.js');
fs.writeFileSync(dest, out.join('\n'));
console.log(`wrote ${path.relative(process.cwd(), dest)}`);
