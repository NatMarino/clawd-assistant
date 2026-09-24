// Claw'd before he hatches. PLACEHOLDER ART until Nat draws the real one
// (as `egg`, `egg-crack-1..3` and `hatch` in her pack format).
//
// A real egg shape: narrow top, full rounded bottom, off-white with a soft
// shaded side and a few fine speckles, 12 x 15 pixels at 6 canvas units a
// pixel on both skins, standing on his usual ground line.
//
//   egg          resting: not set up yet
//   egg_rock     a gentle rock, no crack (checking he can reach the big brain)
//   egg_crack1   setup under way: a gentle rock now and then, first crack
//   egg_crack2   setup further on: rocks more, second crack
//   egg_ready    setup done: fully cracked, eager wobbling ("click me!")
//   hatch        the last crack, the top pops off, his head peeks out
'use strict';

// P shell, q highlight, p shade, g speckle, E crack
const EGG = [
  '....PPPP....',
  '...PqqPPP...',
  '..PqqPPPPp..',
  '..PqPPPPPp..',
  '.PPPPPgPPPp.',
  '.PPPPPPPPPp.',
  '.PPgPPPPPpp.',
  'PPPPPPPPPPpp',
  'PPPPPPPPgPpp',
  'PPPPPPPPPPpp',
  'PPPPgPPPPppp',
  '.PPPPPPPPpp.',
  '.pPPPPPPppp.',
  '..pppPPppp..',
  '....pppp....',
];

const paint = (base, marks) => {
  const g = base.map((r) => r.split(''));
  for (const [r, c, ch] of marks) if (g[r] && g[r][c] !== undefined) g[r][c] = ch;
  return g.map((r) => r.join(''));
};

// cracks around the widest part, growing stage by stage
const CRACK1 = paint(EGG, [[7, 4, 'E'], [6, 5, 'E'], [7, 6, 'E'], [8, 7, 'E']]);
const CRACK2 = paint(CRACK1, [[6, 2, 'E'], [7, 3, 'E'], [8, 8, 'E'], [7, 9, 'E'], [6, 6, 'E']]);
const CRACK3 = paint(CRACK2, [[7, 1, 'E'], [6, 10, 'E'], [8, 5, 'E'], [5, 5, 'E'], [7, 10, 'E']]);

// the top comes off: his head and eyes in the jagged bottom half, the top
// shell flying up and a little to the right
const RIM = 'PP.PPP.PPP.p';
const OPEN = [
  '............', '............', '............', '............',
  '...######...',
  '..#E####E#..',
  '..#E####E#..',
  RIM,
  ...EGG.slice(8),
];
const TOP = CRACK3.slice(0, 7);
const POP = [...TOP.map((r) => '..' + r), ...OPEN.slice(4).map((r) => r + '..')];

// 6 units a pixel on both skins (res = cell / 6); the CLI ground is 6 units
// under his feet (one empty row), the app's 8 (one row plus dy -2)
function clips(cell, dy, palette) {
  const res = cell / 6;
  const pad = (f) => [...f, '.'.repeat(f[0].length)];
  const at = (frame, dx, ms) => ({ frame: pad(frame), dx: 44 - 32 + dx, dy, ms });
  const rock = (f, ms, amp = 6) => [at(f, -amp, ms), at(f, 0, ms), at(f, amp, ms), at(f, 0, ms)];
  const clip = (loop, intro = []) => ({ palette, motion: 'none', overlay: null, res, stages: { intro, loop, outro: [] } });
  return {
    egg: clip([at(EGG, 0, 1000)]),
    egg_rock: clip([at(EGG, 0, 900), ...rock(EGG, 140, 6)]),
    egg_crack1: clip([at(CRACK1, 0, 2600), ...rock(CRACK1, 110)]),
    egg_crack2: clip([at(CRACK2, 0, 1400), ...rock(CRACK2, 90), ...rock(CRACK2, 90)]),
    egg_ready: clip([at(CRACK3, 0, 700), ...rock(CRACK3, 70), ...rock(CRACK3, 70), at(CRACK3, 0, 400), ...rock(CRACK3, 60)]),
    hatch: clip([at(OPEN, 0, 1000)], [
      ...rock(CRACK3, 60), ...rock(CRACK3, 50),
      at(POP, 0, 280),
      at(OPEN, 0, 500),
    ]),
  };
}

const EGG_CLI = clips(1.5, 0, 'office');
const EGG_APP = clips(8, -2, 'office_app');
const HATCH_MS = EGG_CLI.hatch.stages.intro.reduce((t, s) => t + s.ms, 0);

export { EGG_CLI, EGG_APP, HATCH_MS };
