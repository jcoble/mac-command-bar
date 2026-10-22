#!/usr/bin/env bash
# Starts the app built by build-test-app.sh in proof mode (workflow redesign plan 3.11).
# Usage: launch-test-app.sh <run-folder>
# stdin: the planted value, then the fixture label (an empty line for none); the app reads both.
# The caller sets DISPLAY, HOME and the head server's ASSEMBLY_REMOTE_WS_URL.
set -euo pipefail

RUN_FOLDER="${1:?usage: launch-test-app.sh <run-folder>}"
APP="$(cd "$(dirname "$0")/.." && pwd)/tauri-svelte-preview"
BINARY="${CARGO_TARGET_DIR:-$APP/src-tauri/target}/debug/mac-command-bar-webview-preview"

export GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1
exec "$BINARY" --proof-mode "$RUN_FOLDER"
