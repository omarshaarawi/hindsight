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
      escaped_cmd="${escaped_cmd//$'\n'/ }"
      escaped_cmd="${escaped_cmd//$'\r'/ }"
      local escaped_pwd="${PWD//\'/\'\'}"

      sqlite3 "$db_path" "
        PRAGMA trusted_schema=ON;

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

        DROP TRIGGER IF EXISTS history_fts_insert;
        DROP TRIGGER IF EXISTS history_fts_delete;
        DROP TRIGGER IF EXISTS history_fts_update;

        INSERT INTO history (command, exit_code, cwd, hostname, session, start_ts, duration)
        VALUES ('$escaped_cmd', $code, '$escaped_pwd', '$(hostname)', '$HINDSIGHT_SESSION', $(date +%s), $((SECONDS-HINDSIGHT_CMD_START)));

        CREATE TRIGGER IF NOT EXISTS history_fts_insert AFTER INSERT ON history BEGIN
          INSERT INTO history_fts(rowid, command) VALUES (new.rowid, new.command);
        END;
        CREATE TRIGGER IF NOT EXISTS history_fts_delete AFTER DELETE ON history BEGIN
          DELETE FROM history_fts WHERE rowid = old.rowid;
        END;
        CREATE TRIGGER IF NOT EXISTS history_fts_update AFTER UPDATE ON history BEGIN
          UPDATE history_fts SET command = new.command WHERE rowid = new.rowid;
        END;
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
