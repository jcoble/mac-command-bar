# Chunk 1: Quick Fixes + Polish + Rail Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. In this session, tasks are dispatched as codex-exec SOL lanes with these task sections as the briefs; the controller reviews and commits.

**Goal:** Land the roadmap's chunk 1 — composer focus-ring fix, claude model-list investigation, transcript/color/animation polish, and the session-rail cleanup (scroll bug, restyle, quick-jump buttons).

**Architecture:** All frontend except the claude model probe (bridge/Rust read-only investigation plus a possible allow-list change in `acp_client.rs`). Visual work builds on the existing kit (`src/lib/components/ui/*`, DESIGN.md) and the `/next` token palette; behavior changes carry script tests.

**Tech Stack:** Svelte 5 runes, Tailwind-in-Svelte + scoped CSS with `--color-*` tokens, bits-ui kit, node script tests, Rust (probe only).

## Global Constraints

- Use the existing UI kit; never a native `<select>`; 13px body floor (DESIGN.md).
- `pnpm run check:svelte` must end "Files the /next shell owns: 0 error(s), 0 warning(s)".
- No commits/staging by lanes; controller commits. Plain-English copy; no AI-vendor mentions in code or reports.
- One cargo invocation at a time; rustfmt touched Rust files; `RUST_TEST_THREADS=1`.
- Respect `prefers-reduced-motion` in every animation added.
- Lanes must not touch files another concurrent lane owns; no implementation lanes while a UI-verification round runs.

---

### Task 1: Remove the green composer focus ring

**Files:**
- Modify: the global rule that draws it — search `rg -n "focus-visible|--color-focus" tauri-svelte-preview/src/routes/next.css tauri-svelte-preview/src/lib/components/ui -S` (the composer's own styles already set `outline:0!important` on the textarea, `ConversationComposer.svelte:75`, so the ring comes from a global/kit `focus-visible` outline using `--color-focus-solid`).
- Test: `tauri-svelte-preview/scripts/nextTokens.test.mjs` (extend only if it pins focus rules).

**Interfaces:** none consumed/produced.

- [ ] **Step 1:** Locate the rule that outlines the composer textarea on focus (evidence: owner screenshot — teal/green rectangle hugging the textarea inside `.composer-box`).
- [ ] **Step 2:** Scope that rule so the composer textarea gets no outline; focus feedback remains the existing `.composer-box:focus-within` border+shadow (`ConversationComposer.svelte:75`). Do not remove focus-visible outlines from buttons/menus elsewhere — accessibility keeps them.
- [ ] **Step 3:** Verify: `pnpm run check:svelte` → 0 errors /next; visually confirm via the running app if available, else state the exact selector change and why it only affects the composer.
- [ ] **Step 4:** Controller commits (`fix: composer textarea no longer draws a focus outline`).

### Task 2: Claude model list — can we offer Opus/Fable?

**Files:**
- Investigate: `~/.mac-command-bar/claude-acp-wrapper.sh` probe (same JSON-RPC method as the earlier config probe), adapter source `~/.nvm/versions/node/v24.12.0/lib/node_modules/@zed-industries/claude-code-acp/dist/acp-agent.js`.
- Possibly modify: `tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp_client.rs` (standard-fields mapping, ~:893) if extra ids prove accepted.
- Test: extend the fake-adapter test in `acp_client.rs` if the mapping changes.

**Interfaces:** consumes the standard-config mapping added by the claudeconfig lane (`acp_client.rs:717,893`).

- [ ] **Step 1:** Probe the live adapter: initialize → session/new → `session/set_model` with ids `opus`, `claude-opus-5`, `fable`, `claude-fable-5` (one at a time). Record exact accept/reject responses. Terminate only the child you spawned.
- [ ] **Step 2:** Check the adapter source for how it validates modelId (search `setSessionModel|availableModels` in acp-agent.js) — does it pass ids through to the CLI (`--model`) or enforce its own 3-entry list?
- [ ] **Step 3:** If arbitrary ids pass through: extend the mapped `availableModels` for the claude provider with the verified-working ids only, labels via `agentConfigLabels.modelLabel`. If the adapter rejects them: no code change; report the exact rejection so the finding is durable.
- [ ] **Step 4:** `RUSTFLAGS="-D warnings" cargo check`; `RUST_TEST_THREADS=1 cargo test --manifest-path tauri-svelte-preview/src-tauri/Cargo.toml agent_conversation` if code changed.
- [ ] **Step 5:** Controller commits if changed (`feat: offer verified extra claude models in the model menu`).

### Task 3: Transcript rendering + color pass + transitions

**Files:**
- Modify: `tauri-svelte-preview/src/lib/shell/components/conversation/ConversationMessage.svelte` (styles block :62), `ToolItem.svelte`, `ReasoningItem.svelte`, `TimelineItem.svelte`, `ConversationTimeline.svelte`, and `src/routes/next.css` / `nextTokens.css` for tokens (respect the six-name /next-token allowance pinned by `scripts/nextTokens.test.mjs:162-178` — reuse existing tokens rather than adding).
- Test: existing pinned suites for touched files (`scripts/agentTimeline*.test.mjs` etc. — run whatever pins these files); `scripts/nextTokens.test.mjs`.

**Interfaces:** none new; visual only plus one behavior: tool-output collapsing.

- [ ] **Step 1:** Tool output collapsing: any tool-call output over 12 lines renders collapsed to the first 6 lines with an expand control showing the hidden-line count ("Show 34 more lines"); expanded state per item, not global. Add/extend a script test asserting the collapse threshold logic (pure function, e.g. `collapseToolOutput(lines: string[]) → {visible, hiddenCount}`).
- [ ] **Step 2:** Transcript spacing/typography: consistent vertical rhythm between turns (16px between items, 24px between turns), code blocks keep the existing `.code-wrap` chrome but gain consistent margins, user bubble contrast reviewed. Markdown: ensure headings/lists/blockquote/table styles in `ConversationMessage.svelte:62` read cleanly at 14px body.
- [ ] **Step 3:** Color/contrast pass: replace flat near-black panels with the layered surface treatment already used by the composer (`color-mix` surface blends), one accent family applied consistently (working=accent, attention=attention token, success=success token, danger=bad token); no new hex values — tokens only. Sweep the main surfaces: transcript, rail, right-pane tabs, bottom bar.
- [ ] **Step 4:** Transitions: 120–160ms ease transitions for surface/tab switches (opacity/translate), popover fade+scale already in kit, session-open fade-in; all wrapped in `@media (prefers-reduced-motion: no-preference)`.
- [ ] **Step 5:** `pnpm run check:svelte` → 0 errors /next; run pinned suites for touched files; screenshot before/after if the app is running.
- [ ] **Step 6:** Controller commits (`feat: transcript polish, token color pass, and motion`).

### Task 4: Session rail cleanup

**Files:**
- Modify: `tauri-svelte-preview/src/lib/shell/components/SessionsColumn.svelte` (scroll containers :190-193,:315), `MyWorkSessionList.svelte:49`, `SessionsPaneview.svelte`, `WorktreeAgentRow.svelte` (row layout + presence at :93).
- Create: row quick-jump control (inline in `WorktreeAgentRow.svelte` or a small `SessionRowActions.svelte` beside it).
- Test: `tauri-svelte-preview/scripts/centerDock.test.mjs` pins surface ids; add `scripts/sessionRowActions.test.mjs` for the jump mapping.

**Interfaces:**
- Consumes: surface/tab switching — the same mechanism `CenterActivityDock` uses to select right-pane surfaces (find its store/callback; surfaces include session, editor, source-control), and session activation used on row click today.
- Produces: `sessionRowJump(ownedId, surface: 'session' | 'editor' | 'source-control')` — activates the session AND selects the surface in one action (pure mapping + thin dispatch, unit-testable).

- [ ] **Step 1:** Scroll bug: make the My Work list scroll when taller than the window — audit the flex chain (`SessionsColumn.svelte:190,193,315`, `MyWorkSessionList.svelte:49`, the Paneview) for a missing `min-h-0`/`overflow-y-auto`; fix so every grouping mode (None/Status/Project) scrolls. State the exact broken chain in the report.
- [ ] **Step 2:** Write `sessionRowJump` + failing test: given a row and surface id, it activates that session then selects that surface (assert call order and ids with fakes).
- [ ] **Step 3:** Row actions UI: on row hover (and always while working), show compact icon-buttons — the presence indicator itself is the first button (working spinner/attention dot → Session tab), plus Editor and Git icon-buttons (kit IconButton, 24px, tooltips "Open session", "Open editor", "Open source control"). Wire to `sessionRowJump`.
- [ ] **Step 4:** Row restyle: tighter two-line layout (title + meta), presence left-aligned with the title, consistent 8px paddings, hover surface tint, selected state clearly distinct — consistent with Task 3's token pass.
- [ ] **Step 5:** `pnpm run check:svelte` → 0 errors /next; `node scripts/sessionRowActions.test.mjs`; `node scripts/centerDock.test.mjs`; `node scripts/myWorkViewOptions.test.mjs`.
- [ ] **Step 6:** Controller commits (`feat: rail scrolling, row quick-jump actions, and row restyle`).

---

## Execution notes (this session's model)

- Task order: 1 and 2 first (parallel: 1 is CSS-only, 2 is probe/Rust — disjoint). Then 3 and 4 — SEQUENTIAL or with strict file boundaries: Task 3 owns conversation components + tokens; Task 4 owns rail components; both may not touch the other's files. Task 4's restyle uses the tokens as they exist when it runs.
- After all four: one UI-verification round (quiet window), then move to chunk 2 (editor modes).
