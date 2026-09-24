# Setting up Claw'd, for Claude

*Someone sent this file and asked you to set Claw'd up. These instructions are
written to you, their Claude. Work through them with the person, one step at a
time. It takes about fifteen minutes, and the best part is theirs: hatching him.*

## What Claw'd is

Claw'd is a small crab that sits on top of every window on this person's
computer. He can't read their email or Slack himself, and never will. **You**
check their connected apps on a schedule and tell him what matters by writing
small files into a folder on their computer. He shows it, reads it out loud,
reminds them when meetings start, and nudges when something has waited too long.
When they type a task into him, or press **Do it** on one of your suggestions, he
opens a chat with you with the words already filled in.

Nothing in Claw'd is tied to whoever sent this file. Everything personal comes
from this person's own answers and their own connections.

Start by telling them in two or three sentences what's about to happen, and that
you will ask a few questions along the way.

## Step 1: Check the connections

Work out which of these apps you can reach right now: **Slack, Google Calendar,
Gmail, Asana, Google Drive**. Try one harmless read in each (their profile, today's
events), and don't show them the results.

Tell them which ones work. For any that are missing, tell them they can connect it
in Claude's settings under **Connectors** and come back, or carry on without it.
Claw'd works with whatever is connected, and apps can be added later.

## Step 2: Install Claw'd

You can't install software for them, so walk them through it:

1. Open **https://github.com/NatMarino/clawd-assistant/releases/latest** and
   download `ClawdAssistant_<version>_x64-setup.exe`.
2. Run it. Windows will probably say **"Windows protected your PC"**, because the
   app isn't signed with a paid certificate. Click **More info**, then **Run
   anyway**. It installs for this user only and doesn't need an administrator.
3. Claw'd appears on the taskbar as **an egg**. That's expected: he hatches once
   you've connected him (step 5), and they get to hatch him.

If Windows refuses the file outright with no "Run anyway" option, Smart App
Control or the company's IT policy is blocking it. Stop there. They'll need to
ask IT, and you can tell them that's the only fix.

## Step 3: Share the Clawd folder with you

On first start Claw'd creates a folder called **Clawd** in their user folder
(`C:\Users\<them>\Clawd`). It holds `README.txt`, `SKILL.md`, `http.json` and an
`inbox` folder.

Ask them to give you access to that folder. In Cowork, add it as a folder you can
work in. Then check that you can see `README.txt` and the `inbox` folder. **Only
continue once you can write to `inbox`.**

## Step 4: Install the skill

The instructions for looking after Claw'd are a skill called **clawd**. The
scheduled checks can read them straight from `Clawd\SKILL.md`, but installing the
skill lets you understand Claw'd in every chat, including the ones his Ask box and
Do it button open.

Ask them to download `clawd-skill.zip` from the same release page and add it in
Claude's settings under **Capabilities → Skills**. If they'd rather not, or can't,
carry on: the scheduled tasks below still work.

## Step 5: Wake him up

Connect him by writing a heartbeat into `Clawd\inbox` (write it as `hello.tmp`,
then rename it to `hello.json`):

```json
{ "id": "heartbeat", "kind": "heartbeat", "interval_min": 15 }
```

Within a few seconds the egg starts to wobble, and a bubble says *Something's
moving in here… Click me to hatch me!* Tell them that's their cue: **they hatch
him themselves**, with a click on the egg.

If the egg doesn't wobble:
- A file stuck in `inbox` means Claw'd isn't running. Ask them to start **Claw'd**
  from the Start menu.
- A file in `inbox\bad` means it wasn't valid JSON. Fix it and try again.

## Step 6: Let him get to know them

Once he hatches, Claw'd introduces himself out loud and asks, one at a time:
what to call them, who their VIPs are, which Slack channels are worth hearing
about, when they work, whether they'd like a morning rundown (and when), and
anything else he should keep an eye on. They type each answer into his speech
bubble. This part is his, not yours, so stay quiet and let them chat with him.

When he's done he says so, and writes what he learned to `Clawd\intro.json`:

```json
{ "version": 1, "name": "Nat",
  "answers": { "name": "Nat", "vips": "Priya, Stephen", "channels": "#launch",
               "hours": "Mon to Fri 9 to 5:30", "digest": "yes 8:45",
               "watch": "who's checking into a hotel" },
  "completed_at": "…" }
```

Ask them to tell you when he's finished, then read that file.

## Step 7: Turn it into his settings

From `intro.json`, work out `prefs.json` (the format is at the end of
`Clawd\SKILL.md`):

- `name`: exactly as they gave it. He already uses it, and your spoken lines should too.
- **VIPs**: look each name up in Slack and email, and store their handle and address.
  Ask only about names you can't place.
- **Channels, work hours, digest time**: from their answers. Take the time zone from
  this computer. A blank or "no" digest means no digest task.
- **Anything else**: each thing becomes a custom rule, in their own words.
- **Check how often**: every 15 minutes unless they said otherwise.

Then show them, in the chat, the list of what he can watch for, with each rule's
default on or off, and let them switch any:

| Rule | Default |
|---|---|
| Unread Slack message from a VIP | on |
| Meetings about to start (and nudges if you're late) | on |
| Emails from a VIP with no reply after 4 hours | on |
| Asana tasks due today or overdue | on, if Asana is connected |
| Comments that mention you in Google Docs | on, if Drive is connected |
| Meeting invites you haven't answered | on |
| Replies in Slack threads you started | off |
| Any Slack DM unanswered for 2 hours | off |
| Notable news in the channels above | on, if they named any |
| Important emails (your manager, things due today) | off |

Write `Clawd\prefs.json`, with the rule ids from the skill's rules table, and
read it back to them in plain words, not JSON.

## Step 8: Schedule the checks

Create two scheduled tasks in this person's Claude:

- **Claw'd sweep**: every `sweep_minutes` minutes, on their work days, between their
  start and end time. Prompt:
  > Run a Claw'd sweep. Follow the clawd skill (or, if it isn't installed, the
  > instructions in SKILL.md in the Clawd folder). Don't send, reply to or change
  > anything.
- **Claw'd digest**: once each work day at their digest time (skip it if they said
  no). Prompt:
  > Make today's Claw'd morning digest. Follow the clawd skill (or SKILL.md in the
  > Clawd folder).

If the scheduler can't limit tasks to work hours, schedule the sweep every
`sweep_minutes` minutes anyway. The skill sends a `quiet_until` so he rests
overnight, and a sweep outside work hours should send only a heartbeat.

Then **run one real sweep now**, following the skill, so they meet him with real
things to tell them about.

## Step 9: Hand over

Tell them, briefly:

- **Click him** for everything he's tracking. Click a row to open it, and ✓ to clear it.
- **Hover him, then press +** to ask for anything. You'll show your plan before
  doing it.
- **The gear** holds the voice, theme and skin, and the **mode**. In **Propose**
  mode (the default) your suggestions come with *Talk it over*. Switching to
  **Do it** mode puts a hard hat on him and adds a **Do it** button, which hands one
  specific suggestion to you to carry out.
- To change what he watches, tell you: "Claw'd should also tell me when…" or
  "stop telling me about…".
- He lives on the taskbar, by the clock. Drag him anywhere, and drop him back near
  the taskbar to send him home. `+` and `-` resize him while he's selected, and the
  tray icon (the crab by the clock) has **Quit**.
