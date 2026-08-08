# TSK-808 PR-flow receipt

Started: 2026-08-08
Worktree: `/Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave`

## Scope

- Source-control commit-message generation through the active agent conversation runtime.
- New pull-request panel with generated title/description, push/create flow, status polling, and graceful failures.
- Minimal backend git/gh commands and pinned capability registration.
- No commit will be created by this lane.

## Evidence log

- 2026-08-08 — **verified** receipt created before repository edits.

## File/line claims

- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/git_pr.rs:61-158` exposes the five scoped Tauri commands; commit-message generation reads staged/working diffs, PR generation reads branch commits plus committed/pending diff sections, creation pushes with argument arrays then invokes `gh pr create`, and status reads `gh pr view --json number,url,state,statusCheckRollup`.
- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/git_pr.rs:160-195,516-539` assembles bounded prompts, requires raw title/description JSON, forbids generated trailers, and truncates on a UTF-8 boundary.
- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/git_pr.rs:542-559` preserves exact no-`gh` and stderr/stdout failure explanations; `tauri-svelte-preview/src-tauri/src/git_pr.rs:566-656` covers prompt assembly, diff sections, gh/push argument construction, JSON parsing, URL-number parsing, and Unicode bounds without shelling out in tests.
- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:337-357` and `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs:157-244` add the one-shot ACP prompt path; the ACP fixture test also proves text-chunk capture.
- 2026-08-08 — **verified** `tauri-svelte-preview/src-tauri/src/main.rs:1242-1283,5393-5401,8273-8285` registers and pins all five backend command names.
- 2026-08-08 — **verified** `tauri-svelte-preview/src/lib/shell/components/git/ChangesPane.svelte:48-151,187-231` provides the Generate commit message action, visibly explains the no-agent state, and fills the existing commit box; `RepoPane.svelte:163-176` opens the PR panel.
- 2026-08-08 — **verified** `tauri-svelte-preview/src/lib/shell/components/git/SourceControlPanes.svelte:76-174` binds the action to the active structured agent session and mounts the PR panel; `pr/PullRequestPanel.svelte:75-143,168-295` implements generating, ready, pushing, created/status polling, draft, URL, and visible failure states.
- 2026-08-08 — **verified** `tauri-svelte-preview/scripts/prPanel.test.mjs:23-73` passes idle → generating → ready → pushing → created/checks plus no-agent and no-`gh` degradation assertions (`pnpm test:pr-panel`).
- 2026-08-08 — **verified** `RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml git_pr -- --test-threads=1`: 7 passed, 0 failed.
- 2026-08-08 — **verified** `pnpm check:svelte`: `/next` 0 errors and 0 warnings; the command reports 16 pre-existing errors elsewhere in the old shell, outside this lane. `pnpm check` also exited 0.
- 2026-08-08 — **verified** `pnpm build` exited 0 and wrote the static site; existing `+page.svelte`/activity-panel a11y and chunk-size warnings remain outside this lane.
- 2026-08-08 — **assumed** no live push, PR creation, or real `gh` authentication was exercised; tests cover pure argument/message assembly and the UI surfaces backend stderr exactly. No commit was created.
- 2026-08-08 — **verified** final review retained separate committed-branch and staged/working diff sections for PR generation, preserved the active-session/no-agent gate, and left the worktree uncommitted.
- 2026-08-08 — **verified** the panel now renders the requested literal `Generating title & description...` state; `pnpm test:pr-panel`, `pnpm check:svelte`, and `pnpm build` were rerun successfully afterward.
- 2026-08-08 — **verified** `ChangesPane.svelte` visibly renders the no-agent explanation beside the disabled generation action; the Svelte gate and production build were rerun successfully afterward.
- 2026-08-08 — **verified** ACP one-shot capture also accepts the shared `type` update discriminator used by the existing frame projector; the targeted ACP fixture still passes.
- 2026-08-08 — **verified** `pnpm check` exited 0 after the final source-control UI adjustment.
