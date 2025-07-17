export HINDSIGHT_SESSION=${HINDSIGHT_SESSION:-$(uuidgen)}
export HINDSIGHT_MODE=${HINDSIGHT_MODE:-global}

function hindsight_preexec() {
  export HINDSIGHT_CMD_START=$SECONDS
  export HINDSIGHT_CMD=$1
}

function hindsight_precmd() {
  local code=$?
  if [[ -n "$HINDSIGHT_CMD" ]]; then
    local db_path="$HOME/Library/Application Support/com.shaarawi.hindsight/history.sqlite3"
    
    mkdir -p "$(dirname "$db_path")"
    
    if command -v sqlite3 >/dev/null 2>&1; then
      local escaped_cmd="${HINDSIGHT_CMD//\'/\'\'}"
      local escaped_pwd="${PWD//\'/\'\'}"
      
      sqlite3 "$db_path" "
        CREATE TABLE IF NOT EXISTS history (
          id INTEGER PRIMARY KEY,
          command TEXT NOT NULL,
          exit_code INTEGER,
          cwd TEXT,
          hostname TEXT,
          session TEXT,
          start_ts INTEGER,
          duration INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_history_session ON history(session);
        CREATE INDEX IF NOT EXISTS idx_history_cwd ON history(cwd);
        INSERT INTO history (command, exit_code, cwd, hostname, session, start_ts, duration)
        VALUES ('$escaped_cmd', $code, '$escaped_pwd', '$(hostname)', '$HINDSIGHT_SESSION', $(date +%s), $((SECONDS-HINDSIGHT_CMD_START)));
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