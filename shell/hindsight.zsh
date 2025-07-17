export HINDSIGHT_SESSION=$(uuidgen)
export HINDSIGHT_MODE=${HINDSIGHT_MODE:-global}

zle -N hindsight-widget
function hindsight-widget() {
  local selected
  selected=$(hindsight --mode "$HINDSIGHT_MODE")
  
  if [[ "$selected" == "__HINDSIGHT_EDIT__"* ]]; then
    BUFFER="${selected#__HINDSIGHT_EDIT__}"
    CURSOR=${#BUFFER}
  elif [[ -n "$selected" ]]; then
    BUFFER="$selected"
    zle accept-line
  fi
}

bindkey '^R' hindsight-widget