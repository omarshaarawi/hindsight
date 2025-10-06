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

### search
- `ctrl-r` - open fuzzy search
- type to filter commands
- `enter` - execute selected command immediately
- `tab` - insert command into prompt for editing
- `ctrl-r` (while in search) - cycle modes: global → session → cwd → saved
- `esc` - cancel

### saved commands
save frequently used commands with tags:
```bash
hindsight save "docker ps -a" --tags docker,containers --description "List all containers"
hindsight list-saved              # list all saved commands
hindsight list-saved --tags docker # filter by tag
hindsight delete-saved 1          # delete by id
```

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
- `saved` - saved commands with tags

## requirements

- rust
- zsh
- sqlite3 (for recording commands)

## data location

history is stored in:
- macOS: `~/Library/Application Support/com.shaarawi.hindsight/history.sqlite3`
- linux: `~/.local/share/hindsight/history.sqlite3`