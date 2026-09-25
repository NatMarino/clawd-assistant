// Claw'd's voice: he reads you what Claude found.
//
// Everything he says comes from an inbox item (state.rs Item): a VIP's
// message, a meeting about to start, a digest that just arrived. The host
// (index.html) decides WHEN to speak — on arrival, on a nudge, as a meeting
// comes due — and this module decides WHAT to say and makes the sound.
//
// BACKENDS. `speak()` routes to one of these, so the engine can change
// without anything else moving:
//   system — SpeechSynthesis (the Windows voices, via WebView2). No assets,
//            no install, works offline. The default.
//   espeak — eSpeak NG compiled to WebAssembly: the retro formant synth.
//            Slots in here; costs ~25MB embedded in the exe.
// A backend that fails to load falls back to `system` rather than going
// silent, so a broken engine is never a broken pet.
'use strict';

import { lines, traits } from './personality.js';

// --- what he says -----------------------------------------------------
//
// Greeting and body are SEPARATE, because a pet that opens every single
// sentence with "Hello friend" stops being charming by the third time. The
// greeting shows up sometimes (GREET_CHANCE), only on fresh news, and never
// the same one twice running.
// {n} is ", Nat" once he knows your name, and nothing before that.
// The lines themselves live in personality.js: they depend on who he is.
const GREET_CHANCE = 0.35;

// A nudge is a waiting item he already told you about, still sitting there.
// He talks as himself, a small coworker: "I need…", "I just finished…".
// These open a line; the item's own words follow.
const OPENERS = {
  get waiting() { return lines('waiting'); },
  get message() { return lines('message'); },
  get nudge() { return lines('nudge'); },
};

// what to call you; set once you've told him (index.html, the intro)
let userName = '';
function setName(name) { userName = String(name || '').trim(); }
const nameTail = () => (userName ? ', ' + userName : '');

// Items arrive in bursts (one sweep, several finds). He says one thing, then
// gives it room before the next; the host retries on its next tick.
const MIN_GAP_MS = 6000;

const MAX_CHARS = 300;

// --- settings ---------------------------------------------------------

// Preference order when nothing has been picked yet. Substring match on the
// voice name, so it survives the "Microsoft X - English (United States)"
// wrapping and works on a machine with a different set installed.
// The first four are Windows voices; the rest are the Mac's standard ones.
const PREFERRED = ['Mark', 'Guy', 'David', 'Zira', 'Samantha', 'Alex', 'Daniel', 'Karen'];

const DEFAULTS = {
  enabled: true,
  // 'words': he says his lines in the chosen voice. 'animalese': he chatters
  // instead (the words are on screen), like the villagers in Animal Crossing.
  style: 'words',
  backend: 'system',
  voiceName: '',   // '' = fall back to PREFERRED, then the browser default
  volume: 0.75,
  // animalese has its own pitch: the words voice's 1.4–2.0 barely moves a
  // chattering crab, so these are real registers (see ANIMALESE_PITCH)
  animalesePitch: 'normal',
  rate: 1,
  chirp: true,
  // Well above natural, chosen by ear: the Windows voices are newsreader-flat
  // and Mark at this pitch is the one that reads as a small colleague rather
  // than a narrator. Squarely in cartoon territory, which is the point.
  pitch: 1.8,
};

// pitch tops out at 2 because that is the Web Speech API's own maximum, not
// because anything here breaks first
const LIMITS = { pitch: [0.5, 2], rate: [0.6, 1.6], volume: [0, 1] };
const clamp = (v, [lo, hi]) => Math.min(hi, Math.max(lo, Math.round(v * 100) / 100));

let settings = { ...DEFAULTS };
try {
  const raw = localStorage.getItem('voice');
  if (raw) settings = { ...DEFAULTS, ...(JSON.parse(raw) || {}) };
  // the old default was a touch loud: move anyone still on it down
  if (!settings.v2) { if (settings.volume === 0.9) settings.volume = DEFAULTS.volume; settings.v2 = true; }
} catch {}

function save() {
  try { localStorage.setItem('voice', JSON.stringify(settings)); } catch {}
}

function get() { return { ...settings }; }
// While he's hidden, waiting for Claude to open: not a peep. Anything he
// wanted to say waits (canAnnounce is false) and comes out when he does.
let muted = false;
function setMuted(on) { muted = !!on; if (muted) stop(); }
function set(patch) {
  settings = { ...settings, ...patch };
  for (const k of Object.keys(LIMITS)) {
    if (typeof settings[k] === 'number') settings[k] = clamp(settings[k], LIMITS[k]);
  }
  save();
}
// nudge a numeric setting by a step and return the new value (for +/- buttons)
function nudge(key, step) {
  if (!LIMITS[key]) return settings[key];
  set({ [key]: (Number(settings[key]) || 0) + step });
  return settings[key];
}

// --- text shaping -----------------------------------------------------

// The detail is whatever Claude is asking, so it can carry file paths, tool
// names and stack traces. Read literally those turn into a minute of
// punctuation; this keeps the sentence and throws away the machinery.
function sanitise(text) {
  if (!text) return '';
  const basename = (m) => m.split(/[\\/]/).filter(Boolean).pop() || '';
  return String(text)
    // URLs go FIRST (and go entirely: the link is on screen, and "a link" read
    // out means nothing) — otherwise the path rules below eat "example.com/a/b"
    // out of the middle of one and leave the "http" behind
    .replace(/https?:\/\/\S+/g, ' ')
    // C:\Users\Queen Mapache\...\index.html -> index.html. An interior
    // segment may hold ONE space, because real home folders do ("Queen
    // Mapache"), but no more — otherwise a path followed by prose that
    // happens to contain a slash ("...\main.rs then run tests/unit/x.sh")
    // matches the prose as one giant segment and eats the sentence.
    .replace(/[A-Za-z]:[\\/](?:[^\\/\r\n\s]+(?: [^\\/\r\n\s]+)?[\\/])*[^\\/\r\n\s"',;]*/g, basename)
    // relative paths: src/tauri/main.rs -> main.rs
    .replace(/(?:[\w.-]+[\\/]){2,}[\w.-]+/g, basename)
    .replace(/[`*_~#|]+/g, ' ')        // markdown furniture
    .replace(/[-=]{3,}/g, ' ')          // rules
    .replace(/\s+/g, ' ')
    .trim();
}

function cap(text, n = MAX_CHARS) {
  if (text.length <= n) return text;
  const cut = text.slice(0, n);
  const stop = Math.max(cut.lastIndexOf('. '), cut.lastIndexOf(', '), cut.lastIndexOf(' '));
  return (stop > n * 0.6 ? cut.slice(0, stop) : cut).trim();
}

// pick, but never the same one twice running — repetition is what makes a
// canned line sound canned
const lastPicked = {};
function pick(arr, bucket) {
  if (arr.length < 2) return arr[0];
  let v;
  do { v = arr[Math.floor(Math.random() * arr.length)]; } while (bucket && v === lastPicked[bucket]);
  if (bucket) lastPicked[bucket] = v;
  return v;
}

// The sentence for one item. The writer (Claude) may supply `spoken`, which
// always wins for fresh news, because it was written for exactly this.
function itemText(item) {
  const spoken = sanitise(item.spoken);
  if (spoken) return spoken;
  const title = sanitise(item.title);
  const who = sanitise(item.who);
  if (who && title) return `${who}: ${title}`;
  return title || who || 'Something needs a look';
}

const endStop = (s) => (/[.!?]$/.test(s) ? s : s + '.');

// Compose what he says about `item`. `how` is why he is saying it now:
//   new    it just arrived
//   nudge  a waiting item that has sat past the nudge time
//   soon   a reminder coming due (`minutes` until it starts)
//   late   a reminder whose time has passed while it is still open
// `count` > 1 prefixes "3 new things." when a sweep brought several at once.
// Exported so the host can show it, and so it can be checked without sound.
function lineForItem(item, how = 'new', { minutes = 0, count = 1, forceGreeting = false } = {}) {
  const title = sanitise(item.title) || itemText(item);
  const text = endStop(cap(itemText(item)));
  let body;
  if (how === 'soon') body = minutes <= 1 ? `${title} is starting now${nameTail()}!` : `${title} starts in ${minutes} minutes${nameTail()}!`;
  else if (how === 'late') body = `${title} already started${nameTail()}! I think you might be late.`;
  else if (how === 'nudge') body = pick(OPENERS.nudge, 'nudge') + ' ' + text;
  else if (item.kind === 'delivery') body = sanitise(item.spoken) ? text : `I just finished your ${title}!`;
  else if (item.kind === 'waiting') body = pick(OPENERS.waiting, 'waiting') + ' ' + text;
  else if (item.kind === 'message') body = (Math.random() < 0.6 ? pick(OPENERS.message, 'message') + ' ' : '') + text;
  else body = text;
  body = endStop(cap(body));
  const lead = count > 1 ? `I’ve got ${count} new things for you. ` : '';
  const greet = (forceGreeting || (how === 'new' && Math.random() < GREET_CHANCE))
    ? pick(lines('greetings'), 'greet').replace('{n}', nameTail()) + ' ' : '';
  return greet + lead + body;
}

// --- backends ---------------------------------------------------------

// The host listens so the body can bob while he talks (pet.setSpeaking).
// A plain callback rather than events: there is exactly one listener.
let speakingListener = null;
let speakingOffTimer = null;
let speakingOnTimer = null;
// fn(on, kind) — kind is 'speech' or 'chirp', because only the chirp squints
function onSpeaking(fn) { speakingListener = fn; }
function setSpeaking(on, kind = 'speech') {
  clearTimeout(speakingOffTimer);
  clearTimeout(speakingOnTimer);
  if (speakingListener) speakingListener(!!on, kind);
}
// The bob starts a beat after his voice does and stops a beat before it
// ends: a body still bobbing after the last sound reads as a glitch, and one
// that starts early looks like it's lip-syncing to nothing.
const BOB_LATE_MS = 120;
const BOB_EARLY_MS = 150;
function bobFor(ms, kind = 'speech') {
  clearTimeout(speakingOffTimer);
  clearTimeout(speakingOnTimer);
  const off = Math.max(BOB_LATE_MS + 150, ms - BOB_EARLY_MS);
  speakingOnTimer = setTimeout(() => { if (speakingListener) speakingListener(true, kind); }, BOB_LATE_MS);
  speakingOffTimer = setTimeout(() => { if (speakingListener) speakingListener(false, kind); }, off);
}
// how long a line takes in the words voice (it has no progress to read):
// about 3 words a second at rate 1, plus the pauses punctuation makes
function speechMs(text) {
  const words = String(text).split(/\s+/).filter(Boolean).length;
  const pauses = (String(text).match(/[,.!?;:…]/g) || []).length;
  return (words * 320 + pauses * 200 + 200) / (Number(settings.rate) || 1);
}
// the words voice's own "still speaking" can stick; this bounds it
let wordsUntil = 0;
// the chirp has no onend of its own, so it books its own silence
function speakingFor(ms) {
  setSpeaking(true, 'chirp');
  speakingOffTimer = setTimeout(() => setSpeaking(false, 'chirp'), ms);
}

const systemBackend = {
  name: 'system',
  async ready() { return typeof window.speechSynthesis !== 'undefined'; },
  voices() {
    if (typeof window.speechSynthesis === 'undefined') return [];
    return speechSynthesis.getVoices().filter((v) => v.lang && v.lang.startsWith('en'));
  },
  // the chosen voice, else the first PREFERRED one installed, else whatever
  // the engine defaults to
  chosen() {
    const list = this.voices();
    if (!list.length) return null;
    const exact = list.find((x) => x.name === settings.voiceName);
    if (exact) return exact;
    for (const want of PREFERRED) {
      const hit = list.find((x) => x.name.includes(want));
      if (hit) return hit;
    }
    return null;
  },
  speak(text) {
    const u = new SpeechSynthesisUtterance(text);
    const v = this.chosen();
    if (v) u.voice = v;
    u.volume = settings.volume;
    u.rate = settings.rate;
    u.pitch = settings.pitch;
    // the body bobs while the mouth is going: from a beat after it starts
    // to a beat before our estimate of the end, or the real end if sooner
    u.onstart = () => {
      const est = speechMs(text);
      wordsUntil = Date.now() + est * 1.4;
      bobFor(est);
    };
    const done = () => { wordsUntil = 0; setSpeaking(false); };
    u.onend = done;
    u.onerror = done;
    speechSynthesis.cancel(); // an alert supersedes whatever was mid-sentence
    // speaking from now, even before the engine says it started
    wordsUntil = Date.now() + speechMs(text) * 1.4 + 1000;
    speechSynthesis.speak(u);
  },
  stop() { try { speechSynthesis.cancel(); } catch {} setSpeaking(false); },
};

// eSpeak NG (WebAssembly). Lazily loaded on first use, never at boot — alerts
// are rare and this must stay out of the startup path. Left unimplemented
// until the engine is vendored; `ready()` returning false makes speak() fall
// back to the system voice, so wiring it up early costs nothing.
const espeakBackend = {
  name: 'espeak',
  _mod: null,
  async ready() {
    if (this._mod) return true;
    try {
      const m = await import('./vendor/espeak/espeak.js');
      this._mod = await m.init();
      return true;
    } catch (err) {
      console.warn('espeak unavailable, using the system voice:', err && err.message);
      return false;
    }
  },
  speak(text) { this._mod.speak(text, settings); },
  stop() { if (this._mod && this._mod.stop) this._mod.stop(); },
};

const BACKENDS = { system: systemBackend, espeak: espeakBackend };

// --- chirp: the animalese burst ---------------------------------------
//
// A few short pitched blips, the Animal Crossing trick: no synthesizer, no
// samples, just an oscillator with a fast envelope. This is what he uses for
// things that happen CONSTANTLY — finishing a turn, taking a bite — where a
// spoken sentence would be unbearable but silence is a missed beat.
//
// Triangle rather than square: square alone reads as chiptune, triangle
// through a gentle lowpass sits closer to AC's warmth. ~5ms attack, ~70ms
// decay, ~55ms apart, a few cents of random detune so repeats aren't
// mechanical.
let audio = null;
function audioCtx() {
  if (!audio) {
    const C = window.AudioContext || window.webkitAudioContext;
    if (!C) return null;
    audio = new C();
  }
  if (audio.state === 'suspended') audio.resume().catch(() => {});
  return audio;
}

// Every sound he makes goes through one chain: a compressor as a soft
// limiter (so a pile of blips can never clip into crackle), then a gentle
// low-pass that takes the fizz off the top.
let bus = null;
function outBus(ctx) {
  if (bus && bus.context === ctx) return bus;
  const lim = ctx.createDynamicsCompressor();
  lim.threshold.value = -12;
  lim.knee.value = 10;
  lim.ratio.value = 12;
  lim.attack.value = 0.003;
  lim.release.value = 0.15;
  const lp = ctx.createBiquadFilter();
  lp.type = 'lowpass';
  lp.frequency.value = 4500;
  lp.Q.value = 0.5;
  lim.connect(lp);
  lp.connect(ctx.destination);
  bus = lim;
  return bus;
}

// ONE MOUTH. Whatever he's saying (animalese chatter, or a sample while you
// pick a voice), a new line fades the old one out in ~40 ms and stops its
// scheduled sounds, so he never talks over himself.
let mouth = null; // { gain, nodes: [] }
function hush() {
  if (mouth && audio) {
    const t = audio.currentTime;
    try {
      mouth.gain.gain.cancelScheduledValues(t);
      mouth.gain.gain.setValueAtTime(mouth.gain.gain.value, t);
      mouth.gain.gain.linearRampToValueAtTime(0, t + 0.04);
    } catch {}
    for (const n of mouth.nodes) { try { n.stop(t + 0.05); } catch {} }
  }
  mouth = null;
  chatterUntil = 0;
  setSpeaking(false);
}

// Chirp moods. Each is a family, not a fixed tune: every call picks one of
// its contours (pitch steps per blip, as fractions above the base) and
// wobbles the timing a little, so the same moment never sounds canned.
//   up     something finished well: a rising run
//   happy  the big one (celebrating): runs that rise and hop
//   light    a click, a pat: two quick soft high blips
//   flat     a small acknowledgement
//   muffled  from inside the egg: low, soft, through the shell (lowpass), and
//            rising at the end like a question
//   excited  from inside the egg, when he's ready to come out
const CHIRPS = {
  up:    { lift: 1.0,  gap: 0.055, decay: 0.075, level: 0.22, contours: [[0, 0.14, 0.28], [0, 0.1, 0.24, 0.3], [0, 0.18, 0.12, 0.3]] },
  happy: { lift: 1.05, gap: 0.05,  decay: 0.07,  level: 0.22, contours: [[0, 0.16, 0.32, 0.2, 0.4], [0, 0.24, 0.12, 0.36], [0.1, 0, 0.2, 0.3, 0.44]] },
  light: { lift: 1.4,  gap: 0.04,  decay: 0.04,  level: 0.07, contours: [[0, 0.18], [0, 0.22], [0.1, 0.26], [0, 0.14]] },
  talk: { lift: 1.0, gap: 0.065, decay: 0.06, level: 0.18, contours: [[0, 0.2, 0.08, 0.3, 0.14, 0.24], [0.1, 0, 0.26, 0.12, 0.34, 0.18], [0.05, 0.22, 0.1, 0.02, 0.28, 0.16]] },
  muffled: { lift: 0.8, gap: 0.07, decay: 0.09, level: 0.16, lp: 650, contours: [[0, 0.05, 0.3], [0.05, 0, 0.35], [0, 0.1, 0.05, 0.4]] },
  excited: { lift: 0.9, gap: 0.05, decay: 0.07, level: 0.18, lp: 700, contours: [[0, 0.2, 0.1, 0.35, 0.25, 0.45], [0.1, 0.3, 0.2, 0.4, 0.5]] },
  flat:  { lift: 1.0,  gap: 0.055, decay: 0.075, level: 0.22, contours: [[0, 0.07], [0.07, 0], [0, 0.07, 0]] },
};

// `count` trims (or repeats) the chosen contour; omit it for the contour's own
// length. Returns false when sounds are off.
function chirp(count, shape = 'up') {
  if (muted || !settings.enabled || (!settings.chirp && shape !== 'talk')) return false;
  const ctx = audioCtx();
  if (!ctx) return false;
  const mood = CHIRPS[shape] || CHIRPS.up;
  const contour = mood.contours[Math.floor(Math.random() * mood.contours.length)];
  const n = count || contour.length;
  // follows the voice pitch, so the chirp belongs to the same character
  const base = 300 * Math.max(0.6, Number(settings.pitch) || 1) * mood.lift;
  const peak = Math.max(0, Math.min(1, Number(settings.volume))) * mood.level;
  let t = ctx.currentTime + 0.002; // no lead-in: a click should sound on the click
  for (let i = 0; i < n; i++) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    const lp = ctx.createBiquadFilter();
    lp.type = 'lowpass';
    lp.frequency.value = mood.lp || 2600;
    osc.type = 'triangle';
    const detune = 1 + (Math.random() - 0.5) * 0.04;
    osc.frequency.value = base * (1 + contour[i % contour.length]) * detune;
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.linearRampToValueAtTime(peak, t + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + mood.decay);
    osc.connect(lp); lp.connect(gain); gain.connect(outBus(ctx));
    osc.start(t);
    osc.stop(t + mood.decay + 0.015);
    t += mood.gap * (0.9 + Math.random() * 0.2);
  }
  speakingFor(Math.round(n * mood.gap * 1000) + 120); // bob along with the blips
  return true;
}

// Chatter from inside the egg while his bubble says something: a run of
// muffled blips about as long as the line, so it reads as him mumbling
// through the shell. (The speech engine can't be muffled, and words would
// give away that he's not out yet.)
function babble(text, mood = 'muffled') {
  return animalese(text, { muffled: mood === 'muffled' });
}
let chatterUntil = 0;

// --- animalese: his sentence, letter by letter ------------------------
//
// Every letter gets its own tiny sound, fast, the way the villagers in Animal
// Crossing talk. Vowels are little sung vowels (a sawtooth through two
// band-pass "mouth" filters at that vowel's formants, so a/e/i/o/u each sound
// different); consonants are what the mouth does for them: s/f/h/sh hiss,
// p/t/k click, b/d/g click with voice, m/n hum, l/r/w/y glide. Spaces and
// punctuation are pauses, and a question lifts at the end. `muffled` runs
// it all through the eggshell (a low-pass) for before he hatches.
const VOWELS = {
  a: [800, 1200], e: [500, 1900], i: [320, 2300], o: [500, 900], u: [350, 800], y: [320, 2100],
};
const SCHWA = [500, 1500];
const HISS = { s: [6000, 3], z: [5500, 3], c: [5000, 2.5], f: [4000, 1.2], v: [3800, 1.2], h: [1500, 0.7], x: [5000, 2], j: [2800, 2], q: [2500, 2] };
const CLICK = { p: [1500, false], t: [3500, false], k: [2500, false], b: [1200, true], d: [2800, true], g: [2200, true] };
const HUM = { m: [250, 1100], n: [250, 1600] };
const GLIDE = { l: [400, 1200], r: [450, 1300], w: [300, 700] };

// the three animalese registers: base pitch (Hz) and how far the "mouth"
// (formants) moves with it. Super deep is a big rumbly crab; high is
// brighter, not squeaky.
const ANIMALESE_PITCH = {
  deep: { f0: 70, formants: 0.75 },
  normal: { f0: 230, formants: 1 },
  high: { f0: 380, formants: 1.1 },
};

function animalese(text, { muffled = false } = {}) {
  if (muted || !settings.enabled) return false;
  const ctx = audioCtx();
  if (!ctx) return false;
  const clean = String(text || '').toLowerCase().replace(/[^a-z0-9 .,!?'’-]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 140);
  if (!clean) return false;
  hush(); // one mouth: a new line replaces whatever he was saying
  if (typeof speechSynthesis !== 'undefined') { try { speechSynthesis.cancel(); } catch {} }
  const flav = muffled ? { jump: 1, even: false, slow: 1, range: 1 } : traits().voice;
  const reg = ANIMALESE_PITCH[settings.animalesePitch] || ANIMALESE_PITCH.normal;
  // animalese sits ~15% under the chirps
  const vol = Math.max(0, Math.min(1, Number(settings.volume))) * (muffled ? 0.6 : 0.42);
  const f0Base = reg.f0 * (muffled ? 0.85 : 1);
  const fScale = reg.formants;
  // in the egg every letter is a struggle through the shell
  const stretch = (muffled ? 1.2 : 1) * (flav.slow || 1);
  const out = ctx.createGain();
  out.gain.value = vol;
  const nodes = [];
  mouth = { gain: out, nodes };
  if (muffled) {
    // talking into a pillow: a low, slightly resonant low-pass
    const shell = ctx.createBiquadFilter();
    shell.type = 'lowpass';
    shell.frequency.value = 380;
    shell.Q.value = 4;
    out.connect(shell);
    shell.connect(outBus(ctx));
  } else {
    out.connect(outBus(ctx));
  }
  if (!noiseBuf) {
    noiseBuf = ctx.createBuffer(1, Math.round(ctx.sampleRate * 0.2), ctx.sampleRate);
    const d = noiseBuf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
  }

  const t0 = ctx.currentTime + 0.01;
  let t = t0;
  const letters = [...clean];
  const question = /\?\s*$/.test(clean);
  const bang = /!\s*$/.test(clean);
  const n = letters.length;
  const ATTACK = 0.011;
  const RELEASE = 0.012;

  // sometimes, from the egg, a little "mmf" thump before the words
  if (muffled && Math.random() < 0.4) {
    const o = ctx.createOscillator();
    o.type = 'sine';
    o.frequency.setValueAtTime(140, t);
    o.frequency.exponentialRampToValueAtTime(70, t + 0.1);
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.linearRampToValueAtTime(0.6, t + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, t + 0.12);
    o.connect(g); g.connect(out);
    o.start(t); o.stop(t + 0.14);
    nodes.push(o);
    t += 0.14;
  }

  // one sung vowel-ish sound: a sawtooth/triangle blend at f0 through two
  // formant filters, with a soft attack and release so letters meet cleanly
  // instead of clicking into each other
  const voiced = (at, dur, f0, [f1, f2], level = 1) => {
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, at);
    g.gain.linearRampToValueAtTime(0.5 * level, at + ATTACK);
    g.gain.setValueAtTime(0.5 * level, at + Math.max(ATTACK, dur - RELEASE));
    g.gain.linearRampToValueAtTime(0.0001, at + dur + RELEASE);
    for (const [type, mix] of [['sawtooth', 0.65], ['triangle', 0.35]]) {
      const o = ctx.createOscillator();
      o.type = type;
      o.frequency.setValueAtTime(f0, at);
      o.frequency.linearRampToValueAtTime(f0 * 0.97, at + dur);
      const m = ctx.createGain();
      m.gain.value = mix;
      o.connect(m);
      for (const [fr, q, amt] of [[f1 * fScale, 5, 1], [f2 * fScale, 8, 0.6]]) {
        const bp = ctx.createBiquadFilter();
        bp.type = 'bandpass';
        bp.frequency.value = fr;
        bp.Q.value = q;
        const a = ctx.createGain();
        a.gain.value = amt * 1.3;
        m.connect(bp); bp.connect(a); a.connect(g);
      }
      o.start(at);
      o.stop(at + dur + RELEASE + 0.02);
      nodes.push(o);
    }
    g.connect(out);
  };
  const noise = (at, dur, freq, q, level = 1) => {
    const src = ctx.createBufferSource();
    src.buffer = noiseBuf;
    const bp = ctx.createBiquadFilter();
    bp.type = 'bandpass';
    bp.frequency.value = freq;
    bp.Q.value = q;
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, at);
    g.gain.linearRampToValueAtTime(0.3 * level, at + 0.004);
    g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
    src.connect(bp); bp.connect(g); g.connect(out);
    src.start(at, Math.random() * 0.1);
    src.stop(at + dur + 0.01);
    nodes.push(src);
  };

  letters.forEach((ch, i) => {
    // the pitch contour: each letter a little different (seeded by the
    // letter, so the same word sounds the same), falling gently through the
    // sentence, rising over the last few letters of a question. His
    // temperament shapes it: how far letters jump, how even, how wide.
    const seed = flav.even ? 0 : ((ch.charCodeAt(0) * 37 + i * 11) % 23) / 23 - 0.5;
    const fallBy = flav.falling ? 0.25 : 0.12;
    const fall = 1 - fallBy * (i / Math.max(1, n));
    const tail = i > n - 5 ? i - (n - 5) : 0;
    let lift = 1;
    if (question && tail) lift = 1 + 0.08 * tail * (flav.range || 1);
    else if (bang && flav.bang && tail) lift = 1 + 0.1 * tail;
    else if (flav.upturn && tail) lift = 1 + 0.05 * tail;
    // from the egg: a slow wobble, as if the shell were rocking
    const wob = muffled ? 1 + 0.05 * Math.sin(2 * Math.PI * 6 * (t - t0)) : 1;
    const f0 = f0Base * (1 + seed * 0.14 * (flav.jump || 1)) * fall * lift * wob;
    const step = (d) => { t += d * stretch; };
    if (ch === ' ') { step(0.035); return; }
    if (ch === ',' || ch === '-') { step(0.12); return; }
    if (ch === '.' || ch === '!' || ch === '?') { step(0.2); return; }
    if (ch === "'" || ch === '’') return;
    if (/[0-9]/.test(ch)) { voiced(t, 0.06 * stretch, f0, SCHWA); step(0.066); return; }
    if (VOWELS[ch]) { voiced(t, 0.066 * stretch, f0, VOWELS[ch]); step(0.068); return; }
    // Consonants are pitched blips too, never noise: noise bursts under the
    // vowels were the "crunch". A hiss is a short, bright, slightly higher
    // blip; a click a very short one with a quick drop in pitch.
    if (HISS[ch]) { voiced(t, 0.038 * stretch, f0 * 1.12, [450, Math.min(3200, HISS[ch][0] * 0.5)], 0.4); step(0.045); return; }
    if (CLICK[ch]) {
      voiced(t, 0.03 * stretch, f0 * (CLICK[ch][1] ? 0.95 : 1.05), [550, Math.min(2600, CLICK[ch][0])], CLICK[ch][1] ? 0.65 : 0.5);
      step(0.042);
      return;
    }
    if (HUM[ch]) { voiced(t, 0.048 * stretch, f0, HUM[ch], 0.7); step(0.05); return; }
    if (GLIDE[ch]) { voiced(t, 0.048 * stretch, f0, GLIDE[ch], 0.8); step(0.05); return; }
    voiced(t, 0.045 * stretch, f0, SCHWA, 0.6);
    step(0.047);
  });

  // the sound itself ends at t (plus the last letter's release)
  const ms = (t - t0) * 1000 + 30;
  // 'still talking' ends a beat early too, so the talking pose lets go
  // just before the last sound, not after it
  chatterUntil = Date.now() + ms - BOB_EARLY_MS;
  bobFor(ms);
  return true;
}

// Create and wake the audio engine now (from a first pointerdown), so the
// first click chirp isn't the one that pays for starting it up.
function warmAudio() { audioCtx(); }

// The egg cracking: `size` quick, dry, high snaps (a sliver of filtered
// noise) each with a tiny downward chip on top, in his chirp register so it
// sounds like the same little creature. 1 for a hairline, 3 for the big one.
function crack(size = 1) {
  if (!settings.enabled || !settings.chirp) return false;
  const ctx = audioCtx();
  if (!ctx) return false;
  const vol = Math.max(0, Math.min(1, Number(settings.volume)));
  const base = 300 * Math.max(0.6, Number(settings.pitch) || 1);
  if (!noiseBuf) {
    noiseBuf = ctx.createBuffer(1, Math.round(ctx.sampleRate * 0.2), ctx.sampleRate);
    const d = noiseBuf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
  }
  const snaps = Math.max(1, Math.min(4, size + (size > 1 ? 1 : 0)));
  let t = ctx.currentTime + 0.002;
  for (let i = 0; i < snaps; i++) {
    const wob = 0.85 + Math.random() * 0.3;
    // the snap
    const n = ctx.createBufferSource();
    n.buffer = noiseBuf;
    const bp = ctx.createBiquadFilter();
    bp.type = 'bandpass';
    bp.frequency.value = 3200 * wob;
    bp.Q.value = 1.6;
    const ng = ctx.createGain();
    ng.gain.setValueAtTime(0.0001, t);
    ng.gain.linearRampToValueAtTime(vol * (0.22 + 0.06 * size), t + 0.002);
    ng.gain.exponentialRampToValueAtTime(0.0001, t + 0.03);
    n.connect(bp); bp.connect(ng); ng.connect(outBus(ctx));
    n.start(t, Math.random() * 0.1);
    n.stop(t + 0.05);
    // the chip: a high blip falling fast
    const o = ctx.createOscillator();
    o.type = 'triangle';
    o.frequency.setValueAtTime(base * 2.4 * wob, t);
    o.frequency.exponentialRampToValueAtTime(base * 1.3 * wob, t + 0.05);
    const og = ctx.createGain();
    og.gain.setValueAtTime(0.0001, t);
    og.gain.linearRampToValueAtTime(vol * 0.12, t + 0.003);
    og.gain.exponentialRampToValueAtTime(0.0001, t + 0.06);
    o.connect(og); og.connect(outBus(ctx));
    o.start(t);
    o.stop(t + 0.07);
    t += 0.045 + Math.random() * 0.035; // crk-crk, never evenly spaced
  }
  return true;
}

// One "monch": a soft crunch (filtered noise) over a little low thump that
// drops in pitch, like a mouthful. Timed by the host to each bite frame.
let noiseBuf = null;
function munch() {
  if (!settings.enabled || !settings.chirp) return false;
  const ctx = audioCtx();
  if (!ctx) return false;
  const vol = Math.max(0, Math.min(1, Number(settings.volume)));
  const t = ctx.currentTime + 0.005;
  const wob = 0.9 + Math.random() * 0.2; // no two bites alike

  if (!noiseBuf) {
    noiseBuf = ctx.createBuffer(1, Math.round(ctx.sampleRate * 0.2), ctx.sampleRate);
    const d = noiseBuf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
  }
  const noise = ctx.createBufferSource();
  noise.buffer = noiseBuf;
  const bp = ctx.createBiquadFilter();
  bp.type = 'bandpass';
  bp.frequency.value = 900 * wob;
  bp.Q.value = 0.8;
  const ng = ctx.createGain();
  ng.gain.setValueAtTime(0.0001, t);
  ng.gain.linearRampToValueAtTime(vol * 0.28, t + 0.008);
  ng.gain.exponentialRampToValueAtTime(0.0001, t + 0.09);
  noise.connect(bp); bp.connect(ng); ng.connect(outBus(ctx));
  noise.start(t);
  noise.stop(t + 0.12);

  const osc = ctx.createOscillator();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(190 * wob, t);
  osc.frequency.exponentialRampToValueAtTime(85 * wob, t + 0.11);
  const og = ctx.createGain();
  og.gain.setValueAtTime(0.0001, t);
  og.gain.linearRampToValueAtTime(vol * 0.35, t + 0.012);
  og.gain.exponentialRampToValueAtTime(0.0001, t + 0.13);
  osc.connect(og); og.connect(outBus(ctx));
  osc.start(t);
  osc.stop(t + 0.15);
  return true;
}

let active = null;

async function backend() {
  const want = BACKENDS[settings.backend] || systemBackend;
  if (active === want) return active;
  active = (await want.ready()) ? want : systemBackend;
  return active;
}

// Warm the engine without making a sound — call when the user switches the
// toggle on, so the first real alert isn't the one that pays the load cost.
async function warm() { if (settings.enabled) await backend(); }

// --- speaking ---------------------------------------------------------

let lastSpokeAt = 0;

function stop() { hush(); Object.values(BACKENDS).forEach((b) => { try { b.stop(); } catch {} }); }

// Say something now, ignoring the gap: his answers to a click, and his
// first words. Silent when his voice is off, unless `force` (the settings
// Test and voice-pick buttons, which are about hearing him).
async function say(text, { force = false } = {}) {
  if (!text || muted || (!settings.enabled && !force)) return false;
  // animalese by choice, or because this web view has no speech voices at
  // all (it can happen on a Mac): he still talks, just in chatter
  if (settings.style === 'animalese' || typeof window.speechSynthesis === 'undefined') {
    lastSpokeAt = Date.now();
    return animalese(sanitise(text));
  }
  lastSpokeAt = Date.now();
  hush(); // one mouth: no chatter under the words
  const b = await backend();
  try { b.speak(text); return true; } catch (err) { console.warn('speak failed', err); return false; }
}

// The news path. Returns the spoken string, `false` when it is too soon
// after the last line (the caller keeps it and tries again), or null when
// the voice is off (the caller should consider it handled).
async function announce(text) {
  if (!settings.enabled) return null;
  if (muted) return false;
  if (Date.now() - lastSpokeAt < MIN_GAP_MS) return false;
  if (isSpeaking()) return false;
  await say(text);
  return text;
}

// Whether announce() would speak right now, so the host can start an
// animation that leads into the line (the letter arriving) without then
// being told "too soon".
function canAnnounce() {
  if (muted || !settings.enabled) return false;
  if (Date.now() - lastSpokeAt < MIN_GAP_MS) return false;
  return !isSpeaking();
}
function isSpeaking() {
  return Date.now() < chatterUntil
    || (typeof speechSynthesis !== 'undefined' && speechSynthesis.speaking && Date.now() < wordsUntil - BOB_EARLY_MS);
}

export {
  announce, canAnnounce, isSpeaking, setName, setMuted, babble, warmAudio, crack, animalese, chirp, munch, say, warm, stop, get, set, nudge, onSpeaking,
  lineForItem, sanitise, systemBackend, DEFAULTS, LIMITS, ANIMALESE_PITCH,
};
