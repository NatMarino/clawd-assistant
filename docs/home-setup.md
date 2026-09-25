# Home setup: connecting the VM's apps to Claw'd

For whoever sets up the home side (a person, or a Claude session on the VM).
Claw'd runs on Nat's laptop; Plex, Sonarr, Radarr, Overseerr and Home
Assistant run on the VM. This page is everything the VM side needs.

## How it fits together

```
 VM                                        laptop
 Sonarr / Radarr / Overseerr / Plex  ──►   127.0.0.1:4318  (VM loopback)
 Home Assistant automations                     │ reverse SSH tunnel, opened
                                                │ from the laptop:
                                                ▼ tools\vm-tunnel.ps1 -Items
                                           127.0.0.1:4318  Claw'd's item door
 Home Assistant MCP server  ◄──────────  the big brain (laptop → VM:8123)
```

- **The item door** is Claw'd's local web endpoint. The laptop opens a reverse
  tunnel so the VM's `127.0.0.1:4318` reaches it. Nothing on the laptop
  listens on the network.
- **The token** is in `Clawd\http.json` on the laptop. Nat will give it to you;
  never commit it anywhere. Every call needs it.
- **Home Assistant** is reached the other way round: the laptop can reach the
  VM directly, so the big brain talks to HA's own MCP server at
  `http://<vm>:8123/api/mcp`. Nat enters that address and a token in Claw'd's
  gear (**home assistant…**).

## 0. Check the tunnel reaches the VM

On the laptop, the tunnel must run with `-Items`:
`powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm-tunnel.ps1 -Items`

If the key in `~/.ssh/authorized_keys` on the VM is restricted with
`permitlisten=`, it also needs `permitlisten="127.0.0.1:4318"`.

Then on the VM:

```bash
curl -s http://127.0.0.1:4318/health
# {"ok":true,"app":"clawd-assistant"}
```

**Docker:** if an app runs in a container on a bridge network, `127.0.0.1` is
the container itself, not the VM. Either use host networking for that app, or
make the tunnel listen where containers can reach it (sshd
`GatewayPorts clientspecified`, then `-R 172.17.0.1:4318:127.0.0.1:4318`, and
point the app at `http://172.17.0.1:4318`). Ask Nat before changing sshd.

## 1. Sonarr and Radarr

Settings → Connect → **+** → **Webhook**:

| Field | Value |
|---|---|
| Name | Claw'd |
| Triggers | On Import (On File Import), On Health Issue, On Health Restored, On Manual Interaction Required. Not On Grab, not On Upgrade. |
| Webhook URL | `http://127.0.0.1:4318/hooks/sonarr` (Radarr: `/hooks/radarr`) |
| Method | POST |
| Username | `clawd` (anything) |
| Password | the token |

Press **Test**: Claw'd says "Hi Sonarr! I can hear you."

## 2. Overseerr (or Jellyseerr)

Settings → Notifications → **Webhook**: enable it, Webhook URL
`http://127.0.0.1:4318/hooks/overseerr?token=<token>`, keep the **default JSON
payload**, and tick Request Pending Approval, Request Automatically Approved,
Request Approved, Request Declined, Request Available, Request Processing
Failed. **Test** it.

He shows a pending request as "Sam requested Dune. It needs your OK!", clears
it when it's approved or declined, and says when it's available.

## 3. Plex (needs Plex Pass)

Plex Web → Settings → (your account) → **Webhooks** → Add Webhook:
`http://127.0.0.1:4318/hooks/plex?token=<token>`

He mentions when someone else starts watching (not you), clears it when they
stop, and says when something's added. Without Plex Pass, skip this: Sonarr
and Radarr's "ready" already covers new episodes and movies.

## 4. Home Assistant

**a. So the big brain can check and control things:**
1. Settings → Devices & services → Add integration → **Model Context Protocol
   Server**.
2. Settings → Voice assistants → **Expose**: expose what Claw'd may see and
   control (lights, climate, the garage, sensors…).
3. Your profile → Security → **Long-lived access tokens** → Create token
   ("Claw'd"). Nat pastes the address (`http://<vm>:8123`) and this token into
   Claw'd's gear → **home assistant…**, with the home pack on.

**b. Alerts that need no AI** (instant): a `rest_command` plus automations
that call it. In `configuration.yaml`:

```yaml
rest_command:
  clawd:
    url: http://127.0.0.1:4318/items
    method: POST
    headers:
      Authorization: !secret clawd_token_header   # "Bearer <token>" in secrets.yaml
      Content-Type: application/json
    payload: >-
      [{"id": "{{ id }}", "kind": "{{ kind | default('waiting') }}", "source": "home",
        "title": "{{ title }}", "spoken": "{{ spoken | default(title) }}",
        "resolved": {{ resolved | default(false) | lower }} }]
```

An automation: the garage open for 15 minutes.

```yaml
- alias: Claw'd - garage left open
  trigger:
    - platform: state
      entity_id: cover.garage_door
      to: open
      for: "00:15:00"
  action:
    - service: rest_command.clawd
      data:
        id: ha:garage-open
        title: The garage has been open 15 minutes
        spoken: Heads up, the garage has been open for fifteen minutes!
- alias: Claw'd - garage closed
  trigger:
    - platform: state
      entity_id: cover.garage_door
      to: closed
  action:
    - service: rest_command.clawd
      data: { id: ha:garage-open, title: closed, resolved: true }
```

Item kinds: `waiting` (he hops and asks), `message` (worth hearing),
`reminder` (give `due`), `note` (quiet). Reuse an id to update an item;
`resolved: true` clears it. The item format is in `skill/clawd/SKILL.md`.

## 5. Turn it on in Claw'd

Nat turns on the **home** pack in the gear (packs: home). The doorways work
regardless of the pack; the pack is what lets the big brain use Home
Assistant and knows about these items.

## Checking it all

From the VM:

```bash
T=<token>
curl -s -X POST "http://127.0.0.1:4318/hooks/sonarr" -u "clawd:$T" -H 'Content-Type: application/json' \
  -d '{"eventType":"Download","series":{"id":1,"title":"Test Show"},"episodes":[{"seasonNumber":1,"episodeNumber":2}]}'
# {"accepted":1}  and Claw'd says "The new Test Show episode is ready to watch!"
```
