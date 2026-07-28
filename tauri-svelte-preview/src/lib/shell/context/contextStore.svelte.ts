/**
 * contextStore.svelte.ts — Svelte 5 runes state for the /next context cards.
 *
 * Holds the STATE of the five cards the context region shows — runs, runtime
 * processes, agent sessions, worktrees and repository summaries — plus the
 * input they are loaded for (the project list and the active project root).
 *
 * Two rules this module exists to enforce (same shape as
 * `stores/sessionRailStore.svelte.ts`):
 *
 * 1. **No backend, ever.** Nothing here calls Tauri, fetches, or touches
 *    localStorage. Every backend call lives in `contextService.ts` and lands
 *    here as a plain mutation.
 * 2. **No `$effect`.** `$effect` is illegal in a `.svelte.ts` module and against
 *    the constitution. Loading is driven imperatively by the service.
 *
 * Each card carries a monotonic `requestId`. A loader takes a ticket from
 * `beginCardLoad` and hands it back when it finishes; a result whose ticket is
 * no longer the current one is dropped, so a slow first scan can never overwrite
 * a fast second one (the superseded-request guard the old shell used at
 * `src/routes/+page.svelte:3776-3801`).
 *
 * Imports are type-only on purpose: the module stays runnable on its own, which
 * is what `scripts/contextStore.test.mjs` compiles and exercises.
 */
import type {
  AgentSession,
  GitRepositorySummary,
  OrchestrationRun,
  ProjectWorktree,
  RuntimeContext,
  RuntimeContextProject
} from '../../tauriSource.ts';

/** The five cards, by name. */
export type ContextCardKey = 'runs' | 'runtime' | 'agents' | 'worktrees' | 'repositories';

/** Every card key, in the order the panel renders them. */
export const contextCardKeys: ContextCardKey[] = [
  'runs',
  'runtime',
  'agents',
  'worktrees',
  'repositories'
];

/** One card's state. `rows` is whatever the backend last returned for it. */
export interface ContextCard<Row> {
  /** The rows to render. Replaced wholesale by each successful load. */
  rows: Row[];
  /** A load is in flight. */
  loading: boolean;
  /** Why the last load failed, in plain English. `null` when it did not. */
  error: string | null;
  /**
   * Why there is nothing to show through no fault of anyone — for example the
   * data only exists in the desktop app, or no project is selected yet.
   * Non-null means "do not show an empty list, show this sentence".
   */
  unavailableReason: string | null;
  /** Monotonic ticket of the most recent load request. */
  requestId: number;
  /** `Date.now()` of the last successful load, or `null`. */
  loadedAt: number | null;
}

/** Any row any card can hold — used by the key-generic mutators below. */
export type ContextRow =
  | OrchestrationRun
  | RuntimeContext
  | AgentSession
  | ProjectWorktree
  | GitRepositorySummary;

/** What the panel is loaded FOR: the projects to scan and the active project. */
export interface ContextInput {
  /** Projects handed in by the shell; the scans that take a list use this. */
  projects: RuntimeContextProject[];
  /** The active session's project root — worktrees are read for this one. */
  activeRoot: string | null;
  /** Human name of the active project, used in empty-state sentences. */
  projectName?: string;
}

function emptyCard<Row>(): ContextCard<Row> {
  return {
    rows: [] as Row[],
    loading: false,
    error: null,
    unavailableReason: null,
    requestId: 0,
    loadedAt: null
  };
}

/**
 * The single reactive context state. Read fields directly in components
 * (`contextState.runs.rows`); mutate only through the functions below.
 */
export const contextState = $state<{
  /** The panel has been shown at least once and its first load has started. */
  activated: boolean;
  projects: RuntimeContextProject[];
  activeRoot: string | null;
  projectName: string;
  runs: ContextCard<OrchestrationRun>;
  runtime: ContextCard<RuntimeContext>;
  agents: ContextCard<AgentSession>;
  worktrees: ContextCard<ProjectWorktree>;
  repositories: ContextCard<GitRepositorySummary>;
}>({
  activated: false,
  projects: [],
  activeRoot: null,
  projectName: '',
  runs: emptyCard<OrchestrationRun>(),
  runtime: emptyCard<RuntimeContext>(),
  agents: emptyCard<AgentSession>(),
  worktrees: emptyCard<ProjectWorktree>(),
  repositories: emptyCard<GitRepositorySummary>()
});

function cardFor(key: ContextCardKey): ContextCard<ContextRow> {
  return contextState[key] as ContextCard<ContextRow>;
}

// ── Mutations ─────────────────────────────────────────────────────────────────

/** Record that the panel has been shown and its first load has begun. */
export function markContextActivated(): void {
  contextState.activated = true;
}

/**
 * Store what the cards are loaded for. Pure bookkeeping — it starts no load.
 * `projectName` falls back to the last folder name of the active root so the
 * empty states can still say which project they looked in.
 */
export function setContextInput(input: ContextInput): void {
  contextState.projects = input.projects;
  contextState.activeRoot = input.activeRoot;
  contextState.projectName = input.projectName?.trim() || folderName(input.activeRoot);
}

/**
 * A short, stable description of an input, so the service can tell whether the
 * shell handed it something new (a different project was selected) or the same
 * thing again (a repeated `activate` call, which must cost nothing).
 */
export function describeContextInput(input: ContextInput): string {
  const projects = input.projects.map((project) => `${project.id}@${project.path}`).join(',');
  return `${input.activeRoot ?? ''}|${projects}`;
}

/**
 * Take a ticket for a new load of `key`: marks the card loading and clears the
 * last error. Pass the ticket back to `applyCardRows` / `failCardLoad` /
 * `markCardUnavailable` — a stale ticket is ignored.
 */
export function beginCardLoad(key: ContextCardKey): number {
  const card = cardFor(key);
  card.requestId += 1;
  card.loading = true;
  card.error = null;
  card.unavailableReason = null;
  return card.requestId;
}

/** `true` while `requestId` is still the newest ticket handed out for `key`. */
export function isCurrentCardRequest(key: ContextCardKey, requestId: number): boolean {
  return cardFor(key).requestId === requestId;
}

/** Land a successful load. Returns `false` when a newer load superseded it. */
export function applyCardRows(
  key: ContextCardKey,
  requestId: number,
  rows: ContextRow[]
): boolean {
  const card = cardFor(key);
  if (card.requestId !== requestId) return false;
  card.rows = rows;
  card.loading = false;
  card.error = null;
  card.unavailableReason = null;
  card.loadedAt = Date.now();
  return true;
}

/** Land a failed load. Returns `false` when a newer load superseded it. */
export function failCardLoad(key: ContextCardKey, requestId: number, message: string): boolean {
  const card = cardFor(key);
  if (card.requestId !== requestId) return false;
  card.loading = false;
  card.error = message;
  return true;
}

/**
 * Land a load that could not run at all — there is nothing wrong, the data just
 * is not reachable here (desktop-app-only data, or no project selected yet).
 * Returns `false` when a newer load superseded it.
 */
export function markCardUnavailable(
  key: ContextCardKey,
  requestId: number,
  reason: string
): boolean {
  const card = cardFor(key);
  if (card.requestId !== requestId) return false;
  card.rows = [];
  card.loading = false;
  card.error = null;
  card.unavailableReason = reason;
  return true;
}

/** Drop everything back to launch state. Used by tests and by a full reset. */
export function resetContext(): void {
  contextState.activated = false;
  contextState.projects = [];
  contextState.activeRoot = null;
  contextState.projectName = '';
  contextState.runs = emptyCard<OrchestrationRun>();
  contextState.runtime = emptyCard<RuntimeContext>();
  contextState.agents = emptyCard<AgentSession>();
  contextState.worktrees = emptyCard<ProjectWorktree>();
  contextState.repositories = emptyCard<GitRepositorySummary>();
}

// ── Plain-English summaries ───────────────────────────────────────────────────
//
// One sentence per card, shown above its rows. They take the smallest shape
// they can read so they stay easy to test and tolerate raw backend records.

/** Last folder name of a path — `/a/b/my-app` → `my-app`. */
export function folderName(path: string | null | undefined): string {
  if (!path) return '';
  const parts = path.split('/').filter(Boolean);
  return parts[parts.length - 1] ?? '';
}

/** English list: `a`, `a and b`, `a, b and c`. */
function joinWords(words: string[]): string {
  if (words.length === 0) return '';
  if (words.length === 1) return words[0];
  return `${words.slice(0, -1).join(', ')} and ${words[words.length - 1]}`;
}

/** `1 run` / `4 runs` style counting, so no sentence says "1 runs". */
function count(value: number, singular: string, plural = `${singular}s`): string {
  return `${value} ${value === 1 ? singular : plural}`;
}

/** How a run's status reads: running, needs attention, failed, finished, other. */
export function runStatusGroup(
  status: string
): 'running' | 'attention' | 'failed' | 'finished' | 'other' {
  const value = (status ?? '').toLowerCase();
  if (value.includes('fail') || value.includes('error') || value.includes('block')) return 'failed';
  if (
    value.includes('wait') ||
    value.includes('review') ||
    value.includes('attention') ||
    value.includes('approval')
  )
    return 'attention';
  if (
    value.includes('run') ||
    value.includes('active') ||
    value.includes('progress') ||
    value.includes('live')
  )
    return 'running';
  if (
    value.includes('done') ||
    value.includes('complete') ||
    value.includes('success') ||
    value.includes('merged')
  )
    return 'finished';
  return 'other';
}

/** "2 running, 1 waiting on you" — empty string when there is nothing to add. */
export function summarizeRuns(runs: { status: string }[]): string {
  if (runs.length === 0) return '';
  let running = 0;
  let attention = 0;
  let failed = 0;
  for (const run of runs) {
    const group = runStatusGroup(run.status);
    if (group === 'running') running += 1;
    else if (group === 'attention') attention += 1;
    else if (group === 'failed') failed += 1;
  }
  const parts: string[] = [];
  if (running > 0) parts.push(`${running} running`);
  if (attention > 0) parts.push(`${attention} waiting on you`);
  if (failed > 0) parts.push(`${failed} failed`);
  if (parts.length === 0) return `${count(runs.length, 'run')}, none running`;
  return joinWords(parts);
}

/** "3 processes on ports 3000, 5177 and 8080". */
export function summarizeRuntime(contexts: { port: number | string }[]): string {
  if (contexts.length === 0) return '';
  const ports = [...new Set(contexts.map((context) => String(context.port)))].sort(
    (left, right) => Number(left) - Number(right)
  );
  const shown = ports.slice(0, 4);
  const tail = ports.length > shown.length ? ` and ${ports.length - shown.length} more` : '';
  return `${count(contexts.length, 'process', 'processes')} on ${
    shown.length === 1 ? 'port' : 'ports'
  } ${shown.join(', ')}${tail}`;
}

/** "12 sessions from claude and codex". */
export function summarizeAgents(agents: { provider?: string }[]): string {
  if (agents.length === 0) return '';
  const providers = [
    ...new Set(agents.map((agent) => (agent.provider ?? '').trim()).filter(Boolean))
  ].sort();
  const label = count(agents.length, 'session');
  if (providers.length === 0) return label;
  const shown = providers.slice(0, 3);
  const tail = providers.length > shown.length ? ` and ${providers.length - shown.length} more` : '';
  return `${label} from ${joinWords(shown)}${tail}`;
}

/** "4 worktrees, 2 with uncommitted changes". */
export function summarizeWorktrees(worktrees: { isDirty?: boolean }[]): string {
  if (worktrees.length === 0) return '';
  const dirty = worktrees.filter((worktree) => worktree.isDirty).length;
  const label = count(worktrees.length, 'worktree');
  return dirty === 0 ? `${label}, all clean` : `${label}, ${dirty} with uncommitted changes`;
}

/** "5 repositories, 2 with uncommitted changes, 1 ahead of its remote". */
export function summarizeRepositories(
  repos: { isDirty?: boolean; ahead?: number; behind?: number; error?: string | null }[]
): string {
  if (repos.length === 0) return '';
  const dirty = repos.filter((repo) => repo.isDirty).length;
  const ahead = repos.filter((repo) => (repo.ahead ?? 0) > 0).length;
  const behind = repos.filter((repo) => (repo.behind ?? 0) > 0).length;
  const failed = repos.filter((repo) => repo.error).length;
  const parts = [count(repos.length, 'repository', 'repositories')];
  if (dirty > 0) parts.push(`${dirty} with uncommitted changes`);
  if (ahead > 0) parts.push(`${ahead} ahead of its remote`);
  if (behind > 0) parts.push(`${behind} behind its remote`);
  if (failed > 0) parts.push(`${failed} could not be read`);
  if (parts.length === 1 && dirty === 0) parts.push('all clean and in sync');
  return parts.join(', ');
}
