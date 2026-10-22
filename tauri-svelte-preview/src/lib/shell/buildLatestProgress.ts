/**
 * Progress of a Build latest (dev) run, from the lines `scripts/buildLatestDev.ts` prints.
 * Every script line starts with "[step/total]", so any line, even the first one after the
 * page reloads mid-run, is enough to show where the run is.
 */
export type BuildLatestPhase = 'idle' | 'running' | 'restarting' | 'failed';
export type BuildLatestProgress = { phase: BuildLatestPhase; step: number; total: number; text: string; error: string };
/** One app event from the native command: an output line, or the end of the run. */
export type BuildLatestEvent = { line: string } | { done: true; error: string | null };

export const idleBuildLatest: BuildLatestProgress = { phase: 'idle', step: 0, total: 4, text: '', error: '' };

const stepLine = /^\[(\d+)\/(\d+)\] (.*)$/;

export function nextBuildLatest(state: BuildLatestProgress, event: BuildLatestEvent): BuildLatestProgress {
  if ('done' in event) {
    if (!event.error) return { ...state, phase: 'restarting' };
    return { ...state, phase: 'failed', error: event.error.replace(stepLine, '$3') };
  }
  const match = stepLine.exec(event.line);
  if (!match) return state.phase === 'running' ? state : { ...state, phase: 'running', error: '' };
  return { phase: 'running', step: Number(match[1]), total: Number(match[2]), text: match[3], error: '' };
}
