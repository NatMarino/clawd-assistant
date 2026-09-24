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

// --- what he says -----------------------------------------------------
//
// Greeting and body are SEPARATE, because a pet that opens every single
// sentence with "Hello friend" stops being charming by the third time. The
// greeting shows up sometimes (GREET_CHANCE), only on fresh news, and never
// the same one twice running.
const GREETINGS = [
  'Hello friend.', 'Hey.', 'Hi there.', 'Psst.', 'Oh!', 'Hello.',
  'Excuse me.', 'Hey friend.', 'Ahem.',
];
const GREET_CHANCE = 0.35;

// A nudge is a waiting item he already told you about, still sitting there.
const NUDGE_OPENERS = ['Still waiting:', 'Just checking:', 'Psst, still there:', 'Friendly nudge:'];

// Items arrive in bursts (one sweep, several finds). He says one thing, then
// gives it room before the next; the host retries on its next tick.
const MIN_GAP_MS = 6000;

const MAX_CHARS = 300;

// --- settings ---------------------------------------------------------

// Preference order when nothing has been picked yet. Substring match on the
// voice name, so it survives the "Microsoft X - English (United States)"
// wrapping and works on a machine with a different set installed.
const PREFERRED = ['Mark', 'Guy', 'David', 'Zira'];

const DEFAULTS = {
  enabled: true,
  backend: 'system',
  voiceName: '',   // '' = fall back to PREFERRED, then the browser default
  volume: 0.9,
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
} catch {}

function save() {
  try { localStorage.setItem('voice', JSON.stringify(settings)); } catch {}
}

function get() { return { ...settings }; }
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
  let body;
  if (how === 'soon') body = minutes <= 1 ? `${title} is starting now` : `${title} starts in ${minutes} minutes`;
  else if (how === 'late') body = `${title} has started. You might be late!`;
  else if (how === 'nudge') body = pick(NUDGE_OPENERS, 'nudge') + ' ' + itemText(item);
  else if (item.kind === 'delivery' && !sanitise(item.spoken)) body = `Your ${title} is ready`;
  else body = itemText(item);
  body = endStop(cap(body));
  const lead = count > 1 ? `${count} new things. ` : '';
  const greet = (forceGreeting || (how === 'new' && Math.random() < GREET_CHANCE))
    ? pick(GREETINGS, 'greet') + ' ' : '';
  return greet + lead + body;
}

// --- backends ---------------------------------------------------------

// The host listens so the body can bob while he talks (pet.setSpeaking).
// A plain callback rather than events: there is exactly one listener.
let speakingListener = null;
let speakingOffTimer = null;
// fn(on, kind) — kind is 'speech' or 'chirp', because only the chirp squints
function onSpeaking(fn) { speakingListener = fn; }
function setSpeaking(on, kind = 'speech') {
  clearTimeout(speakingOffTimer);
  if (speakingListener) speakingListener(!!on, kind);
}
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
    // the body bobs for exactly as long as the mouth is going
    u.onstart = () => setSpeaking(true);
    u.onend = () => setSpeaking(false);
    u.onerror = () => setSpeaking(false);
    speechSynthesis.cancel(); // an alert supersedes whatever was mid-sentence
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

// `shape` nudges the melody: 'up' for something finished well, 'flat' for a
// small acknowledgement like a bite.
function chirp(count = 3, shape = 'up') {
  if (!settings.enabled || !settings.chirp) return false;
  const ctx = audioCtx();
  if (!ctx) return false;
  // follows the voice pitch, so the chirp belongs to the same character
  const base = 300 * Math.max(0.6, Number(settings.pitch) || 1);
  let t = ctx.currentTime + 0.01;
  for (let i = 0; i < count; i++) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    const lp = ctx.createBiquadFilter();
    lp.type = 'lowpass';
    lp.frequency.value = 2600;
    osc.type = 'triangle';
    const step = shape === 'up' ? i * 0.14 : (i % 2) * 0.07;
    const detune = 1 + (Math.random() - 0.5) * 0.04;
    osc.frequency.value = base * (1 + step) * detune;
    const peak = Math.max(0, Math.min(1, Number(settings.volume))) * 0.22;
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.linearRampToValueAtTime(peak, t + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.075);
    osc.connect(lp); lp.connect(gain); gain.connect(ctx.destination);
    osc.start(t);
    osc.stop(t + 0.09);
    t += 0.055;
  }
  speakingFor(count * 55 + 120); // bob along with the blips
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

function stop() { Object.values(BACKENDS).forEach((b) => { try { b.stop(); } catch {} }); }

// Say something now, ignoring the gap. Used by the settings Test button and
// the mode switch, which are answers to a click.
async function say(text) {
  if (!text) return false;
  lastSpokeAt = Date.now();
  const b = await backend();
  try { b.speak(text); return true; } catch (err) { console.warn('speak failed', err); return false; }
}

// The news path. Returns the spoken string, `false` when it is too soon
// after the last line (the caller keeps it and tries again), or null when
// the voice is off (the caller should consider it handled).
async function announce(text) {
  if (!settings.enabled) return null;
  if (Date.now() - lastSpokeAt < MIN_GAP_MS) return false;
  if (typeof speechSynthesis !== 'undefined' && speechSynthesis.speaking) return false;
  await say(text);
  return text;
}

export {
  announce, chirp, say, warm, stop, get, set, nudge, onSpeaking,
  lineForItem, sanitise, systemBackend, DEFAULTS, LIMITS,
};
