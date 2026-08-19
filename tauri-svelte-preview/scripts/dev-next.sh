#!/bin/sh
# Dev launcher for the /next shell. Registers the real ACP adapters
# (the installed codex CLI bridge, claude-agent-acp) with the provider registry via the
# MCB_*_ACP_PATH/SHA256 env vars that bundled_from_environment() reads.
# Without these the registry is empty and every structured session fails
# activation with "No trusted adapter is registered for this provider".
set -eu

APP_HOME="$HOME/.mac-command-bar"
CODEX_ACP_HOME="$APP_HOME/codex-acp-home"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
CODEX_BRIDGE_MJS="$SCRIPT_DIR/../tools/codex-acp-bridge/bridge.mjs"

resolve() {
  command -v "$1" 2>/dev/null || true
}

CODEX_BIN="$(resolve codex)"
NODE_BIN="$(resolve node)"
CODEX_ACP_FALLBACK_BIN="$(resolve codex-acp)"
CLAUDE_ACP_BIN="$(resolve claude-agent-acp)"
AGY_CLI_BIN="$(resolve agy)"

if [ -z "$CLAUDE_ACP_BIN" ]; then
  echo "dev-next: missing ACP adapter (claude-agent-acp: 'not found')." >&2
  echo "dev-next: install with: npm i -g @agentclientprotocol/claude-agent-acp" >&2
  echo "dev-next: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
  exec pnpm exec tauri dev --config src-tauri/tauri.dev.next.conf.json "$@"
fi

if [ -n "$CODEX_BIN" ]; then
  if [ -z "$NODE_BIN" ]; then
    echo "dev-next: installed codex CLI found, but node is missing; the ACP bridge cannot start." >&2
    echo "dev-next: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
    exec pnpm exec tauri dev --config src-tauri/tauri.dev.next.conf.json "$@"
  fi

  mkdir -p "$APP_HOME"
  CODEX_WRAPPER="$APP_HOME/codex-acp-bridge.sh"
  cat > "$CODEX_WRAPPER" <<EOF
#!/bin/sh
export CODEX_BIN="$CODEX_BIN"
exec "$NODE_BIN" "$CODEX_BRIDGE_MJS" "\$@"
EOF
  chmod +x "$CODEX_WRAPPER"
  CODEX_ADAPTER_DETAIL="installed CLI via app-server"
elif [ -n "$CODEX_ACP_FALLBACK_BIN" ]; then
  # Fallback for machines without the codex CLI. This legacy adapter needs an
  # isolated config because its bundled core cannot parse current user config.
  mkdir -p "$CODEX_ACP_HOME"
  if [ ! -e "$CODEX_ACP_HOME/auth.json" ] && [ -f "$HOME/.codex/auth.json" ]; then
    ln -s "$HOME/.codex/auth.json" "$CODEX_ACP_HOME/auth.json"
  fi
  if [ ! -f "$CODEX_ACP_HOME/config.toml" ]; then
    cat > "$CODEX_ACP_HOME/config.toml" <<'EOF'
# No model pin: the adapter's bundled codex core predates the gpt-5.6
# models and the API rejects them from it ("requires a newer version of
# Codex"), so it must run its own default model.
approval_policy = "untrusted"
sandbox_mode = "workspace-write"
EOF
  fi
  CODEX_WRAPPER="$APP_HOME/codex-acp-dev.sh"
  cat > "$CODEX_WRAPPER" <<EOF
#!/bin/sh
export CODEX_HOME="$CODEX_ACP_HOME"
exec "$CODEX_ACP_FALLBACK_BIN" "\$@"
EOF
  chmod +x "$CODEX_WRAPPER"
  CODEX_ADAPTER_DETAIL="legacy codex-acp fallback"
else
  echo "dev-next: missing codex CLI and codex-acp fallback." >&2
  echo "dev-next: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
  exec pnpm exec tauri dev --config src-tauri/tauri.dev.next.conf.json "$@"
fi

sha() {
  shasum -a 256 "$1" | awk '{print $1}'
}

MCB_CODEX_ACP_PATH="$CODEX_WRAPPER"
MCB_CODEX_ACP_SHA256="$(sha "$CODEX_WRAPPER")"
# claude-agent-acp refuses to start when it thinks it is nested inside another
# Claude Code session; the dev stack is often launched from one, so the adapter
# runs through a wrapper that scrubs the session-marker environment.
CLAUDE_WRAPPER="$HOME/.mac-command-bar/claude-acp-wrapper.sh"
{
  printf '#!/bin/sh\n'
  printf 'unset CLAUDECODE CLAUDE_CODE_ENTRYPOINT CLAUDE_CODE_SSE_PORT CLAUDE_CODE_PLUGIN_ROOT CLAUDE_PLUGIN_DATA\n'
  printf 'exec "%s" "%s" "$@"\n' "$NODE_BIN" "$CLAUDE_ACP_BIN"
} > "$CLAUDE_WRAPPER"
chmod +x "$CLAUDE_WRAPPER"
MCB_CLAUDE_AGENT_ACP_PATH="$CLAUDE_WRAPPER"
MCB_CLAUDE_AGENT_ACP_SHA256="$(sha "$CLAUDE_WRAPPER")"

AGY_ACP_BIN="$SCRIPT_DIR/../artifacts/recon/probe-agy/agy-acp/target/release/agy-acp"
AGY_ADAPTER_CONFIGURED=0
if [ -x "$AGY_ACP_BIN" ] && [ -n "$AGY_CLI_BIN" ]; then
  AGY_WRAPPER="$APP_HOME/agy-acp-wrapper.sh"
  cat > "$AGY_WRAPPER" <<EOF
#!/bin/sh
export AGY_EXTRA_ARGS="--dangerously-skip-permissions"
exec "$AGY_ACP_BIN" "\$@"
EOF
  chmod +x "$AGY_WRAPPER"
  MCB_AGY_ACP_PATH="$AGY_WRAPPER"
  MCB_AGY_ACP_SHA256="$(sha "$AGY_WRAPPER")"
  AGY_ADAPTER_CONFIGURED=1
else
  unset MCB_AGY_ACP_PATH MCB_AGY_ACP_SHA256
  echo "dev-next: Antigravity adapter unavailable; skipping (missing agy-acp or agy CLI)."
fi

export MCB_CODEX_ACP_PATH MCB_CODEX_ACP_SHA256 MCB_CLAUDE_AGENT_ACP_PATH MCB_CLAUDE_AGENT_ACP_SHA256
if [ "$AGY_ADAPTER_CONFIGURED" -eq 1 ]; then
  export MCB_AGY_ACP_PATH MCB_AGY_ACP_SHA256
fi

# Read the Claude login already on this Mac instead of asking for credentials the
# user has entered once. The quota client keeps this behind a flag; nothing set
# it, so the keychain path was dead and the panel fell back to manual entry.
export MCB_ALLOW_KEYCHAIN_CREDENTIALS=1

echo "dev-next: codex adapter  $MCB_CODEX_ACP_PATH ($CODEX_ADAPTER_DETAIL)"
echo "dev-next: claude adapter $MCB_CLAUDE_AGENT_ACP_PATH"
if [ "$AGY_ADAPTER_CONFIGURED" -eq 1 ]; then
  echo "dev-next: antigravity adapter $MCB_AGY_ACP_PATH"
fi

exec pnpm exec tauri dev --config src-tauri/tauri.dev.next.conf.json "$@"
