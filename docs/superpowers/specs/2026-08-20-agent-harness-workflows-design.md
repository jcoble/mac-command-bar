# Agent harness & workflows — design spec

Date: 2026-08-20. Status: approved direction from owner brainstorm (this session). Task: TSK-870.
Research basis: `harness-orchestration-research.md` (`2026-08-20-harness-orchestration-research.md`, same directory).
Prior direction honored: round-2 feedback §9 (`docs/evidence/tsk-808/ui-feedback/round2/FEEDBACK.md`) — app is the orchestrator's host, roles are ACP sessions over subscription CLIs, app-mediated messaging, templates over hand-written JSON.

## Goal

Multi-agent workflows inside Assembly: agents that talk to each other, an orchestrator that
checks in on workers and messages them mid-run, workflows with fan-out/loops/gates — all
visible and steerable by the human. Not fire-and-forget.

## Owner decisions (binding)

1. **Orchestrator = both**: a workflow may run with an orchestrator agent (a normal session
   given the controller role) or purely app-driven from the UI. Same machinery either way.
2. **Workflow definitions are declarative (v1)**: JSON files. A code-script format is a later
   escape hatch, not v1. Dynamic behavior lives in an `agent-decision` step, not the format.
3. **Workers are ordinary ACP sessions** — visible lanes, any provider (Claude/Codex/agy),
   spawned through the same subscription-authenticated CLIs as today. **MUST NOT incur API
   costs** (no API keys; ride ChatGPT/Claude subscription auth).
4. **UI home is the sessions rail**: a running workflow is a collapsible group of session rows
   (orchestrator + workers); hovering a row floats a steps card; a full board opens as a
   center-lane tab. Mockup: `2026-08-20-workflow-board-mockup.html` (same directory).
5. Provider-native sub-agents (Claude's Task tool) remain display-only (W18); they are not the
   harness mechanism. Headless CLI lanes remain possible but are outside the harness.

## Architecture

Three layers beside ACP — ACP itself stays untouched (it has no agent-to-agent anything;
verified against the v2 protocol):

### 1. Broker (Rust, owns all coordination)

- **Message envelope**: `{ id, runId, from, to, kind, body, createdAt }` where `from`/`to` are
  stable agent ids (`runId:roleName`, plus `human` and `orchestrator` aliases). Kinds:
  `message`, `status`, `decision-request`, `decision-response`, `gate`, `system`.
- **Persistence**: two tables in the existing SQLite session DB — `workflow_runs`
  (id, definition snapshot, state, timestamps) and `workflow_events` (append-only envelope
  log). The event log IS the timeline UI and the crash-recovery record. Nothing coordination-
  critical lives only in memory.
- **Delivery**: per-agent FIFO queue drained at safe prompt boundaries — when the recipient
  session is idle, deliver as the next prompt (prefixed `[workflow message from <name>]`);
  when mid-turn, hold until the turn completes. Receipts recorded per message:
  `queued → delivered | failed | expired`. No claim of mid-token injection (matches Pi and
  Claude Code teams behavior). Urgent stop = ACP `session/cancel`, then deliver.
- **Status**: normalize each worker's ACP `session/update` stream + turn completion into
  `running | waiting | blocked | idle | done | failed`; every transition appended to the log.
  `blocked` = the worker sent a `decision-request` (Pi's `need_decision` shape) and is waiting.

### 2. Worker bridge (how agents send)

- Assembly registers a small local MCP server (`assembly-broker`) into each spawned worker's
  provider config, exposing three tools: `send_message(to, body)`,
  `request_decision(question, options?)` (blocks the step, not the process), and
  `report_status(note)`. The orchestrator session gets two more: `list_workers()` and
  `read_events(since?)`.
- Providers without workable MCP injection fall back to a file inbox/outbox in the run
  directory (Pi's mechanism), watched by the broker. Same envelope either way.
- Prompt preamble injected into every workflow session states its role, its name, who it can
  message, and that gate approvals come only from the human.

### 3. Workflow engine (runs the definition)

- **Definition**: JSON in `.assembly/workflows/*.json` in the project (versionable; an
  orchestrator agent can author one — that is "agents create workflows"). Snapshot copied into
  the run row at start.
- **Step types**: `agent-task` (role, provider, model, prompt template, worktree policy),
  `agent-decision` (orchestrator examines state, picks/authors what runs next — the dynamic
  escape hatch), `human-gate` (Approve/Reject card; only the human can pass it),
  `app-action` (create worktree, run command, merge, cleanup — no LLM).
- **Control**: `retry: N`, `until: <condition> max: N` (review→fix loops),
  `for-each: <list from a prior step's structured output> parallel: N`, `join`.
- **Isolation**: worktree-per-worker by default under
  `/Users/blackcolours/dev/work/worktrees/<repo>/<run>-<role>`, app-created and app-removed at
  run end (the standing worktree rules become engine code, not discipline).
- Engine is deterministic Rust: it spawns sessions, waits on status transitions, evaluates
  conditions, fires gates. The orchestrator agent is a participant, not the scheduler.

## UI

1. **Rail group**: bordered collapsible group in the sessions rail — header = workflow name +
   step progress + run state; rows = orchestrator and workers as normal session rows with live
   status dots. Click row → that session in the center, as today.
2. **Hover steps card**: floating card on row hover — step list with per-step state/owner,
   `Open board`, and `Reply` when that worker is blocked on a decision.
3. **Board (center-lane tab, per run)**: left = definition outline with live state and loop
   annotations; center = worker lane cards (provider/model/worktree/current activity/tokens/
   status + View/Message/Stop) and gate cards with Approve/Reject; right = the event log as a
   filterable timeline with receipts and a compose box targeting any agent.
4. **History**: finished runs keep their sessions in History as normal; the run's timeline
   stays readable from the board in a read-only state.
5. Styling: shell tokens, px only, no `:has()`, kit controls (shadcn) for pickers/inputs.

## Delivery slices

1. **Slice 1 — broker + messaging + visibility (no engine)**: envelope log, MCP bridge,
   status normalization, rail group + steps card + timeline for a *manually assembled* group
   (pick N existing sessions, name an orchestrator). Proves messaging + UI end to end.
2. **Slice 2 — engine**: definition format, step types, loops, gates, worktree lifecycle,
   board runs a real workflow (first dogfood: our SDD fix-round shape).
3. **Slice 3 — authoring**: template gallery; orchestrator-authored definitions with a
   human-gate before first run.

## Non-goals (v1)

No API-key execution paths; no code-script workflow format; no cross-machine workers; no
automatic retry policies beyond declared `retry/until` caps; no mid-token message injection;
no replacement of W18's display of provider-native sub-agents.

## Verification

Slice 1: unit tests on envelope persistence/receipt transitions and status normalization
(mock ACP updates); an integration test driving two real sessions exchanging messages via the
MCP bridge; UI screenshot pass at 1710x990 headless. Slice 2: engine tests execute a fixture
workflow (fan-out 2, one until-loop, one gate) against stub sessions; a live dogfood run of
the SDD fix-round workflow gated by the owner.
