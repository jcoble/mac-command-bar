#!/bin/sh
# Dev launcher for the primary Assembly shell. Registers the real ACP adapters
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
  echo "dev-app: missing ACP adapter (claude-agent-acp: 'not found')." >&2
  echo "dev-app: install with: npm i -g @agentclientprotocol/claude-agent-acp" >&2
  echo "dev-app: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
  exec pnpm exec tauri dev "$@"
fi

if [ -n "$CODEX_BIN" ]; then
  if [ -z "$NODE_BIN" ]; then
    echo "dev-app: installed codex CLI found, but node is missing; the ACP bridge cannot start." >&2
    echo "dev-app: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
    exec pnpm exec tauri dev "$@"
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
  echo "dev-app: missing codex CLI and codex-acp fallback." >&2
  echo "dev-app: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
  exec pnpm exec tauri dev "$@"
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

AGY_ACP_DIR="$SCRIPT_DIR/../tools/agy-acp"
AGY_ACP_BIN="$AGY_ACP_DIR/target/release/agy-acp"
CARGO_BIN="$(resolve cargo)"
# The Antigravity adapter is tracked as source, so a dev launch builds it when
# the binary is missing or older than any file it is built from. Without this a
# change to the adapter would run as whatever was compiled last.
if [ -n "$AGY_CLI_BIN" ] && [ -n "$CARGO_BIN" ]; then
  if [ ! -x "$AGY_ACP_BIN" ] || [ -n "$(find "$AGY_ACP_DIR/src" -type f -newer "$AGY_ACP_BIN")" ]; then
    echo "dev-app: building the Antigravity adapter from tools/agy-acp."
    if ! "$CARGO_BIN" build --release --manifest-path "$AGY_ACP_DIR/Cargo.toml"; then
      echo "dev-app: the Antigravity adapter did not build; continuing without it." >&2
    fi
  fi
fi
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
  echo "dev-app: Antigravity adapter unavailable; skipping (missing agy-acp or agy CLI)."
fi

export MCB_CODEX_ACP_PATH MCB_CODEX_ACP_SHA256 MCB_CLAUDE_AGENT_ACP_PATH MCB_CLAUDE_AGENT_ACP_SHA256
if [ "$AGY_ADAPTER_CONFIGURED" -eq 1 ]; then
  export MCB_AGY_ACP_PATH MCB_AGY_ACP_SHA256
fi

# Read the Claude login already on this Mac instead of asking for credentials the
# user has entered once. The quota client keeps this behind a flag; nothing set
# it, so the keychain path was dead and the panel fell back to manual entry.
export MCB_ALLOW_KEYCHAIN_CREDENTIALS=1

echo "dev-app: codex adapter  $MCB_CODEX_ACP_PATH ($CODEX_ADAPTER_DETAIL)"
echo "dev-app: claude adapter $MCB_CLAUDE_AGENT_ACP_PATH"
if [ "$AGY_ADAPTER_CONFIGURED" -eq 1 ]; then
  echo "dev-app: antigravity adapter $MCB_AGY_ACP_PATH"
fi

# A previous shell can exit while its Vite server remains alive. In that case,
# attach the new shell to this checkout's server instead of failing on port 5177.
if [ "$#" -eq 0 ]; then
  VITE_PID="$(lsof -tiTCP:5177 -sTCP:LISTEN 2>/dev/null | head -n 1)"
  if [ -n "$VITE_PID" ]; then
    VITE_CWD="$(lsof -a -p "$VITE_PID" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | head -n 1)"
    APP_DIR="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"
    if [ "$VITE_CWD" = "$APP_DIR" ]; then
      echo "dev-app: attaching to the existing Vite server on 127.0.0.1:5177"
      set -- --config "$APP_DIR/src-tauri/tauri.dev.attach.conf.json"
    fi
  fi
fi

exec pnpm exec tauri dev "$@"
