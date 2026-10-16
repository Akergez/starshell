#!/usr/bin/env bash
# Run Starshell under a private headless sway and capture the screen after
# each IPC command, so the UI can be checked without touching your session.
#
#   build-aux/screenshots.sh --headless toggle-dashboard toggle-audio
#   build-aux/screenshots-container.sh toggle-dashboard
#
# Every command is sent through `starshell <command>`; a screenshot named
# after it lands in $UI_TEST_SHOTS (default target/ui-shots).
set -Eeuo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$root"

app=${STARSHELL_BIN:-$root/target/debug/starshell}
shots=${UI_TEST_SHOTS:-$root/target/ui-shots}
size=${UI_TEST_SIZE:-1280x800}

if [[ ${1:-} == --headless ]]; then
  shift
  runtime=$(mktemp -d)
  chmod 700 -- "$runtime"
  export XDG_RUNTIME_DIR=$runtime
  export HOME=$runtime/home XDG_CONFIG_HOME=$runtime/home/.config
  mkdir -p -- "$XDG_CONFIG_HOME"
  unset DISPLAY WAYLAND_DISPLAY SWAYSOCK NIRI_SOCKET HYPRLAND_INSTANCE_SIGNATURE
  printf '%s\n' 'default_border none' "output * resolution $size" > "$runtime/sway.conf"
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    sway --unsupported-gpu -c "$runtime/sway.conf" >"$runtime/sway.log" 2>&1 &
  compositor=$!
  for _ in $(seq 100); do
    socket=$(find "$runtime" -maxdepth 1 -name 'wayland-*' ! -name '*.lock' -print -quit)
    [[ -n $socket ]] && break
    sleep 0.1
  done
  [[ -n ${socket:-} ]] || { cat "$runtime/sway.log" >&2; exit 1; }
  export WAYLAND_DISPLAY=${socket##*/}
  trap 'kill "$compositor" 2>/dev/null || true; rm -rf -- "$runtime"' EXIT
fi

mkdir -p -- "$shots"
"$app" >"$shots/starshell.log" 2>&1 &
shell=$!
sleep "${UI_TEST_SETTLE:-3}"
grim "$shots/bar.png"
for command in "$@"; do
  "$app" "$command"
  sleep 1
  grim "$shots/$command.png"
  "$app" hide
  sleep 0.5
done
kill "$shell" 2>/dev/null || true
