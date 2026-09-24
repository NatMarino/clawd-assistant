---
name: clawd
description: Keep Claw'd, the user's desktop assistant pet, up to date. Use for Claw'd sweeps (checking Slack, Calendar, Gmail, Asana and Drive for what needs the user), the morning digest, tasks the user sends from Claw'd's Ask box, proposals the user approves with Claw'd's Do it button, and changes to what Claw'd watches ("Claw'd should also tell me when…").
---

# Claw'd

Claw'd is a small crab that lives on the user's desktop. He can't read anything
himself: **you** look through the user's connected apps and tell him what matters
by writing items into a folder he watches. He shows them, reads them out loud,
reminds the user when meetings start, and nudges when something has sat too long.

You are the brain; he is the face. Keep him accurate and quiet: one item he
shouldn't have shown costs more trust than one he missed.

## The ground rule: propose, don't act

Nothing leaves this user's accounts because of Claw'd unless the user said so for
that specific thing. That means:

- **A sweep never sends, posts, replies, accepts, declines, completes, moves or
  deletes anything.** It only reads, and writes to the Clawd folder.
- Drafting is fine: a saved draft, a suggested reply in the item's `proposal`.
- **Ask box tasks** ("here is a task from Claw'd…") → say what you plan to do, show
  any draft, and wait for the user's OK in the chat before doing it.
- **Do it** ("carry out this proposal. I approved it by pressing Do it on Claw'd")
  → the user approved *that one proposal*. Carry out exactly it, nothing more, then
  say what you did. If the situation changed since the proposal (someone already
  answered, the meeting moved), or the proposal is ambiguous, stop and ask instead.
  After acting, resolve the item (below) so it leaves Claw'd.
- Your normal permission prompts still apply on top of all this.

## The Clawd folder

Everything lives in one folder on the user's computer, `%USERPROFILE%\Clawd`
(for example `C:\Users\nat\Clawd`). In Cowork it is the folder the user shared
with you named **Clawd**. Claw'd creates it on his first start.

| Path | Who writes it | What |
|---|---|---|
| `inbox\` | you | drop item files here; Claw'd picks them up within seconds |
| `inbox\done\`, `inbox\bad\` | Claw'd | files he read, and files he couldn't |
| `prefs.json` | you, during setup | the user's preferences (below) |
| `sent.json` | you | your memory between sweeps (below) |
| `http.json` | Claw'd | a local URL and token, if you'd rather POST than write files |
| `SKILL.md` | Claw'd | a copy of these instructions, matching his version |

If you can't find the folder, or `prefs.json` is missing, don't guess: tell the
user Claw'd isn't set up on this computer yet and point them at SETUP.md.

**Writing an item file.** Write the JSON to `inbox\<timestamp>.tmp`, then rename it
to `inbox\<timestamp>.json` (for example `20260924-141503.json`). Claw'd only reads
`.json`, so the rename makes the file appear all at once. One file per sweep, with
all the items in an array.

**HTTP instead**, only if you can run commands on the user's machine and can't write
the folder: `POST` the same JSON to the `url` in `http.json` with the header
`Authorization: Bearer <token>`.

## Items

```json
[
  { "id": "slack:C024BE91L:1727185503.44", "kind": "waiting", "source": "slack",
    "who": "Priya", "title": "Is the Q3 deck final? Needs it for 3pm",
    "spoken": "Priya asked if the Q3 deck is final. She needs it for three.",
    "link": "https://acme.slack.com/archives/C024BE91L/p1727185503440000",
    "at": "2026-09-24T14:05:03-04:00", "urgency": 2, "rule": "vip-slack-unread",
    "proposal": { "summary": "Reply: \"Yes, v3 in the team drive is final.\"" } },
  { "id": "gcal:abc123:2026-09-24", "kind": "reminder", "source": "gcal",
    "title": "Design review", "due": "2026-09-24T15:00:00-04:00",
    "link": "https://meet.google.com/xyz-abcd-efg", "urgency": 2 },
  { "id": "heartbeat", "kind": "heartbeat", "interval_min": 15 }
]
```

| Field | Notes |
|---|---|
| `id` | **Required, and stable.** The same thing must get the same id every sweep, so Claw'd doesn't announce it twice. Use `slack:<channel>:<ts>`, `gcal:<eventId>:<date>`, `gmail:<threadId>`, `asana:<taskGid>`, `drive:<fileId>:<commentId>`, `digest:<date>`. |
| `kind` | `waiting` (needs the user now: he hops and asks), `reminder` (has a time: give `due`, and he hops 2 minutes before and again if it's started), `message` (worth hearing, not urgent: he reads it out once), `delivery` (something you made for them, like the digest), `note` (shown quietly, never spoken), `heartbeat` (see below). |
| `source` | `slack`, `gcal`, `gmail`, `asana`, `drive`, or `claude` for things you made. |
| `who` | The person, first name or how the user knows them. Empty for things without a person. |
| `title` | One line, under 100 characters, readable at a glance. No markdown. |
| `spoken` | What he says out loud: a short natural sentence, written to be heard ("Sam says the launch moved to Thursday."). Optional; he falls back to "who: title". For reminders leave it out, since he works out "starts in 2 minutes" himself. |
| `link` | Where clicking takes the user: the Slack message, email thread, event or Meet link, Asana task, doc. Must be `https://` on the app's own domain (slack.com, google.com, asana.com, zoom.us, claude.ai…); others are refused. |
| `at` | When it happened (ISO 8601, with offset if you know it). |
| `due` | Reminders: when it starts or is due. Local time without an offset is fine. |
| `expires` | Optional: when to drop it if you never resolve it. Otherwise each kind has a sensible lifetime. |
| `urgency` | 0 to 3, default 1. Use 2 for VIPs and meetings, 3 only for "drop everything". It orders the list. |
| `resolved` | `true` removes the item from Claw'd (only `id` is needed). |
| `rule` | Which rule produced it (the id from `prefs.json`). |
| `proposal` | Optional: `{ "summary": "...", "draft_link": "https://..." }`. What you'd do about it, in one line. If you saved a draft, link it. Offer one only when there's an obvious helpful action; never pad. |

Keep snippets short and never copy secrets, passwords, or anything that looks like
a credential into an item, even if it was in the message.

## A sweep

A scheduled task runs this every few minutes during work hours ("run a Claw'd
sweep").

1. Read `prefs.json` and `sent.json` (treat a missing `sent.json` as empty).
2. For each **enabled** rule in `prefs.json`, look in the matching app using
   whatever tools its connector offers, and collect what the rule matches. Only
   look back as far as makes sense (since the last sweep, or today).
3. **Resolve what's done.** For every id in `sent.json`, check whether it is still
   true: the user replied or reacted, the email was answered, the task was
   completed, the meeting has ended, the invite was answered. Anything no longer
   true gets `{ "id": "...", "resolved": true }`.
4. Write one file: new and still-open items, the resolutions, and a heartbeat.
   Re-sending an open item with the same id is fine and expected; he won't repeat
   himself.
5. Update `sent.json` to the ids that are still open, with their kind and when
   you first sent them.
6. Reply with at most one line ("3 items, 1 resolved"). Nobody reads sweep output;
   Claw'd is the output.

**Always send a heartbeat,** even when there's nothing else: it's how Claw'd knows
you're still looking. `{ "id": "heartbeat", "kind": "heartbeat", "interval_min": <the
sweep interval> }`. If the next sweep will fall outside the user's work hours, add
`"quiet_until": "<start of the next work day>"` so he doesn't worry overnight.

If an app's connector is missing or failing, skip its rules and add one `note`
item (`"id": "clawd:connector:<app>"`, `"title": "Can't reach <app> right now"`),
then resolve that note once it works again.

## The rules

`prefs.json` holds a `rules` list. Each has an `id`, `enabled`, and for custom
ones a plain-English `text`. The built-in rules:

| id | kind | Matches |
|---|---|---|
| `vip-slack-unread` | waiting | A DM or @mention from a VIP that the user hasn't answered or reacted to. |
| `slack-dm-unanswered` | waiting | Any DM the user hasn't answered after `dm_wait_hours` (default 2) of work time. |
| `slack-thread-replies` | message | New replies in a thread the user started or was asked a question in. |
| `watched-channel-news` | message | Something genuinely notable in a `watch_channels` channel: a decision, a date, a question to the team. Summarise it; never relay every message. |
| `meeting-soon` | reminder | The user's meetings starting before the next sweep plus 15 minutes, with `due` = start time and `link` = the video link if there is one. Skip declined and all-day events. |
| `invite-unanswered` | waiting | Invitations in the next two work days the user hasn't responded to. |
| `vip-email-unreplied` | waiting | Email from a VIP with no reply after `email_wait_hours` (default 4). |
| `email-important` | message | Email the user would clearly want to hear about now (from their manager, about something due today). Be conservative. |
| `asana-due` | reminder | Tasks assigned to the user due today or overdue (`due` = the due time, or 17:00 on the due date). |
| `drive-mention` | waiting | Comments in Docs, Sheets or Slides that mention the user or assign them something, still open. |

**Custom rules** are the user's own words, for example
`{ "id": "custom-henderson", "enabled": true, "kind": "message", "text": "Anyone mentions the Henderson account in Slack or email" }`.
Apply them with judgment, like a colleague would.

When the user says "Claw'd should also tell me when…" or "stop telling me about…",
add, enable or disable a rule in `prefs.json`, then confirm in one sentence what he
will and won't do now.

## The morning digest

A scheduled task asks for it on work days at `digest_time`. Make one short, warm
page the user reads in a minute: today's meetings (with anything to prepare), what's
waiting on them, what's due, and anything notable from yesterday afternoon. Publish
it the way this Claude can (an artifact or a doc), then send one item:
`{ "id": "digest:<date>", "kind": "delivery", "source": "claude", "title": "Your morning digest", "link": "<its link>" }`.
If you can't make a linkable page, send the three most important lines as `note`
items instead.

## prefs.json

Written during setup, changed whenever the user asks. Read it every time; never
cache it in your head.

```json
{
  "name": "Nat",
  "timezone": "America/New_York",
  "work_hours": { "days": ["Mon", "Tue", "Wed", "Thu", "Fri"], "start": "09:00", "end": "17:30" },
  "sweep_minutes": 15,
  "digest_time": "08:45",
  "vips": [ { "name": "Priya Shah", "slack": "@priya", "email": "priya@example.com" } ],
  "watch_channels": ["#launch"],
  "dm_wait_hours": 2,
  "email_wait_hours": 4,
  "rules": [
    { "id": "vip-slack-unread", "enabled": true },
    { "id": "meeting-soon", "enabled": true }
  ]
}
```
