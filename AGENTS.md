# Project Guide

## Overview

rigmode is a Rust CLI that attaches work modes to AI coding agent prompts. Modes are agent-agnostic Markdown; agent-specific wiring lives under `adapters/`.

## Tech Stack

- Rust (edition 2024), clap 4 (derive)
- Config: TOML (`~/.config/rigmode/config.toml`)
- Attach log: JSONL (`~/.local/share/rigmode/attach.jsonl`)
- Gate log: JSONL (`~/.local/share/rigmode/gates.jsonl`)
- Key crates: anyhow, chrono, dirs, regex, serde, serde_json (preserve_order), toml

## Development

```sh
cargo test
cargo clippy
cargo fmt
```

## Design Decisions

- `attach` always exits 0. Claude Code's `UserPromptSubmit` erases the prompt on exit 2 and surfaces a hook error on other non-zero exits.
- No priority, no single winner: every matching mode attaches, in load order. Modes are phases of one job (implement, then PR); dropping one would drop its stop conditions. Overlapping triggers are by design.
- Triggers are comma-separated literal terms, not regex. Matching is case-insensitive; a term end that is an ASCII letter gets a boundary guard (`pr` stays out of `priority`) while kana ends stay unguarded (`実装して` matches inside `実装してPR作って`). The regex crate has no lookaround, so guards consume a neighbor char — fine because only `is_match` is used.
- `rigmode log` lists attach.jsonl (which modes a prompt actually received), the sibling of `rigmode gate` for gates.jsonl.
- Hook registration uses Claude Code exec form (`command` + `args`) and keys idempotency on `args`, not the binary path.
- gates.jsonl records interventions only (opt-in): a prompt whose first line contains a `[gate]` marker from config.toml. Approvals are not recorded — silence means pass. Mode comes from the session's last attach. Public CLI is list-only (`rigmode gate`).
- Cursor lacks per-prompt context injection (`beforeSubmitPrompt` cannot attach mode text). Codex documents `UserPromptSubmit` + `additionalContext`, but no rigmode adapter yet.
- Harvest (correction extraction) and evolution history are Future; promote modes by hand from `gates.jsonl` for now.
