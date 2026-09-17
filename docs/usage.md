# Usage

## Commands

### `rigmode attach <AGENT>`

Used by the agent's `UserPromptSubmit` hook. Reads JSON on stdin, prints `hookSpecificOutput.additionalContext` JSON on stdout.

```sh
echo '{"prompt":"implement this"}' | rigmode attach claude-code
echo '{"prompt":"implement this"}' | rigmode attach codex
```

Always exits `0` (empty stdout on failure or no match), so the agent never erases the prompt.

| Flag | Description |
|------|-------------|
| `--modes-dir <PATH>` | Override `config.toml` modes dirs (repeatable) |

Agents: `claude-code`, `codex`.

### `rigmode hook install <AGENT>`

```sh
rigmode hook install claude-code
rigmode hook install codex
rigmode hook install claude-code --force   # allow a binary under target/
```

Registers the `UserPromptSubmit` hook, idempotent on the rigmode arguments rather than the binary path. Then runs `check` and prints warnings without failing. Restart the agent after install.

| Agent | File | Notes |
|-------|------|-------|
| `claude-code` | `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`) | Exec form (`command` + `args`) |
| `codex` | `~/.codex/hooks.json` (or `$CODEX_HOME/hooks.json`) | Shell command string with `additionalContextLimit: 0` so mode bodies are never spilled to a file. Codex skips untrusted hooks: run `/hooks` inside Codex and trust the entry after every install |

| Flag | Description |
|------|-------------|
| `--force` | Allow registering a binary under `target/` |

### `rigmode hook uninstall <AGENT>`

```sh
rigmode hook uninstall claude-code
rigmode hook uninstall codex
```

Removes only the rigmode entry.

### `rigmode check`

```sh
rigmode check
rigmode check --modes-dir ./modes
```

Validates modes and hook registration for every agent. Non-zero exit on errors; warnings alone still exit `0`. No agent registered is a warning; a registered hook whose binary is missing, or a hook file that is not valid JSON, is an error (one bad file does not abort the rest of the report). Codex hook trust is not visible in `hooks.json`, so `check` cannot verify it.

| Flag | Description |
|------|-------------|
| `--modes-dir <PATH>` | Override `config.toml` modes dirs (repeatable) |

### `rigmode log`

```sh
rigmode log
rigmode log --mode review --limit 20
```

Lists recorded attaches (newest first) from `attach.jsonl` — the ground truth for which modes a prompt actually received.

Columns: timestamp, attached modes, working directory, session id.

| Flag | Description |
|------|-------------|
| `--mode <NAME>` | Filter by attached mode name |
| `--limit <N>` | Max rows to print |

## Debugging

```sh
# Validate modes dirs, frontmatter, and hook registration
rigmode check

# Dry-run attach without the agent
echo '{"prompt":"implement this"}' | rigmode attach claude-code
echo '{"prompt":"implement this"}' | rigmode attach codex

# See what actually attached in recent prompts
rigmode log --limit 20
```

`attach` always exits `0` and writes logs best-effort. If nothing attaches, check triggers with `check`, then confirm the hook path with `hook install` / `check`.
