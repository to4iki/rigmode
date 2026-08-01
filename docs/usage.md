# Usage

## `rigmode attach <AGENT>`

Used by the Claude Code `UserPromptSubmit` hook. Reads JSON on stdin, prints `hookSpecificOutput.additionalContext` JSON on stdout.

```sh
echo '{"prompt":"実装して"}' | rigmode attach claude-code
```

Always exits `0` (empty stdout on failure or no match), so Claude Code never erases the prompt.

`--modes-dir <PATH>` (repeatable) overrides `config.toml`. Agent today: `claude-code`.

## `rigmode hook install <AGENT>`

```sh
rigmode hook install claude-code
rigmode hook install claude-code --force   # allow a binary under target/
```

Registers the `UserPromptSubmit` hook in `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`). Idempotent on `args`, not the binary path. Then runs `check` and prints warnings without failing. Restart Claude Code after install.

## `rigmode hook uninstall <AGENT>`

```sh
rigmode hook uninstall claude-code
```

Removes only the rigmode entry.

## `rigmode check`

```sh
rigmode check
rigmode check --modes-dir ./modes
```

Validates modes and hook registration. Non-zero exit on errors; warnings alone still exit `0`.

## `rigmode log`

```sh
rigmode log
rigmode log --mode review --limit 20
```

Lists recorded attaches (newest first) from `attach.jsonl` — the ground truth for which modes a prompt actually received. Columns: timestamp, attached modes, working directory.

## `rigmode gate`

```sh
rigmode gate
rigmode gate --mode implement --limit 20
```

Lists recorded interventions (newest first) from `gates.jsonl`.

## Gate recording

`gates.jsonl` records interventions only — the moments a human pushed back on the agent's work. Declare intervention words in `config.toml`:

```toml
[gate]
markers = ["違う", "やり直し", "そうじゃなくて"]
```

When a prompt's **first line** contains a marker (case-insensitive substring) and the session has a prior attach, `attach` appends one line to `gates.jsonl` with the session's last attached modes. An empty list (the default) disables recording. Approvals are not recorded — silence means pass.
