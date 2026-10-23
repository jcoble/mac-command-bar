#!/usr/bin/env bash
# macOS footprint capture: [output-directory] [app-process-pattern].
set -euo pipefail

if [ "$#" -gt 2 ]; then
  printf 'Usage: bash %s [output-directory] [app-process-pattern]\n' "$0" >&2
  exit 2
fi

output=${1:-"$HOME/Workbox/to-box"}
pattern=${2:-mac-command-bar-webview-preview}

# Match the executable, not this script's arguments containing the same pattern.
if app_pid=$(pgrep -of "^(.*/)?(${pattern})([[:space:]]|$)"); then
  :
else
  lookup_status=$?
  if [ "$lookup_status" -eq 1 ]; then
    printf 'App process not found: %s. Launch the app or check its process name.\n' "$pattern" >&2
  else
    printf 'App process lookup failed (exit %s): %s\n' "$lookup_status" "$pattern" >&2
  fi
  exit "$lookup_status"
fi

# Lookups complete before creating output, so an absent app leaves no empty capture.
if helpers=$(pgrep -f 'com\.apple\.WebKit'); then
  :
else
  lookup_status=$?
  [ "$lookup_status" -eq 1 ] || exit "$lookup_status"
  helpers=
fi

mkdir -p "$output"
capture_dir=$(mktemp -d "$output/footprint-$(date +%Y%m%d-%H%M%S)-XXXXXX")
printf '%s\n' "$capture_dir"

footprint -p "$app_pid" > "$capture_dir/app-$app_pid.txt" 2>&1
while IFS= read -r pid; do
  [ -n "$pid" ] || continue
  footprint -p "$pid" > "$capture_dir/webkit-$pid.txt" 2>&1
done <<< "$helpers"
