# Claw'd, the assistant

*A crab. A robot. A very small coworker.*

Claw'd lives on your taskbar and keeps an eye on things for you. At work: a VIP's
Slack message, a meeting you're about to be late for, an email that's been
waiting, the morning rundown. At home: your Claude Code sessions, a new episode
on Plex, the garage left open. He tells you out loud, hops until you look, and
nudges when something has sat too long. Ask him for things in his chat.

**Start here: [docs/STATUS.md](docs/STATUS.md)**, what's built, how it works,
and where each piece of work lives. What he's *for* comes from **packs**
(office, home, dev, or your own). The personal version's next steps are in
[docs/personal-proposal.md](docs/personal-proposal.md).

He grew out of [Claw'dbot](https://github.com/NatMarino/clawdbot), the pet that
watches Claude Code sessions. Same crab; that job is now his dev pack.

![Claw'd napping](docs/clawdbot-sleeping.gif)

## How it works

Claw'd can't read anything himself, and holds no accounts or passwords. He
runs **the big brain** (Claude Code, logged in as you) in the background, and
it does the reading, through the connections you've already given it
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

When you ask him for something in his chat, the big brain does it in the
background while he types on his little laptop, and he tells you when it's done.
Bigger things (a doc, a report) he hands to a Claude chat you can watch.

**Nothing goes to another person without your tap.** In **auto** (the default)
he does what you ask right away, but every message for someone starts as a
draft card: **Love it**, **Don't like it** or **Revise**, then **Send it** or
**Copy it**. The gear's **ask me first** mode makes him propose everything.
Nothing is ever sent by a scheduled check.

**Windows 11 and macOS 11+.** Download the latest from
[Releases](https://github.com/NatMarino/clawd-assistant/releases): the
`-setup.exe` for Windows, the `.dmg` for a Mac (Apple silicon or Intel). Neither
is signed. On Windows, SmartScreen says "Windows protected your PC": click
*More info*, then *Run anyway*. On a Mac, drag Claw'd to Applications and open
him once. macOS refuses ("could not verify…"): click **Done**, then go to System
Settings → Privacy & Security, scroll down, and click **Open Anyway** next to
ClawdAssistant. If it says the app is "damaged" instead, run this once in
Terminal, then open him again:
`xattr -dr com.apple.quarantine /Applications/ClawdAssistant.app`

## Setting it up

Install him and click the egg: he sets himself up, running the big brain
(Claude Code) himself. You need the Claude app installed and signed in, or
Claude Code. He asks where he's living (a work computer, your own, or both)
and what you'd like help with, then works out the rest from your connected
apps while the egg cracks.

For a computer where he can't run the big brain, [SETUP.md](SETUP.md) is the
fallback: give it to Claude (Cowork) and say "set this up".

## Using it

- **He starts as an egg.** Click it and he sets himself up: he asks what
  you'd like his help with, then the big brain (Claude Code, which he runs
  himself in the background) works out the rest from your connected apps while
  the egg cracks. You click the last crack to hatch him, and he asks your name
  (and, on a work computer, your work besties). (The tray's *Start over* puts
  him back in his egg.)
- **Packs** (gear → *packs*): **office** (work apps), **home** (Plex, Sonarr,
  Radarr, Overseerr, Home Assistant: [docs/home-setup.md](docs/home-setup.md)),
  **dev** (your Claude Code sessions: working, thinking, needs you, done).
  Write your own: [docs/packs.md](docs/packs.md).
- **His requests chat (+):** ask him anything. He types on his little laptop
  while the big brain works, and tells you when it's done. In **auto** (the
  default) he just does it; anything that goes to another person comes back as
  a draft card first.
- **He lives on your taskbar** (on a Mac, on the Dock). His home is just left
  of the clock (on a Mac, near the right end). When nothing
  needs you he tucks in behind the taskbar with his eyes peeking over, looks up
  now and then, and every few minutes takes a short stroll and wanders back.
  When something needs you he pops up and hops. Hover his head to bring him up.
- **He starts with Claude.** He starts when you log in, stays out of sight, and
  pops up when the Claude app (or Claude Code) opens. The gear's *start with
  Claude* switches it off.
- **Everyone gets a slightly different Claw'd.** Some are sweet, some silly,
  some very serious. You find out when he hatches.
- **The tray icon** (the crab by the clock, or in the menu bar on a Mac): click
  to call him out; right-click for *Back to the taskbar*, *Start over* and *Quit*.
- **Click** Claw'd for everything he's tracking: *Waiting on you*, *Coming up*,
  *Messages*, *Deliveries*. Click a row to open it, and ✓ to clear it.
- **Hover, then +** for his requests chat.
- **The treat** next to the + feeds him. He gets peckish.
- **The gear** holds the mode (auto or ask me first), panel theme, and
  *customize how I look and sound* (words, animalese or no sound; voice, pitch,
  size).
- **Drag** to move. Drop him near the taskbar and he settles onto it; drop him
  anywhere else and he stays put (the tray menu sends him back). `+`, `-` and
  `0` resize him while he's selected. `q` quits.
- **Grey with a question mark** means he hasn't heard from Claude for longer than
  expected. He never pretends all is quiet when he just can't see.

## The folder

`%USERPROFILE%\Clawd` on Windows, `~/Clawd` on a Mac (not Documents, which work
laptops often sync to OneDrive) contains:

| | |
|---|---|
| `inbox\` | Claude drops JSON items here. He reads them within ~2 s and moves them to `done\` (or `bad\`). |
| `intro.json` | What he learned about you when he hatched: your name, VIPs, hours. |
| `prefs.json` | Your preferences, written by Claude from that. Ask Claude to change them. |
| `sent.json` | Claude's memory between checks. |
| `http.json` | `http://127.0.0.1:4318/items` and a token, for a Claude that can POST instead. |
| `SKILL.md` | The skill, matching this version of the pet. |

The item format is documented in the skill. Posting by hand is handy for testing:

```bash
curl -X POST http://127.0.0.1:4318/items -H "Authorization: Bearer <token from http.json>" -d "{\"id\":\"t1\",\"kind\":\"waiting\",\"who\":\"Test\",\"title\":\"Hello\"}"
```

His own state (position, size, the current inbox) lives in
`%APPDATA%\ClawdAssistant\` (on a Mac, `~/Library/Application Support/ClawdAssistant/`).
If something goes wrong, `brain.log` and `page.log` there say what he tried.

## Building

Windows 11 or macOS, Rust stable, and the Tauri 2 prerequisites. CI builds and
tests both on every push; releases are built there too.

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

**Drawn, on both skins (Nat's animation series):** idle, eating a cookie, talking,
hearts, mad, reading a letter, a pocket watch for meetings, a parcel for
deliveries, a happy wiggle, a celebration, peeking up from behind the taskbar,
and the scuttle (his walk). `tools/import-sheets.js` converts the pack (the CLI
letter is shrunk by `tools/letter-small.js`) into `src/sprites-office.js`:

```bash
node tools/import-sheets.js "<pack>/clawd-pet-sprites"
```

The walk moves him at the pace its stride is drawn for, so his feet don't
slide. **Still placeholders:** the egg and hatching, being held, and the tumble.

## Credits

The crab, both skins and most of the face come from Claw'dbot, originally by
[rubberdonut67](https://github.com/rubberdonut67/clawdbot) under the MIT licence,
with animation and voice work by NatMarino. Claw'd is an independent hobby project,
not made, endorsed or supported by Anthropic. The `cli` skin recreates the Claude
Code terminal buddy.
