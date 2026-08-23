# Configuration

Zero-config default: load modes from `~/.config/rigmode/modes`.

## Config File

**Path:** `~/.config/rigmode/config.toml` (or `$XDG_CONFIG_HOME/rigmode/config.toml`)

Create this file when you want to customize modes dirs.

```toml
modes_dirs = ["~/my/modes"]
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

## Data Files

Under `~/.local/share/rigmode/` (or `$XDG_DATA_HOME/rigmode/`), one JSON object per line, written best-effort so `attach` always exits `0`:

| File | Contents | Browse with |
|------|----------|-------------|
| `attach.jsonl` | One line per attach | `rigmode log` |

Old-schema lines fail to parse and are skipped.
