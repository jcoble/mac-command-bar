#!/usr/bin/env bash
# Tests for ci-checks.sh. Run from anywhere: bash scripts/ci-checks.test.sh
set -u
script="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/ci-checks.sh"
bash=$(command -v bash)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failures=0

# expect <name> <expected exit> <expected text in output> <command...>
expect() {
  local name=$1 code=$2 text=$3 out status
  shift 3
  out=$("$@" 2>&1)
  status=$?
  if [ "$status" = "$code" ] && [[ $out == *"$text"* ]]; then
    echo "ok: $name"
  else
    echo "FAIL: $name (exit $status, wanted $code and '$text')"
    echo "$out"
    failures=$((failures + 1))
  fi
}

# A node that reports the wrong major version.
mkdir "$tmp/old-node"
printf '#!/bin/sh\necho v18.0.0\n' > "$tmp/old-node/node"
chmod +x "$tmp/old-node/node"

expect "wrong Node major fails with the reason" 1 "REASON: node version" \
  env PATH="$tmp/old-node:$PATH" "$bash" "$script" node
expect "wrong Node major is the last line" 0 "REASON: node version" \
  bash -c 'PATH="$1:$PATH" "$2" "$3" node 2>&1 | tail -n 1' _ "$tmp/old-node" "$bash" "$script"
expect "unknown part exits 2" 2 "usage:" "$bash" "$script" bogus
expect "no part exits 2" 2 "usage:" "$bash" "$script"

# An empty PATH hides any installed Flutter.
mkdir "$tmp/empty"
expect "flutter without Flutter is skipped" 0 "skipped: no Flutter" \
  env PATH="$tmp/empty" "$bash" "$script" flutter

[ "$failures" = 0 ] || exit 1
