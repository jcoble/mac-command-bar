/**
 * What the step panel and the Cost tab read from a run: a step's links, its
 * failed commands, its usage, whether a review was accepted with findings
 * still open, and the Cost tab's lines and totals.
 */
import { shortDuration, stepGlyph } from './workflowBoard.ts';
import type { WorkflowTimelineItem } from './workflowTimeline.ts';
import { workflowStepFacts, type WorkflowNodeRunRecord, type WorkflowRunRecord, type WorkflowUsageRow } from './workflowTypes.ts';

const PR_URL = /https:\/\/github\.com\/[^/\s]+\/[^/\s]+\/pull\/\d+/;

const text = (value: unknown): string | null => (typeof value === 'string' && value ? value : null);

/** Tokens as the Cost tab writes them: "445k", or the number under a thousand. */
export const tokenCount = (tokens: number): string => (tokens >= 1000 ? `${Math.round(tokens / 1000)}k` : String(tokens));

/** Dollars from micros; a cost under a cent reads "<$0.01", not "$0.00". */
export const money = (micros: number): string => (micros > 0 && micros < 10_000 ? '<$0.01' : `$${(micros / 1_000_000).toFixed(2)}`);

/** Usage rows' cost: "$1.07", "$1.07 + $0.74 est.", or '' when none is priced. */
export function costTotal(rows: readonly WorkflowUsageRow[]): string {
  const reported = rows.reduce((sum, row) => sum + (row.reportedCostMicros ?? 0), 0);
  const estimated = rows.reduce((sum, row) => sum + (row.estimatedCostMicros ?? 0), 0);
  const priced = rows.some((row) => row.reportedCostMicros !== null);
  if (!priced && !estimated) return '';
  return [priced ? money(reported) : '', estimated ? `${money(estimated)} est.` : ''].filter(Boolean).join(' + ');
}

export interface StepLink {
  label: string;
  value: string;
  /** A web address to open in the browser. */
  url: string | null;
  /** A file or folder on this machine to open. */
  path: string | null;
}

/** A step's pull request (its receipt's, or the first one its commands printed), commit, branch and worktree (the run's when the step has none), task, workflow and files. */
export function stepLinks(run: WorkflowRunRecord, node: WorkflowNodeRunRecord): StepLink[] {
  const output = (node.structuredOutput ?? {}) as Record<string, unknown>;
  const snapshot = (run.inputSnapshot ?? {}) as { task?: { key?: unknown }; worktree?: { path?: unknown; branch?: unknown } };
  const commands = Array.isArray(output.commands) ? (output.commands as Record<string, unknown>[]) : [];
  const printed = commands.map((command) => PR_URL.exec(String(command.output ?? ''))?.[0]).find(Boolean) ?? null;
  const pr = workflowStepFacts(node).prUrl ?? printed;
  const worktree = text(output.worktree) ?? text(snapshot.worktree?.path);
  const links: (StepLink | null)[] = [
    pr ? { label: 'Pull request', value: pr, url: pr, path: null } : null,
    ...(['commit', 'branch'] as const).map((key) => {
      const value = text(output[key]) ?? (key === 'branch' ? text(snapshot.worktree?.branch) : null);
      return value ? { label: key === 'commit' ? 'Commit' : 'Branch', value, url: null, path: null } : null;
    }),
    worktree ? { label: 'Worktree', value: worktree, url: null, path: worktree } : null,
    text(snapshot.task?.key) ? { label: 'Task', value: String(snapshot.task?.key), url: null, path: null } : null,
    { label: 'Workflow', value: run.workflowId.split(':').at(-1) ?? run.workflowId, url: null, path: null },
    ...node.artifacts.map((artifact) => {
      const value = artifact.url ?? artifact.path;
      return value ? { label: artifact.kind, value, url: artifact.url, path: artifact.url ? null : artifact.path } : null;
    })
  ];
  return links.filter((link): link is StepLink => link !== null);
}

export interface FailedCommand {
  command: string;
  /** "exit 1", or the engine's error when the command left no exit code (it timed out or could not start). */
  exit: string;
  /** The last 10 lines it printed; empty when it printed nothing. */
  tail: string;
}

/** The commands of a command step that did not exit 0, as the engine recorded them; none when it recorded none. */
export function failedCommands(node: Pick<WorkflowNodeRunRecord, 'structuredOutput'>): FailedCommand[] {
  const commands = ((node.structuredOutput ?? {}) as { commands?: unknown }).commands;
  if (!Array.isArray(commands)) return [];
  return (commands as Record<string, unknown>[]).filter((command) => command.exitCode !== 0).map((command) => ({
    command: String(command.command ?? ''),
    exit: typeof command.exitCode === 'number' ? `exit ${command.exitCode}` : text(command.error) ?? 'no exit code recorded',
    tail: String(command.output ?? '').replace(/\n+$/, '').split('\n').slice(-10).join('\n')
  }));
}

/** One step attempt's usage from its usage rows: "445k in, 2k out · $0.02"; null when it has none. */
export function stepUsageText(rows: readonly WorkflowUsageRow[], nodeRunId: string): string | null {
  const mine = rows.filter((row) => row.nodeRunId === nodeRunId);
  if (!mine.length) return null;
  const tokensIn = mine.reduce((sum, row) => sum + row.inputTokens + row.cacheReadTokens + row.cacheWriteTokens, 0);
  const tokensOut = mine.reduce((sum, row) => sum + row.outputTokens, 0);
  return `${tokenCount(tokensIn)} in, ${tokenCount(tokensOut)} out · ${costTotal(mine) || 'not priced'}`;
}

/**
 * "Accepted by you with 1 finding open · Oct 22, 2026, 3:00 PM" for a step that finished because its waiting
 * findings were accepted as they were; null for anything else. The time is the step's latest answer in the log.
 */
export function acceptedWithFindings(run: WorkflowRunRecord, node: WorkflowNodeRunRecord, timeline: readonly WorkflowTimelineItem[]): string | null {
  const open = workflowStepFacts(node).findings.length;
  const answers = ((run.inputSnapshot ?? {}) as { answers?: { step?: string; decision?: string; by?: string }[] }).answers ?? [];
  const answer = answers.filter((item) => item.step === node.nodeId).at(-1);
  if (node.state !== 'completed' || !open || answer?.decision !== 'accept') return null;
  const at = timeline.filter((item) => item.nodeId === node.nodeId && (item.kind === 'owner' || item.kind === 'orchestrator')).at(-1)?.atMs;
  const when = at ? ` · ${new Date(at).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}` : '';
  return `Accepted by ${answer.by === 'orchestrator' ? 'the orchestrator' : 'you'} with ${open} finding${open === 1 ? '' : 's'} open${when}`;
}

export interface CostLine {
  key: string;
  step: string;
  attempt: number;
  /** The usage row's model; for a step without one, the model its role names, or "engine" for a command step. */
  model: string;
  time: string;
  row: WorkflowUsageRow | null;
  engine: boolean;
}

// Timeline kinds that end an attempt.
const ENDS = ['finish', 'checks', 'review', 'failed', 'timeout', 'waiting'];

/** How long a step attempt ran: the latest from the run record, earlier ones from the log's start and end; "—" when unknown. */
function attemptTime(run: WorkflowRunRecord, timeline: readonly WorkflowTimelineItem[], step: string, attempt: number, now: number): string {
  const node = run.nodes.find((candidate) => candidate.nodeId === step);
  if (node?.startedAtMs && node.attempt === attempt) {
    return shortDuration((node.finishedAtMs ?? (stepGlyph(node) === 'running' ? now : node.startedAtMs)) - node.startedAtMs);
  }
  const mine = timeline.filter((item) => item.nodeId === step && item.attempt === attempt);
  const start = mine.find((item) => item.kind === 'start');
  const end = start && mine.find((item) => ENDS.includes(item.kind) && item.atMs >= start.atMs);
  return start && end ? shortDuration(end.atMs - start.atMs) : '—';
}

/** Every step attempt, in step order: one line per usage row, and one for each attempt that ran without any. */
export function costLines(run: WorkflowRunRecord, rows: readonly WorkflowUsageRow[] | null, timeline: readonly WorkflowTimelineItem[], now: number): CostLine[] {
  const attemptOf = (nodeRunId: string): [string, number] => {
    const [, ...rest] = nodeRunId.split(':');
    const attempt = Number(rest.pop());
    return [rest.join(':'), attempt];
  };
  const measured = new Set((rows ?? []).map((row) => attemptOf(row.nodeRunId).join(':')));
  const lines: CostLine[] = (rows ?? []).map((row) => {
    const [step, attempt] = attemptOf(row.nodeRunId);
    return { key: `${row.nodeRunId}\n${row.model}`, step, attempt, model: row.model, time: attemptTime(run, timeline, step, attempt, now), row, engine: false };
  });
  for (const node of run.nodes) {
    const attempts = new Set(timeline.filter((item) => item.kind === 'start' && item.nodeId === node.nodeId && item.attempt).map((item) => item.attempt as number));
    if (node.startedAtMs) attempts.add(node.attempt);
    const engine = !node.provider;
    const model = engine ? 'engine' : run.definition.roles?.find((role) => role.id === node.roleId)?.modelPolicy?.value ?? node.provider;
    for (const attempt of attempts) {
      if (measured.has(`${node.nodeId}:${attempt}`)) continue;
      lines.push({ key: `${node.nodeId}:${attempt}`, step: node.nodeId, attempt, model, time: attemptTime(run, timeline, node.nodeId, attempt, now), row: null, engine });
    }
  }
  const order = new Map(run.definition.nodes.map((node, index) => [node.id, index]));
  return lines.sort((a, b) => (order.get(a.step) ?? 0) - (order.get(b.step) ?? 0) || a.attempt - b.attempt);
}

/** The Cost tab's total, and "Partial: N step attempts not measured" when agent attempts ran without usage rows beside measured ones. */
export function costSummary(rows: readonly WorkflowUsageRow[] | null, lines: readonly CostLine[]): { total: string; note: string } {
  if (!rows) return { total: '—', note: '' };
  const unmeasured = lines.filter((line) => !line.row && !line.engine).length;
  const total = costTotal(rows) || (rows.length ? 'Not priced' : 'Not measured');
  const note = rows.length && unmeasured ? `Partial: ${unmeasured} step attempt${unmeasured === 1 ? '' : 's'} not measured` : '';
  return { total, note };
}
