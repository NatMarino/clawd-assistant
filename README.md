# Claw'd, the assistant

*A crab. A robot. A very small coworker.*

Claw'd sits on top of your windows and keeps an eye on your work for you: a VIP's
Slack message, a meeting you're about to be late for, an email that's been waiting,
the morning digest. He reads it to you, hops until you look, and nudges when
something has sat too long. Hover him and press **+** to hand Claude a task.

He's the office-worker sibling of [Claw'dbot](https://github.com/NatMarino/clawdbot),
the pet that watches Claude Code sessions. Same crab, different job.

![Claw'd napping](docs/clawdbot-sleeping.gif)

## How it works

Claw'd can't read anything himself, and holds no accounts or passwords. **Your
own Claude** does the reading, through the connections you've already given it
(Slack, Google Calendar, Gmail, Asana, Drive). It checks on a schedule, following
the [clawd skill](skill/clawd/SKILL.md), and writes what matters into a folder
Claw'd watches:

```
Your apps ──(your Claude's connectors)──▶ Claude, on a schedule
                                             │ writes small JSON files
                                             ▼
                              C:\Users\<you>\Clawd\inbox
                                             │
                                             ▼
                                     Claw'd on your desktop
                     shows it · says it · reminds · nudges · hands tasks back
```

When you ask him for something, or press **Do it** on one of Claude's suggestions,
he opens Claude with the request already written. Claude does the work, where you
can see it.

**Propose, don't act.** By default Claude only suggests ("here's a reply I'd
send"). The gear has a **Do it** mode that adds a button to each suggestion. He
wears a hard hat while it's on, and each press hands exactly one suggestion to
Claude. Nothing is ever sent by a scheduled check.

## Setting it up

Give [SETUP.md](SETUP.md) to Claude and say "set this up". It checks your
connections, walks you through installing the pet, asks who your VIPs are and
when you work, and schedules the checks. The same file works for anyone: nothing
in it is tied to the person who sent it.

## Using it

- **Click** Claw'd for everything he's tracking: *Waiting on you*, *Coming up*,
  *Messages*, *Deliveries*. Click a row to open it, and ✓ to clear it.
- **Hover, then +** to ask Claude for something.
- **The treat** next to the + feeds him. He gets peckish.
- **The gear** holds the mode (Propose or Do it), panel theme, skin (`app` or `cli`),
  and voice (on or off, which voice, pitch, speed).
- **Drag** to move. `+`, `-` and `0` resize him while he's selected. `q` quits.
- **Grey with a question mark** means he hasn't heard from Claude for longer than
  expected. He never pretends all is quiet when he just can't see.

## The folder

`%USERPROFILE%\Clawd` (not Documents, which work laptops often sync to OneDrive)
contains:

| | |
|---|---|
| `inbox\` | Claude drops JSON items here. He reads them within ~2 s and moves them to `done\` (or `bad\`). |
| `prefs.json` | Your preferences, written by Claude. Ask Claude to change them. |
| `sent.json` | Claude's memory between checks. |
| `http.json` | `http://127.0.0.1:4318/items` and a token, for a Claude that can POST instead. |
| `SKILL.md` | The skill, matching this version of the pet. |

The item format is documented in the skill. Posting by hand is handy for testing:

```bash
curl -X POST http://127.0.0.1:4318/items -H "Authorization: Bearer <token from http.json>" -d "{\"id\":\"t1\",\"kind\":\"waiting\",\"who\":\"Test\",\"title\":\"Hello\"}"
```

His own state (position, size, the current inbox) lives in
`%APPDATA%\ClawdAssistant\`.

## Building

Windows 11, Rust stable, and the Tauri 2 prerequisites.

```bash
cd src-tauri
cargo test
cargo build --release --features custom-protocol
```

The `custom-protocol` feature embeds `src/` into the exe. For the installer:
`npx @tauri-apps/cli@2 build`, or push a `v*` tag and let the release workflow
build it.

To work on the face without building anything, run `node tools/dev-server.js`,
open http://localhost:4870, and press **demo inbox** in the debug panel.

## Status and art

**Drawn (CLI skin):** reading a letter (it arrives, then he reads it), a pocket
watch for meetings, carrying a parcel for deliveries, a happy wiggle when
clicked, and peeking up from the ground. They come from Nat's animation
series and are converted by `tools/import-sheets.js`, which reads the pack's
sprite sheets and writes `src/sprites-office.js`:

```bash
node tools/import-sheets.js "<pack>/sprites/cli"
```

**Still placeholders:** `celebrate` and the hard hat on both skins, and every
new clip on the `app` skin, which borrows an existing move until it has its
own. The list is in `src/styles.js`, and the debug panel shows placeholders
with dashed borders.

## Credits

The crab, both skins and most of the face come from Claw'dbot, originally by
[rubberdonut67](https://github.com/rubberdonut67/clawdbot) under the MIT licence,
with animation and voice work by NatMarino. Claw'd is an independent hobby project,
not made, endorsed or supported by Anthropic. The `cli` skin recreates the Claude
Code terminal buddy.
