#!/usr/bin/env bash
# Builds the app the proof step drives (workflow redesign plan 3.11): the frontend with Vite, then
# the Rust app with that frontend embedded, never a Vite dev server. The binary lands in
# ${CARGO_TARGET_DIR:-tauri-svelte-preview/src-tauri/target}/debug/. In a folder without .git,
# pass the head commit in ASSEMBLY_BUILD_STAMP; `--print-stamp` prints what was built.
set -euo pipefail

APP="$(cd "$(dirname "$0")/.." && pwd)/tauri-svelte-preview"

pnpm -C "$APP" install --frozen-lockfile --prefer-offline
pnpm -C "$APP" build
cargo build --locked --manifest-path "$APP/src-tauri/Cargo.toml" --features tauri/custom-protocol
