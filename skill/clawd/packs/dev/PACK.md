# Dev pack

Claw'd as a coding buddy. The sessions themselves (Claude Code working,
thinking, waiting for an answer, done) reach him directly from Claude Code's
hooks, not through you. This pack is for what's around them.

## Setup

- **His hours:** whenever they're at the computer; keep sweeps light.
- If GitHub (or GitLab) is connected, note which repos are theirs.

## Rules

| id | kind | matches |
|---|---|---|
| `review-requested` | waiting | A pull request where they're asked to review, still open. |
| `ci-failed` | waiting | A check failed on their own open pull request or default branch. Name the repo and the check. |
| `pr-merged` | message | Their pull request was merged, or got a comment or approval. |

Ids: `gh:pr:<repo>#<n>`, `gh:ci:<repo>:<sha>`.

## Requests

- "What was I working on?": look at their recent commits and open pull
  requests, and answer in one line.
- Questions about code go to the coding session they're in, not to him: say so
  kindly if asked to change code.
