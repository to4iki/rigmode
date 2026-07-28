# Usage

## Commands

### `rigmode attach <AGENT>`

Used by the Claude Code `UserPromptSubmit` hook. Reads JSON on stdin, prints context on stdout.

```sh
echo '{"prompt":"実装して"}' | rigmode attach claude-code
```

Always exits `0` (empty stdout on failure or no match), so Claude Code never erases the prompt. Output is `hookSpecificOutput.additionalContext` JSON, not plain text.

`--modes-dir <PATH>` (repeatable) overrides `config.toml`. Agent today: `claude-code`.

### `rigmode hook install <AGENT>`

```sh
rigmode hook install claude-code
rigmode hook install claude-code --force   # allow a binary under target/
```

Registers an exec-form `UserPromptSubmit` hook in `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`). Idempotent on `args`, not the binary path. Then runs `check` and prints warnings without failing.

Restart Claude Code after install.

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

### `rigmode explain <PROMPT>`

```sh
rigmode explain "この実装をレビューして"
```

Shows the winning mode, other matches, and why.
