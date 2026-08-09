#!/bin/sh
# Dev launcher for the /next shell. Registers the real ACP adapters
# (codex-acp, claude-code-acp) with the provider registry via the
# MCB_*_ACP_PATH/SHA256 env vars that bundled_from_environment() reads.
# Without these the registry is empty and every structured session fails
# activation with "No trusted adapter is registered for this provider".
set -eu

APP_HOME="$HOME/.mac-command-bar"
CODEX_ACP_HOME="$APP_HOME/codex-acp-home"

resolve() {
  command -v "$1" 2>/dev/null || true
}

CODEX_ACP_BIN="$(resolve codex-acp)"
CLAUDE_ACP_BIN="$(resolve claude-code-acp)"

if [ -z "$CODEX_ACP_BIN" ] || [ -z "$CLAUDE_ACP_BIN" ]; then
  echo "dev-next: missing ACP adapters (codex-acp: '${CODEX_ACP_BIN:-not found}', claude-code-acp: '${CLAUDE_ACP_BIN:-not found}')." >&2
  echo "dev-next: install with: npm i -g @zed-industries/codex-acp @zed-industries/claude-code-acp" >&2
  echo "dev-next: continuing WITHOUT structured providers - new app sessions will fail to connect." >&2
  exec pnpm exec tauri dev --config src-tauri/tauri.dev.next.conf.json "$@"
fi

# codex-acp may bundle an older codex core than the installed CLI; the
# user's ~/.codex/config.toml can contain values it cannot parse (this
# refused "max" reasoning effort outright). Give the adapter its own
# CODEX_HOME with a minimal config and the shared auth.
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

# The registry spawns the manifest executable with no env control, so the
# CODEX_HOME override rides in a wrapper script that is the registered
# executable.
CODEX_WRAPPER="$APP_HOME/codex-acp-dev.sh"
cat > "$CODEX_WRAPPER" <<EOF
#!/bin/sh
export CODEX_HOME="$CODEX_ACP_HOME"
exec "$CODEX_ACP_BIN" "\$@"
EOF
chmod +x "$CODEX_WRAPPER"

sha() {
  shasum -a 256 "$1" | awk '{print $1}'
}

MCB_CODEX_ACP_PATH="$CODEX_WRAPPER"
MCB_CODEX_ACP_SHA256="$(sha "$CODEX_WRAPPER")"
MCB_CLAUDE_AGENT_ACP_PATH="$CLAUDE_ACP_BIN"
MCB_CLAUDE_AGENT_ACP_SHA256="$(sha "$CLAUDE_ACP_BIN")"
export MCB_CODEX_ACP_PATH MCB_CODEX_ACP_SHA256 MCB_CLAUDE_AGENT_ACP_PATH MCB_CLAUDE_AGENT_ACP_SHA256

echo "dev-next: codex adapter  $MCB_CODEX_ACP_PATH (CODEX_HOME=$CODEX_ACP_HOME)"
echo "dev-next: claude adapter $MCB_CLAUDE_AGENT_ACP_PATH"

exec pnpm exec tauri dev --config src-tauri/tauri.dev.next.conf.json "$@"
