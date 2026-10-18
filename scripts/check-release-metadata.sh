#!/usr/bin/env bash
set -euo pipefail

manifest_version="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
lock_version="$(awk '
  $0 == "name = \"tickt\"" { in_tickt = 1; next }
  in_tickt && /^version = / { gsub(/"/, "", $3); print $3; exit }
  /^\[\[package\]\]/ { in_tickt = 0 }
' Cargo.lock)"
notes="release-notes/v${manifest_version}.txt"

test -n "$manifest_version"
test "$manifest_version" = "$lock_version"
test -f "$notes"
grep -q "Tickt v${manifest_version}" "$notes"

printf 'Release metadata is consistent for v%s\n' "$manifest_version"
