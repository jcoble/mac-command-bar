# TSK-808 Work Package A5 — Native CLI Handoff Receipt

Status: implementation complete; native proof deferred

This receipt records the A5 implementation, verification output, and deferred native-app checks. Claims are labelled verified or assumed.

## Work log

- Receipt created before discovery or implementation (verified).
- Test-first Rust pass (verified): the nine named handoff tests were first added as deliberate placeholders; the focused cargo run failed at those placeholders, as required.
- Contract implementation pass (verified): the focused Rust command now passes 50 agent-conversation tests serially, including all nine handoff tests. The output contained warnings only; no test failures.
- Native rebuild/window proof remains deferred (assumed from the dispatch gate): this worktree cannot claim the A2+A3+A4 rebuilt-app handoff proof.

## Implemented contract

- Rust now owns one explicit prepare/commit/rollback command with stable owned id, generation, native-session id, bounded history, writer lease, and process-tree assertions.
- Structured-to-terminal detaches the local transport without sending the destructive close request; the stored native-session id remains available for a later resume.
- Terminal-to-structured validates release and history before resuming, and rejects a provider response that returns a different native-session id.
- Fork is accepted only when the current capability snapshot advertises it and the target owned id is distinct; the structured source remains the owner.
- The existing manager, writer lease, journal, terminal registry, store, and workspace state remain the authorities. No second runtime, journal, store, or PTY map was added.
- The surface now exposes distinct Open in native CLI, Fork to native CLI, and Return to structured actions. Projection starts only from the explicit committed handoff path.

## Verification tails

All commands were run one at a time from `tauri-svelte-preview`. Exit status 0 is verified unless marked otherwise.

| Command | Tail / result |
| --- | --- |
| `pnpm test:agent-conversation-protocol` | `agent conversation protocol tests passed` |
| `pnpm test:agent-conversation-store` | `agent conversation store tests passed` |
| `pnpm test:conversation-session-isolation` | `conversation session isolation tests passed` |
| `node --experimental-strip-types scripts/conversationWorkspaceRestore.test.mjs` | `conversationWorkspaceRestore.test.mjs passed` |
| `node --experimental-strip-types scripts/agentRuntimeContracts.test.mjs` | `agent runtime contract tests passed` |
| `pnpm test:paste-cleanup` | exit 0; no failure output |
| `node --experimental-strip-types scripts/agentConversationTerminalProjection.test.mjs` | `agentConversationTerminalProjection contract: PASS` |
| `node --experimental-strip-types scripts/conversationControls.test.mjs` | `conversationControls.test.mjs passed` |
| `node --experimental-strip-types scripts/conversationCommandCatalog.test.mjs` | `conversationCommandCatalog.test.mjs passed` |
| `node --experimental-strip-types scripts/conversationMessageSafety.test.mjs` | `conversationMessageSafety.test.mjs passed` |
| `node --experimental-strip-types scripts/conversationTimeline.test.mjs` | `conversationTimeline.test.mjs passed` |
| `node --experimental-strip-types scripts/agentConversationHandoff.test.mjs` | `agentConversationHandoff.test.mjs passed` (typed request/generation, receipt/workspace restore, failure restoration, labels, and no passive projection trigger) |
| `pnpm test:live-terminals` | `liveConversationTerminals: all tests passed` |
| `pnpm test:next-terminal-service` | `terminalService tests passed` |
| `RUST_TEST_THREADS=1 MSBUILDDISABLENODEREUSE=1 cargo test --manifest-path src-tauri/Cargo.toml agent_conversation -- --nocapture` | `running 50 tests`; `test result: ok. 50 passed; 0 failed; ... 179 filtered out` |
| `pnpm check:svelte` | `/next`: `0 error(s), 0 warning(s)`; old shell: `16 error(s)` pre-existing outside this gate |
| `pnpm check` | exit 0; TypeScript emitted no errors |
| `git diff --check` | exit 0; no whitespace errors |

The Rust run emitted existing dead-code warnings only. The known socket-bind environment failure was not encountered in this focused command.

## Changed file and line receipt

Line anchors are from the final working tree. These are the only changed tracked files; the raw discovery log `docs/superpowers/evidence/tsk-808/a10-dispatch-spec-raw.log` was pre-existing and untouched.

- `tauri-svelte-preview/src-tauri/src/agent_conversation/handoff.rs:1-755` — new DTOs, state machine, Tauri command, and nine required Rust tests.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/manager.rs:7-10,52-56,180-224,490-757,900` — handoff context, non-destructive detach, identity/history guards, lease transitions, commit/rollback, and fork capability default.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/protocol.rs:141-147` — optional advertised fork capability.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/capabilities.rs:141` and `providers/adapter_support.rs:1350,1426-1433` — capability fixture/parsing updates.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/mod.rs:131-133`, `providers/acp.rs:121-128,188-193`, `providers/acp_client.rs:76-80,224-228` — detach seam and capability parsing.
- `tauri-svelte-preview/src-tauri/src/agent_conversation/mod.rs:3` and `tauri-svelte-preview/src-tauri/src/main.rs:5404` — module and command registration.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationTypes.ts:55,75-199` — public handoff types, request/receipt helpers, labels, and failure restoration.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationService.ts:16-42,278-336` — typed prepare/commit/rollback invokes, receipt application, and failure cleanup.
- `tauri-svelte-preview/src/lib/shell/conversation/conversationStore.svelte.ts:543-547` — transition cleanup helper.
- `tauri-svelte-preview/src/lib/shell/components/ConversationSurface.svelte:27-59,105-114,227-281` — explicit action buttons and removal of passive projection.
- `tauri-svelte-preview/src/routes/next/+page.svelte:85-104,991-1093,1580-1582` — explicit controller callbacks, typed assertions, and committed projection start.
- `tauri-svelte-preview/scripts/agentConversationHandoff.test.mjs:1-96` — named frontend handoff contract tests.
- `tauri-svelte-preview/package.json:18` — handoff test script.
- `docs/superpowers/evidence/tsk-808/a5-handoff-receipt.md:1-80` — this append-only receipt.

## Done-means checklist

- [verified] Nine required Rust test names exist and pass.
- [verified] Stale generation, wrong owner, prompt blocking during transition, live-TUI/release assertions, same-session identity, fork capability/target, rollback, history boundary, duplicate PTY, and restart non-start behavior are covered by focused tests.
- [verified] Frontend receipt round-trip, typed generation checks, explicit labels, failure restoration, and no passive projection trigger pass.
- [verified] Required command list passes serially; the `/next` Svelte gate has zero errors, and the 16 old-shell errors remain outside that gate.
- [verified] No forbidden browser, layout, shell-frame, shell-sidebar, overlay, or browser-component file was changed.
- [verified] No commit was made.
- [assumed] The controller's process-tree assertion is structurally correct from the owned PTY and lease state; it still needs native-window evidence.
- [deferred] Rebuilt native app proof for both handoff directions, same-session resume, fork target creation, actual native TUI release, one-process-tree proof, and end-to-end scrollback/projection behavior.

Final fresh verification pass (verified after the last source edit): every command in the dispatch list exited 0, including the serial Rust run (`50 passed; 0 failed`), `/next` Svelte check (`0 errors; 0 warnings` with 16 old-shell errors outside the gate), and TypeScript check.
