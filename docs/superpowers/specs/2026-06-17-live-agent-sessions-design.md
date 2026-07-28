# CommandBar — Live Agent Sessions (Sub-project 1) — Design (v2, review-hardened)

> Working name: **CommandBar** (rename planned). First sub-project of a larger redesign; its own
> spec → plan cycle. Date: 2026-06-17 (hardened 2026-06-18 after an adversarial sub-agent review —
> see §11). Branch: `tsk-346-324-321-dockview-redesign`.

---

## 1. Vision
A **native, agent-first command center**: run, watch, and work with all your coding-agent sessions
(Claude Code, Codex, future local / other LLMs) in one place — eventually replacing the iTerm +
VS Code juggling. A deliberate mixture: **CMUX** (left rail of sessions, each restoring its own live
"little workspace") + **Codex app** (switch a conversation, the whole workspace auto-loads) + **VS
Code** (a real IDE: LSP, go-to-def/ref/impl, find-refs, peek, code-lens) + **Warp** (text clarity).

**Principles:** native **Rust/Tauri** for speed + low RAM; **cross-platform where reasonable**
(portable-by-default, macOS-first); **modern, spacious, minimal**; **no subscription lock-in**; open
to any agent with a **CLI** (multi-LLM ≈ free). **Enabling insight:** agents run as CLI processes in
embedded terminals, so any CLI agent works; we already have the IDE shell (Monaco, file tree, git,
Dockview), so the missing piece is the **live-agent session core** = this sub-project.

## 2. Problem (today)
The rail **scans external agent logs and `resume`s** them → a *separate, new* process re-loading the
transcript, which looks "way behind / not doing anything new" (resume can't mirror a process in
another terminal). The frontend is a **singleton xterm** that `reset()`s + replays a bounded
scrollback on switch → corrupts/freezes a live TUI and drops hidden output. The rail conflates a
"sessions" list and a "saved workspaces" list. **Git/worktrees** are broken in spots. **LSP**
sometimes needs a manual refresh and likely runs redundant servers. (Precursor fixed this session:
the embedded terminal now spawns a **login shell** (`-l`) so agent CLIs resolve on `PATH`.)

## 3. Goal & Non-goals
**Goal.** A session is a **live, owned, switchable workspace**: start an agent *inside* CommandBar
(or adopt an external one); switching is instant and faithful — the agent keeps running and its
project/branch/worktree/files/browser/terminal load in place, no window reload, bounded RAM.

**Non-goals (this sub-project):** deep IDE *building* (LSP already works — we only consolidate it);
the command-bar **scratchpad** utility; agent **orchestration** (loops: build/test/fix/merge) +
**sub-agent visibility** (later — see §11 CMUX patterns); **split-view** (two terminals side-by-side);
the **rebrand** (anytime).

## 4. Core concepts & identity
- **Available session** — an external Codex/Claude/etc. session found by scanning logs. Listed under
  **Resume**; read-only metadata until adopted.
- **Owned session** — a CommandBar-managed session in its own tab. Created by **start fresh** (project
  + worktree + agent) or **adopt/resume an Available one**. Carries its full **workspace**: project,
  branch, worktree, open files/tabs, browser, terminal running the agent, dock layout.

**Identity — three ids, one primary (a key review fix):**
| id | what | lifetime |
|---|---|---|
| **ownedId** (PRIMARY) | a CommandBar-minted UUID; the single key across rail, manager, snapshot, active-tracking, persistence | stable for the life of the Owned session |
| nativeSessionId | the provider's own id (codex/claude session id); used to locate scanned logs + `~/.cmuxterm` hook files | **mutable** — `resume` mints a *new* one; we re-bind |
| ptySessionId | `terminal.rs` PTY handle (`term-<pid>-<ts>`); bound to ownedId via `liveConversationTerminals.bindSession` | life of the PTY |

- We inject **`COMMANDBAR_SESSION_ID = ownedId`** (plus pane/surface id) into each *spawned* agent
  terminal's env (borrowed from CMUX's `CMUX_WORKSPACE_ID`) so the process + its hooks correlate back.
  We **cannot** retro-inject env into an already-running *foreign* process — only sessions we spawn.
- **Normalize providers** before composing identity: scanner emits `cmux-<agent>` (e.g. `cmux-claude`)
  while snapshots use a fixed enum (`codex|claude|cmux|manual`). Collapse to a base agent + a
  `source: scanned|owned` flag so `cmux-claude` and `claude` don't collide or double-count.
- Manager is currently **1 view : 1 PTY** (`bindSession` is 1:1); plural "terminal(s)" per session is
  future split-view.

## 5. Architecture

### 5.0 Core principle: remove dead code as we go (no calcification)
Dead code calcifies — it accretes accidental references, shifts line numbers, and scares edits.
**Actively removing it is a first-class, continuous discipline here, not a someday-cleanup.** Hence
**Phase 0** below: delete the dead legacy shell + its CSS *before* the live-session surgery, because a
smaller `+page.svelte` makes that surgery far cleaner.

**No massive files; decompose for parallelism.** `+page.svelte` is ~31k lines - the single biggest
blocker to parallel work (only one agent can safely edit it at a time) and a maintenance hazard. After
Phase 0 nearly halves it, **Phase 0b** extracts its cohesive subsystems into focused **`.svelte`
components** + **`.svelte.ts` rune-state modules** (Svelte 5 supports runes in `.svelte.ts`), leaving
`+page.svelte` a thin orchestrator. This kills the big-file pain AND creates **file-disjoint seams** so
Plan 1 / Plan 2 can run across **multiple agents in parallel**.

### 5.1 Rail: Available → Owned
Replace the conflated "Conversations + Saved Workspaces" UI (built on `ConversationList.svelte` +
`src/lib/components/`) with two clear groups: **Owned (live)** — your sessions, each a tab, with a
state badge (Live/Background/Hibernated) + a LIVE dot; and **Resume (available)** — scanned sessions
with an action **Resume → Owned** that adopts one into its own tab.

### 5.2 Live terminal manager + output routing (the heart)
**One terminal surface, N live xterm views, show one; nothing killed on switch.** Each Owned session
owns a persistent xterm view, fed continuously so a hidden session never freezes and a TUI stays
faithful. Wiring (the linchpin — make it explicit in the plan):
- Replace the singleton `embeddedTerminal*` state with `liveConversationTerminals` (built + unit-
  tested, `src/lib/liveConversationTerminals.ts`, currently unwired).
- The single `terminal-output` listener (`+page.svelte:~15567`) dispatches via
  **`feedSession(payload.sessionId, payload.data)`** — **remove the active-only early-return** in
  `handleTerminalOutput` (`+page.svelte:~8426`) that drops hidden output. The `terminated` branch
  calls `markTerminated(key)` + updates the rail badge.
- **Bind ptySessionId → ownedId BEFORE subscribing/replaying** on restore, so no event drops between
  attach and bind (`feedSession` no-ops on an unknown id).

**Backend `terminal.rs` is N-PTY-*capable* but NOT "already ready" — it needs changes (a key review
fix):**
- **Don't kill owned PTYs on webview teardown/reload.** Today `disposeEmbeddedTerminal()` closes the
  active PTY (`+page.svelte:~8452`) and runs in component teardown (`~15739`) → a reload kills the
  agent, breaking re-attach. Guard so Owned sessions survive UI reload.
- **Tombstone terminated sessions.** `spawn_terminal_waiter` (`terminal.rs:~325`) removes a session
  the instant the child exits, and the reader stops on EOF → a finished/crashed agent vanishes from
  `list_terminal_sessions`. Keep it as a tombstone (status + final scrollback; add a `closed`/`exited`
  flag to `TerminalSessionInfo`) so the rail can show "finished — read final output" (what
  `markTerminated` assumes).
- Add **`COMMANDBAR_SESSION_ID`** to `TerminalStartRequest` (`terminal.rs:~13`) + thread into the env
  block (`~88`).

### 5.3 Workspace auto-load on switch
Clicking an Owned session loads its workspace **in place** (no new window/reload): restore
project + branch/worktree context, files/tabs, browser, dock layout (reuse
`restoreConversationWorkspaceSnapshot`), then **show** that session's live view (instant if Live;
revive if Background). LSP **re-points** to the new root (see 5.5).

### 5.4 Lifecycle & RAM — state machine (greenfield; no LRU/cap/hibernate code exists yet)
| State | xterm instance | GPU (WebGL) context | agent process | enter when | revive |
|---|---|---|---|---|---|
| **Live** | mounted in DOM | held | running | one of most-recent **N** | n/a (visible) |
| **Background** | **alive, detached** (`display:none`) — NOT disposed | **released** (canvas/none while hidden) | **running** | beyond N (LRU) | re-insert element + `fit()` + `focus()` + reacquire GPU; **zero replay** |
| **Hibernated** | disposed (after saving a serialize-addon snapshot) | released | **stopped** | **idle ≥ X min (default 30, configurable)** or manual | cold-resume (native resume) + paint grid from the serialize snapshot |

- **Critical fix:** Background must **keep the xterm instance alive, just hidden** — NOT
  `dispose()`+replay. Replaying the backend byte-log (`terminal.rs` scrollback is **256 KB, truncated
  from the front**, not on ANSI boundaries) cannot reconstruct a live TUI (alt-screen, cursor jumps,
  mid-escape cuts) — that's the very corruption we're fixing. The manager already feeds hidden views
  (`feed()` writes regardless of visibility), so keeping the instance alive = instant, faithful revive.
- **RAM honesty:** "freeing the view" frees only **renderer/GPU** RAM. The dominant RAM is the **agent
  processes + their tool subprocesses** (a node-based agent is hundreds of MB), which keep running in
  Background. The real reclaim levers are **Hibernate** (stops the agent) and **LSP dedupe** (5.5).
  So **idle-timer auto-hibernate** is the footprint mechanism; Background is for fidelity/instant-switch.
- **GPU budget (new with Option B):** webviews cap live WebGL contexts (~16/page); Monaco uses some.
  Only the **visible** terminal needs a context → release the **WebglAddon** (not the Terminal) on
  Background. Tie **N to ~12–14**, not an arbitrary number.
- **Throttle hidden-view writes:** a chatty background agent feeds its hidden xterm continuously;
  coalesce/throttle parsing for non-visible views so N background agents don't jank the main thread.

### 5.5 LSP — consolidate (it's THREE mechanisms, not two)
Verified stacks: (a) real backend language servers in `src-tauri/src/lsp.rs`
(`typescript-language-server`, `csharp-ls`, `rust-analyzer`, `svelte-language-server`), sessioned
**per (root, language)**; (b) a **regex** symbol fallback in `src-tauri/src/main.rs`; (c) Monaco's
built-in **ts/json workers** as last-resort. The real waste isn't "two stacks" — it's **spawning a
second server per worktree for the same repo**. Fix: **reuse one server per language across worktree
roots** (dedupe per-root spawns); keep Monaco workers (instant feel) + regex (offline) as fallbacks.
**Re-index on switch = re-point the warm server at the new root, debounced, cancel-in-flight, only on
first activation / detected file changes** — keep servers warm in Background; never tear LSP down on
switch (else switching gets slow — the opposite of the goal).

### 5.6 Git / worktree
- **Invariant:** switching the *view* **never** alters another session's cwd/branch. Each agent stays
  pinned to its own worktree; we restore *display* context only — **never** `cd`/`git switch`/
  `git checkout`/`git worktree add` in a backgrounded PTY. `conversationWorkspaceRestore`.
  `planConversationRestore` (`preCommands: []`) is the **enforcement point** (cwd-only, no git mutation).
- Honor the global safety rule: never auto-create/destroy worktrees; a missing worktree → copyable
  repair plan (reuse the readiness model), never an auto `git worktree add`.
- **Same-worktree collision:** two Owned sessions on the *same* worktree = two agents writing the same
  files. Detect + warn (extend the readiness model); don't silently allow.
- Inventory + fix the broken git/worktree functionality during planning.

### 5.7 Persistence & restore across restarts
- **Within a running app:** never kill; survive **webview reload/HMR**; switching keeps instances
  alive (5.4).
- **Across a full app relaunch (v1 scope):** **cold-resume** each session from a persisted snapshot +
  the agent's native resume command — CMUX's own model (it doesn't keep agents alive across restarts
  either; it cold-resumes from `~/.cmuxterm/<agent>-hook-sessions.json`, which **our scanner already
  reads**). **Defer** true keep-alive-across-full-quit (OS-specific; needs an orphan-reaper). Note the
  "never orphan an agent" requirement for when we revisit.
- **Cold-path rendering:** for Hibernate save + cross-restart revive, snapshot the rendered grid with
  the **SerializeAddon** (already loaded at `+page.svelte:~8012`, currently unused for restore) —
  strictly better than replaying a truncated byte tail. (Cold path fidelity is imperfect but
  acceptable; the live path never replays.)

### 5.8 Terminal renderer: xterm.js now, abstracted + swappable
Keep **xterm.js** (WebGL). It composes natively in our webview + Dockview shell; **libghostty** (CMUX's
renderer) is a native GPU surface that would force native-over-webview compositing fighting the
Dockview layout, or a much more native shell. The renderer is not the differentiator and didn't cause
the stale/behind bug. All logic talks to the **`TerminalView`** seam
(`{write, fit, focus, setVisible, dispose}`), so libghostty stays a future swap behind a stable
interface — revisit only if terminal feel/perf becomes a proven bottleneck. (xterm.js is
cross-platform today; libghostty is mac/Linux, weak on Windows.)

### 5.9 Cross-cutting
- **Observability/diagnostics:** with N PTYs + N xterms + N LSP roots, add a diagnostics surface
  (per-session state, PID, GL-context count, LSP servers running). Today's status strings are
  per-singleton.
- **Single-window assumption** for v1 (no multi-window code exists; the global listener + localStorage
  store would race across windows). State it; design later if needed.
- **Accessibility:** xterm focus across hidden/shown views, screen-reader behavior of `display:none`
  terminals, focus on switch.
- **Error/empty states:** PTY spawn failure mid-switch; agent crash (non-zero exit) while Background;
  WebGL context-loss; dead session in `list_terminal_sessions`; residual resume "command not found";
  missing-worktree on a session you switch *to*; "finished agent" final-output UI for `markTerminated`.

## 6. Data, persistence & migration
Owned sessions + workspace snapshots persist (reuse `workspaceSnapshot.ts` + `persistActive
WorkspaceSessionKey`). Migration story (don't skip): the snapshot `provider` enum vs scanner's
`cmux-*` (normalize, §4); add lifecycle state + N + idle-timeout to `settingsStore`; migrate persisted
`workspace-snapshots` to the Owned model (with ownedId). Lifecycle state + LRU order + the live-set
cap N are tracked in memory; N + idle-timeout are user settings.

## 7. Reuse (don't reinvent)
`liveConversationTerminals.ts`, `conversationWorkspaceRestore.ts`, `ConversationList.svelte`, the
`workspaceSnapshot` pipeline + readiness model, the `setPanelElement`/`attachPanelElement` bridge, the
repair-plan copy, xterm addon/theme setup (incl. the loaded-but-unused SerializeAddon), the
`terminal.rs` PTY registry, the login-shell `-l` fix.

## 8. Scope summary + disposition of OLD code
**In:** Phase 0 deadwood removal; owned/live model + Available→Owned rail; live multi-terminal manager
wired in (Option B) replacing the singleton; workspace auto-load on switch; the three-state lifecycle
(Background keeps-instance; idle-timer Hibernate; WebGL budget; throttled hidden writes); backend
`terminal.rs` changes (no-kill-on-reload, tombstone, `COMMANDBAR_SESSION_ID`); identity normalization;
LSP dedupe + warm/debounced re-index; git/worktree fixes + invariants; serialize-addon cold path.

**Disposition of OLD code (a core deliverable, not an afterthought):** delete the unreachable
**legacy A/B shell** (`useUnifiedWorkbench=false` `{:else}` branch, ~6.3k lines) + its **~9.8k-line
`<style>` block** (≥37 confirmed-orphan selectors) + the now-unused `useUnifiedWorkbench` flag; and the
old **scan-and-resume / external-app-open** paths once the Owned model replaces them. Phase 0 takes the
shell + CSS first (gated only by verification, not by Plan 1).

**Out / later:** scratchpad; orchestration (loops) + sub-agent visibility; split-view; rebrand.

## 9. Cross-platform seams + to-inventory-during-planning
Name the non-portable seams (macOS-first is fine): `homeDirGuess()`/`$HOME` (empty on Windows,
`sessions.rs`), the `-l` login-shell branch (mac/Linux shells only — Windows cmd/pwsh load no env),
any macOS `~/Library/Application Support` persistence. Inventory during planning: exact LSP topology +
the safe dedupe path; specific git/worktree breakages; the precise legacy-shell + dead-CSS line ranges
(drive deletion off Svelte's "unused CSS selector" warnings + confirming the `{:else}` branch is
unreachable).

## 10. Success criteria
Start or resume 2–3 agents inside CommandBar; switch among them — **instant, faithful, still running**
(a building agent keeps progressing while hidden). No stale/corrupt TUI; no manual LSP refresh after
switch. Switching loads the right branch/worktree/files/browser **in place** (no reload). RAM bounded:
Background frees GPU/renderer; idle-timer Hibernate reclaims agent RAM; **one** LSP server per language
(not per-root). Webview reload survives + reattaches; relaunch cold-resumes. `+page.svelte` is
materially smaller (dead shell + CSS gone). `tsc` clean; focused unit tests for pure helpers pass;
verified live in Tauri.

## 11. Review hardening (what the adversarial sub-agent review changed)
- **Background = keep instance alive (hidden), not dispose+replay** — replaying a truncated byte log
  can't reconstruct a TUI (was a self-contradiction). Only Hibernate disposes.
- **RAM honesty** — view-unmount frees only GPU/renderer; the real lever is Hibernate + LSP dedupe;
  added the **WebGL ~16-context cap** as a force (release WebglAddon on Background; N≈12–14).
- **Backend not "already ready"** — `terminal.rs` must stop killing owned PTYs on teardown and must
  tombstone terminated sessions; add `COMMANDBAR_SESSION_ID`.
- **Output routing made explicit** — single listener → `feedSession`; bind-before-subscribe.
- **Identity** — minted ownedId is primary; native ids mutable (resume mints new); normalize `cmux-*`.
- **LSP is three mechanisms** — real waste = per-(root,language) duplication; dedupe + warm + debounce.
- **git switch never touches another agent's cwd**; same-worktree collision warning.
- **Serialize-addon for the cold path**; **v1 PTY scope** = reload-survive + cold-resume (defer
  cross-quit). Plus the many MISSING items folded into §5.9/§6/§8.

## 12. Phasing (three steps; each meaty plan gets its own sub-agent review)
- **Phase 0 — Deadwood removal (FIRST).** Delete the unreachable legacy A/B shell + its ~9.8k-line CSS
  + the `useUnifiedWorkbench` flag, in staged **verified green commits** (Vite-compile + screenshot per
  step; compiler-flagged orphans first; confirm the `{:else}` branch is unreachable). Mechanical
  deletion — verified, not heavily planned. Shrinks `+page.svelte` before the rewrite.
- **Phase 0b — Decompose `+page.svelte` (unlocks parallelism).** Extract cohesive subsystems into
  focused `.svelte` components + `.svelte.ts` rune-state modules (terminal/session state, git/worktree,
  LSP/intelligence, command palette, quick-open, browser, dock wiring, settings), leaving
  `+page.svelte` a thin orchestrator. Done carefully + verified (compile + screenshot per extraction);
  extraction edits the monolith so it is sequential, but it UNLOCKS file-disjoint parallel lanes for
  Plan 1/2. Scope: extract enough to remove the big-file pain + enable parallelism, not to perfectly
  modularize.
- **Plan 1 — Live multi-terminal core (the heart).** Backend `terminal.rs` FIRST (no-kill-on-reload for
  owned; terminated tombstone + `closed` flag; `COMMANDBAR_SESSION_ID`) → wire `liveConversation
  Terminals` into `+page.svelte` replacing the singleton (`feedSession` dispatch; bind-before-subscribe)
  → Background = detached-instance + released WebGL context → three-state FSM + LRU + N + idle-hibernate
  → Available→Owned rail + start-fresh/adopt + identity normalization + serialize-addon cold path.
  Delivers §10 criteria 1–4 (the demo).
- **Plan 2 — Workspace correctness (git + LSP) + remaining cleanup.** LSP dedupe per-(root,language) +
  warm/debounced re-index → git/worktree breakage fixes + in-place-switch invariants + same-worktree
  warning → delete old scan-and-resume paths → diagnostics surface.
- **Order:** Phase 0 → Phase 0b → Plan 1 → Plan 2. Within Plan 1, backend before frontend. Phase 0's
  shell deletion is gated only by its own verification (the unified shell is the proven daily driver;
  git is the net).
- **Parallelism:** Phases 0/0b are mostly sequential (they edit the monolith); Plan 1/2 are written as
  **file-disjoint parallel lanes** (backend `terminal.rs` ‖ terminal-manager module ‖ rail ‖ LSP ‖ git)
  with sequential gates only on true dependencies (backend before frontend wiring). Hard rules: one
  agent per file at a time; never run parallel heavy builds/tests (cargo/dotnet serialized).
