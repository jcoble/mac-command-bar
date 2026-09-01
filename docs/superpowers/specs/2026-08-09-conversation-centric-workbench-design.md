# Conversation-centric workbench — design spec

Date: 2026-08-09
Status: approved design, pending implementation plan
Sources: docs/superpowers/evidence/tsk-808/ui-feedback/round2/ (FEEDBACK.md + screenshots
24–42), docs/superpowers/evidence/tsk-808/round2-discovery/ (codebase discovery + web recon
on the Codex desktop app, T3 Code, and Orca), and the 2026-08-04 ACP amendment
(docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md).

## Problem

The TSK-808 wave shipped surfaces, but the product shape is wrong and the agreed ACP
foundation was never finished:

- The Session view renders only the terminal. The structured view exists but is
  disconnected: live ACP `session/update` frames are discarded
  (src-tauri/src/agent_conversation/providers/acp_client.rs:361-365), the only production
  event feed is transcript-file tailing (terminal_projection.rs), and selecting any live
  session force-closes the structured runtime and switches to raw
  (src/routes/next/+page.svelte:989-992, regression introduced by ba46633).
- The conversation is one tab among six instead of the center of the product.
- Session History data is wrong by construction: bounded transcript scans produce floor
  counts (core/src/scanners/sessions.rs:62-83), group badges are computed pre-paging while
  rows render post-paging (SessionLibraryWorkspace.svelte:110-121), groups key on full path
  but display only the basename (sessionLibraryModel.ts:352-376), and first prompts are
  extracted in Rust but never serialized to the UI (sessions.rs:397-403).
- The browser cannot navigate (`navigate_browser_tab` invoked without its required `input`
  key), usage history fails to read in the native build, and Resources/Usage live in the
  wrong place with no view of the app's own footprint.
- The Workflows tab is a dead form (hand-written JSON input, no runtime behind it).

## Product direction

The conversation owns the workbench (Codex-app model, not Orca's worktree-centric model).
Everything else is a panel that serves the conversation. All agent I/O rides ACP typed
events; the UI never infers state from terminal text.

## 1. Foundation — ACP-only agent sessions

The decision: agent sessions (Codex and Claude) run exclusively through ACP. No dual-mode
toggle, no PTY per agent session.

- Session lifecycle: create/resume via ACP (`initialize`, `session/new`) in
  agent_conversation/manager.rs; prompt via `session/prompt`; live `session/update`
  notifications are parsed and forwarded to the frontend as normalized typed events through
  the existing `manager.emit_payload` path (today only tests call it — manager.rs:265-308).
  The frame drop point at acp_client.rs:361-365 becomes the notification pump.
- Remove the PTY path for agent sessions: `startNewSession` stops calling
  `service.startOwned` (+page.svelte:1128-1149); the unconditional raw-mode transition at
  +page.svelte:989-992 is deleted along with the `closeStructuredConversation()` teardown on
  selection.
- The terminal becomes a read-only inspector: an optional panel showing the agent process's
  raw output for debugging, never the interaction surface and never the source of truth.
- Terminal-transcript projection (terminal_projection.rs) is retained only to render
  sessions that were started outside the app (external CLI runs found on disk).
- A plain terminal session type remains for shells and interactive TUIs. It has no
  structured view and is visually distinct.
- Event contract: one frontend event stream (`agent-conversation-event`) carrying typed
  payloads — turn started/delta/completed, tool call, file change, plan update, subagent
  status, approval request, error. This is the same shape the Codex desktop app uses (its
  app-server protocol streams thread items and turn deltas; the UI is a pure renderer).
- Multi-session: the manager supports many concurrent app-owned ACP sessions with routable
  outputs. This is deliberate groundwork for workflows (section 8).

Non-goals: no protocol other than ACP for agent I/O; no scraping; no per-provider bespoke
event formats reaching the frontend.

## 2. Shell layout

- Center: the conversation, always mounted. Composer pinned at the bottom.
- Left sidebar: projects → conversations (Codex-app style). Each row shows live state:
  spinner + "Working Xm" while a turn runs, a Monitoring/idle state, an unread-done
  indicator when a turn finishes while unfocused, T3-Code style (its Sidebar logic is open
  source and cited in the recon report). Completion also fires an OS notification.
- Right edge: a vertical tab strip — Editor, Diff, Browser, Session History, Agents.
  Clicking docks that panel beside the conversation; a drag handle resizes it; an expand
  button grows it to near-fullscreen (conversation collapses to a sliver but survives —
  its state is never torn down by panel changes). One panel open at a time in v1.
- Bottom status bar: Resources and Usage move here as upward-opening popovers (section 6),
  plus the browser trigger. The top bar keeps only window/app chrome and session title.
- The old center tab strip (25.png) is removed.

## 3. Composer and turn rendering

Composer (Codex-app parity, screenshots 37/40):

- Model picker, effort picker, and permission/approval mode in the input bar, per session,
  defaulting from the last session.
- Attachments: paste an image into the input, drag files from Finder onto the composer;
  both render as chips and are sent with the turn (ACP content blocks).
- Annotation chips ("N annotations") appear when browser annotations are pending (section 4).
- Mic/dictation and slash commands are out of scope for this wave.

Turn rendering, fed only by typed ACP events:

- User and agent messages styled as Codex-like cards.
- Tool calls / commands / file edits render as collapsed rows with "+N previous tool calls"
  grouping and a live "Working for Xm Ys" footer during a turn (T3 Code timeline pattern).
- File-change cards show per-file +adds/−dels with a diff link into the Diff panel.

## 4. Browser

- Opens from the bottom-bar trigger as the same docked/expandable panel as every other
  surface; expand fills almost the whole app (38.png).
- Annotation mode (39/40.png): numbered pins dropped on page elements, inline comment box
  per pin, annotation count chip on the composer, Send attaches all annotations to the next
  turn. Each annotation carries: page URL, a screenshot crop around the pin, the element's
  CSS selector and trimmed DOM snippet, and the comment text (Orca Design Mode captures
  DOM + styles + screenshot + source location; we match the DOM+screenshot core).
- The composer is part of the expanded browser overlay, so annotating and prompting happen
  in one place.
- Bug fix (first commit of this workstream): the frontend invokes `navigate_browser_tab`
  without the required `input` key (33.png) — align the invoke payload with the command
  signature and add a source-level test pinning the argument shape.

## 5. Session History

Replace the bounded filesystem scan with a durable SQLite session index.

- Ingestion: scan provider transcript stores (Claude ~/.claude/projects JSONL, Codex
  sessions) with a per-file cursor so increments are cheap and complete; no bounded-window
  counts. Store per session: provider, native session id, title, FIRST PROMPT (full text,
  truncated at render not at ingest), latest N turns, subagents with message counts,
  true message count, model, timestamps, and project root and worktree path as SEPARATE
  fields (git-derived where possible).
- Reads are one SQL statement each (project rule: aggregation is DB-side, never
  materialize-then-loop): two-level grouping project → worktree with COUNT(*) per group,
  search (LIKE/FTS over title + first prompt + latest turns), filtering, and paging that
  pages within the grouped result, so a group badge always equals the reachable rows.
- UI: default view = all sessions grouped project → worktree, both levels collapsible
  (persisted collapse state). The Workspace/Project/All triple filter is removed. Session
  cards get the rich anatomy (26.png): long first prompt, latest turns, subagents,
  worktree chip, Resume in Worktree / Continue in New Session / View Log.
- Owned-session records stop carrying their own divergent copies of counts — the index is
  the single authority (kills the 6-vs-2760 split; ownedSessions.ts:137-165 defers to it).

## 6. Resources and Usage

- Both move to the bottom status bar as upward-opening popovers.
- "Open full resource view" opens a true full-page overlay (Space-style, 35.png), not an
  inline card.
- Add the app's own footprint: the app process plus every process it owns, as a pinned
  top-level group with a running CPU/RSS total.
- Root-cause and fix the native "Usage history could not be read" failure (36.png) —
  suspected packaged-build DB path/migration difference; diagnose, do not guess.

## 7. Build order

1. Foundation: ACP event pump, ACP-only session creation, delete force-raw handoff,
   terminal demoted to inspector. Includes the two blocking bug fixes (browser navigate,
   usage history read) since they are small and unblock daily use.
2. Shell layout: center conversation, left sidebar with live states + notifications, right
   panel strip with dock/expand, bottom bar placement.
3. Session History index + regrouped UI.
4. Browser overlay annotations end-to-end.
5. Composer parity + turn-rendering polish.
6. Resources full-page overlay + app-self totals.

Each phase is its own branch/PR with tests; phases 3+ can start once phase 1's event
contract is stable.

## 8. Agent workflows (designed now, built later)

- The app is the orchestrator. A workflow template defines roles (e.g. Plan → Implement →
  Review), each role = an ACP session with its own provider/model/effort/permissions.
- Hand-offs are app-mediated: one role's output becomes the next role's prompt;
  agent-to-agent messages are turns the app routes between its sessions; worktree files are
  the shared receipts. The Agent Activity rail (progress / active agents / waiting
  approvals / failures — 42.png) becomes the live view of a run.
- No hand-written JSON: users pick a template and type a plain-text goal.
- Hard rule: zero API-key spend. All sessions spawn through subscription-authenticated
  provider CLIs (the same auth `codex exec` and the `claude` CLI use). No direct API
  integration exists anywhere in the workflow runtime.
- Transport decision: ACP, not an external multi-agent backend (CMUX oh-my-codex /
  oh-my-claude prove the pattern but add a runtime to babysit; section 1's multi-session
  manager already provides spawn + route + render).
- This wave only guarantees the groundwork: multi-session support and routable outputs in
  the manager. The workflow UI/runtime is a follow-up spec.

## Error handling

- ACP session start fails → the session card shows the error with a retry; it never
  silently falls back to a PTY.
- ACP stream drops mid-turn → reconnect/resume via ACP session load; if the provider CLI
  died, surface it in the sidebar state and OS notification.
- Ingestion errors in the session index are per-file: one corrupt transcript never blocks
  the index; failures are counted and visible.
- Browser annotation capture failing (e.g. cross-origin frame) degrades to
  screenshot+coordinates, never blocks sending.

## Testing

- Rust: unit tests for the ACP notification pump (session/update frame → normalized event),
  session-index ingestion (fixture transcripts → exact counts/first prompts), and the
  grouped/paged SQL reads (two providers, repo + worktree fixture → correct two-level
  groups and badges). Pinned capabilities test extended for new commands.
- Frontend: script tests for composer attachment/annotation chip state, sidebar
  working/done state derivation, and grouped history view-model; svelte-check and build
  gates stay at 0 errors for the new shell.
- Playwright web preview: session-centric layout smoke (panel dock/expand/close cycles keep
  the conversation alive), history grouping/collapse, browser annotation pin flow.
- Native acceptance checklist per phase, mirroring MORNING-SUMMARY.md's format.

## Explicitly out of scope for this wave

- Workflow runtime/UI (section 8 groundwork only).
- Mic/dictation, slash commands in the composer.
- Retiring the old shell at "/" (tracked separately as TSK-811) — though phase 2 should not
  add new dependencies on old-shell code.
- Houston terminal palette and inactive-worktree registry plumbing (carried follow-ups).
