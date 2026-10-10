# Workflows and orchestration — design spec

Date: 2026-10-10. Status: approved by the owner section by section in a brainstorm on 2026-10-10;
revised the same day after spec review (`~/dev/work/reports/tsk-1416/spec-review.md` on the
workbox, 10 findings, all accepted).
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
| Main orchestrator | One ordinary agent session with extra tools to start, list, pause and resume runs | The **home machine** (§4.5) |
| Worker sessions | Ordinary Assembly sessions created by the engine for agent and review steps | Same backend as the run |
| Messaging | One endpoint per backend plus the `assembly` MCP server: directory, send, delivery receipts | Every backend |
| Inbox | Every item waiting for the owner: run pauses, gates, agent permission requests | Every backend's database |
| Run history | Every step attempt, message, check result, decision, proof link | Every backend's database |
| Desktop views | New run, run list, run view, inbox, sessions grouped under runs | App |
| Phone app | Chat, runs, inbox — over Tailscale to each backend | Owner's phone |

Principle: **the engine decides what happens next, never an agent.** Agents do the work;
scripts verify it; rules choose the next step; the owner decides anything outside the rules.

Principle: **build on what exists.** Reuse the current engine (`tauri-svelte-preview/src-tauri/src/workflow.rs`),
its ledger (`orchestration_events` via `orchestration.rs:99-101,283-347`), the broker's message
store and safe-boundary delivery (`core/src/broker.rs:81-180`, `manager.rs:1108-1233`), and
remote session routing (`agent_conversation/mod.rs:110-170`). Change only what this spec requires.

## 4. Messaging

### 4.1 Identity and directory

- Every session keeps its permanent hidden id (the owned id). It also gets a **readable name**:
  workers are `<task-key>/<step>` (for example `tsk-1412/build`); the main orchestrator is
  `orchestrator`; owner-created sessions use their title, made unique per machine.
- The **directory** lists every session on the backend: name, id, provider, project, state
  (`running`, `idle`, `waiting`, `stopped`), and its run and step if any.
- Names resolve to owned ids at send time. An ambiguous or unknown name is an error returned
  to the sender, never a guess.

### 4.2 Hosting and caller identity

- Each backend (desktop app, remote backend) exposes **one local messaging endpoint**
  (loopback only).
- The `assembly` MCP server is a **small stdio client** of that endpoint. Assembly passes it to
  every session it starts through ACP `session/new` / `session/load` `mcpServers` (command plus
  environment), including a **backend-issued token bound to that owned session**.
- The endpoint identifies the caller **only from the token**, never from a name in the prompt,
  and checks the caller's allowed tools server-side (orchestrator-only tools, review-only
  `report_review`).
- Adapter support (verify in slice 1, with real tool calls):
  - Claude Code via `claude-agent-acp` 0.88.0: assumed to forward `mcpServers`; verify.
  - Codex via the repo bridge (`tools/codex-acp-bridge/bridge.mjs:1042-1067`): reads `cwd` and
    the session id but **ignores `mcpServers`** today. Slice 1 adds forwarding to
    `thread/start` / `thread/resume`.
  - Codex via the packaged `@agentclientprotocol/codex-acp` 1.13.1 (if it is the one in use):
    verify.
  - Codex mid-turn delivery: the bridge implements `turn/steer` (`bridge.mjs:1152-1163`); verify
    the installed Codex app-server accepts it before enabling immediate delivery.

### 4.3 Agent tools

| Tool | Who gets it | What it does |
|---|---|---|
| `list_sessions()` | every session | The directory above |
| `send_message(to, text)` | every session | Queue a message; returns its id and status |
| `report_review(verdict, findings[])` | review steps | Structured verdict the engine counts (§6.2) |
| `start_run(workflow, tasks[], machine?, i_merge?)` | orchestrator | Start one run per task; each task is `{key, title, body}` (§5.3) |
| `list_runs()`, `get_run(id)` | orchestrator | Run state and history |
| `pause_run(id)`, `resume_run(id)`, `stop_run(id)` | orchestrator | Control runs |
| `ask_owner(question, options[])` | orchestrator | Put a confirm item in the inbox (D13) |

Replies need no separate field: the sender's name travels with every message, and the receiver
answers with `send_message`.

### 4.4 Delivery

- Every message is stored first (existing `workflow_messages` table). Today messages are scoped
  to a broker group and delivery takes the target's first open group
  (`core/src/session_store.rs:618-637`, `manager.rs:1126-1133,1186-1202`). This changes to **one
  queue per backend keyed by the target's owned id**, so any session can reach any session
  whether or not it belongs to a run. Run membership is recorded on the message for the
  timeline, not used for routing.
- **Idle target:** delivered as a normal prompt, prefixed `[message from <sender name>]`.
- **Busy target:** held until the current turn ends, then delivered (works for every
  provider). For Codex, once `turn/steer` is verified (§4.2), deliver into the running turn.
- Status shown to sender and owner: `queued → delivered | failed`. Nothing claims "read".
- Messages appear in the receiving session's chat as a labelled message from the sender, and in
  the run timeline (§7).
- Messages from agents are never approvals (D13).

### 4.5 Across machines, and where the orchestrator lives

- A run and its workers live on one machine, so normal messaging stays on one backend.
- A message to a session on another machine is relayed by the desktop app while it is
  connected to both; otherwise `send_message` returns "target machine not reachable". No
  backend-to-backend link in this version.
- The main orchestrator lives on one **home machine**, chosen in Settings, **default: the
  workbox remote** (it stays up when the Mac is closed). The desktop and the phone both reach it
  directly on that backend. `start_run(..., machine?)` defaults to the home machine; a run on
  another machine is started through the desktop while it is connected to that machine.

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
    prompt: prompts/build.md          # the build agent commits its work on the run branch
  checks:
    needs: [build]
    run:                              # each command counts separately (§5.2)
      - cd tauri-svelte-preview && npx tsc --noEmit
      - cd tauri-svelte-preview && pnpm run check:svelte
      - cd core && cargo test
    repeat: true                      # safe to rerun after a restart (§6.5)
    on_fail: build
  code-review:
    needs: [checks]
    review: { of: build, agent: { provider: codex, model: gpt-6-sol }, prompt: prompts/code-review.md }
  proof:
    needs: [code-review]
    agent: { provider: claude, model: opus }
    prompt: prompts/proof.md
    expect_files: ["${{ run.proof_dir }}/*.png"]
  pr:
    needs: [proof]
    run:
      - git push -u origin HEAD
      - gh pr create --fill --body-file "${{ run.pr_body }}"
  merge:
    needs: [pr]
    if: "${{ !inputs.i_merge }}"
    run:
      - ALLOW_MERGE=1 gh pr merge --merge
```

### 5.2 Step kinds

| Kind | Meaning |
|---|---|
| `agent` | Start a worker session with the prompt file, provider and model. Done when its turn ends; a failed or cancelled turn fails the step. |
| `run` | A list of shell commands the engine runs in the run's worktree, each from the repo root. Every command runs; the step passes only if all exit 0. The **failing-command count** is the step's finding count for the loop rule. Output is stored. |
| `review` | An agent step that must call `report_review` **exactly once**. Findings send the step named in `of` back to redo. |
| `gate` | Pauses for the owner with given options. Not used by the dev pipeline. |

Fields: `needs` lists dependencies. `on_fail` names the step a failed `run` sends back to.
`expect_files` are checked by the engine after an agent step; missing files fail the step.
`if` skips the step when false. `repeat: true` marks a `run` step safe to rerun after a restart.

Engine-provided values: `run.proof_dir` (a folder outside the repo, linked in run history) and
`run.pr_body` (a file the engine writes: task link, step summaries, final review verdict, proof
file list).

### 5.3 Data between steps

- Prompt files are templates. The engine fills `${{ inputs.* }}`, `${{ task.key }}`,
  `${{ task.title }}`, `${{ task.body }}`, `${{ steps.<id>.summary }}`, `${{ steps.<id>.files }}`
  and, on a redo, `${{ redo.findings }}` / `${{ redo.output }}`.
- A step's **summary** is its final agent reply (or the last 200 lines of script output).
- **Notion task binding:** the caller that starts a run sends a **task snapshot**
  `{key, title, body}` to the owning backend; the backend never needs a Notion token.
  - The desktop New run dialog builds it with the existing detail call
    `read_notion_task_detail` (`notion_tasks.rs:366-373,500-534`).
  - The orchestrator builds it with its own Notion access (its CLI's Notion tools) and passes it
    to `start_run`.
  - The task key is recorded on the run.

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
3. Step states: `waiting`, `running`, `passed`, `failed`, `redoing`, `skipped`. Every attempt is
   numbered and kept.
4. Run states: `running`, `paused` (by the owner), `waiting-for-owner`, `done` (merged, or PR
   open when "I'll merge" is on), `stopped`, `failed`.
5. Worktree cleanup: removed when the run's PR is merged and the worktree is clean. A dirty or
   unmerged worktree is **never** removed by the engine; the run keeps its path, branch and
   `git status` and the owner decides.

### 6.2 Loops

- **Reviews.** A review attempt passes only with one valid `report_review` with verdict PASS.
  No call, a second call, or a failed turn fails the attempt and pauses for the owner. Findings
  send the `of` step back.
- **Checks.** A failed `run` step sends its `on_fail` step back with the failing output.
- **Redo invalidates downstream.** When a step is redone, every step that depends on it, directly
  or indirectly, returns to `waiting` and runs again in order. Nothing merges on results from
  before a redo.
- **The rule (D5).** The first set of findings always allows a redo. After that, continue while
  each round's count (review findings, or failing commands) is lower than the previous round's;
  otherwise pause for the owner. At `max_rounds` the run pauses regardless.
- Redo goes to the **same** worker session (it keeps its context). Each review round uses a
  **fresh** reviewer session.

### 6.3 Pause, resume, stop

- **Pause**: no new step starts; running agent turns finish; the run shows `paused`.
- **Resume**: scheduling continues.
- **Stop**: running turns are cancelled and worker sessions stopped. The worktree follows §6.1.5.

### 6.4 Waiting for the owner (inbox)

Each pause becomes one inbox item: what happened, links to evidence, buttons ("Run another
round", "Accept as is", "Stop the run", or a gate's options) and a reply box. Sources: loop rule,
round limit, review without a valid verdict, step timeout, gate, `ask_owner`, restart
uncertainty (§6.5), and **agent permission requests** from any session (the existing
per-session permission prompt is also listed here). An answer is recorded as the owner's
decision in the run history. The reply text goes to the orchestrator labelled as from the owner.

### 6.5 Restart

Run state is in the ledger. Today startup marks interrupted stages failed
(`workflow.rs:2242-2258`, called from `main.rs:5463-5466`). New behaviour on backend start, for
each step that was `running`:
- a recorded completion is honoured;
- a `run` step marked `repeat: true` is rerun;
- an agent step whose native session can be resumed is resumed **without** sending its prompt
  again;
- anything else (including `pr` and `merge`) pauses for the owner with what is known, for
  example whether the PR already exists.

Paused and waiting runs stay as they were.

### 6.6 Remote

Today dispatch is hard-wired to local (`workflow.rs:557-570`) and the engine is built over the
local store only (`main.rs:5443-5466`). The engine moves to code both backends run, and the
remote command set (`remote.rs:72-165`) gains the run, inbox and messaging commands. The desktop
shows runs from every connected machine.

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
- Talks directly to backends over Tailscale through a small HTTP API: orchestrator chat on the
  home machine (send, stream replies), runs (list, state, pause, resume), inbox (list, answer).
- Pairing: a QR code in Assembly Settings gives the phone a token.
- Push: the backend notifies the phone when an inbox item appears; the phone spec chooses the
  mechanism after checking what Rental Command uses.
- Later: live run view, messaging any session, starting runs.

## 9. Build order

Each slice is one PR that works on its own, built on the workbox through the lane pipeline.

| # | Slice | Proven by |
|---|---|---|
| 1 | Messaging: per-backend endpoint, `assembly` MCP stdio client with per-session token, MCP forwarding in the Codex bridge, names and directory, owned-id queue, `list_sessions`, `send_message`, delivery and receipts, messages in chat | Two sessions message each other with real tool calls in all four scenarios (Claude Code and Codex × This Mac and Workbox Test); Codex `turn/steer` verified or left off |
| 2 | Engine: YAML loader, step kinds, loop rule with downstream invalidation, worktree per run, pause/resume/stop, restart rules, local and remote. Includes **minimal commands** to start a run from a YAML file with a task snapshot, inspect it, and answer a pause; the existing Agents-panel run list (`WorkflowRuns.svelte`) shows it until slice 3 | Rust tests for loop rule, invalidation, review-verdict enforcement and restart; a demo workflow whose check fails, loops, then passes, on This Mac and Workbox Test |
| 3 | Desktop views: New run (many tasks, Notion snapshot), run list, run view and timeline, inbox view, rail grouping | Screenshots of a real run on This Mac and Workbox Test |
| 4 | Main orchestrator on the home machine with its tools, permission requests in the inbox, `dev-pipeline.yml` and prompts, merge rule and "I'll merge" | The dev pipeline takes one small real task to a merged PR on the workbox |
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
| An agent claims work it did not do | `run` steps and `expect_files` are checked by the engine; reviews must report through the tool |
| An agent impersonates another session or the owner | Caller identity comes from the per-session token; approvals only from owner buttons |
| Token cost | One orchestrator, workers end with their step, fresh reviewers only per round |
| Message arrives too late to matter | Codex mid-turn delivery once verified; status visible to sender |
| A crash repeats a PR or merge | Restart pauses non-repeatable steps for the owner |
| Quitting the desktop app stops local runs | Long runs and the orchestrator live on the workbox by default |
