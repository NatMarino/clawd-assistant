// Claw'd before he hatches. Until Claude has checked in for the first time
// he is an egg; once it has, the egg wobbles, and a click hatches him.
//
// PLACEHOLDER ART (Nat will draw the real one): a cream egg with Claw'd-
// orange spots, drawn on a 12-wide pixel grid where one pixel is 6 canvas
// units on both skins, so the egg is the same size whichever skin he wears.
// It stands on his usual ground line (canvas unit 140).
//
//   egg         resting, before Claude has connected
//   egg_wobble  ready: every so often it rocks, as if something's moving
//   hatch       crack, rock, crack, the top pops off (played once, then the
//               host hands over to his wave)
'use strict';

// P shell, p shell shade, # spots (his own orange), E crack lines
const EGG = [
  '.....PP.....',
  '...PPPPPP...',
  '..PPPP#PPP..',
  '.PPP###PPPp.',
  '.PPPP#PPPPp.',
  'PPPPPPPPPPpp',
  'P##PPPPPPPpp',
  'P###PPPP#Ppp',
  'PP#PPPP###pp',
  'PPPPPPPPP#pp',
  '.PPPPPPPPpp.',
  '.pPPPPPPppp.',
  '..pppppppp..',
  '....pppp....',
];

// draw on a copy: [row, col, char] triples
const paint = (base, marks) => {
  const g = base.map((r) => r.split(''));
  for (const [r, c, ch] of marks) if (g[r] && g[r][c] !== undefined) g[r][c] = ch;
  return g.map((r) => r.join(''));
};

// a zigzag crack across the middle, then a wider one
const CRACK1 = paint(EGG, [[6, 3, 'E'], [5, 4, 'E'], [6, 5, 'E'], [5, 6, 'E'], [6, 7, 'E']]);
const CRACK2 = paint(CRACK1, [[5, 1, 'E'], [6, 2, 'E'], [5, 8, 'E'], [6, 9, 'E'], [5, 10, 'E'], [4, 6, 'E']]);

// the top comes off: his head and eyes show in the jagged bottom half, and
// the top shell flies up and a little to the right
const RIM = 'PP.PPP.PPP.p';
const OPEN = [
  '............', '............', '............',
  '...######...',
  '..#E####E#..',
  '..#E####E#..',
  RIM,
  ...EGG.slice(7),
];
const TOP = EGG.slice(0, 6);
const POP = [...TOP.map((r) => '..' + r), ...OPEN.slice(3).map((r) => r + '..')];

// Each skin's clip: 6 units per pixel. The CLI grid's cell is 1.5 units and
// the app's 8, so res = cell / 6. Frames stand on the ground line: the CLI
// ground is 6 units below his feet (one empty row does it), the app's is 8
// (one empty row plus dy -2).
function clips(cell, dy) {
  const res = cell / 6;
  const pad = (f) => [...f, '.'.repeat(f[0].length)]; // the ground row
  const at = (frame, dx, ms) => ({ frame: pad(frame), dx: 44 - 32 + dx, dy, ms }); // centred: 72 units wide at 44
  const rock = (f, ms = 90) => [at(f, -6, ms), at(f, 0, ms), at(f, 6, ms), at(f, 0, ms)];
  return {
    egg: { palette: 'office', motion: 'none', overlay: null, res, stages: { intro: [], loop: [at(EGG, 0, 1000)], outro: [] } },
    egg_wobble: {
      palette: 'office', motion: 'none', overlay: null, res,
      stages: { intro: [], loop: [at(EGG, 0, 1600), ...rock(EGG), ...rock(EGG), at(EGG, 0, 900), ...rock(EGG, 70)], outro: [] },
    },
    hatch: {
      palette: 'office', motion: 'none', overlay: null, res,
      stages: {
        intro: [
          ...rock(EGG, 70), ...rock(EGG, 70),
          at(CRACK1, 0, 350), ...rock(CRACK1, 60),
          at(CRACK2, 0, 400), ...rock(CRACK2, 50), ...rock(CRACK2, 50),
          at(POP, 0, 250),
          at(OPEN, 0, 450),
        ],
        loop: [at(OPEN, 0, 1000)],
        outro: [],
      },
    },
  };
}

const EGG_CLI = clips(1.5, 0);
const EGG_APP = Object.fromEntries(Object.entries(clips(8, -2)).map(([k, c]) => [k, { ...c, palette: 'office_app' }]));
// the length of the hatch before the host hands over (the intro)
const HATCH_MS = EGG_CLI.hatch.stages.intro.reduce((t, s) => t + s.ms, 0);

export { EGG_CLI, EGG_APP, HATCH_MS };
