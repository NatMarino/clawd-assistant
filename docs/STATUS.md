# Where Claw'd stands

*Updated 2026-09-25. Read this first: it's the map of what's built, why it's
built that way, and where each piece of work lives. For contributors,
including other Claude sessions.*

## Two Claw'ds, one codebase

| | Office Claw'd | Personal Claw'd |
|---|---|---|
| Branch | `main` | `home` (merges `main` regularly) |
| Version | 0.2.0 betas: **beta 14** is current | 0.3.0 alphas: **alpha 1** is current |
| Where | Nat's work laptop (work Claude account), coworkers (Windows and Mac) | Nat's own laptop and desktop (personal account); later the VM and her phone |
| What he watches | Slack, Google Calendar, Gmail, Asana, Drive, meeting notes | Claude Code sessions, Plex and the *arrs, Home Assistant; projects next ([proposal](personal-proposal.md)) |

Releases: [GitHub Releases](https://github.com/NatMarino/clawd-assistant/releases).
A tag with a suffix (`v0.2.0-beta.14`, `v0.3.0-alpha.1`) is a pre-release and
never becomes the "latest" download. Each release page says how to install and
what to test. `voice-lab` is a scratch branch for voice experiments; it isn't
part of the app.

## How he works

**The pet** is a Tauri 2 app: a transparent, always-on-top window that lives on
the taskbar (the Dock on a Mac). The page (`src/`) draws him, speaks and runs
the UI; Rust (`src-tauri/src/`) does windows, files, the network and running
programs.

**The big brain.** Claw'd reads nothing himself and holds no accounts. He runs
Claude Code headless, logged in as the user, in his Clawd folder
(`%USERPROFILE%\Clawd`, `~/Clawd`), and it reaches the user's claude.ai
connectors (`src-tauri/src/brain.rs`, `src/brain.js`):

- `claude -p --output-format stream-json --setting-sources project`, with:
  - **the instructions on standard input:** the skill plus the enabled packs.
    Windows caps a command line at ~32k characters, and
    `--append-system-prompt-file` is ignored in print mode (checked).
  - **the approvals in a settings file** (`--settings run-settings.json`).
    A work account's tool list alone passed the command-line limit
    (2026-09-25).
- **Approvals learn:** a refused tool that only reads, or changes the user's
  own things, is approved from then on (`allowed-tools.json`), and the run
  carries on once. Anything that reaches another person (send, post, reply,
  invite…) stops for a tap. Claude's start-up tool list can't be trusted: on
  the work laptop it came back empty because the apps connect later.
- Jobs: **setup** (in the egg), **sweep** (every 15–30 min in his hours,
  Haiku), **rundown**, **besties** (office), **request** (his chat, Sonnet).
  Models are in `src/brain.js`.

**What he's for comes from packs** (`skill/clawd/packs/<id>/PACK.md`, written to
`Clawd/packs/`, switched on in `Clawd/packs.json`): **office**, **home** and
**dev**. The core skill (`skill/clawd/SKILL.md`) is his voice, the ground rules,
the item format and the jobs. Anyone can add a pack: [packs.md](packs.md).
*(Packs exist on `home`; `main` still has the one office skill.)*

**Things reach him four ways:**
1. The big brain writes item files into `Clawd/inbox/`.
2. **The item door:** `POST 127.0.0.1:4318/items` with the token from
   `Clawd/http.json`.
3. **Doorways** (`home`): `/hooks/sonarr|radarr|overseerr|plex` translate
   those apps' own webhooks (`src-tauri/src/media.rs`).
4. **Claude Code hooks** (`home`, dev pack): `127.0.0.1:4317`, one state per
   session (`src-tauri/src/sessions.rs`, ported from the old coding Claw'd).

Home Assistant is also a tool for the big brain (`home`): its MCP server, via
`--mcp-config`, set in the gear.

## What he does (built)

- **Hatching:** he starts as an egg. Setup runs inside him:
  - "Where am I living?" (`home`)
  - chips built from the apps actually connected, with a reaction to every tap
  - the customize picker
  - the big brain sets him up while the egg cracks
  - you click the last crack
  - your name (and, at work, your besties)
- **A secret personality** (sweet, silly, serious, sassy, sleepy, dramatic),
  rolled per egg and never named. It changes his lines, animalese, idle
  habits and how the big brain writes for him (`src/personality.js`).
- **Voice:** the system's text-to-speech (default pitch 1.8, up to 2.0) or
  animalese (letters grouped into words; super deep, normal or high). One
  mouth: he never talks over himself.
- **His chat** (the **+**) and **drafts:** every message for someone starts
  as a card with Love it, Don't like it or Revise, then Send it or Copy it
  (plus Open in Claude and Open in Gmail). Send it is the approval, and it
  never sends in the run that wrote the draft. "Believe the user": what they
  say is done stays done. Big deliverables become a "hand this to Claude"
  item.
- **Taskbar life:** he walks (the scuttle), tucks in, peeks (never while
  talking), strolls, and knows which display he's on. He starts with Claude:
  starts at login, hidden, and pops out when Claude opens. Pick him up and
  he flails; drop him and he tumbles, then shakes it off.
- **Art:** Nat's full animation series (idle, eat, talk, hearts, mad, reading,
  pocket watch, parcel, celebrate, peek, scuttle) on both skins, imported by
  `tools/import-sheets.js`.

## Decisions (and why)

- **One Claw'd, customized by packs**, not separate apps: the egg, voice,
  taskbar life and chat are shared, so nothing is built twice.
- **Never signed:** both downloads stay unsigned. Unsigned-open instructions are
  in every release.
- **Voices:** the system's text-to-speech keeps its charm. eSpeak is out; Piper,
  JTalk (Mei reading English in katakana), Flite and SAM were all tried on
  2026-09-25, and none were good enough yet.
- **Nothing reaches another person without a tap**, in any mode.
- **Test days:** don't cut a beta per fix. Batch fixes into one beta after the
  day's fix log.

## Lessons learned the hard way

- **Smart App Control on Nat's laptop blocks anything freshly compiled:**
  release exes and even new build-time libraries. Local `cargo test` works
  for this project, and **CI builds every release**. Voice samples that needed
  new native code were rendered in CI too (`voice-lab` branch).
- **The Windows command line is limited to about 32k characters.** Long things
  go on standard input or in files.
- **Claude Code in print mode can't show permission prompts**, so anything not
  approved is refused. Hence the approvals file and learning.
- **Claude Code only reads the Clawd folder's own settings**
  (`--setting-sources project`). A person's `~/.claude/settings.json` has no
  effect on his runs.

## Working on it

- **Branches:** office fixes on `main`, then merge `main` into `home`.
  Personal features on `home`.
- **Test before committing:**
  - `cd src-tauri && cargo test` (`cargo test live_ -- --ignored` for a real
    run through Claude Code)
  - the page in a browser: `node tools/dev-server.js` → http://localhost:4870,
    or with a fake Tauri stub for the setup and chat flows
- **Releases:** push a `v…` tag and CI builds Windows (NSIS and exe) and a
  universal Mac dmg. A suffixed tag is a pre-release.
- **Logs on a user's machine:** `%APPDATA%\ClawdAssistant\` holds
  `brain.log` (every run, with the tools it was refused), `page.log`,
  `allowed-tools.json` and `inbox.json`.

## The other docs

| Doc | What |
|---|---|
| [personal-proposal.md](personal-proposal.md) | **Next:** the personal Claw'd feature list, for review. |
| [packs.md](packs.md) | Write your own pack. (`home`) |
| [home-setup.md](home-setup.md) | Connecting the VM's apps and Home Assistant. (`home`) |
| [voice-and-personality-plan.md](voice-and-personality-plan.md) | Built in beta 11 (one mouth, animalese pitch, the muffled egg, intake from connected apps, secret personalities). |
| [office-dev-plan.md](office-dev-plan.md) | Superseded by packs: dev became a pack on `home`. |
| [v0.3-plan.md](v0.3-plan.md) | Still future (as v0.4 or later): voice input, reminders, briefing, clipboard. |
| [phase0-checklist.md](phase0-checklist.md) | The first work-laptop checks, including the Claude Code check. |
| [../SETUP.md](../SETUP.md) | The fallback setup through Cowork, for a machine where he can't run the big brain himself. |
