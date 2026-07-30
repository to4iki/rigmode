# Configuration

Zero-config default: load modes from `~/.config/rigmode/modes`.

## Config File

**Path:** `~/.config/rigmode/config.toml` (or `$XDG_CONFIG_HOME/rigmode/config.toml`)

```toml
modes_dirs = ["~/src/github.com/to4iki/prompt-harness/modes"]

[gate]
markers = ["違う", "やり直し"]
```

`~/` is expanded. Earlier directories win on duplicate mode names. `--modes-dir` on the CLI overrides this list.

`[gate]` markers opt into intervention recording (see [Usage](usage.md#gate-recording)); omit it (default) and nothing is recorded.

## Attach Log

**Path:** `~/.local/share/rigmode/attach.jsonl` (or `$XDG_DATA_HOME/...`)

One JSON object per successful attach (`chosen`, `matched`, session metadata). Write failures are ignored so `attach` still exits `0`.

## Gate Log

**Path:** `~/.local/share/rigmode/gates.jsonl` (or `$XDG_DATA_HOME/...`)

One JSON object per intervention: a prompt whose first line contains a configured `[gate]` marker. Written by `attach` when the session has a prior attach record. Write failures are ignored so `attach` still exits `0`. Browse with `rigmode gate`.

## Claude Code Settings

`hook install` / `uninstall` edit `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`). Only the rigmode `UserPromptSubmit` entry is touched. Malformed JSON is refused.
