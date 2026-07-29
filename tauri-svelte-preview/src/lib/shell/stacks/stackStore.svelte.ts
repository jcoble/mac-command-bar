/**
 * stackStore.svelte.ts — Svelte 5 runes state for the /next stack runner.
 *
 * A "stack" here is one command you keep re-running in a project: `pnpm dev`,
 * `docker compose up`, `dotnet watch`. The store holds
 *
 *  - the stacks you have saved, per project folder;
 *  - which terminal session each one was last started in — the tag that makes a
 *    session in the rail recognisable as a stack rather than an agent;
 *  - the processes that are listening on a port right now, as last read.
 *
 * THREE RULES this module exists to enforce (same shape as `browserStore` and
 * `contextStore`):
 *
 * 1. **No backend, ever.** The only IO here is localStorage. Every Tauri call
 *    lives in `stackService.ts` and lands here as a plain mutation.
 * 2. **No `$effect`.** It is illegal in a `.svelte.ts` module and against the
 *    shell rules, so saving is an explicit `persist…()` call at the end of every
 *    mutator that changes something worth keeping.
 * 3. **Nothing loads at launch.** The panel is mounted up front and parked
 *    off-screen; `hydrateStacks()` (storage only) and the service's `activate`
 *    are what wake it up when the user first opens the view.
 *
 * WHY THE SESSION TAG LIVES HERE. `OwnedSession` is shared with the whole shell
 * and this lane may not edit it, so "this session is stack X" is kept in this
 * module, keyed by `ownedId`, and saved under this module's own storage key. If
 * the shell later grows a `kind: 'agent' | 'stack'` field, `stackIdForOwnedId`
 * is the one lookup that has to move.
 *
 * WHY PROCESSES ARE MATCHED BY FOLDER. The backend can only see processes that
 * are LISTENING on a port, and it reports the folder each one was started in —
 * never which terminal spawned it. A stack's shell spawns children with their
 * own pids, so a pid comparison would find nothing. So a port counts as this
 * stack's when it was opened in the stack's folder (or a folder underneath it)
 * AND this stack has a terminal that is still running. Two stacks in the SAME
 * folder will therefore both show that folder's ports; the row says "in this
 * folder", not "definitely yours".
 *
 * Imports are type-only on purpose — the module has none at all — which is what
 * lets `scripts/stackStore.test.mjs` compile and exercise it in plain node.
 */

// ── Stored shapes ─────────────────────────────────────────────────────────────

/** A saved command: what to run, in which folder, under what name. */
export interface StackDefinition {
  /** Minted here; the key everything else points at. */
  id: string;
  /** What the user calls it — "Web", "API", "Database". */
  name: string;
  /** The command line, exactly as it would be typed into a terminal. */
  script: string;
  /** The project folder the command runs in. */
  cwd: string;
}

/** The terminal session a stack was last started in, and how it ended. */
export interface StackRunRecord {
  /** The shell's own session id — the key this record is stored under. */
  ownedId: string;
  /** Which saved stack this session was started for. */
  stackId: string;
  /** `Date.now()` when it was started; `0` when a saved value was unreadable. */
  startedAt: number;
  /** The terminal has ended. */
  exited: boolean;
  /** The exit code the terminal reported, when it reported one. */
  exitCode: number | null;
  /** The signal that killed it, when one did. */
  signal: string | null;
}

/** One listening process, in the shape this store needs it. */
export interface StackProcess {
  pid: number;
  port: number;
  command: string;
  cwd: string;
}

/** Where a stack stands right now. */
export type StackState = 'stopped' | 'starting' | 'running' | 'failed';

/** What `deriveStackState` works out about one stack. */
export interface StackStatus {
  state: StackState;
  /** Ports found in this stack's folder while it is running; empty otherwise. */
  ports: number[];
  /** The processes those ports belong to. */
  processes: StackProcess[];
  exitCode: number | null;
  signal: string | null;
}

/** A stack as the panel draws it: the saved command plus where it stands. */
export interface StackRow extends StackStatus {
  definition: StackDefinition;
  /** The terminal session it is running in, or `null` when it has none. */
  ownedId: string | null;
  /** The state as a sentence: "running on port 5173". */
  statusLabel: string;
}

// ── Storage ───────────────────────────────────────────────────────────────────

/** Where the saved stacks live. */
export const STACK_DEFINITIONS_STORAGE_KEY = 'mac-command-bar.next.stacks.definitions';

/** Where "this session is that stack" lives, so the tag survives a reload. */
export const STACK_RUNS_STORAGE_KEY = 'mac-command-bar.next.stacks.runs';

/** What the user is told when a saved stack could not be written down. */
export const STORAGE_WRITE_FAILED_MESSAGE =
  'This stack will not come back after a reload — browser storage is full';

function trimmedString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function finiteNumber(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

/** Anything that is not a plain, readable array comes back as one that is empty. */
function parseArray(raw: string | null | undefined): unknown[] {
  if (typeof raw !== 'string' || raw.trim() === '') return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

/**
 * Read saved stacks back. Tolerant on purpose: an unreadable value, a value
 * that is not a list, a row that is not an object, a row missing any of its
 * four fields, and a second row re-using an id are all simply dropped — a bad
 * saved value must never be able to stop the panel from opening.
 */
export function parseStackDefinitions(raw: string | null | undefined): StackDefinition[] {
  const seen = new Set<string>();
  const definitions: StackDefinition[] = [];
  for (const entry of parseArray(raw)) {
    if (typeof entry !== 'object' || entry === null) continue;
    const row = entry as Record<string, unknown>;
    const id = trimmedString(row.id);
    const name = trimmedString(row.name);
    const script = trimmedString(row.script);
    const cwd = trimmedString(row.cwd);
    if (!id || !name || !script || !cwd) continue;
    if (seen.has(id)) continue;
    seen.add(id);
    definitions.push({ id, name, script, cwd });
  }
  return definitions;
}

export function serializeStackDefinitions(definitions: StackDefinition[]): string {
  return JSON.stringify(
    definitions.map((definition) => ({
      id: definition.id,
      name: definition.name,
      script: definition.script,
      cwd: definition.cwd
    }))
  );
}

/**
 * Read the saved "which session is which stack" list back. Same tolerance as
 * above; a row keeping only its two ids is repaired rather than dropped, since
 * the ids are the part that matters.
 */
export function parseStackRuns(raw: string | null | undefined): StackRunRecord[] {
  const seen = new Set<string>();
  const runs: StackRunRecord[] = [];
  for (const entry of parseArray(raw)) {
    if (typeof entry !== 'object' || entry === null) continue;
    const row = entry as Record<string, unknown>;
    const ownedId = trimmedString(row.ownedId);
    const stackId = trimmedString(row.stackId);
    if (!ownedId || !stackId || seen.has(ownedId)) continue;
    seen.add(ownedId);
    runs.push({
      ownedId,
      stackId,
      startedAt: finiteNumber(row.startedAt),
      exited: row.exited === true,
      exitCode: typeof row.exitCode === 'number' && Number.isFinite(row.exitCode) ? row.exitCode : null,
      signal: trimmedString(row.signal) || null
    });
  }
  return runs;
}

export function serializeStackRuns(runs: StackRunRecord[]): string {
  return JSON.stringify(
    runs.map((run) => ({
      ownedId: run.ownedId,
      stackId: run.stackId,
      startedAt: run.startedAt,
      exited: run.exited,
      exitCode: run.exitCode,
      signal: run.signal
    }))
  );
}

// ── Derivation (pure) ─────────────────────────────────────────────────────────

/**
 * The listening processes that belong to a stack's folder — the folder itself
 * or anything underneath it. A folder whose name merely STARTS the same way
 * (`/a/ap` against `/a/app`) is a different folder, hence the separator check.
 */
export function stackProcessesFor(
  definition: { cwd: string },
  processes: StackProcess[]
): StackProcess[] {
  const root = definition.cwd.replace(/\/+$/, '');
  if (!root) return [];
  return processes.filter((process) => {
    const cwd = (process.cwd ?? '').replace(/\/+$/, '');
    return cwd === root || cwd.startsWith(`${root}/`);
  });
}

/**
 * Where one stack stands, from its last run record and the processes listening
 * right now. The four answers:
 *
 *  - **stopped** — it has never been started here, or its terminal ended and
 *    said nothing was wrong;
 *  - **failed** — its terminal ended with a non-zero code, or a signal killed it;
 *  - **running** — its terminal is still going AND something in its folder is
 *    listening on a port;
 *  - **starting** — its terminal is still going but no port has appeared yet.
 *
 * "Starting" is honest rather than optimistic: a stack that never opens a port
 * (a test watcher, a build) stays there for as long as it runs, and the sentence
 * the panel shows says exactly that.
 */
export function deriveStackState(
  definition: { cwd: string },
  run: StackRunRecord | null | undefined,
  processes: StackProcess[]
): StackStatus {
  if (!run) {
    return { state: 'stopped', ports: [], processes: [], exitCode: null, signal: null };
  }
  if (run.exited) {
    const failed = (run.exitCode !== null && run.exitCode !== 0) || Boolean(run.signal);
    return {
      state: failed ? 'failed' : 'stopped',
      ports: [],
      processes: [],
      exitCode: run.exitCode,
      signal: run.signal
    };
  }
  const matched = stackProcessesFor(definition, processes);
  const ports = [...new Set(matched.map((process) => process.port).filter((port) => port > 0))].sort(
    (left, right) => left - right
  );
  return {
    state: ports.length > 0 ? 'running' : 'starting',
    ports,
    processes: matched,
    exitCode: null,
    signal: null
  };
}

/** English list: `a`, `a and b`, `a, b and c`. */
function joinWords(words: string[]): string {
  if (words.length === 0) return '';
  if (words.length === 1) return words[0];
  return `${words.slice(0, -1).join(', ')} and ${words[words.length - 1]}`;
}

/** One sentence for a stack's state, in words that need no explaining. */
export function describeStackState(status: {
  state: StackState;
  ports: number[];
  exitCode: number | null;
  signal: string | null;
}): string {
  if (status.state === 'running') {
    const ports = status.ports.map((port) => String(port));
    return `running on ${ports.length === 1 ? 'port' : 'ports'} ${joinWords(ports)}`;
  }
  if (status.state === 'starting') return 'started, no port yet';
  if (status.state === 'failed') {
    if (status.signal) return `stopped by ${status.signal}`;
    return `stopped with error code ${status.exitCode ?? 'unknown'}`;
  }
  return 'not running';
}

/** Every saved stack, with where it stands, in the order they were added. */
export function buildStackRows(
  definitions: StackDefinition[],
  runsByOwnedId: Record<string, StackRunRecord>,
  processes: StackProcess[]
): StackRow[] {
  const runByStack = new Map<string, StackRunRecord>();
  for (const run of Object.values(runsByOwnedId)) runByStack.set(run.stackId, run);
  return definitions.map((definition) => {
    const run = runByStack.get(definition.id) ?? null;
    const status = deriveStackState(definition, run, processes);
    return {
      ...status,
      definition,
      ownedId: run ? run.ownedId : null,
      statusLabel: describeStackState(status)
    };
  });
}

// ── Reactive state ────────────────────────────────────────────────────────────

/**
 * The single reactive stack-runner state. Read fields directly in the component
 * (`stacks.definitions`, `stacks.processes`); mutate only through the functions
 * below, so saving stays in lockstep.
 */
export const stacks = $state<{
  /** Saved stacks have been read out of storage at least once. */
  hydrated: boolean;
  /** The panel has been shown at least once and its first read has started. */
  activated: boolean;
  /** The project folder the panel is showing stacks for; `null` for all of them. */
  activeRoot: string | null;
  /** Human name of that folder, for the empty-state sentences. */
  projectName: string;
  definitions: StackDefinition[];
  /** Run records by `ownedId` — the session-to-stack tag. */
  runs: Record<string, StackRunRecord>;
  /** Processes listening on a port, as last read. */
  processes: StackProcess[];
  /** A read of the listening processes is in flight. */
  loading: boolean;
  /** Why the last read failed, in plain English; `null` when it did not. */
  error: string | null;
  /** Why there is nothing to read through nobody's fault (not the desktop app). */
  unavailableReason: string | null;
  /** Monotonic ticket of the most recent read. */
  requestId: number;
  /** `Date.now()` of the last successful read, or `null`. */
  loadedAt: number | null;
  /** A one-off message for the user (a refused add, a failed save). */
  notice: string | null;
}>({
  hydrated: false,
  activated: false,
  activeRoot: null,
  projectName: '',
  definitions: [],
  runs: {},
  processes: [],
  loading: false,
  error: null,
  unavailableReason: null,
  requestId: 0,
  loadedAt: null,
  notice: null
});

// ── Saving ────────────────────────────────────────────────────────────────────

function writeStorage(key: string, value: string): boolean {
  if (typeof localStorage === 'undefined') return false;
  try {
    localStorage.setItem(key, value);
    return true;
  } catch {
    stacks.notice = STORAGE_WRITE_FAILED_MESSAGE;
    return false;
  }
}

function readStorage(key: string): string | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** Save the saved-stacks list. */
function persistDefinitions(): boolean {
  return writeStorage(STACK_DEFINITIONS_STORAGE_KEY, serializeStackDefinitions(stacks.definitions));
}

/** Save the session-to-stack tags. */
function persistRuns(): boolean {
  return writeStorage(STACK_RUNS_STORAGE_KEY, serializeStackRuns(Object.values(stacks.runs)));
}

/**
 * Read the saved stacks and session tags back. Idempotent — the service calls it
 * every time the panel is opened, and only the first call does any work.
 *
 * A tag pointing at a stack that no longer exists is dropped as it is read: the
 * stack it named is gone, so the session it named is just a terminal now.
 */
export function hydrateStacks(): void {
  if (stacks.hydrated) return;
  stacks.hydrated = true;
  stacks.definitions = parseStackDefinitions(readStorage(STACK_DEFINITIONS_STORAGE_KEY));
  const known = new Set(stacks.definitions.map((definition) => definition.id));
  const runs: Record<string, StackRunRecord> = {};
  for (const run of parseStackRuns(readStorage(STACK_RUNS_STORAGE_KEY))) {
    if (!known.has(run.stackId)) continue;
    runs[run.ownedId] = run;
  }
  stacks.runs = runs;
}

// ── Mutations ─────────────────────────────────────────────────────────────────

/** Record that the panel has been shown and its first read has begun. */
export function markStacksActivated(): void {
  stacks.activated = true;
}

/** Store which project the panel is showing. Pure bookkeeping — reads nothing. */
export function setStackInput(input: { activeRoot: string | null; projectName?: string }): void {
  stacks.activeRoot = input.activeRoot;
  stacks.projectName = (input.projectName ?? '').trim() || folderName(input.activeRoot);
}

/** Last folder name of a path — `/a/b/my-app` → `my-app`. */
export function folderName(path: string | null | undefined): string {
  if (!path) return '';
  const parts = path.split('/').filter(Boolean);
  return parts[parts.length - 1] ?? '';
}

/** Take a ticket for a read of the listening processes. */
export function beginStackLoad(): number {
  stacks.requestId += 1;
  stacks.loading = true;
  stacks.error = null;
  stacks.unavailableReason = null;
  return stacks.requestId;
}

/** Land a successful read. Returns `false` when a newer read superseded it. */
export function applyStackProcesses(requestId: number, processes: StackProcess[]): boolean {
  if (stacks.requestId !== requestId) return false;
  stacks.processes = processes;
  stacks.loading = false;
  stacks.error = null;
  stacks.unavailableReason = null;
  stacks.loadedAt = Date.now();
  return true;
}

/** Land a failed read. Returns `false` when a newer read superseded it. */
export function failStackLoad(requestId: number, message: string): boolean {
  if (stacks.requestId !== requestId) return false;
  stacks.loading = false;
  stacks.error = message;
  return true;
}

/** Land a read that could not run at all — desktop-app-only data, say. */
export function markStacksUnavailable(requestId: number, reason: string): boolean {
  if (stacks.requestId !== requestId) return false;
  stacks.processes = [];
  stacks.loading = false;
  stacks.error = null;
  stacks.unavailableReason = reason;
  return true;
}

function mintStackId(): string {
  const random = globalThis.crypto?.randomUUID?.();
  return random ? `stack-${random}` : `stack-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

/**
 * Save a new stack. Returns the saved record, or `null` when a field was blank —
 * in which case `stacks.notice` says which one, because a button that silently
 * does nothing is worse than one that explains itself.
 */
export function addStack(input: { name: string; script: string; cwd: string }): StackDefinition | null {
  const name = input.name.trim();
  const script = input.script.trim();
  const cwd = input.cwd.trim();
  if (!name) {
    stacks.notice = 'Give the stack a name first.';
    return null;
  }
  if (!script) {
    stacks.notice = 'Type the command this stack should run.';
    return null;
  }
  if (!cwd) {
    stacks.notice = 'Pick a session first, so the stack knows which folder to run in.';
    return null;
  }
  const definition: StackDefinition = { id: mintStackId(), name, script, cwd };
  stacks.definitions = [...stacks.definitions, definition];
  stacks.notice = null;
  persistDefinitions();
  return definition;
}

/**
 * Forget a saved stack, and the tag saying which session belonged to it. The
 * SESSION survives untouched — this only removes CommandBar's saved command.
 */
export function removeStack(stackId: string): void {
  stacks.definitions = stacks.definitions.filter((definition) => definition.id !== stackId);
  const runs: Record<string, StackRunRecord> = {};
  for (const run of Object.values(stacks.runs)) {
    if (run.stackId === stackId) continue;
    runs[run.ownedId] = run;
  }
  stacks.runs = runs;
  persistDefinitions();
  persistRuns();
}

/**
 * Tag a terminal session as this stack's. A stack points at ONE session at a
 * time, so starting it again drops the tag on the previous session — that
 * terminal may still be on screen, it just is not this stack's any more.
 */
export function recordStackStart(stackId: string, ownedId: string): void {
  const runs: Record<string, StackRunRecord> = {};
  for (const run of Object.values(stacks.runs)) {
    if (run.stackId === stackId) continue;
    runs[run.ownedId] = run;
  }
  runs[ownedId] = {
    ownedId,
    stackId,
    startedAt: Date.now(),
    exited: false,
    exitCode: null,
    signal: null
  };
  stacks.runs = runs;
  persistRuns();
}

/**
 * Write down that a terminal ended. Returns `true` when that terminal was a
 * stack's — which is how the service knows whether the exit is worth a refresh —
 * and `false` for every other session in the shell.
 */
export function recordStackExit(
  ownedId: string,
  outcome: { exitCode: number | null; signal: string | null }
): boolean {
  const run = stacks.runs[ownedId];
  if (!run) return false;
  stacks.runs = {
    ...stacks.runs,
    [ownedId]: { ...run, exited: true, exitCode: outcome.exitCode, signal: outcome.signal }
  };
  persistRuns();
  return true;
}

/** Drop a session's tag — used when the session itself is removed. */
export function forgetStackRun(ownedId: string): void {
  if (!stacks.runs[ownedId]) return;
  const runs = { ...stacks.runs };
  delete runs[ownedId];
  stacks.runs = runs;
  persistRuns();
}

/** Which stack a session belongs to, or `null` when it is not a stack's. */
export function stackIdForOwnedId(ownedId: string): string | null {
  return stacks.runs[ownedId]?.stackId ?? null;
}

/** Is this session a stack's? The badge in the session rail asks this. */
export function isStackSession(ownedId: string): boolean {
  return stackIdForOwnedId(ownedId) !== null;
}

/** The session a stack is running in, or `null` when it has none. */
export function ownedIdForStack(stackId: string): string | null {
  for (const run of Object.values(stacks.runs)) {
    if (run.stackId === stackId) return run.ownedId;
  }
  return null;
}

/** The name of the stack a session belongs to, for the rail's badge tooltip. */
export function stackNameForOwnedId(ownedId: string): string | null {
  const stackId = stackIdForOwnedId(ownedId);
  if (!stackId) return null;
  return stacks.definitions.find((definition) => definition.id === stackId)?.name ?? null;
}

/** Clear the one-off message. */
export function clearStackNotice(): void {
  stacks.notice = null;
}

/** The stacks the panel should draw: this project's, or all of them. */
export function visibleStackRows(): StackRow[] {
  const root = (stacks.activeRoot ?? '').trim();
  const definitions = root
    ? stacks.definitions.filter((definition) => definition.cwd === root)
    : stacks.definitions;
  return buildStackRows(definitions, stacks.runs, stacks.processes);
}

/** Drop everything back to launch state. Used by tests and by a full reset. */
export function resetStacks(): void {
  stacks.hydrated = false;
  stacks.activated = false;
  stacks.activeRoot = null;
  stacks.projectName = '';
  stacks.definitions = [];
  stacks.runs = {};
  stacks.processes = [];
  stacks.loading = false;
  stacks.error = null;
  stacks.unavailableReason = null;
  stacks.requestId = 0;
  stacks.loadedAt = null;
  stacks.notice = null;
}
