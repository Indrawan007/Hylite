#!/usr/bin/env bash
# Always create a private X server: never send input to the caller's desktop.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
for tool in xvfb-run Xvfb xauth xdotool xclip import python3; do
  command -v "$tool" >/dev/null || { echo "Missing GUI smoke dependency: $tool" >&2; exit 2; }
done
exec xvfb-run -a -s '-screen 0 1440x1000x24 -nolisten tcp' \
  bash -c 'export HYLITE_SMOKE_DISPLAY="$DISPLAY"; exec python3 "$@"' \
  bash "$root/scripts/gui-smoke-x11.py" "$@"
