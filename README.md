# rigmode

[![Crates.io](https://img.shields.io/crates/v/rigmode.svg)](https://crates.io/crates/rigmode)

Attach work modes to AI coding agent prompts.

A mode declares decision principles, stop conditions, and human gates. Every mode whose triggers match your prompt is injected automatically via an agent hook — modes are phases of one job, so their guardrails add up. Push back in your own words — configured intervention markers record the correction to `gates.jsonl` automatically.

## Agent support

| Capability | Claude Code | Codex | Cursor |
| -------------------------------- | ----------- | ------ | ----- |
| Per-prompt mode attach | ✅ | ❌ | ❌ |
| `hook install` / `uninstall` | ✅ | ❌ | ❌ |

## Quick Start

```sh
# Point at your modes directory (optional — default is ~/.config/rigmode/modes)
mkdir -p ~/.config/rigmode
cat > ~/.config/rigmode/config.toml <<'EOF'
modes_dirs = ["~/src/github.com/to4iki/prompt-harness/modes"]

[gate]
markers = ["wrong", "redo", "that's not it"]
EOF

# Register the Claude Code UserPromptSubmit hook
rigmode hook install claude-code

# Validate modes and hook registration
rigmode check

# Dry-run attach against a prompt
echo '{"prompt":"implement this"}' | rigmode attach claude-code

# List recent attaches (newest first)
rigmode log
rigmode log --mode review --limit 20

# List recorded interventions (newest first)
rigmode gate
rigmode gate --mode implement --limit 20

# Remove the hook
rigmode hook uninstall claude-code
```

Restart Claude Code (or start a new session) after `hook install` so the hook is picked up.

## Install

**Homebrew (macOS):**

```bash
brew install to4iki/tap/rigmode
```

**mise:**

```bash
mise use -g github:to4iki/rigmode
```

**Cargo**

```bash
cargo install rigmode
```

## Documentation

- [Usage](docs/usage.md) — Commands, flags, gate recording, and debugging
- [Configuration](docs/configuration.md) — Config file, modes dirs, and data files
- [Modes](docs/modes.md) — Mode file format and selection rules

## License

MIT
