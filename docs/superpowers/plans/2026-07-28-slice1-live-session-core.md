# Slice 1 — Live Session Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The fresh shell's first vertical slice — an Owned/Resume session rail and N live PTY-backed xterm views with instant, faithful switching, nothing killed on switch or webview reload, on a new `/next` route.

**Architecture:** Backend first: `terminal.rs` keeps exited sessions as tombstones and stamps `COMMANDBAR_SESSION_ID` into spawned PTYs. Frontend: the existing (built, unwired, unit-tested) `liveConversationTerminals` manager becomes the only terminal path, fed by a single `terminal_output` listener via `feedSession`; a thin `terminalService` owns all backend IO; new rune-state stores + presentational components live under `src/lib/shell/`; the orchestrator is `src/routes/next/+page.svelte` (thin, no IO in effects). Spec: `docs/superpowers/specs/2026-07-28-fresh-shell-rebuild-design.md`.

**Tech Stack:** Rust (tauri 2, portable-pty), Svelte 5 runes, `@xterm/xterm` + fit/serialize/webgl addons, dockview later (Slice 2 — NOT here), node `--experimental-strip-types` tests with `node:assert/strict` (repo pattern).

## Global Constraints

- **Never modify the old shell**: `src/routes/+page.svelte` is read-only reference. The ONLY shared files this plan may touch are `src-tauri/src/terminal.rs`, `src-tauri/src/main.rs` (additive), `src/lib/tauriSource.ts` (additive type/param sync), and `package.json` (script entries).
- **Constitution (from the spec, binding on every task):** orchestrator route ≤ 300 lines; every panel is a component; state lives in `.svelte.ts` rune modules; **effects never do IO** — backend calls are explicit service functions; nothing hydrates at launch except the session rail; every backend call goes through the dev invoke-counter.
- **No git mutation on restore**: never emit `cd`-into-and-`git switch`/`checkout`/`worktree add` for a session (matches `planConversationRestore`, which always returns `preCommands: []`).
- **Sequenced builds**: never run `cargo build` and `pnpm build` concurrently; one heavy build at a time.
- Commit after every task; branch is `tsk-346-324-321-dockview-redesign` (do not create worktrees for this plan).
- Test runner pattern: `node --experimental-strip-types scripts/<name>.test.mjs`, plain `node:assert/strict`, registered as a `test:*` script in `package.json`.
- All paths below are relative to `tauri-svelte-preview/` unless they start with `docs/` or `core/`.

---

### Task 0: Old-shell autopsy checklist (timeboxed, doc only)

**Files:**
- Create: `docs/superpowers/notes/2026-07-28-old-shell-autopsy.md`

**Interfaces:**
- Consumes: nothing.
- Produces: the audit checklist every later task's "reuse an old `$lib` module" decision must be checked against.

**Timebox: 60 minutes of investigation. Do not fix anything found. Do not exceed the box — write down what you have and move on.**

- [ ] **Step 1: Collect the evidence** (read-only greps against the old shell)

```bash
cd /Users/blackcolours/dev/work/mac-command-bar/tauri-svelte-preview
# Effects that do IO (the cascade disease):
grep -n '\$effect' src/routes/+page.svelte | wc -l
grep -n -A3 '\$effect' src/routes/+page.svelte | grep -nE 'invoke|FromTauri|scanProject|fetch\(' | head -40
# Backend call sites reachable from mount:
grep -nE 'onMount|FromTauri\(' src/routes/+page.svelte | head -60
# Eager hydration at launch (the csharp-ls burn): find what runs before first user action
grep -n 'scanProject\|restoreAgentSessionWorkspaceSnapshot\|selectStartupWorkspaceSnapshot' src/routes/+page.svelte | head
```

- [ ] **Step 2: Write the checklist doc**

Structure (fill each section with the actual findings from Step 1 — file:line refs required):

```markdown
# Old-shell autopsy — what made it slow (audit checklist for reuse)

## Poison patterns found (file:line each)
1. $effect blocks that trigger backend IO: ...
2. Work done at mount before any user action (scans, snapshot restore, LSP-triggering previews): ...
3. Repeated/duplicated backend calls (same command invoked from several reactive paths): ...

## The audit checklist (apply to ANY old $lib module before reuse in /next)
- [ ] No IO at module import time or component mount
- [ ] No $effect that calls invoke/*FromTauri/fetch
- [ ] No hidden singleton state that assumes one instance
- [ ] Pure logic separated from DOM/runtime access

## Known regression commits (from git log, for reference)
cad76d3 eager fan-out read storm · cd2f525 ghost double-scan · 52ad343/0593c20 code-lens whole-project scan · e606033 localStorage quota
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/notes/2026-07-28-old-shell-autopsy.md
git commit -m "docs: old-shell autopsy checklist (audit gate for /next reuse)"
```

---

### Task 1: Backend — tombstone exited terminal sessions

**Files:**
- Modify: `src-tauri/src/terminal.rs` (struct at :22-32, waiter at :325-347, close at :234-252, registry at :44+)
- Modify: `src/lib/tauriSource.ts:46-54` (type sync)

**Interfaces:**
- Consumes: existing `TerminalRegistry`, `spawn_terminal_waiter`, `close_terminal_session`.
- Produces: `TerminalSessionInfo` gains `exited: bool`, `exit_code: Option<u32>`, `signal: Option<String>` (wire: `exited`, `exitCode`, `signal`). `list_terminal_sessions` now INCLUDES exited sessions; `read_terminal_session_scrollback` works after exit; `close_terminal_session` purges tombstones. Later tasks rely on exactly these wire field names.

- [ ] **Step 1: Add the status fields to `TerminalSessionInfo`** (terminal.rs:22-32)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionInfo {
    pub session_id: String,
    pub cwd: String,
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub pid: Option<u32>,
    pub started_at: u128,
    pub exited: bool,
    pub exit_code: Option<u32>,
    pub signal: Option<String>,
}
```

Then fix the construction site in `start_terminal_session` (the compiler will point at it, ~terminal.rs:114-123): add `exited: false, exit_code: None, signal: None`.

- [ ] **Step 2: Add `mark_exited` to the registry impl** (next to `insert`/`remove`, terminal.rs:255-270)

```rust
fn mark_exited(
    &self,
    session_id: &str,
    exit_code: Option<u32>,
    signal: Option<String>,
) -> Result<bool, String> {
    let mut sessions = self
        .inner
        .lock()
        .map_err(|_| "Terminal registry lock poisoned".to_string())?;
    match sessions.get_mut(session_id) {
        Some(handle) => {
            handle.info.exited = true;
            handle.info.exit_code = exit_code;
            handle.info.signal = signal;
            Ok(true)
        }
        None => Ok(false),
    }
}
```

(Match the existing lock-error message style used by `insert`/`remove` — copy it verbatim from those methods.)

- [ ] **Step 3: Change the waiter to tombstone instead of remove** (terminal.rs:325-347)

Replace `let _ = registry.remove(&session_id);` (:333) with a call that runs AFTER exit_code/signal are computed (they currently are at :340-343 — hoist them above if needed):

```rust
// Keep the session as a tombstone: the rail shows "finished — read final
// output", and the scrollback stays readable. Only an explicit close purges it.
let _ = registry.mark_exited(&session_id, exit_code, signal.clone());
```

The `terminal_output` event emission with `terminated: true` stays exactly as is.

- [ ] **Step 4: Make `close_terminal_session` tombstone-safe** (terminal.rs:234-252)

After `registry.remove` returns the handle, only call `killer.kill()` when the session has not already exited:

```rust
if !handle.info.exited {
    let mut killer = handle
        .killer
        .lock()
        .map_err(|_| "Terminal killer lock poisoned".to_string())?;
    if let Err(error) = killer.kill() {
        return Err(format!("Failed to kill terminal session: {error}"));
    }
}
```

- [ ] **Step 5: Build**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles clean (warnings ok, zero errors).

- [ ] **Step 6: Sync the frontend type** (`src/lib/tauriSource.ts:46-54`)

```ts
export type TerminalSessionInfo = {
  sessionId: string;
  cwd: string;
  shell: string;
  cols: number;
  rows: number;
  pid: number | null;
  startedAt: number;
  exited: boolean;
  exitCode: number | null;
  signal: string | null;
};
```

Run: `pnpm check` — expected: no NEW errors versus a pre-change `pnpm check` baseline (capture the baseline first; the old monolith may have pre-existing noise).
Run: `pnpm test:tauri-source` — if it asserts the `TerminalSessionInfo` shape, update its fixtures to include the three new fields.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/terminal.rs src/lib/tauriSource.ts scripts/
git commit -m "feat(terminal): tombstone exited sessions instead of deleting them"
```

---

### Task 2: Backend — COMMANDBAR_SESSION_ID + collision-proof session ids

**Files:**
- Modify: `src-tauri/src/terminal.rs` (request struct :13-20, env block :87-95, id mint :408-410)
- Modify: `src/lib/tauriSource.ts:39-44` (request type)

**Interfaces:**
- Consumes: Task 1's build state.
- Produces: `TerminalStartRequest` gains `owned_id: Option<String>` (wire: `ownedId`); spawned PTY env contains `COMMANDBAR_SESSION_ID=<ownedId>` when provided. Frontend `TerminalStartRequest` gains `ownedId?: string | null`. Task 6's service passes `ownedId` on every start.

- [ ] **Step 1: Extend the request struct** (terminal.rs:13-20)

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStartRequest {
    pub cwd: String,
    pub shell: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub owned_id: Option<String>,
}
```

- [ ] **Step 2: Stamp the env** (in the env block, terminal.rs:87-95, after `command.env_remove("NO_COLOR");`)

```rust
// Correlates the spawned agent process (and any hook files it writes) back to
// the CommandBar-owned session. Borrowed from CMUX's CMUX_WORKSPACE_ID.
if let Some(owned_id) = request
    .owned_id
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
{
    command.env("COMMANDBAR_SESSION_ID", owned_id);
}
```

(If `request` has been partially moved by this point, clone `owned_id` out of it near the top of `start_terminal_session` — keep the borrow checker happy without restructuring.)

- [ ] **Step 3: Collision-proof the session id** (terminal.rs:408-410)

Two sessions started in the same millisecond currently collide and the second registry insert silently overwrites (leaks) the first PTY. Add a process-wide counter:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static TERMINAL_SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_terminal_session_id() -> String {
    let seq = TERMINAL_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("term-{}-{}-{seq}", std::process::id(), timestamp_millis())
}
```

- [ ] **Step 4: Build**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: compiles clean.

- [ ] **Step 5: Sync the frontend request type** (`src/lib/tauriSource.ts:39-44`)

```ts
export type TerminalStartRequest = {
  cwd: string;
  shell?: string | null;
  cols?: number | null;
  rows?: number | null;
  ownedId?: string | null;
};
```

Run: `pnpm check` (no new errors) and `pnpm test:tauri-source` (update fixtures if the request shape is asserted).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/terminal.rs src/lib/tauriSource.ts scripts/
git commit -m "feat(terminal): COMMANDBAR_SESSION_ID env + collision-proof session ids"
```

---

### Task 3: Register the existing live-terminals manager test (baseline green)

**Files:**
- Modify: `package.json` (scripts block)

**Interfaces:**
- Consumes: `src/lib/liveConversationTerminals.ts` (already built) and `scripts/liveConversationTerminals.test.mjs` (already written, currently NOT registered).
- Produces: `pnpm test:live-terminals` as the regression gate every later task runs.

- [ ] **Step 1: Add the script entry** to `package.json` scripts (alphabetical near the other `test:` entries):

```json
"test:live-terminals": "node --experimental-strip-types scripts/liveConversationTerminals.test.mjs",
```

- [ ] **Step 2: Run it**

Run: `pnpm test:live-terminals`
Expected: PASS (all 14 cases in the file — ensureView idempotent, feed-to-hidden, bindSession round-trip, etc). If it fails, STOP: the manager is the load-bearing module of this slice; fix the test or module before anything else.

- [ ] **Step 3: Commit**

```bash
git add package.json
git commit -m "test: register liveConversationTerminals suite as test:live-terminals"
```

---

### Task 4: `ownedSessions.ts` — identity, adoption, persistence, reconcile (pure module, TDD)

**Files:**
- Create: `src/lib/shell/ownedSessions.ts`
- Test: `scripts/ownedSessions.test.mjs`
- Modify: `package.json` (add `"test:owned-sessions": "node --experimental-strip-types scripts/ownedSessions.test.mjs"`)

**Interfaces:**
- Consumes: `AgentSession` from `$lib/tauriSource` (`{ provider, id, title, description?, model, projectPath, lastActivity, resumeCommands }`) and `TerminalSessionInfo` (with Task 1's `exited` field).
- Produces (exact — later tasks import these):

```ts
export type AgentKind = 'codex' | 'claude' | 'gemini' | 'opencode' | 'other';
export type OwnedSessionState = 'live' | 'background' | 'exited';
export type OwnedSession = {
  ownedId: string;
  agent: AgentKind;
  viaCmux: boolean;
  source: 'scanned' | 'fresh';
  title: string;
  projectPath: string | null;
  cwd: string;
  resumeCommand: string | null;
  nativeSessionId: string | null;
  ptySessionId: string | null;
  state: OwnedSessionState;
};
export function normalizeProvider(provider: string): { agent: AgentKind; viaCmux: boolean };
export function adoptAgentSession(record: AgentSession, mintId?: () => string): OwnedSession;
export function createFreshSession(opts: { cwd: string; title?: string }, mintId?: () => string): OwnedSession;
export function serializeOwnedSessions(sessions: OwnedSession[]): string;
export function parseStoredOwnedSessions(raw: string | null): OwnedSession[];
export function reconcileOwnedSessions(
  stored: OwnedSession[],
  live: Pick<TerminalSessionInfo, 'sessionId' | 'exited'>[]
): { owned: OwnedSession[]; reattachable: OwnedSession[] };
```

Behavior contract:
- `normalizeProvider`: `'codex'→{codex,false}`, `'claude'→{claude,false}`, `'cmux-claude'→{claude,true}`, `'cmux-gemini'→{gemini,true}`, unknown/`'cmux-'`-unknown → `{other, …}`. Case-insensitive, trimmed.
- `adoptAgentSession`: mints `ownedId` via `mintId` (default `crypto.randomUUID`), `source: 'scanned'`, `nativeSessionId = record.id`, `cwd = record.projectPath ?? ''`, `resumeCommand = record.resumeCommands[0] ?? null`, `state: 'background'`, `ptySessionId: null`.
- `createFreshSession`: `source: 'fresh'`, `agent: 'other'`, `viaCmux: false`, `resumeCommand: null`, `nativeSessionId: null`, `title` defaults to the last path segment of `cwd`.
- `parseStoredOwnedSessions`: tolerant — non-JSON/non-array → `[]`; entries missing `ownedId`/`cwd` (non-empty strings) are dropped; unknown `state` coerced to `'exited'`.
- `reconcileOwnedSessions` (runs at mount after webview reload): for each stored session — pty found live & not exited → `state:'background'`, goes in `reattachable`; pty found but exited → `state:'exited'` (tombstone, keep); pty missing (app restarted) → `state:'exited'`, `ptySessionId: null`. Never drops a session.

- [ ] **Step 1: Write the failing test** — `scripts/ownedSessions.test.mjs`

```js
import assert from 'node:assert/strict';
import {
  normalizeProvider, adoptAgentSession, createFreshSession,
  serializeOwnedSessions, parseStoredOwnedSessions, reconcileOwnedSessions,
} from '../src/lib/shell/ownedSessions.ts';

const mint = () => 'owned-1';
const scanRecord = {
  provider: 'cmux-claude', id: 'native-9', title: 'Fix rail', description: null,
  model: null, projectPath: '/tmp/proj', lastActivity: null,
  resumeCommands: ['claude --resume native-9'],
};

{ // normalizeProvider
  assert.deepEqual(normalizeProvider('codex'), { agent: 'codex', viaCmux: false });
  assert.deepEqual(normalizeProvider('CMUX-Claude '), { agent: 'claude', viaCmux: true });
  assert.deepEqual(normalizeProvider('cmux-rovo'), { agent: 'other', viaCmux: true });
  assert.deepEqual(normalizeProvider(''), { agent: 'other', viaCmux: false });
}
{ // adoptAgentSession
  const owned = adoptAgentSession(scanRecord, mint);
  assert.equal(owned.ownedId, 'owned-1');
  assert.equal(owned.agent, 'claude');
  assert.equal(owned.viaCmux, true);
  assert.equal(owned.source, 'scanned');
  assert.equal(owned.nativeSessionId, 'native-9');
  assert.equal(owned.cwd, '/tmp/proj');
  assert.equal(owned.resumeCommand, 'claude --resume native-9');
  assert.equal(owned.state, 'background');
  assert.equal(owned.ptySessionId, null);
}
{ // createFreshSession defaults
  const fresh = createFreshSession({ cwd: '/tmp/deep/proj' }, mint);
  assert.equal(fresh.title, 'proj');
  assert.equal(fresh.source, 'fresh');
  assert.equal(fresh.resumeCommand, null);
}
{ // persistence round-trip + tolerance
  const owned = adoptAgentSession(scanRecord, mint);
  const parsed = parseStoredOwnedSessions(serializeOwnedSessions([owned]));
  assert.deepEqual(parsed, [owned]);
  assert.deepEqual(parseStoredOwnedSessions('not json'), []);
  assert.deepEqual(parseStoredOwnedSessions('{"a":1}'), []);
  assert.deepEqual(parseStoredOwnedSessions(JSON.stringify([{ ownedId: '', cwd: '/x' }])), []);
  const weird = { ...owned, state: 'zombie' };
  assert.equal(parseStoredOwnedSessions(JSON.stringify([weird]))[0].state, 'exited');
}
{ // reconcile after reload
  const a = { ...adoptAgentSession(scanRecord, () => 'a'), ptySessionId: 'term-1' };
  const b = { ...adoptAgentSession(scanRecord, () => 'b'), ptySessionId: 'term-2' };
  const c = { ...adoptAgentSession(scanRecord, () => 'c'), ptySessionId: 'term-3' };
  const { owned, reattachable } = reconcileOwnedSessions([a, b, c], [
    { sessionId: 'term-1', exited: false },
    { sessionId: 'term-2', exited: true },
  ]);
  assert.equal(owned.length, 3);
  assert.equal(owned[0].state, 'background');
  assert.equal(owned[1].state, 'exited');
  assert.equal(owned[2].state, 'exited');
  assert.equal(owned[2].ptySessionId, null);
  assert.deepEqual(reattachable.map((s) => s.ownedId), ['a']);
}
console.log('ownedSessions tests passed');
```

- [ ] **Step 2: Run to verify it fails**

Run: `node --experimental-strip-types scripts/ownedSessions.test.mjs`
Expected: FAIL — cannot find module `../src/lib/shell/ownedSessions.ts`.

- [ ] **Step 3: Implement `src/lib/shell/ownedSessions.ts`**

Pure module — no Svelte, no DOM, no imports beyond types:

```ts
/**
 * Owned-session identity and lifecycle records for the /next shell.
 *
 * ownedId (CommandBar-minted) is the PRIMARY key across rail, manager,
 * persistence, and active-tracking. Provider ids are mutable (a resume mints a
 * new one), so they are carried but never used as keys.
 */
import type { AgentSession, TerminalSessionInfo } from '$lib/tauriSource';

export type AgentKind = 'codex' | 'claude' | 'gemini' | 'opencode' | 'other';
export type OwnedSessionState = 'live' | 'background' | 'exited';

export type OwnedSession = { /* exactly as the Interfaces block above */ };

const KNOWN_AGENTS: AgentKind[] = ['codex', 'claude', 'gemini', 'opencode'];

export function normalizeProvider(provider: string): { agent: AgentKind; viaCmux: boolean } {
  const value = (provider ?? '').trim().toLowerCase();
  const viaCmux = value.startsWith('cmux-');
  const base = viaCmux ? value.slice('cmux-'.length) : value;
  const agent = (KNOWN_AGENTS as string[]).includes(base) ? (base as AgentKind) : 'other';
  return { agent, viaCmux };
}
```

…and the remaining functions per the behavior contract. Note for `mintId` default: use `globalThis.crypto.randomUUID()` (available in Node ≥ 19 and the webview) so tests can inject a deterministic minter.

- [ ] **Step 4: Run tests until green, then the type gate**

Run: `node --experimental-strip-types scripts/ownedSessions.test.mjs` → PASS.
Run: `pnpm check` → no new errors.

- [ ] **Step 5: Register the script + commit**

```bash
# add "test:owned-sessions" to package.json scripts first
git add src/lib/shell/ownedSessions.ts scripts/ownedSessions.test.mjs package.json
git commit -m "feat(shell): owned-session identity, adoption, persistence, reconcile"
```

---

### Task 5: `xtermFactory.ts` — the TerminalView seam with WebGL acquire/release

**Files:**
- Create: `src/lib/shell/xtermFactory.ts`

**Interfaces:**
- Consumes: `TerminalView` type from `$lib/liveConversationTerminals` (`{ write, fit, focus, setVisible, dispose }`).
- Produces:

```ts
export type XtermModules = Awaited<ReturnType<typeof loadXtermModules>>;
export async function loadXtermModules(): Promise<{
  Terminal: typeof import('@xterm/xterm').Terminal;
  FitAddon: typeof import('@xterm/addon-fit').FitAddon;
  SerializeAddon: typeof import('@xterm/addon-serialize').SerializeAddon;
  WebglAddon: typeof import('@xterm/addon-webgl').WebglAddon | null; // null when load fails
}>;
export type ViewHooks = {
  onData(data: string): void;
  onResize(cols: number, rows: number): void;
};
export function makeTerminalView(modules: XtermModules, host: HTMLElement, hooks: ViewHooks): TerminalView;
```

Behavior contract:
- One dynamic-import pass (`loadXtermModules`) so `makeTerminalView` is fully synchronous — that is what `LiveConversationTerminalDeps.createTerminal: (host) => TerminalView` requires.
- xterm options: copy the old shell's proven config (old `+page.svelte:7765-7802`): `convertEol: true, cursorBlink: true, cursorStyle: 'block', allowProposedApi: true, macOptionIsMeta: true, scrollback: 8000`, fonts from `settings.terminal` (import `settings` from `$lib/settingsStore.svelte` — values only, read at creation), and the Dracula theme object copied verbatim from old `+page.svelte:7779-7801`.
- **WebGL is a per-view resource gated on visibility** (the ~16-contexts-per-page cap): `setVisible(true)` → try to attach a fresh `WebglAddon` (with `onContextLoss` → dispose + null the ref, silent canvas fallback); `setVisible(false)` → dispose the WebGL addon (canvas renderer persists) AND `host.style.display = 'none'`; visible → `display = ''` then `fit()`.
- `write` → `terminal.write(data)`; `fit` → `fitAddon.fit()` then `hooks.onResize(terminal.cols, terminal.rows)`; `focus` → `terminal.focus()`; `dispose` → input disposable + addons + `terminal.dispose()`.
- `terminal.onData(hooks.onData)` wired at creation.

- [ ] **Step 1: Implement the module** per the contract above (DOM-bound — no unit test; the gate is `pnpm check` now and the live protocol in Task 9).

- [ ] **Step 2: Type gate**

Run: `pnpm check`
Expected: no new errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/shell/xtermFactory.ts
git commit -m "feat(shell): xterm TerminalView factory with visibility-gated WebGL"
```

---

### Task 6: `terminalService.ts` — the one IO owner (TDD with fakes)

**Files:**
- Create: `src/lib/shell/terminalService.ts`
- Test: `scripts/nextTerminalService.test.mjs`
- Modify: `package.json` (add `"test:next-terminal-service": "node --experimental-strip-types scripts/nextTerminalService.test.mjs"`)

**Interfaces:**
- Consumes: `createLiveConversationTerminals` (real, from `$lib/liveConversationTerminals`), `OwnedSession` (Task 4), `TerminalStartRequest`/`TerminalSessionInfo`/`TerminalOutputPayload` (Tasks 1-2), view factory shape from Task 5.
- Produces (exact):

```ts
export type TerminalBackend = {
  start(request: TerminalStartRequest): Promise<TerminalSessionInfo | null>;
  write(sessionId: string, data: string): Promise<boolean>;
  resize(sessionId: string, cols: number, rows: number): Promise<boolean>;
  close(sessionId: string): Promise<boolean>;
  readScrollback(sessionId: string): Promise<string | null>;
  list(): Promise<TerminalSessionInfo[] | null>;
  listen(handler: (payload: TerminalOutputPayload) => void): Promise<(() => void) | null>;
};
export function tauriTerminalBackend(count: (command: string) => void): TerminalBackend;
export type TerminalService = {
  attach(): Promise<void>;
  startOwned(owned: OwnedSession, host: HTMLElement): Promise<string | null>;
  adoptExisting(owned: OwnedSession, host: HTMLElement): Promise<boolean>;
  show(ownedId: string): void;
  closeOwned(ownedId: string): Promise<void>;
  dispose(): void;
};
export function createTerminalService(opts: {
  backend: TerminalBackend;
  createView: (host: HTMLElement, hooks: { onData(d: string): void; onResize(c: number, r: number): void }) => TerminalView;
  onExit?(ownedId: string, payload: TerminalOutputPayload): void;
}): TerminalService;
```

Behavior contract (each numbered item gets a test):
1. `attach()` registers ONE backend listener. `terminated: false` payloads route via `manager.feedSession(sessionId, data)` — hidden views keep receiving (the manager already feeds regardless of visibility). `terminated: true` → `manager.markTerminated(key)` + `opts.onExit?.(key, payload)`.
2. `startOwned`: `backend.start({ cwd: owned.cwd, ownedId: owned.ownedId })` → `manager.ensureView(ownedId, { host })` → **`manager.bindSession(ownedId, info.sessionId)` immediately after start resolves** → if `owned.resumeCommand`, `backend.write(info.sessionId, owned.resumeCommand + '\r')`. Returns `info.sessionId` (null on start failure — no view left behind: guard with `hasView`/`closeView`).
3. `adoptExisting` (reload re-attach): `const scrollback = await backend.readScrollback(owned.ptySessionId)` → seed the scrollback cache → `manager.ensureView(ownedId, { host, sessionId: owned.ptySessionId })` (the manager's `readScrollback` dep reads the cache synchronously and hydrates) → `manager.bindSession(...)` → clear cache entry. Returns false (and creates nothing) if `ptySessionId` is null.
4. Input routing: the view created for owned session X writes to X's PTY — implemented by capturing the ownedId in a closure around `createView` at `ensureView` time and resolving `ownedId → ptySessionId` from a service-owned map. `onResize` similarly calls `backend.resize(ptyId, cols, rows)`.
5. `closeOwned`: `backend.close(ptyId)` (if any) + `manager.closeView(ownedId)` + map cleanup. **This is the ONLY path that closes a PTY.**
6. `dispose()`: unlisten + `manager.disposeAll()` — **never calls `backend.close`** (this is the no-kill-on-reload fix; the old shell's `disposeEmbeddedTerminal` killed the PTY at old `+page.svelte:8242`).
- The manager instance is created inside `createTerminalService` with deps `{ createTerminal: <closure wrapper>, writeSession: backend.write, resizeSession: <fit-triggered resize>, readScrollback: <cache lookup> }`.
- `tauriTerminalBackend` wraps the `$lib/tauriSource` functions 1:1 (`startTerminalSessionFromTauri`, `writeTerminalSessionFromTauri`, `resizeTerminalSessionFromTauri`, `closeTerminalSessionFromTauri`, `readTerminalSessionScrollbackFromTauri`, `listTerminalSessionsFromTauri`, `listenToTerminalOutput`), calling `count('<command name>')` before each call.

- [ ] **Step 1: Write the failing test** — `scripts/nextTerminalService.test.mjs`

Use the same stub-view pattern as `scripts/liveConversationTerminals.test.mjs`: a fake backend recording calls + resolving canned values, a `createView` stub returning `{ write/fit/focus/setVisible/dispose }` spies. Cover contract items 1-6:

```js
import assert from 'node:assert/strict';
import { createTerminalService } from '../src/lib/shell/terminalService.ts';

function makeView(log, name) {
  return {
    write: (d) => log.push([name, 'write', d]),
    fit: () => log.push([name, 'fit']),
    focus: () => log.push([name, 'focus']),
    setVisible: (v) => log.push([name, 'visible', v]),
    dispose: () => log.push([name, 'dispose']),
  };
}
function makeBackend(log) {
  let listener = null;
  return {
    backend: {
      start: async (req) => { log.push(['start', req]); return { sessionId: 'pty-1', cwd: req.cwd, shell: '/bin/zsh', cols: 96, rows: 28, pid: 1, startedAt: 0, exited: false, exitCode: null, signal: null }; },
      write: async (id, d) => { log.push(['write', id, d]); return true; },
      resize: async () => true,
      close: async (id) => { log.push(['close', id]); return true; },
      readScrollback: async () => 'OLD OUTPUT',
      list: async () => [],
      listen: async (h) => { listener = h; return () => { listener = null; }; },
    },
    emit: (p) => listener?.(p),
  };
}
const ownedA = { ownedId: 'a', agent: 'claude', viaCmux: false, source: 'scanned', title: 't', projectPath: '/p', cwd: '/p', resumeCommand: 'claude --resume n1', nativeSessionId: 'n1', ptySessionId: null, state: 'background' };

{ // start: ensure -> bind -> resume write; output routes to hidden views; exit fires onExit
  const log = []; const { backend, emit } = makeBackend(log);
  const exits = [];
  const svc = createTerminalService({
    backend,
    createView: (host, hooks) => makeView(log, 'viewA'),
    onExit: (ownedId, payload) => exits.push([ownedId, payload.exitCode]),
  });
  await svc.attach();
  const pty = await svc.startOwned(ownedA, {});
  assert.equal(pty, 'pty-1');
  assert.deepEqual(log.find((e) => e[0] === 'start')[1], { cwd: '/p', ownedId: 'a' });
  assert.deepEqual(log.find((e) => e[0] === 'write'), ['write', 'pty-1', 'claude --resume n1\r']);
  emit({ sessionId: 'pty-1', data: 'hello', terminated: false, exitCode: null, signal: null });
  assert.ok(log.some((e) => e[0] === 'viewA' && e[1] === 'write' && e[2] === 'hello'));
  emit({ sessionId: 'pty-1', data: '', terminated: true, exitCode: 0, signal: null });
  assert.deepEqual(exits, [['a', 0]]);
}
{ // adoptExisting hydrates scrollback; dispose never closes the PTY
  const log = []; const { backend } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: (h, hooks) => makeView(log, 'viewB') });
  await svc.attach();
  const ok = await svc.adoptExisting({ ...ownedA, ownedId: 'b', ptySessionId: 'pty-9' }, {});
  assert.equal(ok, true);
  assert.ok(log.some((e) => e[0] === 'viewB' && e[1] === 'write' && e[2] === 'OLD OUTPUT'));
  svc.dispose();
  assert.ok(!log.some((e) => e[0] === 'close'), 'dispose must not close PTYs');
}
{ // closeOwned is the only close path
  const log = []; const { backend } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: (h) => makeView(log, 'viewC') });
  await svc.attach();
  await svc.startOwned({ ...ownedA, ownedId: 'c', resumeCommand: null }, {});
  await svc.closeOwned('c');
  assert.ok(log.some((e) => e[0] === 'close' && e[1] === 'pty-1'));
}
console.log('terminalService tests passed');
```

(Wrap the file body in an async IIFE or use top-level await — match the existing test files' style.)

- [ ] **Step 2: Run to verify it fails**

Run: `node --experimental-strip-types scripts/nextTerminalService.test.mjs`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `src/lib/shell/terminalService.ts`** per the contract. Import `createLiveConversationTerminals` — the real manager, not a fake.

- [ ] **Step 4: Run until green + gates**

Run: `node --experimental-strip-types scripts/nextTerminalService.test.mjs` → PASS.
Run: `pnpm test:live-terminals` → still PASS. `pnpm check` → no new errors.

- [ ] **Step 5: Register script + commit**

```bash
git add src/lib/shell/terminalService.ts scripts/nextTerminalService.test.mjs package.json
git commit -m "feat(shell): terminal service — single listener, feedSession routing, no-kill-on-reload"
```

---

### Task 7: Rail store + dev invoke counter (rune modules)

**Files:**
- Create: `src/lib/shell/stores/sessionRailStore.svelte.ts`
- Create: `src/lib/shell/devInvokeCounter.svelte.ts`

**Interfaces:**
- Consumes: `OwnedSession`, `serializeOwnedSessions`, `parseStoredOwnedSessions` (Task 4); `AgentSession` type.
- Produces (exact):

```ts
// devInvokeCounter.svelte.ts
export const invokeCounts = $state<{ total: number; byCommand: Record<string, number> }>({ total: 0, byCommand: {} });
export function countInvoke(command: string): void;

// sessionRailStore.svelte.ts
export const OWNED_SESSIONS_STORAGE_KEY = 'mac-command-bar.next.owned-sessions';
export const rail = $state<{
  owned: OwnedSession[];
  available: AgentSession[];
  activeOwnedId: string | null;
  scanning: boolean;
  error: string | null;
}>({ owned: [], available: [], activeOwnedId: null, scanning: false, error: null });
export function loadStoredOwned(): OwnedSession[];          // localStorage read, tolerant
export function hydrateOwned(sessions: OwnedSession[]): void;
export function addOwnedSession(session: OwnedSession): void;     // + persist
export function updateOwnedSession(ownedId: string, patch: Partial<OwnedSession>): void; // + persist
export function removeOwnedSession(ownedId: string): void;        // + persist
export function setActiveOwned(ownedId: string | null): void;     // marks active 'live', previous 'background' (unless 'exited'); persists
export function setAvailable(sessions: AgentSession[]): void;
```

Rules: persistence is an explicit `localStorage.setItem` inside each mutator (swallow quota errors, matching `settingsStore` `persist()` style) — **no `$effect`** (illegal in `.svelte.ts` and against the constitution anyway). No IO besides localStorage. The store never calls the backend.

- [ ] **Step 1: Implement both modules** per the contract (follow `src/lib/stores/projectStore.svelte.ts` for file conventions).

- [ ] **Step 2: Type gate**

Run: `pnpm check` → no new errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/shell/stores/sessionRailStore.svelte.ts src/lib/shell/devInvokeCounter.svelte.ts
git commit -m "feat(shell): session rail store + dev invoke counter"
```

---

### Task 8: Components + `/next` route + Tauri dev config

**Files:**
- Create: `src/lib/shell/components/SessionRail.svelte`
- Create: `src/lib/shell/components/TerminalSurface.svelte`
- Create: `src/routes/next/+page.svelte`
- Create: `src-tauri/tauri.dev.next.conf.json`
- Modify: `package.json` (add `"tauri:dev:next"`)

**Interfaces:**
- Consumes: everything above. `SessionRail` may reuse `Chip` / `SearchInput` from `src/lib/components/` (both pass the autopsy audit: presentational, no IO) — but NOT `ConversationList.svelte` (tied to old-shell wiring; new rail is simpler).
- Produces: the running Slice 1 UI.

**SessionRail.svelte** — presentational only:

```ts
interface Props {
  owned: OwnedSession[];
  available: AgentSession[];
  activeOwnedId: string | null;
  scanning: boolean;
  onSelect(ownedId: string): void;
  onAdopt(session: AgentSession): void;
  onClose(ownedId: string): void;
  onRescan(): void;
}
```

Markup: two groups. **Owned** — one row per session: state dot (`live` = accent pulse, `background` = solid, `exited` = hollow + "finished" label), title, agent badge (`claude`, `codex`, `cmux · claude` when `viaCmux`), close ✕ (calls `onClose`; on an `exited` row the label is "dismiss"). **Resume** — scanned sessions (filter out ones already adopted: match on `provider:id` vs owned `nativeSessionId`), row click → `onAdopt`. A rescan button in the Resume header → `onRescan`. Keep styling minimal and dark — tokens come in a later lane; no design work here beyond legibility.

**TerminalSurface.svelte:**

```ts
interface Props {
  owned: OwnedSession[];
  activeOwnedId: string | null;
  registerHost(ownedId: string, host: HTMLElement): void;
}
```

Renders a stacked container: `{#each owned.filter(s => s.state !== 'exited' || s.ptySessionId) as s (s.ownedId)}` → one `<div class="term-host">` per session, `use:host={s.ownedId}` where the `host` action calls `registerHost(s.ownedId, node)` once on mount. Visibility itself is handled by the manager (`setVisible` toggles `display`), NOT by Svelte conditionals — **never `{#if active}` around a host** (that would destroy the xterm DOM on switch, which is the old bug in new clothes). Container is `position:relative; height:100%`, hosts `position:absolute; inset:0`.

**`src/routes/next/+page.svelte`** — the orchestrator, ≤ 300 lines, structure:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import SessionRail from '$lib/shell/components/SessionRail.svelte';
  import TerminalSurface from '$lib/shell/components/TerminalSurface.svelte';
  import { rail, loadStoredOwned, hydrateOwned, addOwnedSession, updateOwnedSession, removeOwnedSession, setActiveOwned, setAvailable } from '$lib/shell/stores/sessionRailStore.svelte';
  import { invokeCounts, countInvoke } from '$lib/shell/devInvokeCounter.svelte';
  import { createTerminalService, tauriTerminalBackend } from '$lib/shell/terminalService';
  import { loadXtermModules, makeTerminalView } from '$lib/shell/xtermFactory';
  import { adoptAgentSession, reconcileOwnedSessions, type OwnedSession } from '$lib/shell/ownedSessions';
  import { listAgentSessionsFromTauri, listAgentSessionsFromLocalBridge, type AgentSession } from '$lib/tauriSource';

  // hosts land before the service is ready; park them
  const pendingHosts = new Map<string, HTMLElement>();
  let service: ReturnType<typeof createTerminalService> | null = null;

  async function scanRail(): Promise<void> { /* explicit IO: countInvoke('list_agent_sessions'); tauri-first, bridge fallback; setAvailable */ }
  function registerHost(ownedId: string, host: HTMLElement): void { /* park or ensure via service */ }
  async function selectOwned(ownedId: string): Promise<void> { setActiveOwned(ownedId); service?.show(ownedId); }
  async function adopt(record: AgentSession): Promise<void> { /* adoptAgentSession -> addOwnedSession -> startOwned with parked host -> updateOwnedSession(ptySessionId) -> selectOwned */ }
  async function closeOwned(ownedId: string): Promise<void> { /* service.closeOwned -> removeOwnedSession */ }

  onMount(() => {
    let disposed = false;
    void (async () => {
      const backend = tauriTerminalBackend(countInvoke);
      const modules = await loadXtermModules();
      service = createTerminalService({
        backend,
        createView: (host, hooks) => makeTerminalView(modules, host, hooks),
        onExit: (ownedId, payload) => updateOwnedSession(ownedId, { state: 'exited' }),
      });
      await service.attach();
      // Rail hydration — the ONLY launch IO (constitution):
      const live = (await backend.list()) ?? [];
      const { owned, reattachable } = reconcileOwnedSessions(loadStoredOwned(), live);
      hydrateOwned(owned);
      for (const session of reattachable) { /* adoptExisting with parked host when it arrives */ }
      await scanRail();
    })();
    return () => { disposed = true; service?.dispose(); }; // dispose = views + listener only; PTYs SURVIVE
  });
</script>

<main class="next-shell">
  <aside><SessionRail owned={rail.owned} available={rail.available} activeOwnedId={rail.activeOwnedId} scanning={rail.scanning}
    onSelect={selectOwned} onAdopt={adopt} onClose={closeOwned} onRescan={scanRail} /></aside>
  <section><TerminalSurface owned={rail.owned} activeOwnedId={rail.activeOwnedId} {registerHost} /></section>
  {#if import.meta.env.DEV}<footer class="invoke-counter">invokes: {invokeCounts.total}</footer>{/if}
</main>
```

The `/* ... */` bodies above are structural placeholders for THIS plan block only — the implementer writes them out fully; each is ≤ 15 lines and fully specified by the Task 6/7 interfaces. Note the host-parking detail: `TerminalSurface` mounts hosts before `service` finishes async init, so `registerHost` parks elements in `pendingHosts` and the init path drains it.

**`src-tauri/tauri.dev.next.conf.json`** (same shape as `tauri.dev.attach.conf.json`, plus the window URL):

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "build": {
    "beforeDevCommand": "",
    "devUrl": "http://127.0.0.1:5177/next",
    "frontendDist": "../build"
  }
}
```

`package.json`: `"tauri:dev:next": "tauri dev --config src-tauri/tauri.dev.next.conf.json"` (run `pnpm dev` separately, like `tauri:dev:attach`).

- [ ] **Step 1: Implement the four files** per the contracts above.
- [ ] **Step 2: Gates**

Run: `pnpm check` → no new errors. Run: `pnpm test:tauri-config` → if it enumerates config files, extend it to cover the new conf; otherwise confirm PASS.

- [ ] **Step 3: Commit**

```bash
git add src/lib/shell/components/ src/routes/next/ src-tauri/tauri.dev.next.conf.json package.json
git commit -m "feat(shell): /next route — session rail + live terminal surface"
```

---

### Task 9: Live verification — the Slice 1 success criteria

**Files:** none created (fix-forward commits as needed).

This is the phase gate from the spec: *"start 2–3 agents inside the app, switch instantly, talk to each, no TUI corruption, hidden agents keep progressing, webview reload survives."*

- [ ] **Step 1: Launch**

```bash
cd tauri-svelte-preview
pnpm dev          # terminal A (leave running)
pnpm tauri:dev:next   # terminal B — window opens on /next
```

- [ ] **Step 2: Adopt + converse.** In the Resume group, adopt a scanned `claude` session for one project. Verify: PTY spawns in its cwd, the resume command is typed automatically, the agent TUI renders. Type a short prompt; confirm the reply renders cleanly.
- [ ] **Step 3: Second session + switch fidelity.** Adopt a second session (different project). Switch back and forth rapidly ×10. Verify: **instant** switches, no `reset()` flash, no dropped/duplicated TUI rows, cursor lands correctly, scroll position of each view preserved.
- [ ] **Step 4: Hidden progress.** In session A start something long (`ask the agent to run a build`, or just `sleep 5 && echo DONE-WHILE-HIDDEN`). Switch to B for the duration. Switch back: `DONE-WHILE-HIDDEN` must already be on screen (fed while hidden), NOT replayed on focus.
- [ ] **Step 5: Exit tombstone.** In a fresh session type `exit`. Verify the rail row flips to "finished", the final output stays readable (scrollback via tombstone), and dismissing it removes the row.
- [ ] **Step 6: Reload survival (the headline fix).** With both agents running, reload the webview (Cmd+R in the window). Verify: rail rehydrates from storage, both sessions re-attach with scrollback, the agents NEVER restarted (check: ask each "what was my last message?" — context intact; or verify PIDs unchanged via `pnpm` → `list_terminal_sessions`).
- [ ] **Step 7: Invoke-count sanity (perf gate).** Watch the dev footer: launch to interactive rail should cost a handful of invokes (list_terminal_sessions + list_agent_sessions + per-adopted-session calls). A switch must cost **zero** invokes (except one `resize`). If switching triggers scans/reads, that is a constitution violation — fix before closing the task.
- [ ] **Step 8: Record results + commit fixes** — append a short "verified live YYYY-MM-DD" note with observations to this plan file; commit any fix-forward changes with focused messages.

---

## Out of scope for this plan (explicitly)

Hibernate / idle-timer / LRU cap N (lifecycle beyond Live/Background/exited), workspace auto-load (files/git/browser on switch), LSP re-pointing, the Dockview skeleton (Slice 2), cold-resume across full app relaunch, start-fresh-session UI beyond the minimum, styling/tokens polish, deleting old scan-and-resume paths.

## Self-review notes

- Spec coverage: terminal.rs no-kill-on-reload → frontend `dispose()` semantics (Task 6) since research showed the backend never killed PTYs — the kill lived in old `disposeEmbeddedTerminal`; tombstones → Task 1; COMMANDBAR_SESSION_ID → Task 2; feedSession + bind-before-feed → Task 6; Owned/Resume rail + identity normalization → Tasks 4/7/8; WebGL release on hidden → Task 5; reload-survive → Tasks 6/8/9.
- Known accepted gap: a few ms of output between `start()` resolving and `bindSession()` can be dropped for a brand-new PTY (shell banner only; scrollback replay on adopt covers the reload path). Documented here deliberately — do not "fix" it with reset+replay, that is the old corruption bug.

---

## Verified live — 2026-07-28

All Slice 1 success criteria confirmed by the user in the running Tauri app (`/next` route):
adopt from rail (instant shells after zshrc fix), instant faithful switching (0 backend calls per
switch; focus-report keystrokes shown as a separate "input" bucket), hidden sessions keep
progressing, exit → finished tombstone with readable final output, webview reload re-attaches all
sessions correctly (views pre-sized to PTY geometry + one repaint nudge), 16MB scrollback with
20k-line views. Four fix rounds during verification: 73b3f58 (exit chain, switch cost, hidden-view
geometry), 4042a87 (subagent-transcript filtering, 3-tier titles, honest HUD labels), 3f3c94c
(input bucket, repaint nudge), e7a8479 (16MB scrollback, amortized trim, kill-then-remove close).
