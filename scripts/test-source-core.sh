#!/usr/bin/env bash
# Requires rustc, but no downloaded crates, GUI, or native dialogs.
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/hylite-core-tests.XXXXXX")"
trap 'rm -rf -- "$work_dir"' EXIT
"${RUSTC:-rustc}" --edition=2021 -D warnings --test \
    "$root/tools/source-core-tests.rs" -o "$work_dir/source-core-tests"
"$work_dir/source-core-tests" "$@"
