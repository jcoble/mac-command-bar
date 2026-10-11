# Workflow authoring tools Implementation Plan

Date: 2026-10-11. Task: [TSK-1416](https://app.notion.com/p/3f5394b0689d81e5b2aef3bb8bddfea8).

**Goal:** An owner describes a workflow to an ordinary Assembly agent. The agent
reads the format tool, writes repository YAML and prompt files, checks them, and
shows the owner a file that can be chosen from the YAML-only picker.

**Architecture:** Extend the existing session-authenticated MCP endpoint with two
tools. Reuse the Task 7 loader for a validate-only path. The agent writes files;
Assembly supplies the reference and validation. ACP carries the existing MCP server.

**Spec:** `docs/superpowers/specs/2026-10-10-workflows-orchestration-design.md`
§2 D14, §4.3, §5–6; format reference `docs/workflows/format.md`.

## Global constraints and execution rules

- KISS: deliver the smallest diff that makes the named tests/behavior pass. If two designs work, ship the one with fewer moving parts.
- YAGNI: no code for hypothetical futures. No new interface/abstraction/base class, config flag, compatibility shim, factory/strategy, adjacent refactor, retries or cache. No editor UI, schema framework, JSON Schema export or generator code. Reuse one loader; delete any duplicated parsing path.
- Stop and propose if implementation needs about twice the expected diff or a file not named below. One problem per lane. Reviewers mark unnecessary complexity REWORK and do not expand scope.
- All database filtering, joins, counts, sorting and paging happen in one SQL statement; inspect SQL. Do not load all sessions to find the caller's project. Validation does not write the ledger.
- No UI regression tests, CSS `:has()` or Tailwind `has-[...]`. New frontend/tooling code is TypeScript. Do not add scheduler-scan hits or idle work.
- Identity comes only from the backend session credential. Every session/provider gets both tools; no orchestrator restriction, caller-supplied project root, Notion token or owner approval through agent text.
- Remote proof uses only **Workbox Test** (`ssh workbox-test`, `tools/workbox-test/README.md`), added with **Install and connect**. Never attach the proof app to the owner's saved machines. Do not install/update/deploy a backend as part of this proof; coordinate with the controller for its prepared build.
- Use `workbox-native-ui` for the isolated real app. Browser sessions are explicitly headless at 1710×990, verified after every start. Stop exact task-owned app/browser/helper trees and report cleanup. Proof stays within four screenshots and one video of at most two minutes.
- Before cargo commands read `free -g`; start only above 8 GB available. Use `CARGO_BUILD_JOBS=3`, `MSBUILDDISABLENODEREUSE=1`, `~/.cargo/bin/cargo`, one heavy job at a time; app builds need controller coordination.
- Before committing inspect status and staged diff. Include the Task link; update `main-architecture-explained.html` in the same implementation commit (sections touched, true file:line receipts, section-01 change log). A docs/proof-only commit uses `Architecture: unchanged`. End with `Committed-by: Codex`; no co-author trailer. Push; do not merge without explicit owner request using `ALLOW_MERGE=1`.

## File map and serial order

The slices 1–2 plan remains the execution plan. Insert **A immediately after Task
7**, once slice 1 Task 3's tool bodies and Task 7 are integrated; then continue
Tasks 8–12. Insert **B immediately after Task 12 and before Task 13**. In full:
**1 → 2 → 3 → 4 → 5 → 6 → 7 → A → 8 → 9 → 10 → 11 → 12 → B → 13**.
The controller may coordinate independent branches as stated in the lane rules;
A must not edit a concurrent loader/endpoint branch before its dependency lands.

Task 7 anchors below are from `7817ca1bb`; re-find symbols after integration.
`messaging.rs` is absent from the docs branch's `origin/main`; its anchors are
symbols from Task 2's lane, not guessed main line numbers. Task 3 may move them.
No changes to the original slices 1–2 plan are required by this addendum.

### Task A: Expose the format and validate a file without creating a run

**Files:** Modify `tauri-svelte-preview/src-tauri/src/agent_conversation/messaging.rs`
(`MessagingEndpoint::handle`, `caller`, `post` tools/list branch); modify
`tauri-svelte-preview/src-tauri/src/workflow.rs` (`contained_file`:77,
`WorkflowEngine::create_run_from_file`:1298, `WorkflowPolicy::validate_definition`:948,
`tests::yaml_fixture`:3065). Read `docs/workflows/format.md`; compile it into the
endpoint. Update `main-architecture-explained.html`. Keep tests in these Rust files.
Use Task 3's caller-project lookup; do not add a second session lookup mechanism.

**Interfaces:** MCP `workflow_format()` takes no arguments and returns one text
content item with the exact markdown from
`include_str!("../../../../docs/workflows/format.md")` relative to `messaging.rs`.
MCP `check_workflow(path: String)` takes only a project-relative path. Resolve the
project root from the credential-bound caller on that owning backend, never from
tool arguments, process cwd or another session. Missing project is an error.
Extract the existing parsing/snapshot construction into
`WorkflowEngine::validate_file(repo_root: &Path, relative_yaml: &Path) -> Result<WorkflowDefinitionV1, WorkflowError>`.
It performs the loader checks and `WorkflowPolicy::validate_definition`, returning
the definition with prompts read. `create_run_from_file` calls this entry, inserts
task/cwd into input, then calls `create_run`; validation stops before that call.
Return text `ok` followed by one deterministic line per step: ID, kind, needs,
and `of`/`on_fail` when present; on failure return the loader's exact error with
MCP `isError: true`. Do not aggregate or replace the loader's first error.

- [ ] **Red tests:** `workflow_authoring_format_is_embedded_for_every_session` invokes through two ordinary credential-bound sessions with different providers/projects, checks tools/list advertises both tools and returned text equals the file. `workflow_authoring_check_is_read_only` checks a valid agent→checks file and exact step summary; asserts ledger row count unchanged and no runtime dispatch/worktree allocation. `workflow_authoring_check_uses_callers_project` places valid same-named YAML in two project roots and checks each caller gets its own summary; missing project, caller-supplied root/identity, absolute path, `../`, wrong directory/extension and escaping YAML/prompt symlink fail. `workflow_authoring_check_preserves_loader_errors` covers duplicate key, bad step ID, unknown/cyclic `needs`, unknown `of`/`on_fail` and missing/escaping prompt; compare against the shared entry's exact errors. Include a valid two-reviewer file with the same `of` and transitive needs, not a reviewers list.
- [ ] **Verify failure:** `cd tauri-svelte-preview/src-tauri && CARGO_BUILD_JOBS=3 MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test workflow_authoring` must run the new tests (not zero tests) and fail because tools/validate-only entry do not exist. Record exact assertion/error before edits.
- [ ] **Minimum edit:** Add two tool declarations/dispatch branches using existing caller identity/project access. Extract the current loader up to definition construction and call its policy validator. Retain existing file containment and `.assembly/workflows/*.yml` checks, including rejecting `../` before canonicalization. Return success text or existing errors. No run ID, task snapshot requirement, command execution, ledger append, worker, timer or worktree. No new field validation framework or runtime behavior in this task.
- [ ] **Green:** Rerun the red command; every named test reports `ok`. Run `cd tauri-svelte-preview/src-tauri && CARGO_BUILD_JOBS=3 MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test yaml_definition_is_snapshot_and_rejects_escape` and `CARGO_BUILD_JOBS=3 MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test task_snapshot_contains_body`. Run slices 1–2 gates: `cd tauri-svelte-preview && npx tsc --noEmit && pnpm run check:svelte` (owned shell: `0 error(s)`), `pnpm --dir tauri-svelte-preview test:async-lifecycle`, and `cd core && MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test`. Record one combined code/spec review and resolve findings before handing off.

**Done means:** Any ordinary session can read the same compiled reference and
validate only its own project's workflow with the same parser used to start runs.
No persisted or runtime side effects occur. **Expected diff:** around 100–160
implementation/test lines, plus the required architecture receipts.
**Serial overlap:** Task 3 endpoint/caller access; Tasks 7–11 loader/engine.

### Task B: Show conversational authoring in a real Workbox Test session

**Files:** Write only in a disposable Workbox Test proof project:
`.assembly/workflows/two-reviewers.yml`, `prompts/build.md`, `prompts/review.md`,
`prompts/review-2.md` under that workflow folder. Record proof in TSK-1416 via the
controller and `~/Workbox/screenshots/`; no product code unless a defect is sent
back to its owning implementation task. Depends on A and Task 12's YAML picker.

**Interfaces / contract:** In an ordinary signed-provider Assembly session on
Workbox Test, send: “Write a three-step workflow: one worker builds the change,
then two separate reviewers review that same build. Put it in YAML and show me
it after checking it.” Three steps means `build`, `review`, `review-2`, with both
reviews naming `of: build`. This proves file authoring and validation, not a run,
review verdict, PR creation or merge. Do not instruct a special orchestrator to do it.

- [ ] **Red proof first:** Before the turn, record the calling session's provider, owned ID, machine and project root, the exact request above, and the expected three step IDs. Missing tool calls, prose-only YAML, an invented reviewers field, validation error or missing picker entry is FAIL. No automated component/layout test.
- [ ] **Verify failure / observation command:** Before authoring, run `ssh workbox-test 'test -f <absolute-proof-project>/.assembly/workflows/two-reviewers.yml'` with the actual quoted absolute path substituted; record expected exit 1 for absence. In the session call both tools; a tool unavailable/error is captured verbatim and stops proof pending the owning fix. Do not manufacture a red failure if the prepared build already supports them.
- [ ] **Minimum action:** Let the agent call `workflow_format()`, write YAML plus all three prompts, then call `check_workflow(".assembly/workflows/two-reviewers.yml")`. It fixes errors before presenting the file. Open Task 12's picker for that same remote project and verify the authored file is listed. Do not start the workflow or create templates through the tab.
- [ ] **Green / captured proof:** Save the exact session transcript showing the owner's plain request, both MCP tool names/arguments/results, the final `ok` and all three summary lines. Save the authored YAML and prompt contents plus project-relative paths as a readable artifact. Capture one screenshot of the real chat with the tool results and one of the picker showing workflow name/path/steps on Workbox Test; optionally one video under two minutes connects the turn to the picker. Register each screenshot/video immediately with `evidence add <file> --task TSK-1416 --desc "<what it proves>"` (`--key` for the picker). Re-run the substituted `ssh ... test -f ...` command (exit 0), check `of: build` on both reviews, and record the app/backend commits and signed adapter path/version. Stop task-owned processes and verify their PIDs exited.

**Done means:** Direct proof links let the owner see their words become checked,
versionable files and a selectable workflow. Exact tool errors block completion;
Task 13 still supplies engine-loop proof. **Expected diff:** 0–30 repository lines
(proof links only; disposable files/evidence outside this repository).
**Serial overlap:** Task 12 picker/source cutover, Task 13 prepared proof app.
