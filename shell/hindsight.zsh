export HINDSIGHT_SESSION=$(uuidgen)

zle -N hindsight-widget
function hindsight-widget() {
  local selected
  selected=$(hindsight --mode "${HINDSIGHT_MODE:-global}")
  
  if [[ -n "$selected" ]]; then
    BUFFER="$selected"
    CURSOR=${#BUFFER}
  fi
}

bindkey '^R' hindsight-widget