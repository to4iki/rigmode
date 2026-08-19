# Configuration

Zero-config default: load modes from `~/.config/rigmode/modes`.

## Config File

**Path:** `~/.config/rigmode/config.toml` (or `$XDG_CONFIG_HOME/rigmode/config.toml`)

Create this file when you want to customize modes dirs or gate markers.

```toml
modes_dirs = ["~/my/modes"]

[gate]
markers = ["wrong", "redo"]
```

`~/` is expanded. Earlier directories win on duplicate mode names. `--modes-dir` on the CLI overrides this list.

## Options

### `modes_dirs`

Directories to load `*.md` mode files from.

- **Type:** Array of paths
- **Default:** `["~/.config/rigmode/modes"]` (when unset / empty)

```toml
modes_dirs = ["~/my/modes", "./modes"]
```

### `[gate].markers`

Words that mark a prompt as a human intervention (a rejection of the agent's work). Matched on the prompt's **first line** with the same rules as mode triggers: case-insensitive, literal, ASCII-letter ends guarded so `no` stays out of `notification`. An empty list disables recording.

- **Type:** Array of strings
- **Default:** `[]`

```toml
[gate]
markers = ["wrong", "redo", "that's not it"]
```

See [Usage](usage.md#gate-recording) for when a record is written.

## Data Files

Under `~/.local/share/rigmode/` (or `$XDG_DATA_HOME/rigmode/`), one JSON object per line, written best-effort so `attach` always exits `0`:

| File | Contents | Browse with |
|------|----------|-------------|
| `attach.jsonl` | One line per attach | `rigmode log` |
| `gates.jsonl` | One line per intervention | `rigmode gate` |

Old-schema lines fail to parse and are skipped.
