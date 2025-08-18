export HINDSIGHT_SESSION=${HINDSIGHT_SESSION:-$(uuidgen)}
export HINDSIGHT_MODE=${HINDSIGHT_MODE:-global}

function hindsight_preexec() {
  export HINDSIGHT_CMD_START=$SECONDS
  export HINDSIGHT_CMD=$1
}

function hindsight_precmd() {
  local code=$?
  if [[ -n "$HINDSIGHT_CMD" ]]; then
    local db_path
    if command -v hindsight >/dev/null 2>&1; then
      db_path=$(hindsight init 2>/dev/null | grep -o '"[^"]*"' | tr -d '"')
    fi

    if [[ -z "$db_path" ]]; then
      if [[ "$OSTYPE" == "darwin"* ]]; then
        db_path="$HOME/Library/Application Support/com.shaarawi.hindsight/history.sqlite3"
      else
        db_path="$HOME/.local/share/hindsight/history.sqlite3"
      fi
    fi

    if command -v sqlite3 >/dev/null 2>&1 && [[ -f "$db_path" ]]; then
      local escaped_cmd="${HINDSIGHT_CMD//\'/\'\'}"
      local escaped_pwd="${PWD//\'/\'\'}"
      local hostname_val=$(hostname)
      local escaped_hostname="${hostname_val//\'/\'\'}"
      local escaped_session="${HINDSIGHT_SESSION//\'/\'\'}"

      sqlite3 "$db_path" "
        INSERT INTO history (command, exit_code, cwd, hostname, session, start_ts, duration)
        VALUES ('$escaped_cmd', $code, '$escaped_pwd', '$escaped_hostname', '$escaped_session', $(date +%s), $((SECONDS-HINDSIGHT_CMD_START)));
      " 2>/dev/null
    fi

    unset HINDSIGHT_CMD
    unset HINDSIGHT_CMD_START
  fi
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec hindsight_preexec
add-zsh-hook precmd hindsight_precmd

zle -N hindsight-widget
function hindsight-widget() {
  local selected
  selected=$(hindsight --mode "$HINDSIGHT_MODE")

  if [[ "$selected" == __HINDSIGHT_MODE__* ]]; then
    local rest=${selected#__HINDSIGHT_MODE__}
    HINDSIGHT_MODE=${rest%%__*}
    selected=${rest#*__}
  fi

  if [[ "$selected" == "__HINDSIGHT_EDIT__"* ]]; then
    BUFFER="${selected#__HINDSIGHT_EDIT__}"
    CURSOR=${#BUFFER}
  elif [[ -n "$selected" ]]; then
    BUFFER="$selected"
    zle accept-line
  fi
}

bindkey '^R' hindsight-widget
