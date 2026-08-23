# Project Guide

## Overview

rigmode is a Rust CLI that attaches work modes to AI coding agent prompts. Modes are agent-agnostic Markdown; agent-specific wiring lives under `adapters/`.

## Tech Stack

- Rust (edition 2024), clap 4 (derive)
- Config: TOML (`~/.config/rigmode/config.toml`)
- Attach log: JSONL (`~/.local/share/rigmode/attach.jsonl`)
- Key crates: anyhow, chrono, dirs, regex, serde, serde_json (preserve_order), toml

## Development

```sh
cargo test
cargo clippy
cargo fmt
```

## Design Decisions

- `attach` always exits 0. Claude Code's `UserPromptSubmit` erases the prompt on exit 2 and surfaces a hook error on other non-zero exits.
- No priority, no single winner: every matching mode attaches, in load order (rationale in docs/modes.md).
- Triggers are comma-separated literal terms, not regex (semantics in docs/modes.md). The regex crate has no lookaround, so the ASCII boundary guards consume a neighbor char — fine because only `is_match` is used.
- `rigmode log` lists attach.jsonl (which modes a prompt actually received). Old-schema lines fail to parse and are skipped.
- Hook registration uses Claude Code exec form (`command` + `args`) and keys idempotency on `args`, not the binary path.
- Cursor lacks per-prompt context injection (`beforeSubmitPrompt` cannot attach mode text). Codex documents `UserPromptSubmit` + `additionalContext`, but no rigmode adapter yet.
- Gate recording (`rigmode gate`, gates.jsonl, `[gate]` markers) was removed: it was confirmation-only, not essential. Harvest (correction extraction) and evolution history remain Future.
