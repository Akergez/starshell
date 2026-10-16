#!/usr/bin/env bash
# build-aux/screenshots.sh --headless inside podman: the same compositor,
# output and fonts everywhere. The shell is built on the host and mounted in.
set -Eeuo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$root"

image=${UI_TEST_IMAGE:-localhost/starshell-screenshots}
fedora=$(. /etc/os-release 2>/dev/null && [[ ${ID:-} == fedora ]] && echo "$VERSION_ID" || echo 44)

cargo build -p starshell
podman build -q --build-arg FEDORA="$fedora" -t "$image" -f build-aux/ui-tests.Containerfile build-aux >/dev/null

exec podman run --rm --userns=keep-id --security-opt label=disable \
  -v "$root:$root" -w "$root" \
  -e UI_TEST_SIZE -e UI_TEST_SETTLE \
  "$image" dbus-run-session -- build-aux/screenshots.sh --headless "$@"
