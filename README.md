# rigmode

[![Crates.io](https://img.shields.io/crates/v/rigmode.svg)](https://crates.io/crates/rigmode)

Attach work modes to AI coding agent prompts.

A mode declares decision principles, stop conditions, and human gates. Every mode whose triggers match your prompt is injected automatically via an agent hook — modes are phases of one job, so their guardrails add up.

## Agent support

| Capability | Claude Code | Codex | Cursor |
| -------------------------------- | ----------- | ------ | ----- |
| Per-prompt mode attach | ✅ | ✅ | ❌ |
| `hook install` / `uninstall` | ✅ | ✅ | ❌ |

## Quick Start

```sh
# Point at your modes directory (optional — default is ~/.config/rigmode/modes)
mkdir -p ~/.config/rigmode
cat > ~/.config/rigmode/config.toml <<'EOF'
modes_dirs = ["~/src/github.com/to4iki/prompt-harness/modes"]
EOF

# Register the UserPromptSubmit hook (claude-code or codex)
rigmode hook install claude-code
rigmode hook install codex

# Validate modes and hook registration
rigmode check

# Dry-run attach against a prompt
echo '{"prompt":"implement this"}' | rigmode attach claude-code

# List recent attaches (newest first)
rigmode log
rigmode log --mode review --limit 20

# Remove the hook
rigmode hook uninstall claude-code
rigmode hook uninstall codex
```

Restart the agent (or start a new session) after `hook install` so the hook is picked up. Codex additionally skips untrusted hooks: run `/hooks` inside Codex and trust the rigmode entry.

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

- [Usage](docs/usage.md) — Commands, flags, and debugging
- [Configuration](docs/configuration.md) — Config file, modes dirs, and data files
- [Modes](docs/modes.md) — Mode file format and selection rules

## Releasing

Releases are managed by [release-plz](https://release-plz.dev/). Merging the release PR creates a version tag, publishes to crates.io, and uploads Homebrew binaries to the GitHub Release.

## License

MIT
