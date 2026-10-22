#!/usr/bin/env bash
# The one list of checks that CI runs. Usage: scripts/ci-checks.sh <node|rust|flutter>
# Exit 0 = every command passed. On failure the last line is "REASON: <command>".
# Setup (pnpm install, system packages, git config) stays in the CI workflow.
set -u
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

fail() {
  echo "REASON: $1"
  exit 1
}

# run <directory> <command...>: run one command from <directory> (relative to the repo root).
run() {
  local dir=$1
  shift
  echo "+ $*"
  (cd "$root/$dir" && "$@") || fail "$*"
}

case "${1:-}" in
  node)
    want=$(<"$root/.nvmrc")
    have=$(node --version | sed 's/^v//; s/\..*//')
    [ "$have" = "$want" ] || fail "node version"
    app=tauri-svelte-preview
    run $app pnpm test:async-lifecycle
    run $app pnpm exec node --experimental-strip-types scripts/scanSchedulers.test.ts
    run $app pnpm test:workspace-invoke
    run $app pnpm test:git-diff-timeout
    run $app pnpm test:regressions
    ;;
  rust)
    # Skipped on this runner only: the two lsp tests need the language servers that pnpm install puts in node_modules/.bin.
    run tauri-svelte-preview/src-tauri cargo check --locked --no-default-features
    run tauri-svelte-preview/src-tauri cargo test --locked --no-default-features -- \
      --skip files::lsp::tests::a_read_mode_workspace_starts_no_language_server \
      --skip files::lsp::tests::command_search_paths_prefer_pinned_application_language_servers
    run core cargo test --locked
    ;;
  flutter)
    if ! command -v flutter > /dev/null; then
      echo "skipped: no Flutter"
      exit 0
    fi
    run mobile flutter test
    ;;
  *)
    echo "usage: scripts/ci-checks.sh <node|rust|flutter>" >&2
    exit 2
    ;;
esac
