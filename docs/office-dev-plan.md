# One Claw'd: office and dev modes

Planned 2026-09-24, for right after v0.2.0 ships (before the voice, reminders,
briefing and clipboard work in `v0.3-plan.md`, which moves back one release).

## What Nat asked for

- He knows when Claude is **thinking** and when it's **working**, and shows it,
  even where there are no hooks to say so ("at least pretend").
- The two apps become **one Claw'd with a toggle**: **office** (this app: your
  apps, reminders, rundowns, requests, Cowork sessions) and **dev** (the coding
  Claw'd: Claude Code sessions).

## Decisions

- **Dev mode is both, sessions first.** Claude Code sessions lead his popover
  and his reactions (hop for an ask, flag when done). His office items stay
  underneath. Office mode is office items plus Cowork sessions.
- **The old coding Claw'd retires.** `NatMarino/clawdbot` gets a note pointing
  here, and its credits carry over.

## Thinking vs working, by source

| Source | How he knows | Honest? |
|---|---|---|
| His own big brain jobs (setup, requests, checks) | The run's event stream: a tool in use means **working** (he types on his laptop); writing means **thinking** (the dots) | Exact |
| Claude Code sessions (dev) | Claude Code hooks on port 4317, as the coding Claw'd does it | Exact |
| Cowork sessions (office) | The coding Claw'd's Cowork watcher (the Claude app's notifications and log): **busy**, and **needs your answer**. Thinking vs working inside "busy" is a believable guess from how the log moves | Busy and asks are real; thinking vs working is pretend |

## How the merge works

This app is the base: it has the taskbar life, voice, the big brain, the egg
and onboarding. From `clawdbot` come:
- the hooks server (`hooks.rs`)
- the session reducer (thinking, working, done, needs input, error, and
  jump-to-session links)
- the Cowork watcher (`cowork/`)

All of it arrives as the **sessions** feed, beside the existing items feed.

- **The toggle:** gear → *mode: office / dev*, with a short line from him when
  switched. Dev defaults to the CLI skin, office to whatever you chose.
- **Hooks:** switching to dev checks for the Claude Code hooks, and offers to
  install them (with your OK: it edits `~/.claude/settings.json`, keeps a
  backup, and can undo it). The coding Claw'd's `merge-hooks.js` logic moves
  into the app.
- **One app listens on 4317.** The merged app replaces the old one, and setup
  says so if the old one is still running.
- **The popover:** in dev, a **Sessions** section on top (as the coding Claw'd
  lists them), then his office sections.

## Order

1. v0.2.0 ships (current betas).
2. This merge, as **v0.3**.
3. Voice, reminders, briefing and clipboard (`v0.3-plan.md`), as **v0.4**.
