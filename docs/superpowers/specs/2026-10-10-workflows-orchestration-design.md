# Workflows and orchestration — design spec

Date: 2026-10-10. Status: approved by the owner section by section in a brainstorm on 2026-10-10.
Task: TSK-1416 (Workflows and orchestration) — https://app.notion.com/p/3f5394b0689d81e5b2aef3bb8bddfea8

Builds on, and where it differs replaces:
- `2026-08-20-agent-harness-workflows-design.md` (approved direction, TSK-870)
- `2026-08-20-harness-orchestration-research.md`
- `2026-08-20-workflow-board-mockup.html` (visual target for the run view)

Research for this spec (prior art, 2026-10-10): Claude Code cross-session messaging and agent
teams, Codex app-server and `codex queue`, Google A2A, MCP Agent Mail, ACP v1, Conductor /
Vibe Kanban / Claude Squad, GitHub Actions, Temporal, Inngest. Report:
`~/Workbox/from-box/2026-10-10-agent-messaging-research.md` (owner's synced folder).
Code recon: `~/dev/work/reports/workflows/recon.md` on the workbox.

## 1. Goal

The owner wants to stop writing long prompts and stop being the relay between agents:

- set up workflows where some steps are **deterministic** (scripts the app runs, whose result
  is a fact, not an agent's claim);
- **see into runs**: where each run is, what each step did, what was said between sessions;
- one **main orchestrator** the owner chats with, which starts runs and supervises them;
- **any session can message any session**, addressed by a readable name;
- routine calls are made by **rules**; anything outside the rules **waits and pings the owner**,
  who can answer from the desktop app or a phone app.

Why deterministic steps matter: on 2026-10-10 another model was caught hand-editing generated
files and claiming it had not. A workflow must never advance on an agent saying "tests pass".
The engine runs the checks itself.

## 2. Owner decisions (binding)

| # | Decision |
|---|---|
| D1 | Routine decisions follow rules written in the workflow; anything outside the rules pauses the run and pings the owner. |
| D2 | Messaging covers sessions **inside Assembly only** (This Mac and connected remote machines). Outside processes do not join. |
| D3 | Messaging is **provider-neutral**: one Assembly MCP tool server attached to every session of every provider (Claude Code, Codex, and any later ACP provider). Copy Claude Code's cross-session messaging behaviour; do not adopt A2A or ANP. |
| D4 | First workflow: the **dev pipeline** — recon → spec → spec review → build → checks → code review → proof → PR → merge — with **loops**: a review with findings sends its step back to redo. |
| D5 | Loop rule: keep looping while each round has **fewer** findings than the last; pause and ping when a round has the same or more; hard ceiling **5 rounds**. |
| D6 | Merge rule: merge automatically when checks pass, the final review passes and proof is attached, then send the owner a summary. Each run has an **"I'll merge"** switch (default off) that stops at the open PR instead. The workflow's merge rule is the owner's standing go-ahead for that run's merge. |
| D7 | A run executes **on the machine where it is started** — the local app backend or a remote backend — exactly like sessions today. Its worker sessions live on the same machine. |
| D8 | Workflow **definitions are YAML files in the project's git repo**. Run history lives in the owning machine's database. |
| D9 | A run can be started from **one or many Notion tasks**, or by telling the orchestrator "start working on these tasks". One run per task, sharing one concurrency limit. |
| D10 | **One main orchestrator** session the owner chats with, from the desktop or the phone. Runs have no orchestrator session of their own. |
| D11 | Phone app: **Flutter, Android first**, starting as a shell of the Rental Command app, talking to Assembly through an API like Rental Command does, reusing its messaging system. Three parts: main chat with the orchestrator; runs (see, pause, resume); permissions and decisions. |
| D12 | Workers ride subscription-authenticated CLIs as today. **No API keys, no API costs** (carried from the 2026-08-20 design). |
| D13 | Only the owner approves. A message from an agent never counts as approval; chat text the orchestrator relays is turned into a confirm button the owner taps. |

## 3. Overall picture

| Piece | What it is | Where it runs |
|---|---|---|
| Workflow file | `.assembly/workflows/<name>.yml` plus prompt files beside it | Git |
| Run engine | Reads the file, starts steps, runs script steps, applies loop and merge rules, owns run state | Backend where the run was started |
| Main orchestrator | One ordinary agent session per machine, with extra tools to start, list, pause and resume runs | Same backend |
| Worker sessions | Ordinary Assembly sessions created by the engine for agent and review steps | Same backend as the run |
| Messaging | Assembly MCP server: directory, send, delivery receipts | Every backend |
| Inbox | Every item waiting for the owner: run pauses, gates, agent permission requests | Every backend's database |
| Run history | Every step attempt, message, check result, decision, proof link | Every backend's database |
| Desktop views | New run, run list, run view, inbox, sessions grouped under runs | App |
| Phone app | Chat, runs, inbox — over Tailscale to each backend | Owner's phone |

Principle: **the engine decides what happens next, never an agent.** Agents do the work;
scripts verify it; rules choose the next step; the owner decides anything outside the rules.

Principle: **build on what exists.** Reuse the current engine (`tauri-svelte-preview/src-tauri/src/workflow.rs`),
its ledger (`orchestration_events` via `orchestration.rs:99-101,283-347`), the broker tables and
safe-boundary delivery (`core/src/broker.rs:81-180`, `manager.rs:1108-1233`), owned-session ids
and remote routing (`protocol.rs:596-652`). Change only what this spec requires.

## 4. Messaging

### 4.1 Identity and directory

- Every session keeps its permanent hidden id (the owned id). It also gets a **readable name**:
  workers are `<task-key>/<step>` (for example `tsk-1412/build`); the main orchestrator is
  `orchestrator`; owner-created sessions use their title, made unique per machine.
- The **directory** lists every session the caller can reach: name, id, provider, project,
  machine, state (`running`, `idle`, `waiting`, `stopped`), and its run and step if any.
- Names resolve to ids at send time. An ambiguous or unknown name is an error returned to the
  sender, never a guess.

### 4.2 Agent tools (one MCP server, every provider)

Assembly runs an MCP server, `assembly`, and passes it to every session it starts through the
ACP `session/new` / `session/load` `mcpServers` field (today `acp_client.rs:621` sends only the
working folder). Tools:

| Tool | Who gets it | What it does |
|---|---|---|
| `list_sessions()` | every session | The directory above |
| `send_message(to, text, reply_to?)` | every session | Queue a message; returns its id and status |
| `report_review(verdict, findings[])` | review steps | Structured verdict the engine counts (§5.3) |
| `start_run(workflow, tasks[], machine?, i_merge?)` | orchestrator | Start one run per task |
| `list_runs()`, `get_run(id)` | orchestrator | Run state and history |
| `pause_run(id)`, `resume_run(id)`, `stop_run(id)` | orchestrator | Control runs |
| `ask_owner(question, options[])` | orchestrator | Put a confirm item in the inbox (D13) |

Providers that cannot take an MCP server at session start are out of scope until one exists;
Claude Code and Codex both accept MCP servers.

### 4.3 Delivery

- Every message is stored first (existing `workflow_messages` table; sender id, target id,
  message id, text, time, `reply_to`, status).
- **Idle target:** delivered as a normal prompt, prefixed `[message from <sender name>]`.
- **Busy target:** held until the current turn ends, then delivered (works for every
  provider). Where a provider supports adding input to a running turn — Codex app-server
  `turn/steer` — deliver immediately instead. Other mid-turn routes are added only when
  verified.
- Status shown to sender and owner: `queued → delivered | failed`. Nothing claims "read".
- Messages appear in the receiving session's chat as a labelled message from the sender, and in
  the run timeline (§7.3).
- Messages from agents are never approvals (D13).

### 4.4 Across machines

A run and its workers live on one machine, so normal messaging stays on one backend. A message
to a session on another machine is relayed by the desktop app while it is connected to both;
otherwise it fails with a clear error. No backend-to-backend link in this version.

## 5. Workflow files

### 5.1 Format

```yaml
# .assembly/workflows/dev-pipeline.yml
name: Dev pipeline
inputs:
  task: { description: Notion task, required: true }
  i_merge: { description: "I'll merge", default: false }
limits: { max_sessions: 3, step_timeout_minutes: 60, max_rounds: 5 }

steps:
  recon:
    agent: { provider: codex, model: gpt-6-sol }
    prompt: prompts/recon.md
  spec:
    needs: [recon]
    agent: { provider: claude, model: opus }
    prompt: prompts/spec.md
  spec-review:
    needs: [spec]
    review: { of: spec, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/spec-review.md }
  build:
    needs: [spec-review]
    agent: { provider: codex, model: gpt-6-sol }
    prompt: prompts/build.md
  checks:
    needs: [build]
    run: |
      npx tsc --noEmit
      pnpm run check:svelte
      cd core && cargo test
    on_fail: build
  code-review:
    needs: [checks]
    review: { of: build, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/code-review.md }
  proof:
    needs: [code-review]
    agent: { provider: claude, model: opus }
    prompt: prompts/proof.md
    expect_files: ["proof/*.png"]
  pr:
    needs: [proof]
    run: gh pr create --fill
  merge:
    needs: [pr]
    gate: { ask_owner_if: "${{ inputs.i_merge }}" }
    run: gh pr merge --merge
```

### 5.2 Step kinds

| Kind | Meaning |
|---|---|
| `agent` | Start a worker session with the prompt file, provider and model. Done when its turn ends. |
| `run` | A shell script the engine runs in the run's worktree. Exit code is the result; output is stored. |
| `review` | An agent step that must call `report_review`. Findings send the step named in `of` back to redo. |
| `gate` | Pauses for the owner when its condition is true; otherwise passes. May carry a `run`. |

`needs` lists dependencies. `on_fail` names the step a failed `run` sends back to. `expect_files`
are checked by the engine after an agent step; missing files fail the step.

### 5.3 Data between steps

- Prompt files are templates. The engine fills `${{ inputs.* }}`, `${{ task.title }}`,
  `${{ task.body }}`, `${{ steps.<id>.summary }}`, `${{ steps.<id>.files }}` and, on a redo,
  `${{ redo.findings }}` / `${{ redo.output }}`.
- A step's **summary** is its final agent reply (or the last 200 lines of script output).
- **Notion task binding:** the engine reads the task's title, key and body when the run starts
  (existing Notion task API, `notion_tasks.rs:91-115`) and records the task key on the run.

### 5.4 Existing templates

Today's templates live in app storage (`agents.workflow-templates`,
`WorkflowRuns.svelte:91-109,370-408`) and the built-in task template is TypeScript
(`taskWorkflowTemplate.ts`). The YAML file replaces them as the definition source. The built-in
template is rewritten as `dev-pipeline.yml`; app-stored templates are not migrated (none are in
use by the owner).

## 6. Run engine

### 6.1 Lifecycle

1. A run starts on a machine. The engine creates a worktree from fresh `main` under the
   project's worktree root, on branch `tsk-<id>-<slug>`.
2. Steps start when their `needs` are done, at most `max_sessions` agent sessions at once
   across all runs on that machine.
3. Step states: `waiting`, `running`, `passed`, `failed`, `redoing`. Every attempt is numbered
   and kept.
4. Run states: `running`, `paused` (by the owner), `waiting-for-owner`, `merged`, `stopped`,
   `failed`. The worktree is removed when the run merges or is stopped.

### 6.2 Loops

- A review with findings sends its `of` step back. The redo goes to the **same** worker session
  (it keeps its context). Each review round uses a **fresh** reviewer session.
- A failed `run` step sends its `on_fail` step back with the failing output.
- Both count as rounds against one loop per step pair. Continue while each round's finding
  count (or failing-check count) is lower than the last; otherwise pause for the owner. Stop
  at `max_rounds`.

### 6.3 Pause, resume, stop

- **Pause**: no new step starts; running agent turns finish; the run shows `paused`.
- **Resume**: scheduling continues.
- **Stop**: running turns are cancelled, worker sessions stopped, worktree removed.

### 6.4 Waiting for the owner (inbox)

Each pause becomes one inbox item: what happened, links to evidence, buttons ("Run another
round", "Accept as is", "Stop the run", or the gate's options) and a reply box. Sources:
loop rule, red check at the round limit, step timeout, gate, `ask_owner`, and **agent permission
requests** from any session (the existing per-session permission prompt is also listed here).
An answer is recorded as the owner's decision in the run history. The reply text goes to the
orchestrator labelled as from the owner.

### 6.5 Restart

Run state is in the ledger. On backend start, the engine reloads runs that were `running`:
agent steps resume their native session where the provider supports it; otherwise the step is
restarted once, then pauses for the owner. `run` steps are re-run. Paused and waiting runs stay
as they were.

### 6.6 Remote

Today dispatch is hard-wired to local (`workflow.rs:557-570`) and the engine is built over the
local store only (`main.rs:5443-5466`). The engine moves to code both backends run, and the
remote command set (`remote.rs:72-165`) gains the run and inbox commands. The desktop shows runs
from every connected machine.

## 7. Desktop views

Visual target: `2026-08-20-workflow-board-mockup.html`. DESIGN.md is binding.

1. **New run** dialog: workflow, one or many Notion tasks (or free text), machine, "I'll merge".
2. **Run list**: task key and title, current step, state, machine, elapsed, loop round. A
   "Waiting for you" count opens the inbox.
3. **Run view**: a pipeline strip (each step's state and attempt count); clicking a step opens
   its session in the normal chat view, a check's output, or a review's findings per round. A
   **timeline** below lists, in order: step starts and ends, messages (from, to, text, status),
   check results, owner decisions, proof thumbnails, PR link.
4. **Sessions rail**: a run's sessions are grouped under the run with their readable names.
5. **Main orchestrator**: an ordinary chat session pinned at the top of the rail.
6. **Inbox**: the items from §6.4 with their buttons and reply box.

## 8. Phone app (own spec follows)

- Flutter, Android first, in `mobile/` in this repo; project shell and messaging UI taken from
  Rental Command's Android app (`~/dev/work/rental-management/mobile`).
- Talks to each backend over Tailscale through a small HTTP API: orchestrator chat (send,
  stream replies), runs (list, state, pause, resume), inbox (list, answer).
- Pairing: a QR code in Assembly Settings gives the phone a token.
- Push: the backend notifies the phone when an inbox item appears; the phone spec chooses the
  mechanism after checking what Rental Command uses.
- Later: live run view, messaging any session, starting runs.

## 9. Build order

Each slice is one PR that works on its own, built on the workbox through the lane pipeline.

| # | Slice | Proven by |
|---|---|---|
| 1 | Messaging: MCP server on every session, names and directory, `list_sessions`, `send_message`, delivery and receipts, messages in chat | Two sessions message each other in all four scenarios (Claude Code and Codex × This Mac and Workbox Test) |
| 2 | Engine: YAML loader, four step kinds, loop rule, worktree per run, pause/resume/stop, restart, local and remote | Rust tests for loop rule, gate and restart; a demo workflow whose check fails, loops, then passes, on This Mac and Workbox Test |
| 3 | Desktop views: New run (many tasks), run list, run view and timeline, rail grouping | Screenshots of a real run on This Mac and Workbox Test |
| 4 | Main orchestrator, inbox (including permission requests), Notion binding, `dev-pipeline.yml` and prompts, merge rule and "I'll merge" | The dev pipeline takes one small real task to a merged PR on the workbox |
| 5 | Phone app (separate spec): Android shell, pairing, chat, runs, inbox, push | Answering a paused run and a permission request from the phone |

## 10. Testing

- Logic tests in Rust (`cargo test`) and frontend scripts (`scripts/*.test.ts`). No UI
  regression tests (owner moratorium).
- Gates: `npx tsc --noEmit`, `pnpm run check:svelte` ends "Files the /next shell owns: 0
  error(s)", `cd core && cargo test`.
- Chat-affecting slices are verified in the four scenarios; remote tests use only Workbox Test.
- Proof per PR: at most 4 screenshots and one video of 2 minutes or less.
- One combined code and spec review per slice.

## 11. Not in this version

- A2A, ANP, or any protocol for agents outside Assembly.
- Backend-to-backend messaging; a run spanning machines.
- A visual workflow editor (files are edited as text; the app shows them read-only).
- Temporal/Inngest-style distributed durable execution.
- Fan-out steps (`fanOut` stays in the old types but is not exposed in YAML yet).
- iPhone and desktop builds of the phone app.

## 12. Risks

| Risk | Guard |
|---|---|
| Agents chatter endlessly or loop | Loop rule, round ceiling, `max_sessions`, step timeout |
| An agent claims work it did not do | `run` steps and `expect_files` are checked by the engine |
| Token cost | One orchestrator, workers end with their step, fresh reviewers only per round |
| Message arrives too late to matter | Codex mid-turn delivery; status visible to sender |
| Quitting the desktop app stops local runs | Long runs are started on the workbox (D7) |
