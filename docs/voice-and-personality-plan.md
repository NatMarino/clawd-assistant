# His voice, a smarter intake, and a secret personality

*A handoff. Nothing here is built yet. It comes from Nat's notes after
hatching beta 9 (2026-09-24), and sits alongside `office-dev-plan.md` and
`v0.3-plan.md` as future work. It could be the last polish before v0.2.0 ships,
or its own release after that; not decided.*

## What Nat asked for

- "He would talk over himself sometimes, and then the animalese was coming out
  kind of scratchy."
- The overall volume a tiny bit lower.
- The deep animalese SUPER deep; the high one not too high, but higher than now.
- In the egg, even more muffled, "like funny".
- When testing voices, clicking them fast plays them all at once: only one at a
  time.
- A more general onboarding: he could connect to any connected app, so the
  intake should come from what's actually there, and he should really react to
  what you say.
- Tamagotchi-style personality: sometimes you hatch a serious one, a silly one,
  a sweet one, and his lines change to match.

**Decisions already made:**
- Five or more personalities.
- Rolled at random, and a **secret**: never shown, named or referenced anywhere.
  You just experience it.
- The intake starts with a quick big-brain look at the connected apps.

## 1. One mouth

**Why it happens.** `say()` in `src/voice.js` cancels Windows speech but not the
animalese, which schedules dozens of Web Audio nodes ahead of time. A new line,
or a second quick tap on a voice sample, stacks a second voice on the first.
Summed sawtooth voices through high-Q bandpasses (gain 2.2 each) then clip,
which is the scratchiness.

**The fix, in `src/voice.js`:**
- **Every line goes through one mouth**, a single gain node. Starting a line
  fades the current one out over ~40 ms and stops its scheduled nodes (keep
  them in a list), across both styles. This also fixes the sample pile-up: only
  the newest tap plays.
- **A master chain for all his sounds** (voice, chirps, cracks, munches): a
  `DynamicsCompressor` as a soft limiter, then a gentle ~4.5 kHz low-pass to take
  the fizz off.
- **Cleaner animalese:**
  - a 10–12 ms attack and release per letter
  - a sawtooth and triangle blend
  - formant gain 2.2 → ~1.3
  - letters crossfade instead of stacking
- **Volume:** the default 0.9 → 0.75, and animalese ~15% lower relative to
  chirps.

## 2. Animalese pitch that means it

Give animalese its own pitch setting (`animalesePitch`: deep / normal / high),
separate from the word voice's Web Speech pitch (1.4 / 1.8 / 2.0, which stays).
Today both use `f0 = 170 * pitch`, which is why "Lower" isn't very low.

| Choice | f0 | Formants |
|---|---|---|
| Super deep | ~70 Hz | down ~25% (a big rumbly crab) |
| Normal | ~230 Hz | as now |
| High | ~380 Hz | up ~10% (brighter, not squeaky) |

The "How high is my voice?" step shows these chips when the style is
animalese, and plays a sample on each tap (through the one mouth).

## 3. The egg, extra muffled

`babble()` → `animalese(…, { muffled: true })` becomes:
- a ~380 Hz low-pass with a little resonance (Q ~4): talking into a pillow
- the pitch down 15%, with a slow 6 Hz wobble (±5%)
- letters 20% longer, as if struggling through the shell
- sometimes a little "mmf" thump at the start of a line

Crack sounds stay crisp; they're the shell, not him.

## 4. An intake built from what's connected

**A quick big-brain job first** (`Job: intake`, on the quick model, ~5–10 s).
The egg rocks and says "Let me see what you've got…". The job sees the
connected apps and tools, and writes the intake as JSON:

```json
{
  "apps": ["Slack", "Asana", "Granola"],
  "questions": [
    { "id": "uses", "ask": "I can see Slack, Asana and Granola! What should I help with?",
      "chips": [
        { "label": "Keeping up with Slack", "use": "messages", "react": "Ooh, I love a good chat!" },
        { "label": "Your Asana tasks", "use": "tasks", "react": "Tasks! I'll keep them in line." },
        { "label": "Meeting notes from Granola", "use": "meeting_notes", "react": "I'll read them so you don't have to." },
        { "label": "A morning rundown", "use": "rundown", "react": "Coffee and a rundown, got it." },
        { "label": "Reminders for little things", "use": "reminders", "react": "Nothing slips past me." }
      ],
      "other": true },
    { "id": "messages_focus", "if": "messages", "ask": "Which Slack messages matter most?",
      "chips": [ { "label": "DMs" }, { "label": "Mentions" }, { "label": "A few channels" } ],
      "input": "Which channels? (optional)" }
  ]
}
```

**How it plays:**
- **Chips come from what's connected,** including apps he's never heard of.
  Reminders and the rundown are always offered.
- **He reacts to every tap,** instantly, from the `react` lines (in the egg: a
  muffled mumble plus the bubble). Several picks get one combined reaction on
  "That's it".
- **"Something else…"** gets a reaction from a tiny follow-up call, or a warm
  generic line if that takes longer than ~2 s.
- **Follow-ups only show when they matter** (the `if` field).
- **If the job fails,** fall back to today's fixed questions (`askUses()`), so
  setup never gets stuck.

Answers still go to `intro.json`, and the setup job uses them as it does now.

`skill/clawd/SKILL.md` gets a **Job: intake** section:
- the JSON shape
- only offer what the connected apps can actually do, plus reminders and the
  rundown
- reactions in character, under 10 words
- never name an app that isn't connected

## 5. Secret personalities

**Rolled once, when the egg first appears,** and kept in localStorage
(`temperament`). Start over re-rolls it. It is never shown, named or
referenced: not in the UI, not in anything he says, not in anything the user
can read.

| Temperament | How it feels |
|---|---|
| sweet | warm and encouraging, lots of "yay"; chirps often |
| silly | goofy bits, bouncy; more wiggles and peeks |
| serious | calm, crisp, efficient; few flourishes |
| sassy | playful teasing ("Oh, *another* meeting?") |
| sleepy | yawny and cozy; naps sooner and longer |
| dramatic | everything is a big deal ("A NEW MESSAGE?!"); bigger hops |

**What changes with it:**
- **His lines** in `voice.js` (greetings, openers, nudges, "done", late-meeting
  lines): one table per temperament.
- **His first words:** Nat's script is the sweet version, word for word. The
  others keep its meaning in their own voice. For example:
  - serious: "Hello. I'm Claw'd. I've just hatched, and I think we'll work well
    together."
  - silly: "Ta-da! I'm Claw'd! Freshly hatched, still a bit eggy."
- **The intake's reactions** are written in his temperament.
- **The big brain's spoken lines:** each job's instructions carry the
  temperament, so sweeps, replies and the rundown sound like him. It goes in the
  job text only, not in `prefs.json`.
- **Animalese flavour:**
  - silly: bigger pitch jumps
  - serious: even rhythm
  - sleepy: 15% slower, falling at the end
  - dramatic: wide range, a big rise on "!"
  - sassy: an upturn at the end of lines
- **Idle life:**
  - silly and dramatic: more strolls and peeks
  - sleepy: tucks in sooner, fewer strolls
  - serious: fewest flourishes

## Where the code is

- `src/voice.js`: `say()`, `animalese()`, `CHIRPS`, `DEFAULTS` (volume 0.9,
  pitch 1.8), `isSpeaking()`
- `src/index.html`:
  - onboarding: `panel()`, `askUses()`, `customize()` with `PITCHES`, `runSetup`
    and `firstWords`
  - `startOver`
  - idle life (strolls, tuck, peek)
- `src/brain.js`: `run`, `MODELS.quick`, `readSummary`; add `readIntake`
- `skill/clawd/SKILL.md`: the jobs and his voice

## Testing

- **Browser rehearsal with the fake big brain** (the `_tauri-test.html`
  pattern). Use a fake intake with odd apps ("Granola", "Linear", "Weirdo CRM"):
  - the chips are built from it
  - every tap reacts
  - follow-ups appear only for their use
  - a failed job falls back to the fixed questions
- **One mouth:** five `say()` calls in 200 ms leave one line playing; three fast
  sample taps play only the last.
- **Temperament:**
  - 60 rolls hit all six temperaments
  - the chosen one's lines and animalese settings are used
  - no temperament name appears anywhere in the page
- **Nat's ears on a beta:**
  - no scratchiness
  - super deep is super deep
  - high is higher but fine
  - the egg sounds pillow-muffled
  - the volume is a touch lower

**Note:** The Mac port is being built in this same repo, so pull first and
commit in small pieces.
