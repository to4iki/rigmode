# Configuration

Zero-config default: load modes from `~/.config/rigmode/modes`.

## Config File

**Path:** `~/.config/rigmode/config.toml` (or `$XDG_CONFIG_HOME/rigmode/config.toml`)

```toml
modes_dirs = ["~/my/modes"]

[gate]
markers = ["違う", "やり直し"]
```

`~/` is expanded. Earlier directories win on duplicate mode names. `--modes-dir` on the CLI overrides this list. `[gate]` markers opt into intervention recording (see [Usage](usage.md#gate-recording)).

## Data Files

Under `~/.local/share/rigmode/` (or `$XDG_DATA_HOME/rigmode/`), one JSON object per line, written best-effort so `attach` always exits `0`:

- `attach.jsonl` — one line per attach. Browse with `rigmode log`.
- `gates.jsonl` — one line per intervention. Browse with `rigmode gate`.
