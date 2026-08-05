import type {
  AgentConfigValue,
  AgentContent,
  AgentEvent,
  AgentItem,
  AgentItemType,
  AgentApprovalRequest,
  AgentConversationEvent,
  AgentUserInputRequest,
  AgentUserInputField,
  ConversationTimelineEntry
} from './conversationTypes.ts';

export interface AgentPlanStep {
  id: string;
  title: string;
  detail: string | null;
  state: 'pending' | 'in-progress' | 'completed' | 'failed' | 'blocked';
  ownerAgentId: string | null;
  startedAt: string | null;
  completedAt: string | null;
}

export interface ConversationTask {
  id: string;
  title: string;
  state: AgentPlanStep['state'];
  detail?: string | null;
  ownerAgentId?: string | null;
}

export type ConversationTextDisplayItem = {
  kind: 'user' | 'assistant' | 'reasoning' | 'command' | 'file' | 'error';
  itemId: string;
  text: string;
  timestampMs: number;
  completed?: boolean;
  label?: string;
  metadata?: Record<string, AgentConfigValue>;
};

export type ConversationDisplayItem =
  | (ConversationTextDisplayItem & { kind: 'user' })
  | (ConversationTextDisplayItem & { kind: 'assistant' })
  | (ConversationTextDisplayItem & { kind: 'reasoning' })
  | (ConversationTextDisplayItem & { kind: 'command' })
  | (ConversationTextDisplayItem & { kind: 'file' })
  | (ConversationTextDisplayItem & { kind: 'error' })
  | { kind: 'plan'; itemId: string; title: string; steps: AgentPlanStep[]; timestampMs: number; text?: string }
  | { kind: 'tasks'; itemId: string; title: string; tasks: ConversationTask[]; timestampMs: number; text?: string }
  | { kind: 'tool'; itemId: string; name: string; state: string; summary?: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> }
  | { kind: 'subagent'; itemId: string; childId: string; parentId: string; label: string; state: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> }
  | { kind: 'approval'; itemId: string; requestId: string; title: string; summary: string; state: string; options: string[]; timestampMs: number }
  | { kind: 'input'; itemId: string; requestId: string; title: string; description?: string; fields: AgentUserInputField[]; timestampMs: number }
  | { kind: 'unknown'; itemId: string; text: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> };

const textOf = (content: readonly AgentContent[] | undefined): string =>
  (content ?? []).map((entry) => entry.text).join('');

function recordOf(value: unknown): Record<string, AgentConfigValue> | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  return value as Record<string, AgentConfigValue>;
}

function stringOf(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback;
}

function stateOf(value: unknown, fallback = 'pending'): string {
  return typeof value === 'string' && value ? value : fallback;
}

function itemText(item: AgentItem): string {
  return textOf(item.content);
}

function stepsOf(value: unknown): AgentPlanStep[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry, index) => {
    const row = recordOf(entry);
    if (!row) return [];
    return [{
      id: stringOf(row.id, `step-${index + 1}`),
      title: stringOf(row.title, stringOf(row.name, `Step ${index + 1}`)),
      detail: typeof row.detail === 'string' ? row.detail : null,
      state: ['pending', 'in-progress', 'completed', 'failed', 'blocked'].includes(stringOf(row.state))
        ? stringOf(row.state) as AgentPlanStep['state']
        : 'pending',
      ownerAgentId: typeof row.ownerAgentId === 'string' ? row.ownerAgentId : null,
      startedAt: typeof row.startedAt === 'string' ? row.startedAt : null,
      completedAt: typeof row.completedAt === 'string' ? row.completedAt : null
    }];
  });
}

function tasksOf(value: unknown): ConversationTask[] {
  return stepsOf(value).map((step) => ({
    id: step.id,
    title: step.title,
    state: step.state,
    detail: step.detail,
    ownerAgentId: step.ownerAgentId
  }));
}

function kindFor(item: AgentItem): ConversationDisplayItem['kind'] {
  switch (item.type) {
    case 'user-message': return 'user';
    case 'assistant-message': return 'assistant';
    case 'reasoning': return 'reasoning';
    case 'plan': return 'plan';
    case 'task-list': return 'tasks';
    case 'command': return 'command';
    case 'file-change': return 'file';
    case 'mcp-tool':
    case 'web-search':
    case 'image-view':
    case 'image-generation': return 'tool';
    case 'subagent': return 'subagent';
    case 'error': return 'error';
    default: return 'unknown';
  }
}

export function displayItemFromAgentItem(item: AgentItem, timestampMs = Date.now()): ConversationDisplayItem {
  const metadata = item.providerMetadata;
  const kind = kindFor(item);
  if (kind === 'plan') return {
    kind, itemId: item.id, title: stringOf(metadata?.title, 'Plan'), steps: stepsOf(metadata?.steps), text: itemText(item), timestampMs
  };
  if (kind === 'tasks') return {
    kind, itemId: item.id, title: stringOf(metadata?.title, 'Tasks'), tasks: tasksOf(metadata?.tasks ?? metadata?.steps), text: itemText(item), timestampMs
  };
  if (kind === 'tool') return {
    kind, itemId: item.id, name: stringOf(metadata?.name, item.type), state: stateOf(metadata?.state, 'started'), summary: stringOf(metadata?.summary, itemText(item)) || undefined, metadata, timestampMs
  };
  if (kind === 'subagent') return {
    kind, itemId: item.id, childId: stringOf(metadata?.childId, item.id), parentId: stringOf(metadata?.parentId, item.turnId ?? ''), label: stringOf(metadata?.label, 'Sub-agent'), state: stateOf(metadata?.state, 'working'), metadata, timestampMs
  };
  if (kind === 'unknown') return { kind, itemId: item.id, text: itemText(item), metadata, timestampMs };
  return { kind: kind as ConversationTextDisplayItem['kind'], itemId: item.id, text: itemText(item), completed: true, timestampMs, metadata } as ConversationDisplayItem;
}

export function displayItemFromApproval(
  request: AgentApprovalRequest & { state: string },
  timestampMs = Date.now()
): ConversationDisplayItem {
  return {
    kind: 'approval',
    itemId: `approval:${request.requestId}`,
    requestId: request.requestId,
    title: request.title,
    summary: request.description ?? request.title,
    state: request.state,
    options: request.options,
    timestampMs
  };
}

export function displayItemFromInput(
  request: AgentUserInputRequest,
  timestampMs = Date.now()
): ConversationDisplayItem {
  return {
    kind: 'input',
    itemId: `input:${request.requestId}`,
    requestId: request.requestId,
    title: request.title,
    description: request.description,
    fields: request.fields,
    timestampMs
  };
}

function displayItemFromLegacy(entry: ConversationTimelineEntry): ConversationDisplayItem {
  if (entry.kind === 'user' || entry.kind === 'assistant') return {
    kind: entry.kind, itemId: entry.itemId, text: entry.text, completed: entry.completed, timestampMs: entry.timestampMs
  };
  if (entry.kind === 'tool') return {
    kind: 'tool', itemId: entry.itemId, name: entry.name, state: entry.state, summary: entry.summary, timestampMs: entry.timestampMs
  };
  if (entry.kind === 'approval') return {
    kind: 'approval', itemId: entry.itemId, requestId: entry.requestId, title: 'Approval requested', summary: entry.summary,
    state: entry.state, options: ['accept', 'decline'], timestampMs: entry.timestampMs
  };
  if (entry.kind === 'error') return {
    kind: 'error', itemId: entry.itemId, text: entry.message, timestampMs: entry.timestampMs, metadata: { code: entry.code, recoverable: entry.recoverable }
  };
  if (entry.kind === 'turn') return {
    kind: 'unknown', itemId: entry.itemId, text: `${entry.turnId} · ${entry.state}`, timestampMs: entry.timestampMs
  };
  return { kind: 'unknown', itemId: entry.itemId, text: '', timestampMs: 0 };
}

/** Merge typed provider items with legacy reducer entries without flattening them. */
export function typedConversationTimeline(
  items: readonly AgentItem[] = [],
  legacy: readonly ConversationTimelineEntry[] = [],
  timestamps: Readonly<Record<string, number>> = {}
): ConversationDisplayItem[] {
  const byId = new Map<string, ConversationDisplayItem>();
  legacy.forEach((entry) => byId.set(entry.itemId, displayItemFromLegacy(entry)));
  const legacyEnd = legacy.reduce((latest, entry) => Math.max(latest, entry.timestampMs), 0);
  items.forEach((item, index) => byId.set(item.id, displayItemFromAgentItem(item, timestamps[item.id] ?? legacyEnd + index + 1)));
  return [...byId.values()].sort((left, right) => left.timestampMs - right.timestampMs);
}

/** Stable, bounded row selection for a long history. */
export function visibleConversationRange(
  count: number,
  scrollTop: number,
  viewportHeight: number,
  rowEstimate = 96,
  overscan = 6
): { start: number; end: number; offsetTop: number } {
  if (count <= 0) return { start: 0, end: 0, offsetTop: 0 };
  const safeRow = Math.max(48, rowEstimate);
  const start = Math.max(0, Math.floor(Math.max(0, scrollTop) / safeRow) - overscan);
  const visible = Math.ceil(Math.max(1, viewportHeight) / safeRow) + overscan * 2;
  return { start, end: Math.min(count, start + visible), offsetTop: start * safeRow };
}

export function conversationScrollShouldFollow(
  scrollTop: number,
  clientHeight: number,
  scrollHeight: number,
  threshold = 80
): boolean {
  return scrollHeight - (scrollTop + clientHeight) <= threshold;
}

export function agentItemFromEvent(event: AgentEvent | AgentConversationEvent): AgentItem | null {
  const payload = event.payload as Record<string, unknown>;
  const raw = recordOf(payload.item);
  if (raw && typeof raw.id === 'string' && typeof raw.type === 'string') {
    return {
      id: raw.id,
      type: raw.type as AgentItemType,
      turnId: typeof raw.turnId === 'string' ? raw.turnId : ('turnId' in event ? event.turnId : undefined),
      content: Array.isArray(raw.content)
        ? raw.content.flatMap((content) => {
          const row = recordOf(content);
          return row && typeof row.channel === 'string' && typeof row.text === 'string'
            ? [{ channel: row.channel as AgentContent['channel'], text: row.text, mimeType: typeof row.mimeType === 'string' ? row.mimeType : undefined }]
            : [];
        })
        : [],
      providerMetadata: recordOf(raw.providerMetadata)
    };
  }
  const id = ('itemId' in event ? event.itemId : undefined) ?? stringOf(payload.itemId);
  if (!id) return null;
  const text = stringOf(payload.text, stringOf(payload.delta));
  const eventType = 'type' in event ? event.type : '';
  const channel = stringOf(payload.channel, eventType === 'content.delta' ? 'assistant' : 'assistant') as AgentContent['channel'];
  let type: AgentItemType = 'unknown';
  if (payload.kind === 'userMessage') type = 'user-message';
  else if (payload.kind === 'assistantMessage' || payload.kind === 'assistantDelta') type = 'assistant-message';
  else if (payload.kind === 'tool') type = 'mcp-tool';
  else if (payload.kind === 'error') type = 'error';
  else if (channel.startsWith('reasoning')) type = 'reasoning';
  else if (channel === 'plan') type = 'plan';
  else if (channel === 'command-output') type = 'command';
  else if (channel === 'file-change-output') type = 'file-change';
  return {
    id,
    type,
    turnId: 'turnId' in event ? event.turnId : undefined,
    content: text ? [{ channel, text }] : [],
    providerMetadata: recordOf(payload.providerMetadata)
  };
}
