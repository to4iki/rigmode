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

- `attach` always exits 0. Claude Code's and Codex's `UserPromptSubmit` erase the prompt on exit 2 and surface a hook error on other non-zero exits. Codex also injects non-JSON stdout as raw context, so the success path prints nothing but the JSON.
- No priority, no single winner: every matching mode attaches, in load order (rationale in docs/modes.md).
- Triggers are comma-separated literal terms, not regex (semantics in docs/modes.md). The regex crate has no lookaround, so the ASCII boundary guards consume a neighbor char — fine because only `is_match` is used.
- `rigmode log` lists attach.jsonl (which modes a prompt actually received). Old-schema lines fail to parse and are skipped.
- Claude Code hook registration uses its exec form (`command` + `args`) and keys idempotency on `args`, not the binary path. Removal is per hook object, so a hook another tool put in the same entry survives.
- Codex hooks (`~/.codex/hooks.json`, `$CODEX_HOME`) have no exec form: `command` is one shell string run via `$SHELL -lc`, so the binary path is shell-quoted and idempotency keys on the `attach codex` marker (extra flags after it stay ours, so a reinstall replaces rather than duplicates). The handler sets `additionalContextLimit: 0` because Codex otherwise spills output over ~2,500 tokens to a file and shows the model only a preview, dropping the mode bodies; `check` warns on oversized bodies instead — per mode, so several large modes matching one prompt can still exceed it. Codex skips non-managed hooks until trusted via `/hooks`, and trust is keyed on the hook hash, so every reinstall needs re-trusting.
- Cursor lacks per-prompt context injection (`beforeSubmitPrompt` cannot attach mode text).
- Gate recording (`rigmode gate`, gates.jsonl, `[gate]` markers) was removed: it was confirmation-only, not essential. Harvest (correction extraction) and evolution history remain Future and need a new capture mechanism first.
