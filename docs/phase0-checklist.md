# Phase 0: what to check on the work laptop

These are the questions the design still rests on. Each one takes a minute or two
to check. Tick it, or note what happened, and the plan adjusts.

## Already answered

- [x] **The unsigned exe runs.** The coding Claw'd installed and ran with no admin
      prompt and no Smart App Control block (Nat, 2026-09-24).
- [x] **A scheduled task can read Slack and Calendar** (Nat, 2026-09-24).

## Step 0 for v0.2: can he run the big brain himself? (do this first)

On the work laptop, open PowerShell and paste:

```powershell
$c = Get-ChildItem "$env:USERPROFILE\.local\bin\claude.exe","$env:APPDATA\Claude\claude-code\*\claude.exe","$env:LOCALAPPDATA\Packages\Claude_*\LocalCache\Roaming\Claude\claude-code\*\claude.exe" -ErrorAction SilentlyContinue | Sort-Object FullName | Select-Object -Last 1
$c.FullName
& $c.FullName -p "Reply with just OK" --output-format stream-json --verbose --max-turns 1 --model claude-haiku-4-5-20251001
```

- [ ] **A path printed.** The program is there.
- [ ] **The last line says `"is_error":false` and "OK".** It's logged in and can
      run. (If it asks you to log in, that's fine: the egg has a **Log me in** button.)
- [ ] **The first long line lists `mcp_servers`.** It shows which apps the program
      can see (`connected`) and which need a one-time OK (`needs-auth`, which the
      egg handles too). Slack and Asana should appear if they're connected at work.

If no path prints, or it errors in a way that isn't about logging in, he falls
back to SETUP.md (the egg says to paste his setup message into Claude).

## Still to check

- [ ] **Can a scheduled task write into `C:\Users\<you>\Clawd\inbox`?**
      This is the main door. Share the Clawd folder with Cowork, then ask a
      scheduled task to write a file called `hello.json` containing
      `{"id":"hello","kind":"message","title":"hi from a scheduled task"}`.
      Claw'd should read it out within a few seconds.
      *If not:* the fallback is HTTP (`http.json` in the same folder). Check the
      next box.
- [ ] **Can Claude reach `http://127.0.0.1:4318` from where it runs?** Only matters
      if the folder can't be used. Ask Claude to `GET http://127.0.0.1:4318/health`.
      Cowork runs in its own sandbox, so this may well be **no**. That's fine as
      long as the folder works.
- [ ] **Do scheduled tasks run while the Claude app is closed, or the laptop is
      asleep?** Close the app for longer than the sweep interval. Claw'd should
      turn grey and say "haven't heard from Claude in…". Then check whether the
      sweeps caught up when the app reopened.
- [ ] **Does the Ask box land in Claude with the text filled in?** Hover Claw'd,
      press +, type something, press Enter. Either Claude opens with the text in the
      message box (great), or it opens empty and Ctrl+V pastes it (the fallback
      works; note which one happened).
- [ ] **Can Claude save drafts?** Ask Claude to save a Slack draft and a Gmail draft
      without sending them. If it can't, proposals stay as text on the item, which
      is still useful.
- [ ] **Which Artifact features does the work account have?** Ask Claude to make the
      morning digest as an artifact and give you the link. If artifacts aren't
      available, the digest arrives as a few short notes instead.

## For v0.3 (voice)

- [ ] **Is Windows online speech recognition allowed?** Settings, then Privacy &
      security, then Speech: is *Online speech recognition* on, and can you turn
      it on, or is it greyed out by policy? This decides which recogniser v0.3
      uses (see docs/v0.3-plan.md).
- [ ] **Does Windows voice typing work?** Press Win+H in any text box and say a
      sentence. This is the last-resort fallback.

## Checked on the personal laptop (2026-09-24)

- Rust builds locally (Smart App Control no longer blocks the compiler here), and
  the unit tests pass.
- The face, inbox popover, bubble, voice lines, Do-it mode and Ask box were
  exercised in the browser demo (`node tools/dev-server.js`, then press "demo inbox").
