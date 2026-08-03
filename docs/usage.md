# Usage

## Commands

### `rigmode attach <AGENT>`

Used by the Claude Code `UserPromptSubmit` hook. Reads JSON on stdin, prints `hookSpecificOutput.additionalContext` JSON on stdout.

```sh
echo '{"prompt":"implement this"}' | rigmode attach claude-code
```

Always exits `0` (empty stdout on failure or no match), so Claude Code never erases the prompt.

| Flag | Description |
|------|-------------|
| `--modes-dir <PATH>` | Override `config.toml` modes dirs (repeatable) |

Agent today: `claude-code`.

### `rigmode hook install <AGENT>`

```sh
rigmode hook install claude-code
rigmode hook install claude-code --force   # allow a binary under target/
```

Registers the `UserPromptSubmit` hook in `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`). Idempotent on `args`, not the binary path. Then runs `check` and prints warnings without failing. Restart Claude Code after install.

| Flag | Description |
|------|-------------|
| `--force` | Allow registering a binary under `target/` |

### `rigmode hook uninstall <AGENT>`

```sh
rigmode hook uninstall claude-code
```

Removes only the rigmode entry.

### `rigmode check`

```sh
rigmode check
rigmode check --modes-dir ./modes
```

Validates modes and hook registration. Non-zero exit on errors; warnings alone still exit `0`.

| Flag | Description |
|------|-------------|
| `--modes-dir <PATH>` | Override `config.toml` modes dirs (repeatable) |

### `rigmode log`

```sh
rigmode log
rigmode log --mode review --limit 20
```

Lists recorded attaches (newest first) from `attach.jsonl` — the ground truth for which modes a prompt actually received.

Columns: timestamp, attached modes, working directory.

| Flag | Description |
|------|-------------|
| `--mode <NAME>` | Filter by attached mode name |
| `--limit <N>` | Max rows to print |

### `rigmode gate`

```sh
rigmode gate
rigmode gate --mode implement --limit 20
```

Lists recorded interventions (newest first) from `gates.jsonl`.

Columns: timestamp, modes, marker, note, session id.

| Flag | Description |
|------|-------------|
| `--mode <NAME>` | Filter by mode name |
| `--limit <N>` | Max rows to print |

## Gate recording

`gates.jsonl` records interventions only — the moments a human pushed back on the agent's work. Declare intervention words in `config.toml`:

```toml
[gate]
markers = ["wrong", "redo", "that's not it"]
```

When a prompt's **first line** contains a marker (case-insensitive substring) and the session has a prior attach, `attach` appends one line to `gates.jsonl` with the session's last attached modes. An empty list (the default) disables recording. Approvals are not recorded — silence means pass.

## Debugging

```sh
# Validate modes dirs, frontmatter, and hook registration
rigmode check

# Dry-run attach without Claude Code
echo '{"prompt":"implement this"}' | rigmode attach claude-code

# See what actually attached in recent prompts
rigmode log --limit 20

# See recorded pushbacks
rigmode gate --limit 20
```

`attach` always exits `0` and writes logs best-effort. If nothing attaches, check triggers with `check`, then confirm the hook path with `hook install` / `check`.
