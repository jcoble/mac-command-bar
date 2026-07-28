/**
 * contextService.ts — the ONLY place the context cards talk to the backend.
 *
 * Five independent loaders (runs, runtime processes, agent sessions, worktrees,
 * repository summaries), each imperative: nothing here runs from an `$effect`,
 * nothing runs at import, and nothing polls. Loads happen exactly twice:
 *
 *  - the shell calls `activate(input)` when the context region is first shown
 *    (and again whenever the selected project changes — only the cards that
 *    depend on what changed reload);
 *  - the user presses Refresh in the panel, which calls `refreshAll()` or
 *    `refreshCard(key)`.
 *
 * Every backend call is counted with `countInvoke('<command name>')`
 * immediately before it, so the dev invoke counter stays honest.
 *
 * A `null` result from a `…FromTauri` wrapper means "not running in the desktop
 * app" — nothing was invoked. Those cards say so; they never show made-up data.
 */
import { countInvoke } from '../devInvokeCounter.svelte.ts';
import {
  listAgentSessionsFromLocalBridge,
  listAgentSessionsFromTauri,
  listGitRepositorySummariesFromTauri,
  listOrchestrationRunsFromTauri,
  listProjectWorktreesFromTauri,
  listRuntimeContextsFromTauri,
  type AgentSession
} from '../../tauriSource.ts';
import {
  applyCardRows,
  beginCardLoad,
  contextState,
  describeContextInput,
  failCardLoad,
  markCardUnavailable,
  markContextActivated,
  setContextInput,
  type ContextCardKey,
  type ContextInput
} from './contextStore.svelte.ts';

export type { ContextInput } from './contextStore.svelte.ts';

/** Shown when a card's data only exists inside the desktop app. */
const DESKTOP_ONLY = 'This runs in the desktop app only.';

/** What the last `activate` was given, so a repeat call costs nothing. */
let loadedInputKey: string | null = null;

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Show the context cards and load them.
 *
 * Idempotent: calling it again with the same projects and active project does
 * no work at all. Calling it with a different selection reloads only the cards
 * that read the part which changed — the worktree card follows the active
 * project, the other four follow the project list.
 */
export function activate(input: ContextInput): void {
  const key = describeContextInput(input);
  const firstTime = !contextState.activated;
  const previousRoot = contextState.activeRoot;
  const previousProjects = contextState.projects
    .map((project) => `${project.id}@${project.path}`)
    .join(',');
  const nextProjects = input.projects.map((project) => `${project.id}@${project.path}`).join(',');

  setContextInput(input);
  markContextActivated();
  if (!firstTime && key === loadedInputKey) return;
  loadedInputKey = key;

  if (firstTime) {
    void refreshAll();
    return;
  }
  if (nextProjects !== previousProjects) {
    void loadRuns();
    void loadRuntime();
    void loadAgents();
    void loadRepositories();
  }
  if (input.activeRoot !== previousRoot) {
    void loadWorktrees();
  }
}

/** Reload every card. The panel's Refresh button. */
export async function refreshAll(): Promise<void> {
  await Promise.all([
    loadRuns(),
    loadRuntime(),
    loadAgents(),
    loadWorktrees(),
    loadRepositories()
  ]);
}

/** Reload one card. */
export async function refreshCard(key: ContextCardKey): Promise<void> {
  if (key === 'runs') return loadRuns();
  if (key === 'runtime') return loadRuntime();
  if (key === 'agents') return loadAgents();
  if (key === 'worktrees') return loadWorktrees();
  return loadRepositories();
}

/** Forget which input was last loaded — used when the shell tears the panel down. */
export function resetContextActivation(): void {
  loadedInputKey = null;
}

// ── The five loaders ──────────────────────────────────────────────────────────

async function loadRuns(): Promise<void> {
  const ticket = beginCardLoad('runs');
  const projects = snapshotProjects();
  try {
    countInvoke('list_orchestration_runs');
    const runs = await listOrchestrationRunsFromTauri(projects);
    if (runs === null) {
      markCardUnavailable('runs', ticket, DESKTOP_ONLY);
      return;
    }
    applyCardRows('runs', ticket, runs);
  } catch (error) {
    failCardLoad('runs', ticket, `Could not read runs: ${describeError(error)}`);
  }
}

async function loadRuntime(): Promise<void> {
  const ticket = beginCardLoad('runtime');
  const projects = snapshotProjects();
  try {
    countInvoke('list_runtime_contexts');
    const contexts = await listRuntimeContextsFromTauri(projects);
    if (contexts === null) {
      markCardUnavailable('runtime', ticket, DESKTOP_ONLY);
      return;
    }
    applyCardRows('runtime', ticket, contexts);
  } catch (error) {
    failCardLoad('runtime', ticket, `Could not read running processes: ${describeError(error)}`);
  }
}

/**
 * Agent sessions have two transports: the desktop app first, the local dev
 * bridge second. The counted name says which one actually ran — the same rule
 * the session rail's scan uses, so the counter never claims a Tauri command was
 * invoked when the browser bridge answered.
 */
async function loadAgents(): Promise<void> {
  const ticket = beginCardLoad('agents');
  let nativeError: string | null = null;
  try {
    let sessions: AgentSession[] | null = null;
    try {
      sessions = await listAgentSessionsFromTauri();
    } catch (error) {
      nativeError = describeError(error);
    }
    // A `null` native result with no error = not in the desktop app: nothing invoked.
    countInvoke((sessions ?? nativeError) ? 'list_agent_sessions' : 'bridge:agent-sessions');
    sessions ??= await listAgentSessionsFromLocalBridge();
    if (sessions === null) {
      if (nativeError) {
        failCardLoad('agents', ticket, `Could not read agent sessions: ${nativeError}`);
        return;
      }
      markCardUnavailable('agents', ticket, DESKTOP_ONLY);
      return;
    }
    applyCardRows('agents', ticket, sessions);
  } catch (error) {
    failCardLoad(
      'agents',
      ticket,
      `Could not read agent sessions: ${nativeError ?? describeError(error)}`
    );
  }
}

async function loadWorktrees(): Promise<void> {
  const ticket = beginCardLoad('worktrees');
  const root = contextState.activeRoot;
  if (!root) {
    markCardUnavailable('worktrees', ticket, 'Pick a session to see its worktrees.');
    return;
  }
  try {
    countInvoke('list_project_worktrees');
    const worktrees = await listProjectWorktreesFromTauri(root);
    if (worktrees === null) {
      markCardUnavailable('worktrees', ticket, DESKTOP_ONLY);
      return;
    }
    applyCardRows('worktrees', ticket, worktrees);
  } catch (error) {
    failCardLoad('worktrees', ticket, `Could not read worktrees: ${describeError(error)}`);
  }
}

async function loadRepositories(): Promise<void> {
  const ticket = beginCardLoad('repositories');
  const projects = snapshotProjects();
  try {
    countInvoke('list_git_repository_summaries');
    const repos = await listGitRepositorySummariesFromTauri(projects);
    if (repos === null) {
      markCardUnavailable('repositories', ticket, DESKTOP_ONLY);
      return;
    }
    applyCardRows('repositories', ticket, repos);
  } catch (error) {
    failCardLoad('repositories', ticket, `Could not read repositories: ${describeError(error)}`);
  }
}

/**
 * Plain copies of the project rows for the backend call. A runes proxy cannot
 * cross the Tauri boundary, so the array is flattened before it is sent.
 */
function snapshotProjects(): { id: string; name: string; path: string }[] {
  return contextState.projects.map((project) => ({
    id: project.id,
    name: project.name,
    path: project.path
  }));
}
