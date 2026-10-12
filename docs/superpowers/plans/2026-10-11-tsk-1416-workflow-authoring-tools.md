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
**1 → 2 → 3 → 4 → C → 5 → 6 → 7 → A → 8 → 9 → 10 → 11 → 12 → B → 13**.
Task C (D15, added 2026-10-12) goes right after slice 1 Task 4, because it needs Task 4's delivery at turn boundaries.
The controller may coordinate independent branches as stated in the lane rules;
A must not edit a concurrent loader/endpoint branch before its dependency lands.

Task 7 anchors below are from `7817ca1bb`; re-find symbols after integration.
`messaging.rs` is absent from the docs branch's `origin/main`; its anchors are
symbols from Task 2's lane, not guessed main line numbers. Task 3 may move them.
No changes to the original slices 1–2 plan are required by this addendum.

### Task A: Expose the format and share strict file validation with run creation

**Files:** Modify `tauri-svelte-preview/src-tauri/src/agent_conversation/messaging.rs`
(`MessagingEndpoint::handle`, `caller`, `post` tools/list branch); modify
`tauri-svelte-preview/src-tauri/src/workflow.rs` (`contained_file`:77,
`WorkflowEngine::create_run_from_file`:1298, `WorkflowPolicy::validate_definition`:948,
`tests::yaml_fixture`:3065). Read `docs/workflows/format.md`; compile it into the
endpoint. Inspect `tauri-svelte-preview/tools/assembly-mcp.ts` tools/list relay and
self-test; add the two declarations only if the integrated client owns that list.
Update `main-architecture-explained.html`. Keep loader/endpoint tests in these Rust files.
Use Task 3's caller-project lookup; do not add a second session lookup mechanism.

**Interfaces:** MCP `workflow_format()` takes no arguments and returns one text
content item with the exact markdown from
`include_str!("../../../../docs/workflows/format.md")` relative to `messaging.rs`.
MCP `check_workflow(path: String)` takes only a project-relative path. Resolve the
project root from the credential-bound caller on that owning backend, never from
tool arguments, process cwd or another session. Missing project is an error.
Extract the existing parsing/snapshot construction into
`WorkflowEngine::validate_file(repo_root: &Path, relative_yaml: &Path) -> Result<WorkflowDefinitionV1, WorkflowError>`.
It performs the existing loader checks plus the controller's strict checks below
and `WorkflowPolicy::validate_definition`, returning
the definition with prompts read. `create_run_from_file` calls this entry, inserts
task/cwd into input, then calls `create_run`; validation stops before that call.
Return text `ok` followed by one deterministic line per step: ID, kind, needs,
and `of`/`on_fail` when present; on failure return the loader's exact error with
MCP `isError: true`. Do not aggregate or replace the loader's first error.

**Strict checks (controller decision, 2026-10-11):** Allow only top-level
`name`, `inputs`, `limits`, `steps` and step-level `needs`, `agent`, `prompt`,
`run`, `review`, `gate`, `on_fail`, `expect_files`, `if`, `repeat`. Reject an
unknown key with its name and the step ID for step-level errors. Require a string
`agent.provider` for agent steps and `review.agent.provider` for review steps;
require `review.of`. Each `of`/`on_fail` target must exist in the current step's
direct/transitive `needs` ancestry. Require `run` to be a non-empty list of strings.
Both check and start call this same validation; no MCP-only strengthening. No
nested unknown-key validation, type system, JSON Schema or installed model/provider
check. These additions deliberately supersede receipt item 4's old restriction.

- [ ] **Red tests:** `workflow_authoring_format_is_embedded_for_every_session` invokes through two ordinary credential-bound sessions with different providers/projects, checks tools/list advertises both tools and returned text equals the file. `workflow_authoring_check_is_read_only` checks a valid agent→checks file and exact step summary; asserts ledger row count unchanged and no runtime dispatch/worktree allocation. `workflow_authoring_check_uses_callers_project` places valid same-named YAML in two project roots and checks each caller gets its own summary; missing project, caller-supplied root/identity, absolute path, `../`, wrong directory/extension and escaping YAML/prompt symlink fail. `workflow_authoring_check_preserves_loader_errors` covers duplicate key, bad step ID, unknown/cyclic `needs`, unknown `of`/`on_fail` and missing/escaping prompt; compare against the shared entry's exact errors. `workflow_authoring_strict_validation_is_shared` invokes both check and start against unknown top-level/step-level keys (assert key and step ID), missing/non-string providers for both agent and review, missing review.of, nonexistent/self/downstream/unrelated of/on_fail, and empty/scalar/mixed-type run values. Assert rejection before any ledger/run/worktree side effect. Valid direct and transitive targets and a non-empty string command list pass both paths. Include a valid two-reviewer file with the same `of` and transitive needs, not a reviewers list.
- [ ] **Verify failure:** `cd tauri-svelte-preview/src-tauri && CARGO_BUILD_JOBS=3 MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test workflow_authoring` must run the new tests (not zero tests) and fail because tools/validate-only entry do not exist. Record exact assertion/error before edits.
- [ ] **Minimum edit:** Add two tool declarations/dispatch branches using existing caller identity/project access. Extract the current loader up to definition construction, add only the strict checks above and call its policy validator. Use the existing dependency graph to check ancestry; no separate validator or schema. Retain existing file containment and `.assembly/workflows/*.yml` checks, including rejecting `../` before canonicalization. Return success text or existing errors. No run ID, task snapshot requirement, command execution, ledger append, worker, timer or worktree. No new field validation framework or runtime behavior in this task.
- [ ] **Green:** Rerun the red command; every named test reports `ok`. Run `cd tauri-svelte-preview/src-tauri && CARGO_BUILD_JOBS=3 MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test yaml_definition_is_snapshot_and_rejects_escape` and `CARGO_BUILD_JOBS=3 MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test task_snapshot_contains_body`. Run slices 1–2 gates: `cd tauri-svelte-preview && npx tsc --noEmit && pnpm run check:svelte` (owned shell: `0 error(s)`), `pnpm --dir tauri-svelte-preview test:async-lifecycle`, and `cd core && MSBUILDDISABLENODEREUSE=1 ~/.cargo/bin/cargo test`. Run `cd tauri-svelte-preview && node --experimental-strip-types tools/assembly-mcp.ts --self-test` with both authoring tools advertised through the real stdio path. Record one combined code/spec review and resolve findings before handing off.

**Done means:** Any ordinary session can read the same compiled reference and
validate only its own project's workflow with the same parser used to start runs.
No persisted or runtime side effects occur. **Expected diff:** around 180–260
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

## Task C: `start_session` — visible helper sessions (D15, owner 2026-10-12)

**Why:** helpers started with background `codex exec` were invisible in Assembly, could be lost, and never woke the session that started them. A helper must be a real Assembly session the owner can see and talk to.

**Files:** `agent_conversation/messaging.rs` (tool list, permission check, handler), `agent_conversation/manager.rs` (create the session with the existing ensure path used by workflow dispatch, `AgentRuntimeManager::ensure_async`; on the helper's terminal turn event, queue its final reply to the parent), `core/src/session_store.rs` only for the parent link, the parent's sub-agent timeline item, and `main-architecture-explained.html`. Re-find anchors by symbol.

**Interface:** `start_session(provider: "codex"|"claude", model?: string, prompt: string, name: string) -> {name, owned_id}`. The caller comes only from its session credential. The helper starts in the caller's project folder on the caller's machine, gets the readable name `name` (made unique by the directory rule), and is sent `prompt` as its first turn. When that helper's turn completes, fails or is interrupted, the backend queues one message from the helper to the parent: `[<name> finished|failed|stopped] <final reply, bounded>`. Task 4 delivers it at the parent's next turn boundary, which wakes an idle parent.

**Display decision (recommended; confirm during recon):** keep the helper a top-level owned session, so the rail, the directory and `send_message` work unchanged. Do not reuse `sessions.parent_owned_id`: today it marks imported Claude sub-agent transcripts and the directory excludes those rows. Show the helper in the parent's existing sub-agent section with the same `subagent` timeline item shape the Claude adapter's sub-agents use (`SubagentSection.svelte`), carrying the helper's owned id so a click opens it. Record the parent link once (a column or the session's existing extra JSON, whichever the recon shows is already read by the rail) — no new table.

- [ ] **Red tests:** `start_session_creates_named_child_in_callers_project` (messaging.rs, fake runtime: one ensure call with the caller's cwd and project, name resolvable, subagent item appended to the parent); `child_final_reply_is_queued_to_parent` (manager.rs: a terminal turn event of a child queues exactly one message to the parent, none for a session without a parent); `start_session_rejects_unknown_provider_and_missing_credential`.
- [ ] **Verify failure:** `cd tauri-svelte-preview/src-tauri && CARGO_BUILD_JOBS=3 cargo test start_session_creates_named_child_in_callers_project` fails because the tool is not listed.
- [ ] **Minimum edit:** one tool handler calling the existing ensure path; one hook on the existing terminal-turn path; the subagent item through the existing journal write. No new process supervisor, no polling, no second queue.
- [ ] **Green:** the three tests, then the slice 1 gates. Live proof on **Workbox Test**: a Claude session calls `start_session` for a Codex helper; screenshot the helper in the rail and in the parent's sub-agent section, and the parent waking on the helper's finished message.

**Done means:** a helper is visible, openable and messageable, and its finish wakes the caller. **Expected diff:** 200–300 lines.

### Note: Claude Code plugin agents that call Codex (owner, 2026-10-12)

The owner uses an OpenAI/Codex plugin for Claude Code whose agents (a Codex reviewer, an executor and others) show as real sub-agents in the Claude CLI. Those are Claude sub-agents, so in Assembly they would appear in the existing sub-agent section **if** Claude sessions started through `claude-agent-acp` load the user's plugins. Not verified: the plugin is not installed on the workbox (`~/.claude/plugins/installed_plugins.json` lists only superpowers and warp, checked 2026-10-12). Check before building anything: the owner names the plugin; install it inside **Workbox Test** only; start a Claude session there and invoke one plugin agent; record whether it appears and runs. If it works, nothing more is needed for that path; if plugins do not load, that is a separate task, not part of Task C.

