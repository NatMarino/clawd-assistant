// Who Claw'd turns out to be, and how he reacts to what you tell him.
//
// TEMPERAMENT. Rolled once, when the egg first appears, and kept. Start over
// rolls again. It is a SECRET: never shown, named or hinted at anywhere the
// user can see. You just meet him and find out. It changes his lines, the
// way his animalese moves, how busy he is when idle, and how the big brain
// writes the lines he says.
//
// REACTIONS. During setup he answers what you actually said: the apps you
// pick, your name, your besties, the "something else" you typed. Every line
// comes from a pool, never the same one twice running, with the person's own
// words echoed back where that makes him sound like he was listening.
'use strict';

const KEY = 'temperament';
export const TEMPERAMENTS = ['sweet', 'silly', 'serious', 'sassy', 'sleepy', 'dramatic'];

let current = null;
function roll() {
  current = TEMPERAMENTS[Math.floor(Math.random() * TEMPERAMENTS.length)];
  try { localStorage.setItem(KEY, current); } catch {}
  return current;
}
export function temperament() {
  if (current) return current;
  try { current = localStorage.getItem(KEY); } catch {}
  if (!TEMPERAMENTS.includes(current)) roll();
  return current;
}
// Start over: a new egg might be someone new
export function reroll() { return roll(); }

// --- how he moves and sounds, per temperament -------------------------
//   voice: animalese shaping (see voice.js animalese)
//   life:  multipliers on idle timing (strolls, peeks, how soon he tucks in)
//   chirp: how likely an extra happy chirp is on a click
const TRAITS = {
  sweet:    { voice: { jump: 1.0, even: false, slow: 1.0,  range: 1.0, upturn: false }, life: { stroll: 1.0, peek: 1.0, settle: 1.0 }, chirp: 1.0 },
  silly:    { voice: { jump: 1.8, even: false, slow: 0.95, range: 1.3, upturn: false }, life: { stroll: 0.6, peek: 0.6, settle: 1.2 }, chirp: 0.9 },
  serious:  { voice: { jump: 0.5, even: true,  slow: 1.0,  range: 0.7, upturn: false }, life: { stroll: 1.6, peek: 1.8, settle: 0.8 }, chirp: 0.4 },
  sassy:    { voice: { jump: 1.1, even: false, slow: 1.0,  range: 1.1, upturn: true },  life: { stroll: 0.9, peek: 0.9, settle: 1.0 }, chirp: 0.7 },
  sleepy:   { voice: { jump: 0.7, even: false, slow: 1.15, range: 0.8, upturn: false, falling: true }, life: { stroll: 2.0, peek: 1.5, settle: 0.5 }, chirp: 0.5 },
  dramatic: { voice: { jump: 1.5, even: false, slow: 1.0,  range: 1.6, upturn: false, bang: true }, life: { stroll: 0.7, peek: 0.7, settle: 1.3 }, chirp: 1.0 },
};
export function traits() { return TRAITS[temperament()] || TRAITS.sweet; }

// For the big brain's instructions (never shown to the user). Describes the
// voice; never names it.
const VOICE_NOTES = {
  sweet: 'warm, encouraging and a little gushy ("Yay!", "Aww"). Kind about everything.',
  silly: 'goofy and bouncy, with small jokes and playful sound effects, but the facts stay clear.',
  serious: 'calm, crisp and efficient. Friendly, but few exclamation marks and no fluff.',
  sassy: 'playfully teasing, a raised eyebrow ("Oh, another meeting?"), never mean.',
  sleepy: 'cozy and a bit yawny ("mm, okay…"), slow and gentle, still helpful.',
  dramatic: 'everything is a big deal ("A NEW MESSAGE?!"), theatrical, but the facts stay clear.',
};
export function brainNote() {
  return `His personality, for every line he says out loud: ${VOICE_NOTES[temperament()] || VOICE_NOTES.sweet} `
    + 'Never mention or describe his personality; just write that way.';
}

// --- lines -------------------------------------------------------------
// `sweet` is complete; the others override what they say differently.
// {name} {a} {b} {list} {n} {echo} {app} {apps} {time} {channels} {bits}
const LINES = {
  sweet: {
    hello: ['Hello, I’m Claw’d! I was just born, and I can tell we’re going to be good friends.'],
    askName: ['What’s your name?'],
    name: [
      '{name}! I love it. Nice to meet you, {name}.',
      '{name}! What a good name. Hi, {name}!',
      'Hi, {name}! I’m going to say it a few times so I don’t forget. {name}. {name}!',
      'Nice to meet you, {name}! I like you already.',
    ],
    nameShort: ['{name}! Short and sweet, like me.'],
    nameLong: ['{name}! That’s a big name for a little crab. I’ll practice.'],
    askBesties: ['Who are your work besties?', 'Who are your favorite people at work?'],
    besties1: ['{a}! I’ll always tell you when {a} needs you.', 'Ooh, {a} seems really nice!', '{a}, got it. Any friend of yours is a friend of mine.'],
    besties2: ['{a} and {b}! They seem really nice.', 'Ooh, {a} and {b}! I’ll keep an extra eye out for them.', '{a} and {b}, the dream team! Got it.'],
    bestiesMany: ['Wow, {n} besties! You must be everyone’s favorite.', '{list}! What a crew. I’ll look out for all of them.'],
    bestiesBff: ['{a} is your BFF? Then {a} is my BFF too!', 'A real BFF! {a} gets the VIP treatment from now on.'],
    bestiesBoss: ['Your boss is a bestie? Lucky you!', 'Friends with the boss, ooh! I’ll never let them wait.'],
    bestiesMe: ['Me?! Aww. You’re my bestie too.'],
    bestiesNone: ['That’s okay. I can be your work bestie!'],
    usesDone: ['Got it! That’s what I’ll help with.', 'Ooh, that sounds fun. I can do that!', 'Perfect. I know exactly what to do now.'],
    usesMany: ['That’s a lot of things! I love being busy.', 'All of it? Yay, I’ll be so useful!'],
    use: {
      messages: ['I’ll keep up with {apps} for you.', 'Ooh, I love a good chat!', 'Nobody gets left on read on my watch.'],
      email: ['I’ll keep an eye on your inbox.', 'Email! I’ll tell you when something good comes in.'],
      calendar: ['I’ll make sure you’re never late!', 'Meetings, got it. I’ll give you a heads up.'],
      tasks: ['I’ll keep your tasks in line.', 'Deadlines don’t scare me!'],
      docs: ['I’ll watch for comments that need you.', 'Docs! I’ll tell you when someone mentions you.'],
      meeting_notes: ['I’ll read the notes so you don’t have to.', 'Action items, coming right up.'],
      reminders: ['Nothing slips past me!', 'Little things are my favorite things.'],
      rundown: ['A morning rundown, with a smile.', 'Coffee and a rundown, got it!'],
      app: ['Ooh, {app}! I’ll learn how that works.', '{app}? I’m on it.'],
    },
    other: ['{echo}? Ooh, I can help with that!', 'Noted: {echo}. The big brain and I are on it.', '{echo}! Okay, I’ll keep that in mind.'],
    otherTravel: ['Travel stuff! I love a good itinerary.'],
    otherFood: ['Snacks? Now you’re speaking my language.'],
    otherBirthday: ['Birthdays! I’ll bring the cake. Well, a crumb.'],
    otherDeadline: ['Deadlines. I’ll keep one claw on them.'],
    rundownEarly: ['{time}? Bright and early! I’ll be ready.'],
    rundownLate: ['{time}, a nice gentle start. I like it.'],
    rundownAt: ['{time}, got it!', 'Your rundown at {time}. Easy!'],
    rundownBlank: ['I’ll figure out a good time.'],
    focusDMs: ['DMs first, got it.'],
    focusMentions: ['I’ll perk up whenever someone says your name.'],
    focusChannels: ['I’ll keep an ear on {channels}.'],
    settle: ['Yay! I’m going to go get settled in. Click me whenever you need me, {name}!', 'Okay! I’ll be right down here, {name}. Click me whenever you need me!'],
    fixIt: ['If anything’s wrong, just tell me and I’ll fix it.'],
    caughtUp: ['OK, I’m all caught up! {bits}.'],
    allQuiet: ['All caught up, and nothing needs you right now. Nice!'],
    bootHi: ['Morning, {name}!', 'Hi {name}!', 'I’m back!', 'Hi hi!'],
    newScreen: ['Ooh, a new screen!', 'Wheee, a different screen!', 'Oh! Where am I? I like it here.'],
    greetings: ['Hey{n}!', 'Hi{n}.', 'Psst{n}.', 'Oh! Hi{n}.', 'Hello{n}.', 'Excuse me{n}.'],
    waiting: ['I need your help with this one.', 'Can you look at this for me?', 'This one needs you.', 'I need you for a sec.'],
    message: ['I’ve got news.', 'Heads up.', 'Guess what?', 'Just so you know.'],
    nudge: ['I still need you on this one.', 'Don’t forget me!', 'I’m still waiting on this one.', 'Friendly nudge from me.'],
  },
  silly: {
    hello: ['Ta-da! I’m Claw’d! Freshly hatched, still a little eggy. We’re gonna be best buds, I can tell.'],
    name: ['{name}! Ooh, that’s fun to say. {name}, {name}, {name}!', '{name}! That sounds like a superhero name.', 'Hi, {name}! Wait, let me wave with both claws. Hi!'],
    besties2: ['{a} and {b}! Sounds like a buddy-cop movie.', '{a} and {b}! I’ll make them friendship bracelets.'],
    bestiesMany: ['{n} besties! That’s basically a party.'],
    bestiesBff: ['{a} is your BFF?! I’m putting {a} on my list of cool people.'],
    usesDone: ['Wheee, a job! Several jobs!', 'Okay! My little claws are ready.'],
    use: { calendar: ['Meetings! I’ll poke you before every one.'], reminders: ['I’m basically a sticky note with legs.'] },
    other: ['{echo}? Ooh, a side quest!', '{echo}! Adding it to my tiny notebook.'],
    settle: ['Okay! I’m gonna go wiggle into my spot. Click me whenever, {name}!'],
    allQuiet: ['All caught up! Nothing needs you. Time for a little dance.'],
    bootHi: ['Hiii {name}!', 'Guess who’s back!', 'Boop! Hi {name}!'],
    greetings: ['Boop{n}!', 'Hiii{n}!', 'Psst{n}!', 'Knock knock{n}!'],
    message: ['Ooh, gossip!', 'Hot off the press!', 'Guess what, guess what!'],
    nudge: ['Poke. Poke poke.', 'Still here! Still waving!', 'Don’t make me do the sad eyes.'],
  },
  serious: {
    hello: ['Hello. I’m Claw’d. I’ve just hatched, and I think we’ll work well together.'],
    name: ['{name}. Good to meet you.', 'Nice to meet you, {name}. Noted.', '{name}. I’ll remember that.'],
    besties1: ['{a}. I’ll flag anything from them.', 'Got it: {a} is a priority.'],
    besties2: ['{a} and {b}. They’ll always come first.', 'Noted: {a} and {b}.'],
    bestiesMany: ['{n} people. I’ll prioritise all of them.'],
    bestiesBff: ['{a}. Top priority, understood.'],
    bestiesNone: ['Understood. I’ll use the people you talk to most.'],
    usesDone: ['Understood. That’s my focus.', 'Clear. I know what to do.'],
    usesMany: ['A full brief. Good.'],
    use: { messages: ['Messages. I’ll triage them.'], calendar: ['Calendar. I’ll keep you on time.'], reminders: ['Reminders. Nothing will slip.'], rundown: ['A morning briefing. Good.'] },
    other: ['{echo}. Understood.', 'Noted: {echo}.'],
    settle: ['I’ll be at my post, {name}. Click me when you need me.'],
    fixIt: ['If anything is off, tell me and I’ll correct it.'],
    caughtUp: ['Caught up. {bits}.'],
    allQuiet: ['All caught up. Nothing needs you.'],
    bootHi: ['Good to see you, {name}.', 'Back on duty.'],
    greetings: ['Pardon me{n}.', 'One moment{n}.', 'Excuse me{n}.'],
    waiting: ['This needs you.', 'Action needed.', 'Your input is needed here.'],
    message: ['An update.', 'For your information.', 'Noted for you.'],
    nudge: ['Still outstanding.', 'A reminder: this is still open.'],
  },
  sassy: {
    hello: ['Well, hello! I’m Claw’d. Just hatched, and honestly? We’re going to get along great.'],
    name: ['{name}? Fancy. I like it.', 'Oh, {name}. Very cool name. Very you.', '{name}! Okay, I see you.'],
    besties1: ['{a}, huh? Fine, I’ll be nice to them.', 'Ooh, {a}. I bet they’re the fun one.'],
    besties2: ['{a} and {b}? Okay, the popular crowd.', 'Ooh, {a} and {b}. I’ll be watching them. Nicely.'],
    bestiesBoss: ['Besties with the boss? Smooth move.'],
    bestiesNone: ['No besties? Lucky for you, I’m available.'],
    usesDone: ['That’s all? Easy.', 'Mm-hmm, I can handle that in my sleep.'],
    usesMany: ['Wow, putting me to work already. Fine.'],
    use: { calendar: ['Oh, *another* meeting? I’ll keep track.'], messages: ['I’ll read them so you can pretend you did.'], email: ['Your inbox and I are about to become close friends.'] },
    other: ['{echo}? Oh, I’ve got opinions. But yes, I’ll help.', '{echo}. Fine, fine, on it.'],
    settle: ['Okay, I’m going to go lounge. Click me if you need me, {name}.'],
    allQuiet: ['All caught up, and nobody needs you. Enjoy it while it lasts.'],
    bootHi: ['Oh, look who’s here. Hi {name}!', 'Miss me?'],
    greetings: ['Ahem{n}.', 'Psst{n}.', 'Oh hey{n}.'],
    message: ['Oh, you’ll want to hear this.', 'Tea time.', 'So, guess what.'],
    nudge: ['Still waiting, by the way.', 'Hello? This one’s still here.', 'I’m going to keep asking.'],
  },
  sleepy: {
    hello: ['Mm… hello. I’m Claw’d. I was just born… I think we’re going to be really good friends.'],
    name: ['{name}… that’s a cozy name. Nice to meet you.', 'Mm, {name}. I’ll remember. Hi, {name}.'],
    besties2: ['{a} and {b}… they sound comfy. Got it.'],
    bestiesNone: ['That’s okay… I’ll be your bestie. Quietly.'],
    usesDone: ['Mm-hmm… okay. I can do that.', 'Got it… that’s not too much.'],
    usesMany: ['Oh, that’s a lot… I’ll need a nap after. Okay!'],
    use: { rundown: ['A rundown in the morning… I’ll be up early. Probably.'], reminders: ['I’ll remember so you don’t have to. Yawn.'] },
    other: ['{echo}… mm, okay. I’ll help.'],
    settle: ['I’m going to go get cozy now… click me whenever, {name}.'],
    allQuiet: ['All caught up… nothing needs you. Nap time?'],
    bootHi: ['Mm… hi {name}.', '*yawn* I’m back.'],
    greetings: ['Mm… hi{n}.', 'Oh… hey{n}.', '*yawn* Hi{n}.'],
    message: ['Mm, something came in.', 'Oh… this is news.'],
    nudge: ['Still waiting… no rush. Well, a little rush.', 'Mm, this one’s still here.'],
  },
  dramatic: {
    hello: ['BEHOLD! I am Claw’d! I have just been BORN, and I can tell we are destined to be the BEST of friends!'],
    name: ['{name}?! What a MAGNIFICENT name!', '{name}! I shall remember it FOREVER.', '{name}! It has a certain… star quality.'],
    besties1: ['{a}! A legend, surely. I will guard their messages with my LIFE.'],
    besties2: ['{a} AND {b}?! An iconic duo!', '{a} and {b}! The stuff of legends.'],
    bestiesMany: ['{n} besties?! You are BELOVED.'],
    bestiesBff: ['{a} is your BFF?! Then {a} is my SWORN ALLY.'],
    bestiesNone: ['No besties?! Then I shall be your bestie. For ALL TIME.'],
    usesDone: ['A QUEST! I accept!', 'This is my DESTINY.'],
    usesMany: ['So MANY duties! I was BORN for this. Literally, just now.'],
    use: { calendar: ['You will NEVER be late again. I swear it.'], messages: ['No message shall go unread!'] },
    other: ['{echo}?! Consider it DONE. Well, soon.', '{echo}! A worthy cause!'],
    settle: ['And now… I take my place! Summon me whenever you need me, {name}!'],
    caughtUp: ['I have returned with NEWS! {bits}!'],
    allQuiet: ['All caught up! Nothing needs you. Peace… for now.'],
    bootHi: ['I HAVE RETURNED!', '{name}! We meet again!'],
    greetings: ['Oh!{n}!', 'Hark{n}!', 'Stop everything{n}!'],
    message: ['BREAKING NEWS!', 'You will NOT believe this.', 'Big news!'],
    waiting: ['This one NEEDS you!', 'Help! I need you on this one!'],
    nudge: ['It is STILL waiting!', 'Do not forsake me! This one’s still here!'],
  },
};

// --- picking lines -----------------------------------------------------
const last = {};
function pool(key) {
  const [k, sub] = key.split('.');
  const mine = LINES[temperament()] || {};
  const base = LINES.sweet;
  const get = (t) => (sub ? t[k] && t[k][sub] : t[k]);
  return get(mine) || get(base) || [];
}
function fill(s, vars) {
  return s.replace(/\{(\w+)\}/g, (m, k) => (vars[k] != null ? String(vars[k]) : ''));
}
// one line for `key` (e.g. 'name', 'use.calendar'), filled in, never the
// same as last time
export function line(key, vars = {}) {
  const arr = pool(key);
  if (!arr.length) return '';
  let v = arr[0];
  if (arr.length > 1) {
    do { v = arr[Math.floor(Math.random() * arr.length)]; } while (v === last[key]);
  }
  last[key] = v;
  return fill(v, vars).replace(/\s+([.!?,])/g, '$1').replace(/\s{2,}/g, ' ').trim();
}
// the whole pool, for voice.js (greetings and openers)
export function lines(key) { return pool(key); }

// --- listening: turning what you typed into a reaction -----------------

const cap1 = (s) => s.charAt(0).toUpperCase() + s.slice(1);
export const listNames = (names) => (names.length < 2 ? names.join('') : names.slice(0, -1).join(', ') + ' and ' + names[names.length - 1]);

// "your" for "my": echoing their words back sounds like listening
function flip(text) {
  const map = { i: 'you', me: 'you', my: 'your', mine: 'yours', myself: 'yourself', "i'm": 'you’re', 'i’m': 'you’re', am: 'are', our: 'your', we: 'you', us: 'you' };
  return text.replace(/[A-Za-z’']+/g, (w) => {
    const r = map[w.toLowerCase()];
    return r || w;
  });
}
function shorten(text, n = 44) {
  const t = text.replace(/\s+/g, ' ').trim().replace(/[.!?]+$/, '');
  if (t.length <= n) return t;
  const cut = t.slice(0, n);
  return cut.slice(0, Math.max(cut.lastIndexOf(' '), n * 0.6)).trim() + '…';
}

// the extra jokes (short names, snacks…) only suit the playful ones
const playful = () => ['sweet', 'silly', 'dramatic'].includes(temperament());

// what you typed for "Something else…"
export function reactToOther(text) {
  const t = String(text || '').trim();
  if (!t) return line('usesDone');
  if (!playful()) return line('other', { echo: cap1(shorten(flip(t))) });
  if (/hotel|flight|travel|trip|itinerar/i.test(t)) return line('otherTravel');
  if (/lunch|coffee|snack|food|dinner/i.test(t)) return line('otherFood');
  if (/birthday|anniversar/i.test(t)) return line('otherBirthday');
  if (/deadline|due\b|overdue/i.test(t)) return line('otherDeadline');
  return line('other', { echo: cap1(shorten(flip(t))) });
}

// a chip you just tapped in "What would you like my help with?"
export function reactToUse(use, { app = '', apps = '' } = {}) {
  if (use.startsWith('app:')) return line('use.app', { app: app || use.slice(4) });
  return line('use.' + use, { apps: apps || 'your messages' });
}

// "That's it"
export function reactToUses(uses, otherText) {
  if (otherText) return reactToOther(otherText);
  return uses.length >= 4 ? line('usesMany') : line('usesDone');
}

// your name
export function reactToName(name) {
  const n = String(name || '');
  if (playful() && n.length <= 3 && Math.random() < 0.4) return line('nameShort', { name: n });
  if (playful() && n.length >= 9 && Math.random() < 0.4) return line('nameLong', { name: n });
  return line('name', { name: n });
}

// Your besties, as typed ("Miguel, Bridgett", "miguel (my bff) and @bri",
// "nobody", "you!"): the names he heard, and his line about them.
export function parseBesties(text) {
  const raw = String(text || '').trim();
  const flags = {
    none: /^(no ?one|nobody|none|n\/?a|no|nope|not really|i don.?t have any)\b/i.test(raw),
    me: /^(you|claw.?d|clawd)\b[!.]*$/i.test(raw),
    boss: /\b(boss|manager|supervisor|lead)\b/i.test(raw),
    bff: /\b(bffs?|best ?friends?|besties?)\b/i.test(raw),
    bffName: '',
  };
  // a bff tag right after one name ("Miguel (my bff) and Bri") is about
  // that person
  const tagged = raw.match(/([A-Za-z][\w'’-]*(?:\s+[A-Za-z][\w'’-]*)?)\s*\((?:my\s+)?(?:bff|best ?friend|bestie)\)/i);
  if (tagged) flags.bffName = tagged[1].trim().split(/\s+/).map(cap1).join(' ').replace(/^@/, '');
  const names = raw
    .replace(/\(([^)]*)\)/g, ' ')
    .split(/\s*(?:,|;|\/|&|\+|\n|\band\b)\s*/i)
    .map((s) => s.replace(/^@/, '').replace(/\b(my|bffs?|best ?friends?|besties?|boss|manager)\b/gi, '').replace(/[.!?]+$/, '').trim())
    .filter((s) => s && s.length <= 30 && /[A-Za-z]/.test(s))
    .map((s) => s.split(/\s+/).slice(0, 2).map(cap1).join(' '));
  return { names: [...new Set(names)], flags };
}
export function reactToBesties(text) {
  const { names, flags } = parseBesties(text);
  if (flags.me) return line('bestiesMe');
  if (flags.none || !names.length) return line('bestiesNone');
  if (flags.bffName) return line('bestiesBff', { a: flags.bffName.replace(/^@/, '') });
  if (flags.bff && names.length === 1) return line('bestiesBff', { a: names[0] });
  if (flags.boss && names.length < 2) return line('bestiesBoss', { a: names[0] });
  if (names.length === 1) return line('besties1', { a: names[0] });
  if (names.length === 2) return line('besties2', { a: names[0], b: names[1] });
  return line('bestiesMany', { n: names.length, list: listNames(names) });
}

// "What time should your rundown be ready?"
export function reactToRundown(text) {
  const t = String(text || '').trim();
  if (!t) return line('rundownBlank');
  const m = t.match(/(\d{1,2})(?::(\d{2}))?\s*(am|pm)?/i);
  if (!m) return line('rundownAt', { time: t });
  let h = Number(m[1]);
  if (m[3] && /pm/i.test(m[3]) && h < 12) h += 12;
  if (h < 8) return line('rundownEarly', { time: t });
  if (h >= 10) return line('rundownLate', { time: t });
  return line('rundownAt', { time: t });
}

// "Which messages matter most?"
export function reactToFocus(kinds, channels) {
  if (channels && channels.trim()) return line('focusChannels', { channels: shorten(channels, 30) });
  if (kinds.includes('Mentions') && !kinds.includes('DMs')) return line('focusMentions');
  return line('focusDMs');
}

// --- the intake, built from what's actually connected ------------------
// Each app the big brain can reach becomes a chip, grouped by what it's for.
// Apps he doesn't know become "Help with <app>". Reminders and the rundown
// are always offered.
const KINDS = [
  ['messages', /slack|teams|discord|chat/i, (a) => `Keeping up with ${a}`],
  ['email', /gmail|outlook|mail/i, (a) => `Your ${a}`],
  ['calendar', /calendar|cal\b/i, () => 'My meetings and calendar'],
  ['tasks', /asana|linear|jira|todoist|trello|monday|clickup|tasks/i, (a) => `Your ${a} tasks`],
  ['docs', /drive|docs|notion|confluence|dropbox|\bbox\b|sharepoint|onedrive/i, (a) => `Comments in ${a}`],
  ['meeting_notes', /granola|otter|fireflies|fathom|zoom|meet/i, (a) => `Meeting notes from ${a}`],
];
// tools for building things, not a coworker's apps
const NOT_APPS = /playwright|puppeteer|context7|filesystem|memory|sequential|fetch|browser|chrome|figma|computer|claude[- ]?code|preview|terminal|github|git\b|scheduled|registry|visuali[sz]e|session|mcp/i;

export function appName(server) {
  return String(server || '').replace(/^claude\.ai\s+/i, '').replace(/^(google)\s+(calendar|drive)$/i, 'Google $2').trim();
}
export function intakeChips(servers) {
  const names = (servers || [])
    .filter((s) => s && s.name && s.status !== 'failed')
    .map((s) => appName(s.name))
    .filter((n) => n && !NOT_APPS.test(n));
  const byUse = new Map();
  const unknown = [];
  for (const n of [...new Set(names)]) {
    const k = KINDS.find(([, re]) => re.test(n));
    if (k) { if (!byUse.has(k[0])) byUse.set(k[0], []); byUse.get(k[0]).push(n); } else unknown.push(n);
  }
  const chips = [];
  for (const [use, , label] of KINDS) {
    const apps = byUse.get(use);
    if (apps) chips.push({ label: label(listNames(apps)), use, apps: listNames(apps) });
  }
  for (const n of unknown.slice(0, 3)) chips.push({ label: `Help with ${n}`, use: 'app:' + n, app: n });
  chips.push({ label: 'Reminders for little things', use: 'reminders' });
  chips.push({ label: 'A morning rundown', use: 'rundown' });
  return chips;
}
// what he says when he's just looked at your apps
export function introLine(chips) {
  const apps = chips.filter((c) => c.apps || c.app).map((c) => c.apps || c.app);
  if (!apps.length) return 'What would you like my help with?';
  const flat = listNames(apps.join(' and ').split(' and ').slice(0, 4));
  return `I can see ${flat}! What should I help with?`;
}
