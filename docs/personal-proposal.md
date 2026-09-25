# Personal Claw'd: proposed features (draft 1)

*Status: **a proposal, nothing here is built yet** unless it says so. Written
2026-09-25 from Nat's wish list, for Nat and the Claude on the clawgoose VM to
review. Items marked ★ are suggestions, not requests: cut freely.*

*Background: [STATUS.md](STATUS.md) explains what Claw'd is today, how he works
and what's already built. The personal version lives on the `home` branch
(v0.3 alphas); the office version on `main` (v0.2 betas).*

## What he's for

A little companion that **keeps Nat's projects and home moving**, lives on
**every device** she uses, and stays quiet unless something actually matters.
Not an office assistant: no work besties, no work apps, no work data.

## 1. On every device (the foundation)

Today each Claw'd is alone on one machine: his inbox, chat and brain all live
there. The personal one should be the same pet on the laptop, the desktop and
the iPhone, in sync, and in touch with the VM.

- **Claw'd Hub on the clawgoose VM.** One brain, one inbox, one chat, always
  on. It runs the checks and the big brain (Claude Code, logged in to Nat's
  personal account), and holds the doorways (Plex, Sonarr, Radarr, Overseerr,
  Home Assistant).
- **Laptop and desktop pets become views of the hub:** the same state
  everywhere. Dismiss something once and it's gone everywhere.
- **iPhone Claw'd:** a home-screen web app (a PWA) served by the hub, with the
  pet, his panel, chat, and real notifications (iOS supports web push for
  home-screen apps).
- **Tailscale** for private access from anywhere, with nothing exposed to the
  internet. It replaces the SSH tunnel (`tools/vm-tunnel.ps1`).
- **The work laptop stays completely separate:** no work data ever reaches the
  personal hub.
- ★ **Quiet sync:** only the device in use speaks out loud; the others just show
  it.

## 2. Talking to him

- **Click him → one panel with two tabs: Notifications and Chat.** The cookie
  and chat buttons appear on click, not only on hover (the tiny **+** is gone).
- **Chat that remembers the thread** across devices (it lives in the hub).
- ★ **Quick capture:** "idea for Mapache TV: Halloween bumpers" is filed under
  that project, not lost in chat. Good from the phone.
- ★ **Talk to him:** hold a key on the computer, or tap the mic on the phone.

## 3. Projects

Nat's projects: **Casa** (a Home Assistant dashboard, built in the clawgoose VM),
**Closet Vampy**, **Dream Mapache Network TV** (an ErsatzTV live channel on
Plex, with bumpers and commercials), and **ITRX** (a side business: site
development, Zoho email).

- **Project board:** each project with a status (active, paused, done), "last
  touched 3 days ago", and what's in progress. Sources: Claude Code sessions
  (his hooks already see them, including the VM's through the tunnel), git
  commits, and the VM.
- **Where you left off:** when a coding session ends, he saves what got done and
  what's next. Click a project: the recap, then **Continue**, which opens a
  Claude Code session in that project.
- **Stalled nudges:** "Mapache TV hasn't moved in 8 days. Pick it back up
  tonight?" Active projects only, at most one a day, in free time.
- ★ **Next steps per project,** updated from sessions or by telling him ("next
  for Casa: the bills card").
- ★ **Sunday recap:** what moved this week and what stalled, in one short
  message. An alternative to nudges for fewer interruptions.

Per project (the VM Claude knows these far better; see section 7):

| Project | Proposed |
|---|---|
| Casa | Dev status; **bills and deadlines** (source to decide). |
| Closet Vampy | Dev status. *What else? To be described.* |
| Mapache TV | Dev status; ★ a channel down or not streaming; ★ a to-make list of bumpers and commercials. |
| ITRX | Site dev status; ★ site down; ★ domain and hosting renewals; **Zoho email:** tell her when mail needs a reply, and draft replies with the Love it → Send it flow. |

## 4. Life admin

- **Deadlines and bills:** due dates from email, a calendar, or a list Nat gives
  him; a heads-up a few days before and on the day.
- **Reminders:** "remind me Friday to renew the domain". *Built.*
- ★ **Evening rundown** (not morning): "Tonight: the Casa bill is due tomorrow,
  and Closet Vampy's been waiting 5 days." Off by default.

## 5. Home and media (built, in v0.3.0-alpha.1)

- New episodes and movies, requests needing approval, who's streaming, and
  server problems, all through the doorways. Setup: [home-setup.md](home-setup.md).
- Home Assistant: "is the garage closed?", "turn off the lights" (locks, alarms
  and garage doors: he checks first).

## 6. Fun and personality

- **Dance to music:** he notices music playing (Windows' media controls say what
  and whether) and hears the beat from the system's audio, then picks a slow,
  medium or fast dance. Nat draws the 2–3 dances.
- Built: the secret personality, voices and animalese (in word groups), cookies,
  walking the taskbar, hearts, the mad face.
- ★ **Celebrates wins:** a party when a session finishes a big task or a project
  ships.
- ★ **Sleepy at night:** tucks in and naps after bedtime, and stops nudging.

## Not in this version

New voice engines (tried Piper, JTalk, Flite and SAM on 2026-09-25; none
sounded right yet, revisit later), the robot filter, a native iPhone app (the
web app covers it), and anything that touches work data.

## Open questions

1. **Closet Vampy:** what is it, and what would Nat want to know about it?
2. **Casa bills:** where do bills and deadlines live today (email, the Casa
   dashboard, Home Assistant, a spreadsheet, nowhere yet)?
3. **ITRX:** where's the site hosted? Is Zoho the only ITRX inbox?
4. **Nudges:** to the phone, the computer, or both? Which hours are free time?
5. **Main device** in the evenings: laptop, desktop or phone?
6. **The hub:** how much of the VM can it use (it's a small always-on service
   plus Claude Code runs)? Is Tailscale OK to install on the VM, laptop,
   desktop and phone?

## 7. For the VM Claude

You have the context on Casa, Closet Vampy, Mapache TV and ITRX that the
laptop session doesn't. Please:

1. **Write a one-page brief per project** in `docs/projects/<name>.md` on the
   `home` branch: what it is, where it lives (paths, repos, services, ports),
   its current state, what's next, and what Nat would plausibly want Claw'd to
   watch or remind her about. No secrets, tokens or personal data: the repo is
   public.
2. **Answer the open questions you can** (especially 1–3 and 6) right in this
   file, or in the briefs.
3. **Mark up this proposal:** what's realistic given how the projects actually
   work, what's missing, what's pointless.
4. **Don't build app features yet.** Nat wants the feature list settled first.
   When building starts, the conventions are in [STATUS.md](STATUS.md#working-on-it):
   `home` branch for personal work, CI builds the releases, small commits.

Once the list is settled, the next step is a phased plan, with the hub first,
since everything else sits on it.
