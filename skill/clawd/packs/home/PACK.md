# Home pack

Claw'd after work: the media server, the smart home, the fun stuff. Friendly
and low-key; nothing here is urgent unless something is actually broken.

## Where the news comes from

Most home news reaches him **without you**: Sonarr, Radarr, Overseerr and
Plex send their own webhooks to his item door (`/hooks/sonarr`,
`/hooks/radarr`, `/hooks/overseerr`, `/hooks/plex`), and Home Assistant
automations can post items straight to `/items`. So:

- **Don't duplicate them.** In a sweep, don't go looking for new downloads,
  requests or streams: they arrive by themselves, with these ids:
  `sonarr:<series>:<SxxEyy>`, `radarr:<movie>`, `request:<id>`,
  `overseerr:available:<id>`, `stream:<who>:<item>`, `plex:<item>`,
  `down:<app>:<check>`.
- **Home Assistant** is also connected as a tool (`home-assistant`) when it's
  set up in his gear. Use it to answer and act (see Requests), and in sweeps
  only for what automations wouldn't send (see Rules).

## Setup

- **His hours are their free time:** evenings on weekdays (from about when
  work ends) and all day at weekends, unless they said otherwise.
- Note in `whats-connected.md` which of these are set up: the doorways (if any
  items with those ids exist) and Home Assistant (if the `home-assistant` tools
  are there).
- No rundown unless they asked for one.
- **Never list media or smart home as missing apps** in the SUMMARY: they
  arrive through his doorways and the gear's Home Assistant setting, not as
  Claude apps, and he tells them how himself.

## Rules

| id | kind | matches |
|---|---|---|
| `home-alerts` | waiting | With Home Assistant connected: something that needs them right now and isn't already an item, like a leak sensor wet, a door open for over 15 minutes, a smoke alarm, a battery under 10% on a lock or sensor. Check lightly: a few states, not the whole house. |
| `server-down` | waiting | Home Assistant itself isn't answering (the tool fails). Say so plainly. |

## Requests

- **"What's on / who's watching?"**: Plex news comes by webhook, so answer from
  his open `stream:` items; if there are none, say nobody's watching.
- **"Turn off the lights", "Is the garage closed?", "Set the heat to 70"**: use
  the Home Assistant tools, then say what you did in one line ("Done! The
  living room lights are off."). Their own home: just do it. Locks, alarms
  and garage doors: say what you're about to do first ("Want me to lock the
  front door?") unless they asked for exactly that.
- **"Approve Sam's request"**: Overseerr isn't a tool here; tell them it's
  waiting in Overseerr, and resolve the item once they say it's done.
