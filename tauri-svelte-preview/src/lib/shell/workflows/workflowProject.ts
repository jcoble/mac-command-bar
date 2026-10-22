/**
 * A project's progress, read from run records: its slices are the runs whose
 * `parent_task` input is the project's task key. A slice run again shows its
 * latest run; slices are in the order they started, which is plan order.
 */
import { boardColumn, runTask, type BoardColumnId, type StepGlyph } from './workflowBoard.ts';
import type { WorkflowRunRecord } from './workflowTypes.ts';

export interface ProjectSlice {
  key: string;
  title: string;
  /** The slice's number in the approved plan ("1.2"), when the run was given one. */
  number: string | null;
  run: WorkflowRunRecord;
  glyph: StepGlyph;
}

export interface ProjectProgress {
  slices: ProjectSlice[];
  /** Slices whose run completed. */
  done: number;
  total: number;
  /** The first slice not completed, or null when all are. */
  current: ProjectSlice | null;
}

const input = (run: WorkflowRunRecord, name: string): string | null => {
  const value = (run.inputSnapshot as { inputs?: Record<string, unknown> } | null)?.inputs?.[name];
  return typeof value === 'string' && value ? value : null;
};

export const parentTask = (run: WorkflowRunRecord): string | null => input(run, 'parent_task');

/** A whole run's state as a step glyph. */
export function runGlyph(run: WorkflowRunRecord): StepGlyph {
  if (run.state.startsWith('waiting') || run.nodes.some((node) => node.state.startsWith('waiting'))) return 'needs-you';
  switch (run.state) {
    case 'completed': return 'passed';
    case 'running': return 'running';
    case 'failed': return 'failed';
    case 'cancelled': return 'stopped';
  }
  return 'waiting';
}

/** The project's slices from `runs`, or null when no run names it as its parent. */
export function projectProgress(project: WorkflowRunRecord, runs: readonly WorkflowRunRecord[]): ProjectProgress | null {
  const key = runTask(project).key;
  // A project run again (say after its plan was sent back) leaves the slices to its latest run.
  if (!key || runs.some((run) => runTask(run).key === key && run.createdAtMs > project.createdAtMs)) return null;
  const latest = new Map<string, { first: number; run: WorkflowRunRecord }>();
  for (const run of runs) {
    const task = runTask(run).key;
    if (run.id === project.id || parentTask(run) !== key || !task) continue;
    const seen = latest.get(task);
    latest.set(task, {
      first: Math.min(seen?.first ?? run.createdAtMs, run.createdAtMs),
      run: seen && seen.run.createdAtMs > run.createdAtMs ? seen.run : run
    });
  }
  if (!latest.size) return null;
  const slices = [...latest.entries()].sort((a, b) => a[1].first - b[1].first).map(([task, { run }]) => ({
    key: task, title: runTask(run).title, number: input(run, 'slice'), run, glyph: runGlyph(run)
  }));
  const done = slices.filter((slice) => slice.run.state === 'completed').length;
  return { slices, done, total: slices.length, current: slices.find((slice) => slice.run.state !== 'completed') ?? null };
}

export const sliceCount = (progress: ProjectProgress): string => `${progress.done} / ${progress.total} slices`;

/** A project's Board column is its current slice's; any other run's is its own. */
export function projectColumn(run: WorkflowRunRecord, runs: readonly WorkflowRunRecord[]): BoardColumnId {
  const progress = projectProgress(run, runs);
  if (!progress) return boardColumn(run);
  return progress.current ? boardColumn(progress.current.run) : 'done';
}
