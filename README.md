# rigmode

Attach work modes to AI coding agent prompts.

A mode declares decision principles, stop conditions, and human gates. The mode whose triggers match your prompt is injected automatically via an agent hook.

Claude Code only, for now. Cursor and Codex do not expose an equivalent per-prompt context-injection hook.

## Quick Start

```sh
mkdir -p ~/.config/rigmode
cat > ~/.config/rigmode/config.toml <<'EOF'
modes_dirs = ["~/src/github.com/to4iki/prompt-harness/modes"]
EOF

cargo install --path .
rigmode hook install claude-code
rigmode check
```

Restart Claude Code (or start a new session) so the hook is picked up.

## Install

```sh
cargo install --git https://github.com/to4iki/rigmode
# or from a checkout:
cargo install --path .
```

## Documentation

- [Usage](docs/usage.md) — Commands, flags, and debugging
- [Configuration](docs/configuration.md) — Config file, modes dirs, and attach log
- [Modes](docs/modes.md) — Mode file format and selection rules

## License

MIT
