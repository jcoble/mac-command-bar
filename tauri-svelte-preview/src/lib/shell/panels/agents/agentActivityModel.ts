/**
 * The Agents panel's view model.
 *
 * A session's subagents arrive as `ConversationChildAgent` records, which carry
 * a label, a provider state string, and a time — and nothing else. In
 * particular there is no message count and no path to a log on that record, so
 * this module is careful to say "not known" rather than invent either one.
 *
 * A message count exists only for an agent whose transcript has actually been
 * read, because the store keeps a timeline for the selected child alone. That
 * is why `messageCount` is `number | null`: `null` means nobody has looked yet,
 * and `0` means we looked and the transcript was empty. The panel shows blank
 * for the first and a real zero for the second.
 */
import type {
  ConversationChildAgent,
  ConversationTimelineEntry
} from '../../conversation/conversationTypes.ts';

export type AgentStatus = 'working' | 'done' | 'failed' | 'idle';

export interface AgentActivityRow {
  childId: string;
  label: string;
  status: AgentStatus;
  /** One plain-English line about what this agent is doing, or '' when nothing is known. */
  activity: string;
  /** Message count, or null when it cannot be known for this agent. */
  messageCount: number | null;
  /** Absolute path of a log to open, or null when none is available. */
  logPath: string | null;
  updatedAtMs: number;
}

/** How long an activity line may run before it is cut short with an ellipsis. */
const ACTIVITY_LINE_LIMIT = 80;

const WORKING_STATES = new Set(['active', 'running', 'in_progress', 'in-progress', 'working', 'started']);
const DONE_STATES = new Set(['completed', 'complete', 'done', 'finished', 'succeeded', 'success']);
const FAILED_STATES = new Set(['failed', 'failure', 'error', 'errored']);

/** Maps the provider's raw state string onto the four statuses. Unknown states become 'idle'. */
export function agentStatus(state: string): AgentStatus {
  const normalized = state.trim().toLowerCase();
  if (WORKING_STATES.has(normalized)) return 'working';
  if (DONE_STATES.has(normalized)) return 'done';
  if (FAILED_STATES.has(normalized)) return 'failed';
  return 'idle';
}

function firstLine(text: string): string {
  const line = text.split('\n').find((candidate) => candidate.trim().length > 0)?.trim() ?? '';
  return line.length > ACTIVITY_LINE_LIMIT ? `${line.slice(0, ACTIVITY_LINE_LIMIT - 1).trimEnd()}…` : line;
}

/**
 * One line describing a single timeline entry, or '' when that entry says
 * nothing a person would want on a one-line summary.
 */
function lineFor(entry: ConversationTimelineEntry): string {
  switch (entry.kind) {
    case 'user':
    case 'assistant':
      return firstLine(entry.text);
    case 'tool':
      if (entry.state === 'failed') return firstLine(`${entry.name} failed`);
      if (entry.state === 'completed') return firstLine(`Finished ${entry.name}`);
      return firstLine(`Running ${entry.name}`);
    case 'approval':
      return entry.state === 'requested' ? 'Waiting for approval' : firstLine(entry.summary);
    case 'plan':
      return 'Working through a plan';
    case 'error':
      return firstLine(entry.message);
    default:
      return '';
  }
}

/** The newest entry that describes something, searching backwards. */
function activityFrom(timeline: readonly ConversationTimelineEntry[]): string {
  for (let index = timeline.length - 1; index >= 0; index -= 1) {
    const line = lineFor(timeline[index]);
    if (line) return line;
  }
  return '';
}

/** Rows for one session's children, most recently updated first.
 *  `timelineByChild` supplies a count only for children whose transcript has been read. */
export function agentActivityRows(
  children: readonly ConversationChildAgent[],
  timelineByChild: Readonly<Record<string, readonly ConversationTimelineEntry[]>>
): AgentActivityRow[] {
  return [...children]
    .sort((left, right) => right.updatedAtMs - left.updatedAtMs)
    .map((child) => {
      const timeline = timelineByChild[child.childId] ?? null;
      return {
        childId: child.childId,
        label: child.label,
        status: agentStatus(child.state),
        activity: timeline ? activityFrom(timeline) : '',
        messageCount: timeline ? timeline.length : null,
        // A child agent record carries no log path, so there is never one to
        // offer. This stays here rather than being dropped so the panel can say
        // so honestly, and so the day a path arrives there is one place to fill in.
        logPath: null,
        updatedAtMs: child.updatedAtMs
      };
    });
}
