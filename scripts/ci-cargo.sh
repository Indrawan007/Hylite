#!/usr/bin/env bash
# Preserve the exact command result while exposing diagnostics via Checks API.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
log="$(mktemp "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/hylite-ci.XXXXXX")"
trap 'rm -f -- "$log"' EXIT
status=0
"$@" >"$log" 2>&1 || status=$?
python3 "$root/scripts/ci-report.py" "$log" "$status"
exit "$status"
