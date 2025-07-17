# hindsight

ctrl-r fuzzy search for your shell history

## install

```bash
./install.sh
```

adds this to your .zshrc:
```bash
source /path/to/hindsight/shell/hindsight.zsh
```

## usage

- `ctrl-r` - open fuzzy search
- type to filter commands
- `enter` - execute selected command immediately
- `tab` - insert command into prompt for editing
- `ctrl-r` (while in search) - cycle modes: global → session → cwd
- `esc` - cancel

## config

optional. create `~/.config/hindsight/config.toml`:

```toml
default_mode = "global"  # or "session" or "cwd"
default_limit = 1000
height = "50%"
```

modes:
- `global` - all history
- `session` - current terminal session only
- `cwd` - current directory only

## requirements

- rust
- zsh