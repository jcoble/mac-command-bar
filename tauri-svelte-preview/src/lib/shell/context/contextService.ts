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
 * `refreshAll` reaches the Playwright card too, even though that card keeps its
 * own state next door in `playwrightService` — one Refresh button in the panel
 * has to mean every card in the panel, or the one it skips looks freshly read
 * when it is not.
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
  killProcess,
  PROCESS_KILL_CAPABILITY,
  readBackendCapabilities
} from '../processes/processBackend.ts';
import { refresh as refreshPlaywrightCard } from '../processes/playwrightService.ts';
import {
  applyCardRows,
  beginCardLoad,
  beginProcessStop,
  contextState,
  describeContextInput,
  failCardLoad,
  finishProcessStop,
  markCardUnavailable,
  markContextActivated,
  setContextInput,
  setProcessKillSupport,
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
    // Deliberately NOT `refreshAll()`: the shell activates the Playwright card
    // in the same breath as this one, and `refreshAll` reads it too, so calling
    // it here would read that card twice on the very first show.
    void loadRuns();
    void loadRuntime();
    void loadAgents();
    void loadWorktrees();
    void loadRepositories();
    void askWhatTheAppCanDo();
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

/**
 * Reload EVERY card in the panel. The panel's Refresh button.
 *
 * "Every" includes the Playwright card, which keeps its own state in
 * `playwrightService` and used to sit there unreloaded while the five cards
 * around it went and read fresh data. A refresh button that skips a card is
 * worse than no refresh button: the stale card looks as freshly read as its
 * neighbours.
 *
 * The same pass is also where the panel finds out whether this build of the
 * desktop app can stop a process, so the stop buttons can never be left greyed
 * out by a question nobody got round to asking.
 */
export async function refreshAll(): Promise<void> {
  await Promise.all([
    loadRuns(),
    loadRuntime(),
    loadAgents(),
    loadWorktrees(),
    loadRepositories(),
    refreshPlaywrightCard(),
    askWhatTheAppCanDo()
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

// ── Stopping one running process ──────────────────────────────────────────────

/**
 * Ask the desktop app whether it can stop a process by id.
 *
 * Asked rather than tried: see the long note in `processBackend.ts`. Asking
 * again is cheap and keeps the answer right after the user updates and restarts
 * the app, so this runs on every full refresh rather than once per launch.
 */
export async function askWhatTheAppCanDo(): Promise<void> {
  countInvoke('read_backend_capabilities');
  const capabilities = await readBackendCapabilities();
  if (capabilities === null) {
    // Not the desktop app at all; nothing here can stop anything.
    setProcessKillSupport('unavailable');
    return;
  }
  setProcessKillSupport(
    capabilities.includes(PROCESS_KILL_CAPABILITY) ? 'available' : 'unavailable'
  );
}

/**
 * Stop the process with this id, then read the running processes again.
 *
 * The list is re-read rather than edited in place, because the desktop app's
 * answer says what it ASKED the process to do, not whether the process has
 * actually gone. Reading again is the only honest way to show what survived.
 *
 * Refuses outright when the app has not said it can do this, so a wired-up
 * keyboard shortcut or a stale screen can never send a request that would be
 * silently ignored.
 */
export async function stopProcess(pid: number): Promise<void> {
  if (contextState.processKill !== 'available') return;
  beginProcessStop(pid);
  try {
    countInvoke('kill_process');
    const result = await killProcess(pid);
    if (result === null) {
      finishProcessStop(DESKTOP_ONLY);
      return;
    }
    finishProcessStop(result.message);
  } catch (error) {
    finishProcessStop(`Process ${pid} was not stopped: ${describeError(error)}`);
  }
  await loadRuntime();
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
