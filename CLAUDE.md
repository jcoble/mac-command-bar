# mac-command-bar — project instructions

The point of this project: a very low-resource, fast app — half IDE, half modern
agent harness. Memory and speed are first-class requirements, not polish.
Every change is judged against them.

## ⛔ The orchestrator's job (owner directive, 2026-08-20)

The controller session ORCHESTRATES. It does not implement — when it starts
implementing it gets lost in the work, stops listening, and loses what the
owner said. The controller:

1. **Writes everything down, immediately.** Everything the owner says that
   comes from their issue lists and screenshots, the nuances of what we have
   been working on, what was tried, what was agreed — and especially
   **decisions and WHY**. Record them the moment they are said, in the handoff
   brief (`~/.claude/handoffs/mac-command-bar/`) and in the plan/spec docs
   (`docs/superpowers/`). Losing a decision and making the owner repeat
   themselves is the project's worst recurring failure.
2. Guides subagents: writes specs, dispatches lanes, monitors them.
3. Plans WITH the owner, reviews diffs, and helps make decisions.
4. Slows down and does things correctly rather than quickly.
5. **Invokes `superpowers:systematic-debugging`** for any debugging-shaped
   work — especially memory reduction. No fixes before the cause is
   understood and measured.

## Standing project decisions

- Session-switch unloading is the accepted memory direction: panels are torn
  down to bare (component stays, content unloaded); editors are disposed —
  they hold the most RAM and cannot be "stripped down" like a panel.
  The `{#key}` remount mechanism is banned (froze the app); use explicit
  teardown sequenced before the workspace replay.
- Monaco stays. Replacing it with another editor + tree-sitter was discussed
  and rejected: far more work, and the owner wants Monaco, CodeLens, and the
  language-server features. Tree-sitter appears only as the highlighter in the
  Editor/Supercharged design (`docs/superpowers/specs/2026-08-19-tsk-932-wave3-design.html` §A).
- Editor/Supercharged switch + Settings → Supercharged (per-server on/off,
  switch-off stops all servers) is designed in wave-3 (§A, plan T24). Not
  built yet.
- VS Code views host + custom SCM provider are adopted
  (`src/lib/shell/editor/csharpLanguageClient.ts:379`,
  `src/lib/shell/extensions/rustGitScmProvider.ts`).
- Memory judged by Activity Monitor at idle/after actions — never the Web
  Inspector (it inflates memory).
- No UI regression tests (owner moratorium). No CSS `:has()`.
  DESIGN.md (`tauri-svelte-preview/src/lib/components/ui/DESIGN.md`) is
  binding for /next UI.

## Mechanics

- Frontend gates: `npx tsc --noEmit` + `pnpm run check:svelte` must end
  "Files the /next shell owns: 0 error(s)". `core/` is a standalone crate:
  `cd core && cargo test`.
- Worktrees under `/Users/blackcolours/dev/work/worktrees/mac-command-bar/`,
  deleted immediately after merge. Never edit a worktree the owner's dev
  server is running from without warning them (HMR storms).
- Commits end with `Committed-by: <name>`; never Co-Authored-By. The
  commit-msg hook wants the architecture page staged or
  `Architecture: unchanged` in the body.
