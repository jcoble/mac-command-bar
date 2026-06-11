# MacCommandBar

Native macOS menu-bar command center for daily developer operations.

The app is intentionally split into:

- SwiftUI/AppKit shell: menu bar UI, settings, clipboard capture, confirmations, launch-at-login, terminal/IDE launchers.
- Rust helper: JSON protocol, scanners, command planning, encrypted content primitives, and fast filesystem/process/session adapters.

## Current V1 Surface

- Project launch profiles
- Port and process doctor
- Codex and Claude session discovery
- Worktree manager
- Clipboard vault
- Paste-to-artifact templates
- Dev health dashboard
- Repo cleanup assistant
- Command palette
- Focus/indexing control panel

Destructive actions are confirm-first. Dirty or unmerged worktree deletion is blocked before confirmation.

## Build

```bash
scripts/build-app.sh
open dist/MacCommandBar.app
```

The script builds `mcb-core`, builds the Swift executable, creates `dist/MacCommandBar.app`, copies the helper into `Contents/Resources`, and ad-hoc signs the bundle for local use.

Local/ad-hoc builds use a private file-backed AES key at `~/Library/Application Support/MacCommandBar/content-key.bin` for clipboard vault encryption. This avoids repeated macOS Keychain prompts after every rebuild. To opt back into Keychain storage for a stable signed build, launch with `MCB_USE_KEYCHAIN_CIPHER=1`.

For development without the app bundle:

```bash
cargo build --manifest-path core/Cargo.toml
MCB_CORE_PATH="$PWD/core/target/debug/mcb-core" swift run MacCommandBar
```

## Test

```bash
cargo test --manifest-path core/Cargo.toml
swift test
```

## Orchestration Events

External agent loops can write dashboard events with the stable repo-level
bridge:

```bash
scripts/mcb-orch --run-id run-tsk-127 --preset issue-found --issue-id AUTH-7
scripts/mcb-orch --run-id run-tsk-127 --preset batch-delegated --agent-role fix-agent
scripts/mcb-orch --run-id run-tsk-127 --preset ui-verified --scenario "Google auth"
```

Events are appended to
`~/Library/Application Support/MacCommandBar/orchestration-events.jsonl` by
default. Set `MAC_COMMAND_BAR_ORCHESTRATION_EVENTS=/path/to/events.jsonl` to
redirect them for tests or isolated sessions.

## Profiles

V1 uses global app profiles rather than repo-local config files. See `config/profiles.example.toml` for the intended shape.

## Safety Rules

- Process kills are planned as `kill -TERM <pid>` and require confirmation.
- Escalation to `kill -KILL` should be a separate confirmed action.
- Worktree deletion is blocked when dirty or unmerged.
- Cleanup and indexing actions should be reversible where macOS allows it.
- No privileged helper or `sudo` actions in v1.
