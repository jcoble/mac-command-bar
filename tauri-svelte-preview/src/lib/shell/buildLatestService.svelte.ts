import {
  idleBuildLatest, nextBuildLatest, type BuildLatestEvent, type BuildLatestProgress
} from './buildLatestProgress.ts';
import { trackTauriListener } from './resourceDiagnostics.svelte.ts';
import { isTauriRuntime } from '../tauriSource.ts';

/**
 * Build latest (dev builds only): pull main, build, deploy the backend to every saved machine
 * and restart Assembly. The native command runs `scripts/buildLatestDev.ts` and reports by app
 * event; the home button and Settings → Updates read this state.
 */
export const buildLatestState = $state<BuildLatestProgress & { logPath: string }>({ ...idleBuildLatest, logPath: '' });

function apply(event: BuildLatestEvent): void {
  Object.assign(buildLatestState, nextBuildLatest(buildLatestState, event));
}

/** Follow progress until `stopSignal` aborts. Listens all app long, so a page reload mid-run picks it up again. */
export async function listenToBuildLatest(stopSignal: AbortSignal): Promise<void> {
  if (!import.meta.env.DEV || !isTauriRuntime() || stopSignal.aborted) return;
  const { listen } = await import('@tauri-apps/api/event');
  const stop = trackTauriListener(await listen<BuildLatestEvent>('build_latest_dev', (event) => apply(event.payload)));
  if (stopSignal.aborted) stop();
  else stopSignal.addEventListener('abort', stop, { once: true });
}

export async function startBuildLatest(): Promise<void> {
  if (buildLatestState.phase === 'running' || buildLatestState.phase === 'restarting') return;
  Object.assign(buildLatestState, { ...idleBuildLatest, phase: 'running', step: 1, text: 'Starting…' });
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    buildLatestState.logPath = await invoke<string>('build_latest_dev');
  } catch (error) {
    apply({ done: true, error: error instanceof Error ? error.message : String(error) });
  }
}

/** Ends the run; the script stops what it is running and reports "Stopped.". */
export async function stopBuildLatest(): Promise<void> {
  if (buildLatestState.phase !== 'running') return;
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('stop_build_latest_dev');
}
