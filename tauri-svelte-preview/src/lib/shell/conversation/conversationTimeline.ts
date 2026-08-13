import type {
  AgentCommandDescriptor,
  AgentConfigValue,
  AgentContent,
  AgentEvent,
  AgentItem,
  AgentItemType,
  AgentApprovalRequest,
  AgentConversationEvent,
  AgentPermissionOption,
  AgentPermissionRequest,
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

export type ConversationToolKind = 'command' | 'file-edit' | 'search' | 'fetch' | 'tool';
export type ConversationToolState = 'pending' | 'running' | 'completed' | 'failed';

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
  | {
      kind: 'tool';
      itemId: string;
      title: string;
      toolKind: ConversationToolKind;
      state: ConversationToolState;
      output?: string;
      diff?: string;
      path?: string;
      summary?: string;
      timestampMs: number;
      metadata?: Record<string, AgentConfigValue>;
    }
  | { kind: 'subagent'; itemId: string; childId: string; parentId: string; label: string; state: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> }
  | {
      kind: 'approval';
      itemId: string;
      requestId: string;
      title: string;
      toolTitle: string;
      summary: string;
      state: string;
      options: AgentPermissionOption[];
      timestampMs: number;
    }
  | { kind: 'input'; itemId: string; requestId: string; title: string; description?: string; fields: AgentUserInputField[]; timestampMs: number }
  | { kind: 'unknown'; itemId: string; text: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> };

type ConversationEvent = AgentEvent | AgentConversationEvent;
type StringRecord = Record<string, unknown>;

function recordOf(value: unknown): StringRecord | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return undefined;
  return value as StringRecord;
}

function metadataRecord(value: unknown): Record<string, AgentConfigValue> | undefined {
  return recordOf(value) as Record<string, AgentConfigValue> | undefined;
}

function stringOf(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback;
}

function numberOf(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function booleanOf(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback;
}

function normalizedPayloadKind(payload: StringRecord): string {
  const raw = stringOf(payload.kind, stringOf(payload.sessionUpdate));
  return raw.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase());
}

function textFromValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) return value.map(textFromValue).filter(Boolean).join('');
  const record = recordOf(value);
  if (!record) return '';
  for (const field of ['text', 'output', 'delta', 'message']) {
    const text = textFromValue(record[field]);
    if (text) return text;
  }
  return textFromValue(record.content);
}

function firstNestedString(value: unknown, keys: readonly string[]): string {
  if (Array.isArray(value)) {
    for (const entry of value) {
      const found = firstNestedString(entry, keys);
      if (found) return found;
    }
    return '';
  }
  const record = recordOf(value);
  if (!record) return '';
  for (const key of keys) {
    const direct = stringOf(record[key]);
    if (direct) return direct;
  }
  for (const nested of Object.values(record)) {
    const found = firstNestedString(nested, keys);
    if (found) return found;
  }
  return '';
}

const textOf = (content: readonly AgentContent[] | undefined): string =>
  (content ?? []).map((entry) => entry.text).join('');

function planStateOf(status: string): AgentPlanStep['state'] {
  const normalized = status.trim().toLowerCase().replaceAll('_', '-');
  if (normalized === 'in-progress' || normalized === 'completed' || normalized === 'failed' || normalized === 'blocked') {
    return normalized;
  }
  return 'pending';
}

function stepsOf(value: unknown): AgentPlanStep[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry, index) => {
    const row = recordOf(entry);
    if (!row) return [];
    return [{
      id: stringOf(row.id, `step-${index + 1}`),
      title: stringOf(row.title, stringOf(row.text, stringOf(row.content, stringOf(row.name, `Step ${index + 1}`)))),
      detail: typeof row.detail === 'string' ? row.detail : null,
      state: planStateOf(stringOf(row.state, stringOf(row.status))),
      ownerAgentId: typeof row.ownerAgentId === 'string' ? row.ownerAgentId : null,
      startedAt: typeof row.startedAt === 'string' ? row.startedAt : null,
      completedAt: typeof row.completedAt === 'string' ? row.completedAt : null
    }];
  });
}

function planItemsOf(items: readonly { text: string; status: string }[]): AgentPlanStep[] {
  return items.map((item, index) => ({
    id: `step-${index + 1}`,
    title: item.text,
    detail: null,
    state: planStateOf(item.status),
    ownerAgentId: null,
    startedAt: null,
    completedAt: null
  }));
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
    case 'command':
    case 'file-change': return 'tool';
    case 'mcp-tool':
    case 'web-search':
    case 'image-view':
    case 'image-generation': return 'tool';
    case 'subagent': return 'subagent';
    case 'error': return 'error';
    default: return 'unknown';
  }
}

function toolKindOf(value: unknown, title = ''): ConversationToolKind {
  const normalized = `${stringOf(value)} ${title}`.trim().toLowerCase().replaceAll('_', '-');
  if (/file|edit|patch|diff|write/.test(normalized)) return 'file-edit';
  if (/search|grep|find|query/.test(normalized)) return 'search';
  if (/fetch|read|view|open|http|mcp|web/.test(normalized)) return 'fetch';
  if (/command|execute|terminal|shell|run/.test(normalized)) return 'command';
  return 'tool';
}

function toolStateOf(value: unknown): ConversationToolState {
  const normalized = stringOf(value, 'pending').toLowerCase().replaceAll('_', '-');
  if (['completed', 'complete', 'success', 'succeeded'].includes(normalized)) return 'completed';
  if (['failed', 'error', 'declined', 'cancelled', 'canceled'].includes(normalized)) return 'failed';
  if (['started', 'updated', 'running', 'in-progress', 'working'].includes(normalized)) return 'running';
  return 'pending';
}

function permissionOptionsOf(value: unknown): AgentPermissionOption[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((entry) => {
    if (typeof entry === 'string') {
      const labels: Record<string, string> = { accept: 'Allow', decline: 'Deny', cancel: 'Deny' };
      return [{ optionId: entry, name: labels[entry] ?? entry }];
    }
    const row = recordOf(entry);
    const optionId = stringOf(row?.optionId, stringOf(row?.id));
    if (!row || !optionId) return [];
    return [{ optionId, name: stringOf(row.name, stringOf(row.label, optionId)), kind: stringOf(row.kind) || undefined }];
  });
}

function defaultPermissionOptions(): AgentPermissionOption[] {
  return [
    { optionId: 'accept', name: 'Allow', kind: 'allow_once' },
    { optionId: 'decline', name: 'Deny', kind: 'reject_once' }
  ];
}

function displayTimestamp(item: AgentItem, fallback: number): number {
  return numberOf(item.providerMetadata?.startedAtMs, fallback);
}

export function displayItemFromAgentItem(item: AgentItem, timestampMs = Date.now()): ConversationDisplayItem {
  const metadata = item.providerMetadata;
  const kind = kindFor(item);
  const startedAt = displayTimestamp(item, timestampMs);
  if (kind === 'plan') return {
    kind,
    itemId: item.id,
    title: stringOf(metadata?.title, 'Plan'),
    steps: stepsOf(metadata?.steps ?? metadata?.entries),
    text: textOf(item.content),
    timestampMs: startedAt
  };
  if (kind === 'tasks') return {
    kind,
    itemId: item.id,
    title: stringOf(metadata?.title, 'Tasks'),
    tasks: tasksOf(metadata?.tasks ?? metadata?.steps),
    text: textOf(item.content),
    timestampMs: startedAt
  };
  if (kind === 'tool') {
    const title = stringOf(metadata?.title, stringOf(metadata?.name, item.type));
    const output = textOf(item.content) || stringOf(metadata?.output);
    const diff = stringOf(metadata?.diff, item.type === 'file-change' ? output : '');
    return {
      kind,
      itemId: item.id,
      title,
      toolKind: toolKindOf(metadata?.toolKind ?? item.type, title),
      state: toolStateOf(metadata?.state ?? metadata?.status),
      output: output || undefined,
      diff: diff || undefined,
      path: stringOf(metadata?.path) || undefined,
      summary: stringOf(metadata?.summary) || undefined,
      metadata,
      timestampMs: startedAt
    };
  }
  if (kind === 'subagent') return {
    kind,
    itemId: item.id,
    childId: stringOf(metadata?.childId, item.id),
    parentId: stringOf(metadata?.parentId, item.turnId ?? ''),
    label: stringOf(metadata?.label, 'Sub-agent'),
    state: stringOf(metadata?.state, 'working'),
    metadata,
    timestampMs: startedAt
  };
  if (kind === 'unknown') return { kind, itemId: item.id, text: textOf(item.content), metadata, timestampMs: startedAt };
  const completed = typeof metadata?.completed === 'boolean'
    ? metadata.completed
    : metadata?.streaming === true ? false : true;
  return {
    kind: kind as ConversationTextDisplayItem['kind'],
    itemId: item.id,
    text: textOf(item.content),
    completed,
    timestampMs: startedAt,
    metadata
  } as ConversationDisplayItem;
}

export function displayItemFromApproval(
  request: AgentPermissionRequest | (AgentApprovalRequest & { state: string }),
  timestampMs = Date.now()
): Extract<ConversationDisplayItem, { kind: 'approval' }> {
  const options = permissionOptionsOf(request.options);
  const toolTitle = 'toolTitle' in request ? request.toolTitle : request.title;
  return {
    kind: 'approval',
    itemId: `approval:${request.requestId}`,
    requestId: request.requestId,
    title: request.title || 'Approval requested',
    toolTitle,
    summary: request.description ?? toolTitle,
    state: request.state,
    options: options.length ? options : defaultPermissionOptions(),
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
    kind: entry.kind,
    itemId: entry.itemId,
    text: entry.text,
    completed: entry.completed,
    timestampMs: entry.timestampMs
  };
  if (entry.kind === 'tool') return {
    kind: 'tool',
    itemId: entry.itemId,
    title: entry.name,
    toolKind: toolKindOf('', entry.name),
    state: toolStateOf(entry.state),
    output: entry.summary,
    summary: entry.summary,
    timestampMs: entry.timestampMs
  };
  if (entry.kind === 'approval') return {
    kind: 'approval',
    itemId: entry.itemId,
    requestId: entry.requestId,
    title: 'Approval requested',
    toolTitle: entry.summary,
    summary: entry.summary,
    state: entry.state,
    options: defaultPermissionOptions(),
    timestampMs: entry.timestampMs
  };
  if (entry.kind === 'plan') return {
    kind: 'plan',
    itemId: entry.itemId,
    title: 'Plan',
    steps: planItemsOf(entry.items),
    timestampMs: entry.timestampMs
  };
  if (entry.kind === 'error') return {
    kind: 'error',
    itemId: entry.itemId,
    text: entry.message,
    timestampMs: entry.timestampMs,
    metadata: { code: entry.code, recoverable: entry.recoverable }
  };
  if (entry.kind === 'turn') return {
    kind: 'unknown',
    itemId: entry.itemId,
    text: `${entry.turnId} · ${entry.state}`,
    timestampMs: entry.timestampMs
  };
  return { kind: 'unknown', itemId: entry.itemId, text: '', timestampMs: 0 };
}

/** Merge typed provider items with legacy reducer entries without flattening them. */
export function typedConversationTimeline(
  items: readonly AgentItem[] = [],
  legacy: readonly ConversationTimelineEntry[] = [],
  timestamps: Readonly<Record<string, number>> = {},
  previous: readonly ConversationDisplayItem[] = []
): ConversationDisplayItem[] {
  const byId = new Map<string, ConversationDisplayItem>();
  legacy
    .filter((entry) => entry.kind !== 'turn')
    .forEach((entry) => byId.set(entry.itemId, displayItemFromLegacy(entry)));
  const legacyEnd = legacy.reduce((latest, entry) => Math.max(latest, entry.timestampMs), 0);
  items.forEach((item, index) => byId.set(
    item.id,
    displayItemFromAgentItem(item, timestamps[item.id] ?? displayTimestamp(item, legacyEnd + index + 1))
  ));
  const next = [...byId.values()].sort((left, right) => left.timestampMs - right.timestampMs);
  return reuseConversationDisplayItems(next, previous);
}

function shallowRecordEqual(
  left: Readonly<Record<string, unknown>> | undefined,
  right: Readonly<Record<string, unknown>> | undefined
): boolean {
  if (left === right) return true;
  if (!left || !right) return false;
  const leftKeys = Object.keys(left);
  if (leftKeys.length !== Object.keys(right).length) return false;
  return leftKeys.every((key) => Object.is(left[key], right[key]));
}

function sameDisplayItem(left: ConversationDisplayItem, right: ConversationDisplayItem): boolean {
  if (left === right) return true;
  if (left.kind !== right.kind || left.itemId !== right.itemId || left.timestampMs !== right.timestampMs) return false;
  const leftRecord = left as unknown as Readonly<Record<string, unknown>>;
  const rightRecord = right as unknown as Readonly<Record<string, unknown>>;
  const keys = Object.keys(leftRecord);
  if (keys.length !== Object.keys(rightRecord).length) return false;
  return keys.every((key) => {
    if (key === 'metadata') {
      return shallowRecordEqual(
        leftRecord[key] as Readonly<Record<string, unknown>> | undefined,
        rightRecord[key] as Readonly<Record<string, unknown>> | undefined
      );
    }
    return Object.is(leftRecord[key], rightRecord[key]);
  });
}

/** Reuse unchanged keyed rows so Svelte only invalidates the touched subtree. */
export function reuseConversationDisplayItems(
  next: readonly ConversationDisplayItem[],
  previous: readonly ConversationDisplayItem[]
): ConversationDisplayItem[] {
  if (previous.length === 0) return next.slice();
  const previousById = new Map(previous.map((item) => [item.itemId, item]));
  return next.map((item) => {
    const prior = previousById.get(item.itemId);
    return prior && sameDisplayItem(item, prior) ? prior : item;
  });
}

function eventIdentity(event: ConversationEvent, payload: StringRecord, prefix: string): string {
  const fromEvent = 'itemId' in event ? event.itemId : undefined;
  const turnId = 'turnId' in event ? event.turnId : undefined;
  return fromEvent
    || stringOf(payload.itemId)
    || stringOf(payload.toolCallId)
    || stringOf(payload.messageId)
    || `${prefix}:${turnId ?? stringOf(payload.turnId, String(event.sequence))}`;
}

function eventStartedAt(event: ConversationEvent, payload: StringRecord): number {
  const meta = recordOf(payload._meta);
  return numberOf(meta?.timestampMs, event.timestampMs);
}

function eventReplay(payload: StringRecord): boolean {
  return booleanOf(recordOf(payload._meta)?.replay);
}

function eventMetadata(
  event: ConversationEvent,
  payload: StringRecord,
  extra: Record<string, AgentConfigValue | undefined> = {}
): Record<string, AgentConfigValue> {
  const values = {
    ...(metadataRecord('providerMetadata' in event ? event.providerMetadata : undefined) ?? {}),
    ...(metadataRecord(payload.providerMetadata) ?? {}),
    ...extra,
    startedAtMs: eventStartedAt(event, payload),
    replay: eventReplay(payload)
  };
  const compact: Record<string, AgentConfigValue> = {};
  for (const [key, value] of Object.entries(values)) {
    if (value !== undefined) compact[key] = value;
  }
  return compact;
}

function toolItemFromPayload(event: ConversationEvent, payload: StringRecord, payloadKind: string): AgentItem {
  const title = stringOf(payload.title, stringOf(payload.name, stringOf(payload.command, stringOf(payload.path, 'Tool'))));
  const contentText = textFromValue(payload.content) || stringOf(payload.summary);
  const rawKind = stringOf(payload.toolKind, stringOf(payload.category, stringOf(payload.type)));
  const toolKind = toolKindOf(rawKind, title);
  const diff = stringOf(payload.diff)
    || firstNestedString(payload.content, ['diff', 'patch'])
    || (toolKind === 'file-edit' ? contentText : '');
  const path = stringOf(payload.path)
    || firstNestedString(payload.locations, ['path', 'uri'])
    || firstNestedString(payload.content, ['path', 'uri']);
  const status = payloadKind === 'toolCall' ? stringOf(payload.status, 'started') : stringOf(payload.status, stringOf(payload.state, 'updated'));
  return {
    id: eventIdentity(event, payload, 'tool'),
    type: 'mcp-tool',
    turnId: 'turnId' in event ? event.turnId : stringOf(payload.turnId) || undefined,
    content: contentText ? [{ channel: 'command-output', text: contentText }] : [],
    providerMetadata: eventMetadata(event, payload, {
      title,
      name: title,
      toolKind,
      state: status,
      status,
      summary: stringOf(payload.summary) || undefined,
      output: contentText || undefined,
      diff: diff || undefined,
      path: path || undefined,
      completed: toolStateOf(status) === 'completed'
    })
  };
}

function planItemFromPayload(event: ConversationEvent, payload: StringRecord): AgentItem {
  const entries = Array.isArray(payload.items) ? payload.items : Array.isArray(payload.entries) ? payload.entries : [];
  return {
    id: eventIdentity(event, { ...payload, itemId: stringOf(payload.planId) || payload.itemId }, 'plan'),
    type: 'plan',
    turnId: 'turnId' in event ? event.turnId : stringOf(payload.turnId) || undefined,
    content: [],
    providerMetadata: eventMetadata(event, payload, { title: stringOf(payload.title, 'Plan'), steps: entries as AgentConfigValue })
  };
}

export function agentItemFromEvent(event: ConversationEvent): AgentItem | null {
  const payload = event.payload as unknown as StringRecord;
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
      providerMetadata: eventMetadata(event, payload, metadataRecord(raw.providerMetadata) ?? {})
    };
  }

  const payloadKind = normalizedPayloadKind(payload);
  if (payloadKind === 'permissionRequest' || payloadKind === 'approval') return null;
  if (payloadKind === 'connection' || payloadKind === 'turn' || payloadKind === 'usage') return null;
  if (payloadKind === 'plan') return planItemFromPayload(event, payload);
  if (payloadKind === 'toolCall' || payloadKind === 'toolCallUpdate' || payloadKind === 'tool') {
    return toolItemFromPayload(event, payload, payloadKind);
  }
  if (payloadKind === 'turnDiff') {
    return toolItemFromPayload(event, { ...payload, title: stringOf(payload.title, 'Files changed'), toolKind: 'file-edit', status: stringOf(payload.status, 'completed') }, payloadKind);
  }
  if (payloadKind === 'availableCommandsUpdate') {
    const commands = Array.isArray(payload.availableCommands) ? payload.availableCommands : [];
    return {
      id: eventIdentity(event, { ...payload, itemId: `commands:${event.generation}` }, 'commands'),
      type: 'mcp-tool',
      content: [],
      providerMetadata: eventMetadata(event, payload, {
        title: 'Commands updated',
        toolKind: 'search',
        state: 'completed',
        completed: true,
        summary: `${commands.length} command${commands.length === 1 ? '' : 's'} available`
      })
    };
  }

  const id = eventIdentity(event, payload, payloadKind || 'item');
  const text = textFromValue(payload.content) || stringOf(payload.text, stringOf(payload.delta));
  const eventType = 'type' in event ? event.type : '';
  const channel = stringOf(payload.channel, eventType === 'content.delta' ? 'assistant' : 'assistant') as AgentContent['channel'];
  let type: AgentItemType = 'unknown';
  let contentChannel = channel;
  let completed = true;
  if (payloadKind === 'userMessage' || payloadKind === 'userMessageChunk') {
    type = 'user-message';
    contentChannel = 'assistant';
    completed = payloadKind === 'userMessage' ? booleanOf(payload.completed, true) : eventReplay(payload);
  } else if (payloadKind === 'assistantMessage' || payloadKind === 'assistantDelta' || payloadKind === 'agentMessageChunk') {
    type = 'assistant-message';
    contentChannel = 'assistant';
    completed = payloadKind === 'assistantMessage' && booleanOf(payload.completed, true);
  } else if (payloadKind === 'agentThoughtChunk' || channel.startsWith('reasoning')) {
    type = 'reasoning';
    contentChannel = 'reasoning';
    completed = false;
  } else if (payloadKind === 'error') {
    type = 'error';
    completed = true;
  } else if (channel === 'plan') {
    type = 'plan';
  } else if (channel === 'command-output') {
    type = 'command';
  } else if (channel === 'file-change-output') {
    type = 'file-change';
  }
  return {
    id,
    type,
    turnId: 'turnId' in event ? event.turnId : stringOf(payload.turnId) || undefined,
    content: text ? [{ channel: contentChannel, text }] : [],
    providerMetadata: eventMetadata(event, payload, { completed, streaming: !completed })
  };
}

export function conversationEventAppendsItemContent(event: ConversationEvent): boolean {
  const kind = normalizedPayloadKind(event.payload as unknown as StringRecord);
  return kind === 'assistantDelta'
    || kind === 'agentMessageChunk'
    || kind === 'agentThoughtChunk'
    || kind === 'toolCallUpdate'
    || kind === 'turnDiff'
    || ('type' in event && event.type === 'content.delta');
}

export function mergeAgentItem(items: readonly AgentItem[], incoming: AgentItem, append: boolean): AgentItem[] {
  const index = items.findIndex((item) => item.id === incoming.id);
  if (index < 0) return [...items, incoming];
  const existing = items[index];
  const replay = incoming.providerMetadata?.replay === true;
  let content = incoming.content.length ? incoming.content : existing.content;
  if (append && existing.content.length && incoming.content.length
    && existing.content[existing.content.length - 1].channel === incoming.content[0].channel) {
    const previous = existing.content[existing.content.length - 1];
    const nextText = incoming.content[0].text;
    const duplicateReplay = replay && (previous.text === nextText || previous.text.endsWith(nextText));
    content = duplicateReplay
      ? existing.content
      : [
        ...existing.content.slice(0, -1),
        { ...previous, text: `${previous.text}${nextText}` },
        ...incoming.content.slice(1)
      ];
  }
  const providerMetadata: Record<string, AgentConfigValue> = {
    ...(existing.providerMetadata ?? {}),
    ...(incoming.providerMetadata ?? {})
  };
  const startedAtMs = existing.providerMetadata?.startedAtMs ?? incoming.providerMetadata?.startedAtMs;
  if (startedAtMs !== undefined) providerMetadata.startedAtMs = startedAtMs;
  if (existing.type === incoming.type
    && existing.turnId === incoming.turnId
    && existing.content.length === content.length
    && existing.content.every((entry, contentIndex) => {
      const candidate = content[contentIndex];
      return entry.channel === candidate.channel
        && entry.text === candidate.text
        && entry.mimeType === candidate.mimeType;
    })
    && shallowRecordEqual(existing.providerMetadata, providerMetadata)) {
    return items as AgentItem[];
  }
  const next = items.slice();
  next[index] = { ...existing, ...incoming, content, providerMetadata };
  return next;
}

export function agentItemsFromEvents(events: readonly ConversationEvent[]): AgentItem[] {
  return events.reduce<AgentItem[]>((items, event) => {
    const item = agentItemFromEvent(event);
    return item ? mergeAgentItem(items, item, conversationEventAppendsItemContent(event)) : items;
  }, []);
}

export function permissionRequestFromEvent(event: ConversationEvent): AgentPermissionRequest | null {
  const payload = event.payload as unknown as StringRecord;
  const kind = normalizedPayloadKind(payload);
  if (kind !== 'permissionRequest' && kind !== 'approval') return null;
  const requestId = ('requestId' in event ? event.requestId : undefined) ?? stringOf(payload.requestId);
  if (!requestId) return null;
  const tool = recordOf(payload.toolCall);
  const toolTitle = stringOf(payload.toolTitle, stringOf(tool?.title, stringOf(payload.title, stringOf(payload.summary, 'Permission requested'))));
  const options = permissionOptionsOf(payload.options);
  return {
    ownedId: event.ownedId,
    generation: event.generation,
    requestId,
    turnId: (('turnId' in event ? event.turnId : undefined) || stringOf(payload.turnId)) || undefined,
    itemId: (('itemId' in event ? event.itemId : undefined) || stringOf(payload.itemId) || stringOf(payload.toolCallId)) || undefined,
    title: stringOf(payload.title, 'Approval needed'),
    toolTitle,
    description: stringOf(payload.description, stringOf(payload.summary)) || undefined,
    options: options.length ? options : defaultPermissionOptions(),
    state: stringOf(payload.state, 'requested')
  };
}

export function availableCommandsFromEvent(event: ConversationEvent): AgentCommandDescriptor[] | null {
  const payload = event.payload as unknown as StringRecord;
  if (normalizedPayloadKind(payload) !== 'availableCommandsUpdate') return null;
  if (!Array.isArray(payload.availableCommands)) return [];
  return payload.availableCommands.flatMap((entry) => {
    const row = recordOf(entry);
    const id = stringOf(row?.id, stringOf(row?.name));
    if (!row || !id) return [];
    const input = recordOf(row.input);
    return [{
      id,
      label: stringOf(row.label, id),
      description: stringOf(row.description) || undefined,
      inputHint: stringOf(row.inputHint, stringOf(input?.hint)) || undefined,
      providerMetadata: metadataRecord(row.providerMetadata)
    }];
  });
}

/** Pure replay/live projection used by the store and timeline script tests. */
export function displayItemsFromConversationEvents(events: readonly ConversationEvent[]): ConversationDisplayItem[] {
  const items = typedConversationTimeline(agentItemsFromEvents(events));
  const approvals = new Map<string, ConversationDisplayItem>();
  for (const event of events) {
    const request = permissionRequestFromEvent(event);
    if (!request) continue;
    approvals.set(request.requestId, displayItemFromApproval(request, event.timestampMs));
  }
  return [...items, ...approvals.values()].sort((left, right) => left.timestampMs - right.timestampMs);
}
