# hindsight

ctrl-r fuzzy search for your shell history

## install

```bash
./install.sh
```

## config

`~/.config/hindsight/config.toml`:

```toml
default_mode = "global"  # or "session" or "cwd"
default_limit = 1000
height = "50%"
```

## usage

ctrl-r in your shell

## requirements

- rust
- zsh
- existing zhistory database