// A smaller letter for the `reading` clip (Nat: "reduce the size of the
// letter"). The pack draws the envelope and the letter as wide as his whole
// body (12 px), which hides everything below his eyes. This keeps the pack's
// choreography and timing (drop in, bounce off his head, slide down, open,
// pull the letter out, read) with an 8x5 envelope and an 8x4 letter, so his
// arms, body and feet stay visible around them.
//
// Frames are the importer's cropped 16x21 frames (art x 4..19, y 2..22), by
// their index in the clip's frame table. Each redraw names the fingerprint of
// the frame it replaces: if the pack changes, the import stops and says so
// instead of pasting old poses over new art.
'use strict';

const HEAD = '..############..';
const BAND = '################';
const BODY = '..############..';
const LEG = '...#.#....#.#...';
const EYES = '..##E######E##..';

// the letter, held: his arm nubs on the top two rows, and under them the
// darker tone the typing clip shades his arms with ('%'), two rows fading
// out, so the arms read as in front of him, holding it
const P0 = '.###PPPPPPPP###.';
const P1 = '.###PggggggP###.';
const P2 = '..%%PggggPPP%%..';
const P3 = '..%#pppppppp#%..';
// the same shadow under his arms when it is the envelope he is holding
const shade = (inner, fade) => (fade ? '..%#' + inner + '#%..' : '..%%' + inner + '%%..');

// the envelope, 8x5: flap lines meeting at the wax seal
const ENV = ['pqqqqqqp', 'qpqqqqpq', 'qqprrpqq', 'qqqrrqqq', 'pppppppp'];

const W = 16, H = 21;
function frame(rows) {
  return Array.from({ length: H }, (_, r) => {
    const row = rows[r] || '.'.repeat(W);
    if (row.length !== W) throw new Error(`letter-small: row ${r} is ${row.length} wide`);
    return row;
  });
}

const REDRAWN = {
  // lands on his head
  12: ['189691a5', { 8: '....pqqqqqqp....', 9: '....qpqqqqpq....', 10: '..##qqprrpqq##..', 11: '..##qqqrrqqq##..',
    12: '..##pppppppp##..', 13: EYES, 14: BAND, 15: BAND, 16: BODY, 17: BODY, 18: LEG, 19: LEG }],
  // sliding down in front of him
  13: ['782a8425', { 10: '..##pqqqqqqp##..', 11: '..##qpqqqqpq##..', 12: '..##qqprrpqq##..', 13: '..##qqqrrqqq##..',
    14: '####pppppppp####', 15: BAND, 16: BODY, 17: BODY, 18: LEG, 19: LEG }],
  14: ['535128e5', { 10: HEAD, 11: HEAD, 12: '..##pqqqqqqp##..', 13: '..##qpqqqqpq##..', 14: '####qqprrpqq####',
    15: '####qqqrrqqq####', 16: shade('pppppppp'), 17: BODY, 18: LEG, 19: LEG }],
  15: ['79332f25', { 10: HEAD, 11: HEAD, 12: HEAD, 13: '..##pqqqqqqp##..', 14: '####qpqqqqpq####',
    15: '####qqprrpqq####', 16: shade('qqqrrqqq'), 17: shade('pppppppp', true), 18: LEG, 19: LEG }],
  // catches it low, crouching
  16: ['c4d61905', { 11: HEAD, 12: HEAD, 13: HEAD, 14: EYES, 15: '.###pqqqqqqp###.', 16: '.###qpqqqqpq###.',
    17: shade('qqprrpqq'), 18: shade('qqqrrqqq', true), 19: '..##pppppppp##..' }],
  // holds it up to look
  17: ['2cc09385', { 10: HEAD, 11: HEAD, 12: HEAD, 13: EYES, 14: '.###pqqqqqqp###.', 15: '.###qpqqqqpq###.',
    16: shade('qqprrpqq'), 17: shade('qqqrrqqq', true), 18: '..##pppppppp##..', 19: LEG }],
  // seal broken, flap open
  18: ['a01acfe5', { 10: HEAD, 11: HEAD, 12: HEAD, 13: '..##E##pp##E##..', 14: '.###pppppppp###.',
    15: '.###qqqqqqqq###.', 16: shade('qqqqqqqq'), 17: shade('qqqqqqqq', true), 18: '..##pppppppp##..', 19: LEG }],
  // the letter rises out, his eyes peeking either side of it
  19: ['c983f125', { 10: HEAD, 11: HEAD, 12: '..##EPPPPPPE##..', 13: '..##EPggggPE##..', 14: '.###pppppppp###.',
    15: '.###qqqqqqqq###.', 16: shade('qqqqqqqq'), 17: shade('qqqqqqqq', true), 18: '..##pppppppp##..', 19: LEG }],
  // pulled out, up high
  20: ['291751a5', { 10: HEAD, 11: P0, 12: P1, 13: P2, 14: P3, 15: '..##qppppppq##..', 16: '..##qqqqqqqq##..',
    17: '..##qqqqqqqq##..', 18: '..##pppppppp##..', 19: LEG }],
  // envelope dropped
  21: ['7b34e025', { 10: HEAD, 11: P0, 12: P1, 13: P2, 14: '####pppppppp####', 15: BAND, 16: BODY, 17: BODY,
    18: LEG, 19: LEG }],
  // lowers it to read, envelope at his feet
  22: ['7102845', { 10: HEAD, 11: HEAD, 12: EYES, 13: P0, 14: P1, 15: P2, 16: P3, 17: BODY, 18: LEG, 19: LEG,
    20: '....qppppppq....' }],
  23: ['b0b07da5', { 10: HEAD, 11: HEAD, 12: EYES, 13: EYES, 14: P0, 15: P1, 16: P2, 17: P3, 18: LEG, 19: LEG }],
  // reading: eyes down, down-left, down-right, a bob, a blink, a look up
  24: ['63f2d065', { 10: HEAD, 11: HEAD, 12: HEAD, 13: EYES, 14: P0, 15: P1, 16: P2, 17: P3, 18: LEG, 19: LEG }],
  25: ['95071925', { 10: HEAD, 11: HEAD, 12: HEAD, 13: '..#E######E###..', 14: P0, 15: P1, 16: P2, 17: P3, 18: LEG, 19: LEG }],
  26: ['412cf025', { 10: HEAD, 11: HEAD, 12: HEAD, 13: '..###E######E#..', 14: P0, 15: P1, 16: P2, 17: P3, 18: LEG, 19: LEG }],
  27: ['bf0bb0a5', { 11: HEAD, 12: HEAD, 13: HEAD, 14: '..#E######E###..', 15: P0, 16: P1, 17: P2, 18: P3, 19: LEG }],
  28: ['3532e065', { 10: HEAD, 11: HEAD, 12: HEAD, 13: HEAD, 14: P0, 15: P1, 16: P2, 17: P3, 18: LEG, 19: LEG }],
  29: ['dc77c865', { 10: HEAD, 11: HEAD, 12: EYES, 13: EYES, 14: P0, 15: P1, 16: P2, 17: P3, 18: LEG, 19: LEG }],
};

const LETTER = /[pPqgr]/;

function fingerprint(rows) {
  let x = 5381;
  for (const c of rows.join('\n')) x = ((x * 33) ^ c.charCodeAt(0)) >>> 0;
  return x.toString(16);
}

// Frames where the envelope is still falling or bouncing, in open air above
// him: swap the 12-wide envelope for the small one, bottom edge and centre
// unchanged. Refuses a frame where the letter touches him (those need a
// hand redraw).
function shrinkFlying(rows) {
  let x0 = W, x1 = -1, y0 = H, y1 = -1;
  rows.forEach((row, r) => [...row].forEach((ch, c) => {
    if (LETTER.test(ch)) { x0 = Math.min(x0, c); x1 = Math.max(x1, c); y0 = Math.min(y0, r); y1 = Math.max(y1, r); }
  }));
  if (x1 < 0) return rows; // no letter in this frame
  for (let r = y0; r <= y1; r++) {
    if (/[#E]/.test(rows[r].slice(x0, x1 + 1))) throw new Error('letter-small: a falling frame overlaps him; it needs a redraw');
  }
  const out = rows.map((row, r) => (r >= y0 && r <= y1 ? row.replace(/[pPqgr]/g, '.') : row));
  const cx = Math.round((x0 + x1 + 1) / 2);
  const top = y1 - ENV.length + 1;
  ENV.forEach((line, i) => {
    const r = top + i;
    if (r < 0) return;
    const row = out[r].split('');
    for (let c = 0; c < line.length; c++) row[cx - line.length / 2 + c] = line[c];
    out[r] = row.join('');
  });
  return out;
}

// the importer's hook: the reading clip's frame table in, a smaller letter out
function smallLetter(table, skin = 'cli') {
  if (skin !== 'cli') return table;
  if (table.length !== 30) throw new Error(`letter-small: expected 30 reading frames, the pack has ${table.length}`);
  return table.map((rows, i) => {
    const redraw = REDRAWN[i];
    if (redraw) {
      const [want, spec] = redraw;
      if (fingerprint(rows) !== want) throw new Error(`letter-small: reading frame ${i} changed in the pack; update tools/letter-small.js`);
      return frame(spec);
    }
    return shrinkFlying(rows);
  });
}

module.exports = { smallLetter };
