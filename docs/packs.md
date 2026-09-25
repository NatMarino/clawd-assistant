# Packs: teach Claw'd a new job

Claw'd himself knows nothing about offices, media servers or code. What he
watches for, and what counts as "this needs you", comes from **packs**:
plain-English instructions for the big brain (Claude) that runs his checks.
You don't need to write any code to make one.

He comes with three:

| Pack | What it's for |
|---|---|
| **office** | Work messages, meetings, email, tasks, docs, meeting notes, and a morning rundown. |
| **home** | Plex and the *arrs, Home Assistant, and the fun stuff after work. |
| **dev** | Claude Code sessions (working, thinking, needs you), pull requests and CI. |

Turn them on and off in his gear (**packs:**). Setup turns on the ones that
fit what you told him.

## Where packs live

In your Clawd folder (`%USERPROFILE%\Clawd` on Windows, `~/Clawd` on a Mac):

```
Clawd/
  packs.json            which packs are on: { "enabled": ["home", "dev"] }
  packs/
    office/PACK.md      the built-in ones (rewritten when he starts)
    home/PACK.md
    dev/PACK.md
    minecraft/          one of yours
      PACK.md
      pack.json         optional: a name and a line for the gear
```

Edits to the built-in packs don't stick: he rewrites them each time he
starts. Copy one to a new folder to make your own version.

## Writing one

A PACK.md is a note to Claude. Say what the pack is for, which apps it uses,
and what should reach you. Three sections help Claude the most:

- **Setup:** anything to work out once, during setup (which server, whose
  messages matter, what hours he's on duty).
- **Rules:** what each check looks for. Give each rule an id and a kind:
  `waiting` (he hops and asks), `reminder` (has a time), `message` (worth
  hearing), `note` (shown quietly).
- **Requests:** how to answer things you'll ask him.

Claude only sees the tools you've connected (claude.ai connectors or MCP
servers), so a pack can only use apps it can reach.

### Example: a Minecraft server

`packs/minecraft/PACK.md`:

```markdown
# Minecraft pack

Keeps an eye on my Minecraft server (the "minecraft" MCP server).

## Rules

| id | kind | matches |
|---|---|---|
| `mc-friends` | message | One of my friends joined the server: "Sam just hopped on Minecraft!" |
| `mc-empty` | note | The server has been empty for an hour. |
| `mc-down` | waiting | The server isn't answering. |

## Requests

- "Who's on?": list the players in one line.
```

`packs/minecraft/pack.json`:

```json
{ "name": "Minecraft", "blurb": "Who's on my server, and whether it's up" }
```

### Example: no app at all, just reminders

`packs/plants/PACK.md`:

```markdown
# Plants pack

In every morning sweep, if it's Monday or Thursday, write a reminder at
10:00: "Water the plants! The fern is thirsty." (id `plants:<date>`).
```

## Things that aren't Claude

Anything that can make a web request can hand him an item directly, with no
pack at all: a Plex webhook, a script, a cron job. POST JSON to
`http://127.0.0.1:4318/items` with the token from `http.json`:

```bash
curl -X POST http://127.0.0.1:4318/items -H "Authorization: Bearer <token>" -d "{\"id\":\"plex:123\",\"kind\":\"message\",\"source\":\"plex\",\"title\":\"Severance S2E4 is ready\"}"
```

From a VM, `tools/vm-tunnel.ps1 -Items` forwards that door to it.

## Sharing

A pack is a folder. Zip it, send it, and whoever gets it drops it into their
own `Clawd/packs/` and turns it on in the gear.
