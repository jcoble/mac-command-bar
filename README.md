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

## Roadmap

The active implementation plan is tracked in
[`docs/TSK-127-IMPLEMENTATION-PLAN.md`](docs/TSK-127-IMPLEMENTATION-PLAN.md).
Use it as the execution order for the command-center/IDE work before starting
new feature slices.

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
scripts/mcb-orch --run-id run-tsk-127 --preset batch-delegated --agent-role fix-agent --issue-count 4 --fix-count 2
scripts/mcb-orch --run-id run-tsk-127 --preset ui-verified --scenario "Google auth" --resolved-count 3 --verified-count 2
```

Inside the app, open the command palette and run `Copy run-started event
command`, `Copy scenario-started event command`, `Copy issue-found event
command`, `Copy fix-batch event command`, `Copy UI-verified event command`, or
`Copy approval-required event command`. The copied command includes the current
project path, task ID when known, and a stable run ID, so it can be pasted into an
agent prompt, shell, or orchestrator script and then edited for the specific
scenario or issue.

Count flags such as `--issue-count`, `--fix-count`, `--resolved-count`, and
`--verified-count` feed the run tally directly, so a batched agent update can
represent more than one issue or fix.

Events are appended to the `orchestration_events` table in
`~/Library/Application Support/MacCommandBar/sessions.db` by default. Set
`MAC_COMMAND_BAR_ORCHESTRATION_DB=/path/to/sessions.db` to redirect the command
for an isolated session.

## Profiles

V1 uses global app profiles rather than repo-local config files. See `config/profiles.example.toml` for the intended shape.

## Safety Rules

- Process kills are planned as `kill -TERM <pid>` and require confirmation.
- Escalation to `kill -KILL` should be a separate confirmed action.
- Worktree deletion is blocked when dirty or unmerged.
- Cleanup and indexing actions should be reversible where macOS allows it.
- No privileged helper or `sudo` actions in v1.
