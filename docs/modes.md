# Modes

One Markdown file per mode. Frontmatter decides when it attaches; the body is injected as-is.

```markdown
---
name: review
priority: 10
triggers: レビュー|コードレビュー|review
---

## 判定原則
...
```

| Key | Default | Notes |
|-----|---------|-------|
| `name` | filename stem | Prefer matching the stem (`check` warns) |
| `priority` | `0` | Higher wins when several modes match |
| `triggers` | — | Rust [`regex`](https://docs.rs/regex), or a list `[a, b]` joined with `\|`. Quotes on scalars are stripped |

Body headings (判定原則 / 停止条件 / Gate) are a convention only. Keep bodies under 10,000 characters (Claude Code truncates hook output).

## Selection

1. Load `*.md` from each `modes_dirs` entry; earlier directory wins on duplicate `name`.
2. Collect modes whose `triggers` match the prompt.
3. Pick `(priority desc, name asc)`.

Overlaps are not an attach error — use `explain` / `check` to surface them.
