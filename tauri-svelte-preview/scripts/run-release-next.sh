#!/bin/sh
# Launches the RELEASE build of the /next shell with the same trusted ACP
# adapters the dev script registers. Without these env vars the provider
# registry is empty and every send fails with "No trusted adapter is
# registered for this provider". Reuses the wrapper scripts dev-next.sh
# already wrote under ~/.mac-command-bar.
set -eu

APP_HOME="$HOME/.mac-command-bar"
CODEX_WRAPPER="$APP_HOME/codex-acp-bridge.sh"
[ -x "$CODEX_WRAPPER" ] || CODEX_WRAPPER="$APP_HOME/codex-acp-dev.sh"
CLAUDE_WRAPPER="$APP_HOME/claude-acp-wrapper.sh"
AGY_WRAPPER="$APP_HOME/agy-acp-wrapper.sh"

if [ ! -x "$CODEX_WRAPPER" ] || [ ! -x "$CLAUDE_WRAPPER" ]; then
  echo "run-release-next: adapter wrappers not found under $APP_HOME." >&2
  echo "run-release-next: run 'pnpm tauri:dev:next' once first — it writes them." >&2
  exit 1
fi

sha() { shasum -a 256 "$1" | awk '{print $1}'; }

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
AGY_ACP_BIN="$SCRIPT_DIR/../tools/agy-acp/target/release/agy-acp"
AGY_CLI_BIN="$(command -v agy 2>/dev/null || true)"

MCB_CODEX_ACP_PATH="$CODEX_WRAPPER"
MCB_CODEX_ACP_SHA256="$(sha "$CODEX_WRAPPER")"
MCB_CLAUDE_AGENT_ACP_PATH="$CLAUDE_WRAPPER"
MCB_CLAUDE_AGENT_ACP_SHA256="$(sha "$CLAUDE_WRAPPER")"
export MCB_CODEX_ACP_PATH MCB_CODEX_ACP_SHA256 MCB_CLAUDE_AGENT_ACP_PATH MCB_CLAUDE_AGENT_ACP_SHA256

if [ -x "$AGY_ACP_BIN" ] && [ -n "$AGY_CLI_BIN" ]; then
  cat > "$AGY_WRAPPER" <<EOF
#!/bin/sh
export AGY_EXTRA_ARGS="--dangerously-skip-permissions"
exec "$AGY_ACP_BIN" "\$@"
EOF
  chmod +x "$AGY_WRAPPER"
  MCB_AGY_ACP_PATH="$AGY_WRAPPER"
  MCB_AGY_ACP_SHA256="$(sha "$AGY_WRAPPER")"
  export MCB_AGY_ACP_PATH MCB_AGY_ACP_SHA256
  echo "run-release-next: antigravity adapter $MCB_AGY_ACP_PATH"
else
  unset MCB_AGY_ACP_PATH MCB_AGY_ACP_SHA256
  echo "run-release-next: Antigravity adapter unavailable; skipping (missing agy-acp or agy CLI)."
fi

# Read the Claude login already on this Mac instead of asking for credentials the
# user has entered once. The quota client keeps this behind a flag; nothing set
# it, so the keychain path was dead and the panel fell back to manual entry.
export MCB_ALLOW_KEYCHAIN_CREDENTIALS=1

exec "$SCRIPT_DIR/../src-tauri/target/release/mac-command-bar-webview-preview" "$@"
