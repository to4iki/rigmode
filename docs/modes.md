# Modes

One Markdown file per mode. Frontmatter decides when it attaches; the body is injected as-is.

```markdown
---
name: review
triggers: review, code review
---

## Decision principles
...
```

| Key | Default | Notes |
|-----|---------|-------|
| `name` | filename stem | Prefer matching the stem (`check` warns) |
| `triggers` | — | Literal terms separated by `,` (or a list `[a, b]`). Quotes are stripped |

Trigger matching is case-insensitive and literal — regex metacharacters have no special meaning. Only term ends that are ASCII letters are guarded against adjoining ASCII letters, so `pr` matches `open a PR` but stays out of `priority`. Ends that are not ASCII letters have no such guard. A space inside a term is optional (`pull request` also matches `pullrequest`).

Keep bodies under 10,000 characters (Claude Code truncates hook output). Under **Gate**, state what the human should judge; pushbacks are recorded via `[gate]` markers (see [Usage](usage.md#gate-recording)).

## Selection

Load `*.md` from each `modes_dirs` entry (earlier directory wins on duplicate `name`), then attach **every** mode whose triggers match, in load order. Modes are phases of one job — a request routinely spans several (implement this, then open the PR) — so overlapping triggers are by design.
