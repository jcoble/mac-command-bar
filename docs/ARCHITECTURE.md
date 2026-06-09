# Architecture

## Runtime Shape

`MacCommandBar` is a native macOS utility app with a Swift shell and Rust helper.

- `Sources/MacCommandBar`: SwiftUI menu-bar app and settings window.
- `Sources/MacCommandBarKit`: testable Swift models, view state, confirmation policy, profile parsing, clipboard vault, and helper client.
- `core`: Rust `mcb-core` helper. It receives one JSON request on stdin and returns one JSON response on stdout.

## JSON Protocol

Requests use this shape:

```json
{
  "id": "request-id",
  "action": "scan.worktrees",
  "dryRun": true,
  "payload": {}
}
```

Responses use this shape:

```json
{
  "id": "request-id",
  "ok": true,
  "summary": "found 2 worktrees",
  "data": {},
  "warnings": []
}
```

## Implemented Helper Actions

- `scan.worktrees`: runs `git worktree list --porcelain` and enriches records with dirty, unmerged, disk, and last-activity data.
- `scan.processes`: parses `lsof -nP -iTCP -sTCP:LISTEN`.
- `scan.sessions`: reads Codex and Claude session indexes from local home directories.
- `health.snapshot`: returns basic host/cwd/home state.
- `plan.killProcess`: returns a confirmable kill plan without executing it.

## V1 Deliberate Limits

- No privileged helper.
- No background daemon.
- No cloud sync.
- No immediate destructive actions.
- Rust helper is short-lived per request until persistent polling is proven necessary.

