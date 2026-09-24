# Setting up Claw'd, for Claude (the fallback)

*Normally Claw'd sets himself up: the person clicks his egg and he runs setup
himself. You're reading this because he couldn't reach Claude Code on this
computer, and asked them to paste his setup message to you. These instructions
are written to you, their Claude. Keep it warm, quick and easy. The best part is
theirs: hatching him.*

Throughout, call Claude Code / yourself "the big brain" when you speak as him.
He says it that way because "Claude" and "Claw'd" sound the same out loud.

## Step 1: What's connected

See which apps you can reach (messages, email, calendar, tasks, docs, meeting
notes like Granola or Otter…). Try one harmless read in each; don't show the
results. You don't need everything connected: only what serves what they want
(step 3).

## Step 2: The Clawd folder

Claw'd is already installed. He's the egg on their taskbar, and he created
`C:\Users\<them>\Clawd`. Ask them to share that folder with you (in Cowork, add it
as a folder you can work in). Continue once you can write to `Clawd\inbox`.

## Step 3: How they want to use him

Ask one question, conversationally, and let them pick any:

> What would you like Claw'd's help with? Keeping up with messages, your meetings
> and calendar, reminders for little things, a morning rundown, tasks and
> deadlines, or something else?

Only if it isn't obvious from their apps, one follow-up per pick: which messages
matter most (DMs, mentions, a few channels), and what time the rundown should be
ready. **Don't** ask about VIPs, check frequency or work hours: take VIPs from
Slack's own VIP list (or their manager and the people they DM most), and hours
from their calendar.

## Step 4: Set him up

Follow the **setup job** in `Clawd\SKILL.md` (you can skip the STAGE markers and
the SUMMARY line; those are for when he runs it himself):
- write `prefs.json` for exactly their uses, with `sweep_minutes: 30`
- write `whats-connected.md`
- tell them in two plain sentences what he'll watch for, and anything a pick
  needs that isn't connected

## Step 5: Schedule his checks

He can't run the big brain himself here, so you schedule it. Create in this
person's Claude:

- **Claw'd sweep**: every 30 minutes on their work days, within their hours:
  > Run a Claw'd sweep. Follow SKILL.md in the Clawd folder (Job: sweep).
- **Claw'd rundown**: once each work day at their rundown time, if they want one:
  > Make today's Claw'd morning rundown. Follow SKILL.md in the Clawd folder (Job: rundown).

## Step 6: Let them hatch him

Wake him by writing a heartbeat into `Clawd\inbox` (write `hello.tmp`, then
rename it to `hello.json`):

```json
{ "id": "heartbeat", "kind": "heartbeat", "interval_min": 30 }
```

The egg cracks and starts wobbling. Tell them: "Look at your taskbar. He's ready.
Click the egg to hatch him!" From there he takes over, and asks their name and
their work besties himself. Then run one sweep so he has something to tell them.

If the egg doesn't change:
- A file stuck in `inbox` means Claw'd isn't running. Start **Claw'd** from the
  Start menu.
- A file in `inbox\bad` wasn't valid JSON. Fix it and try again.

## Step 7: Hand over

Briefly:
- **Click him** for everything he's tracking, and **+** for his requests chat.
- **The gear:** his voice, theme and skin, and **auto** (he does what they ask
  right away; anything to another person waits for their OK) or **ask me first**.
- To change what he watches, tell him: "Claw'd, also tell me when…".
- He lives on the taskbar by the clock. The tray icon (the crab) has **Quit** and
  **Start over**.
