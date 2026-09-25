# Keeps a reverse SSH tunnel open so a dev VM can reach Claw'd on this
# machine:
#
#   VM 127.0.0.1:4317  ->  (ssh)  ->  this machine 127.0.0.1:4317   Claude Code hooks (dev pack)
#   VM 127.0.0.1:4318  ->  (ssh)  ->  this machine 127.0.0.1:4318   the item door (with -Items)
#
# With the item door forwarded, anything in the VM (a Plex webhook, a cron
# job, a script) can hand him an item:
#
#   curl -X POST http://127.0.0.1:4318/items -H "Authorization: Bearer <token>" \
#        -d '{"id":"plex:123","kind":"message","source":"plex","title":"Severance S2E4 is ready"}'
#
# (the token is in http.json in the Clawd folder on this machine).
#
# Why a REVERSE tunnel: the network routes host -> VM but not VM -> host, so
# the host dials out. Claw'd keeps listening on loopback only: no 0.0.0.0
# bind, no firewall hole. SSH is the auth.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm-tunnel.ps1 -Items
#
# Runs until killed, reconnecting if the link drops. If the key is limited on
# the VM side (authorized_keys `restrict,port-forwarding,permitlisten=...`),
# add permitlisten="127.0.0.1:4318" before using -Items.

param(
  [string]$VmUser = 'mapache',
  [string]$VmHost = '10.0.17.103',
  [int]$Port      = 4317,
  [int]$ItemPort  = 4318,
  [switch]$Items,
  [string]$Key    = "$env:USERPROFILE\.ssh\clawdbot_tunnel"
)

if (-not (Test-Path $Key)) { Write-Host "missing key: $Key"; exit 1 }

# 127.0.0.1, NOT localhost: Claw'd binds IPv4 only, and `localhost` resolves
# to ::1 first on Windows, where nothing is listening.
$forwards = @('-R', "${Port}:127.0.0.1:$Port")
if ($Items) { $forwards += @('-R', "${ItemPort}:127.0.0.1:$ItemPort") }

Write-Host "tunnel: VM ${VmHost}:$Port -> localhost:$Port$(if ($Items) { ", and :$ItemPort for items" })  (ctrl-c to stop)"

while ($true) {
  $started = Get-Date
  # ExitOnForwardFailure: fail loudly rather than sit there connected but not
  # forwarding. ServerAlive*: notice a dead link within ~90s. -n: with -N and
  # no command ssh still reads stdin, and a detached process has stdin at EOF,
  # so it would exit the instant it starts.
  & ssh -n -N @forwards `
      -i $Key `
      -o IdentitiesOnly=yes `
      -o BatchMode=yes `
      -o ExitOnForwardFailure=yes `
      -o ServerAliveInterval=30 `
      -o ServerAliveCountMax=3 `
      -o StrictHostKeyChecking=accept-new `
      "$VmUser@$VmHost" 2>&1 | ForEach-Object { Write-Host "  ssh: $_" }

  $up = [int]((Get-Date) - $started).TotalSeconds
  Write-Host "$(Get-Date -Format 'HH:mm:ss') tunnel dropped after ${up}s - reconnecting in 5s"
  # a tunnel that dies at once is usually a real fault (key, host, port in
  # use): back off a little so a broken config doesn't spin
  Start-Sleep -Seconds $(if ($up -lt 5) { 15 } else { 5 })
}
