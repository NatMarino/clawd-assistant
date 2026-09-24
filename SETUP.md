# Setting up Claw'd, for Claude

*Someone sent this file and asked you to set Claw'd up. These instructions are
written to you, their Claude. Work through them with the person, one step at a
time. It takes about ten minutes.*

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
3. Claw'd waves hello. He'll then go **grey with a question mark**. That's
   expected: he hasn't heard from you yet.

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

## Step 4: Learn their preferences

Ask these questions a few at a time, conversationally, and keep it quick. Offer
the defaults in brackets.

1. What should Claw'd call you?
2. Who are your VIPs, the people whose messages should never wait? Name plus
   Slack handle or email, whichever you know. Look them up yourself where you can.
3. Any Slack channels where you'd like to hear notable news? (none)
4. Which days and hours do you work, and in what time zone? (Mon–Fri, 9:00–17:30,
   this computer's time zone)
5. How often should I check? (every 15 minutes)
6. Want a morning digest? What time? (yes, 15 minutes after you start)
7. Then show the list of what he can watch for, with the default on/off, and let
   them switch any:

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

   And ask: "Anything else you'd like him to watch for?" Turn each answer into a
   custom rule, in their own words.

Write the answers to `Clawd\prefs.json` in the format shown at the end of
`Clawd\SKILL.md`, with the rule ids from its rules table. Read the file back to
them in plain words, not JSON.

## Step 5: Install the skill

The instructions for looking after Claw'd are a skill called **clawd**. The
scheduled checks can read them straight from `Clawd\SKILL.md`, but installing the
skill lets you understand Claw'd in every chat, including the ones his Ask box and
Do it button open.

Ask them to download `clawd-skill.zip` from the same release page and add it in
Claude's settings under **Capabilities → Skills**. If they'd rather not, or can't,
carry on: the scheduled tasks below still work.

## Step 6: Schedule the checks

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

## Step 7: Say hello through him

1. Write a test file into `Clawd\inbox` (named with a `.tmp` extension, then renamed to `.json`):
   ```json
   [
     { "id": "setup-hello", "kind": "message", "source": "claude",
       "title": "Claw'd is connected", "spoken": "Hi! Claude and I are connected now." },
     { "id": "heartbeat", "kind": "heartbeat", "interval_min": 15 }
   ]
   ```
   (Use their real sweep interval.)
2. Ask them: did he stop being grey and say hello? If yes, clear the test by
   writing `{ "id": "setup-hello", "resolved": true }`.
3. Run one real sweep now, following the skill, so they see real items straight away.

If he didn't react:
- A file stuck in `inbox` means Claw'd isn't running. Ask them to start **Claw'd**
  from the Start menu.
- A file in `inbox\bad` means it wasn't valid JSON. His popover (click him) shows
  the error. Fix it and try again.
- A file that moved to `inbox\done` but no reaction usually means the sound is off:
  click him, then the gear, and check **voice: on**.

## Step 8: Hand over

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
- Drag him anywhere. `+` and `-` resize him while he's selected, and `q` quits.
