# Modes

One Markdown file per mode. Frontmatter decides when it attaches; the body is injected as-is.

```markdown
---
name: review
triggers: レビュー, コードレビュー, review
---

## 判定原則
...
```

| Key | Default | Notes |
|-----|---------|-------|
| `name` | filename stem | Prefer matching the stem (`check` warns) |
| `triggers` | — | Literal terms separated by `,` (or a list `[a, b]`). Quotes on scalars are stripped |

Trigger matching is case-insensitive and literal — regex metacharacters have no special meaning. A term end that is an ASCII letter may not touch another one, so `pr` matches `PRを作って` but stays out of `priority`; kana ends carry no such guard, so `実装して` still matches inside `実装してPR作って`. A space inside a term is optional (`pull request` also matches `pullrequest`).

Body headings (判定原則 / 停止条件 / Gate) are a convention only. Keep bodies under 10,000 characters (Claude Code truncates hook output).

Under **Gate**, state what the human should judge. Answers are given in natural language; only pushbacks are recorded, via `[gate]` markers in config.toml (see [Usage](usage.md#gate-recording)).

## Selection

1. Load `*.md` from each `modes_dirs` entry; earlier directory wins on duplicate `name`.
2. Collect modes whose `triggers` match the prompt.
3. Attach **all** of them, in load order.

Modes are phases of one job, and a request routinely spans several — implement this, then open the PR. Picking a single winner would drop the other phase's stop conditions, so every match attaches and their guardrails add up. Overlapping triggers are by design, not an error.
