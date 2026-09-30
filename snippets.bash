
# logging:

function init_logging() {
  LOG_FILE="/tmp/devenv.log"
  # ensure tmp exists:
  mkdir -p /tmp
  # recreate logfile
  echo "" >"$LOG_FILE"
  # write current date and time to logfile
  echo "Logfile created at $(date)" >>"$LOG_FILE"
}

function loga() { # log "always"
  txt="$1"
  logaction "$txt"
  echoe "$txt"
}

function logi() {
  txt="INFO    $1"
  logaction "$txt"
  if [[ "$VERBOSE" == 1 ]]; then
    echoe "$txt"
  fi
}

logw() {
  YELLOW='\033[0;33m'
  DEFAULT='\033[0m'
  txt="WARNING $1"
  txt_print="${YELLOW}WARNING${DEFAULT} $1"
  logaction "$txt"
  echoe "$txt_print"
}

loge() {
  RED='\033[0;31m'
  DEFAULT='\033[0m'
  txt="ERROR   $1"
  txt_print="${RED}ERROR${DEFAULT}   $1"
  logaction "$txt"
  echoe "$txt_print"
}

function logaction() {
  echo "$1" >>"$LOG_FILE"
}

function echoe() {
  printf "$1\n" 1>&2
}
