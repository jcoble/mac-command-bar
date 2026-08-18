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
  ConversationAttachment,
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

export type ConversationDisplayItem = (
  | (ConversationTextDisplayItem & { kind: 'user'; attachments?: readonly ConversationAttachment[] })
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
  | { kind: 'unknown'; itemId: string; text: string; timestampMs: number; metadata?: Record<string, AgentConfigValue> }
) & {
  readonly turnId?: string | null;
  readonly completed?: boolean;
};

export interface ConversationTurnGroup {
  readonly turnId: string | null;
  readonly items: readonly ConversationDisplayItem[];
  readonly workItemIds: readonly string[];
  readonly tailItemIds: readonly string[];
  readonly completed: boolean;
  readonly elapsedMs: number | null;
}

const FOLDABLE_TURN_KINDS = new Set<ConversationDisplayItem['kind']>([
  'reasoning',
  'tool',
  'command',
  'file',
  'subagent',
  'plan',
  'tasks'
]);

function displayItemCompleted(item: ConversationDisplayItem): boolean {
  if (typeof item.completed === 'boolean') return item.completed;
  if (item.kind === 'tool') return item.state === 'completed' || item.state === 'failed';
  if (item.kind === 'subagent') {
    return ['completed', 'complete', 'failed', 'cancelled', 'canceled', 'stopped', 'done']
      .includes(item.state.trim().toLowerCase());
  }
  if (item.kind === 'plan') {
    return item.steps.every((step) => !['pending', 'in-progress'].includes(step.state));
  }
  if (item.kind === 'tasks') {
    return item.tasks.every((task) => !['pending', 'in-progress'].includes(task.state));
  }
  if (item.kind === 'approval') {
    return !['requested', 'pending', 'running'].includes(item.state.trim().toLowerCase());
  }
  if (item.kind === 'input') return false;
  return true;
}

function turnGroup(turnId: string | null, items: readonly ConversationDisplayItem[], activeTurnId: string | null): ConversationTurnGroup {
  let tailStart = items.length;
  while (tailStart > 0 && items[tailStart - 1].kind === 'assistant') tailStart -= 1;
  const hasFoldableWork = items.some((item) => FOLDABLE_TURN_KINDS.has(item.kind));
  const workItemIds = hasFoldableWork
    ? items
      .filter((item, index) => FOLDABLE_TURN_KINDS.has(item.kind) || (item.kind === 'assistant' && index < tailStart))
      .map((item) => item.itemId)
    : [];
  const tailItemIds = items
    .filter((item, index) => item.kind === 'user' || (item.kind === 'assistant' && index >= tailStart))
    .map((item) => item.itemId);
  const firstTimestamp = items[0]?.timestampMs;
  const lastTimestamp = items[items.length - 1]?.timestampMs;
  const span = Number.isFinite(firstTimestamp) && Number.isFinite(lastTimestamp)
    ? Math.max(0, lastTimestamp - firstTimestamp)
    : 0;
  // Zero is not a length of time anyone worked for. Every row of an older
  // import carries the moment the import ran rather than the moment the work
  // happened, so the whole turn reads as one instant; the fold says "Worked"
  // in that case instead of claiming "Worked for 0.0s".
  const elapsedMs = span > 0 ? span : null;
  return {
    turnId,
    items,
    workItemIds,
    tailItemIds,
    completed: turnId !== activeTurnId && items.every(displayItemCompleted),
    elapsedMs
  };
}

/** The turn each row belongs to, filling in the ones a stored transcript lacks.
 *
 * A live turn is told to us: every row the agent sends carries the id of the
 * turn that produced it. A transcript read back out of the store carries none,
 * because the provider's own file never wrote one down. Those rows all arrived
 * with a null id, and grouping by that id put an entire resumed conversation
 * into a single turn — one fold covering everything, which is why a resumed
 * conversation read as a flat column of prose.
 *
 * A prompt is what starts a turn, so a user message is where a stored turn
 * begins and everything after it belongs to that turn until the next prompt.
 * The id is the first row's own id, which keeps it stable across re-renders —
 * the fold's open state is keyed by it. */
function turnIdsOf(items: readonly ConversationDisplayItem[]): (string | null)[] {
  let storedTurnId: string | null = null;
  return items.map((item) => {
    if (item.turnId) return item.turnId;
    if (item.kind === 'user' || storedTurnId === null) storedTurnId = `stored-turn:${item.itemId}`;
    return storedTurnId;
  });
}

/** Groups adjacent display rows without changing their transcript order. */
export function conversationTurnGroups(
  items: readonly ConversationDisplayItem[],
  activeTurnId: string | null = null
): readonly ConversationTurnGroup[] {
  const turnIds = turnIdsOf(items);
  const groups: ConversationTurnGroup[] = [];
  let groupItems: ConversationDisplayItem[] = [];
  let groupTurnId: string | null = null;
  const publish = (): void => {
    if (groupItems.length === 0) return;
    groups.push(turnGroup(groupTurnId, groupItems, activeTurnId));
    groupItems = [];
  };
  items.forEach((item, index) => {
    const itemTurnId = turnIds[index];
    if (groupItems.length > 0 && itemTurnId !== groupTurnId) publish();
    groupTurnId = itemTurnId;
    groupItems.push(item);
  });
  publish();
  return groups;
}

export function formatWorkedFor(elapsedMs: number): string {
  const safeElapsedMs = Math.max(0, elapsedMs);
  if (safeElapsedMs < 10_000) return `${(safeElapsedMs / 1_000).toFixed(1)}s`;
  const seconds = Math.round(safeElapsedMs / 1_000);
  if (seconds < 60) return `${seconds}s`;
  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}

/*
 * There is no render window here any more, and adding one back is a decision,
 * not a tidy-up.
 *
 * What stood here kept the last 120 items and put the rest behind a "Show
 * earlier" button. The count was of stored events, not of messages, and a
 * transcript carries far more of the former than the latter — usage and config
 * updates outnumber the writing several times over. So the button hid whole
 * conversations while the window filled with rows that draw nothing, and it
 * hid them by default: what the reader saw first was the boundary.
 *
 * What the database holds, the transcript shows. If a transcript ever needs a
 * budget, it belongs in a virtualizer that renders every item and only mounts
 * the visible ones — never in a button that asks the reader to request their
 * own history back.
 *
 * "Load more" means one thing now: fetching older turns that are NOT in the
 * database yet, from the provider's own transcript on disk. That is
 * `extendAgentConversationImportFromTauri`, and it is a different act from
 * drawing a row this app already stores.
 */

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
  // The web is checked before searching in general, so a web search is drawn
  // with a globe and a local one with a magnifier. Both are searches; only one
  // of them left the machine.
  if (/web/.test(normalized)) return 'fetch';
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
  const turnId = item.turnId ?? null;
  if (kind === 'plan') return {
    kind,
    itemId: item.id,
    turnId,
    title: stringOf(metadata?.title, 'Plan'),
    steps: stepsOf(metadata?.steps ?? metadata?.entries),
    text: textOf(item.content),
    timestampMs: startedAt
  };
  if (kind === 'tasks') return {
    kind,
    itemId: item.id,
    turnId,
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
      turnId,
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
    turnId,
    childId: stringOf(metadata?.childId, item.id),
    parentId: stringOf(metadata?.parentId, item.turnId ?? ''),
    label: stringOf(metadata?.label, 'Sub-agent'),
    state: stringOf(metadata?.state, 'working'),
    metadata,
    timestampMs: startedAt
  };
  if (kind === 'unknown') return { kind, itemId: item.id, turnId, text: textOf(item.content), metadata, timestampMs: startedAt };
  const completed = typeof metadata?.completed === 'boolean'
    ? metadata.completed
    : metadata?.streaming === true ? false : true;
  return {
    kind: kind as ConversationTextDisplayItem['kind'],
    itemId: item.id,
    turnId,
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
    turnId: request.turnId ?? null,
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
    turnId: request.turnId ?? null,
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
    // The body and the row's one line are different things. This handed the
    // summary over as both, so a row either repeated itself or, far more often,
    // opened onto nothing at all because the summary was all that was stored.
    output: entry.output,
    diff: entry.diff,
    path: entry.path,
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
    text: conversationErrorText(entry.message),
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

/**
 * What an error card says. An error that arrives without a message still has
 * to say something: a card with an empty body tells the reader only that the
 * app is confused.
 */
export function conversationErrorText(message: string | null | undefined): string {
  return (message ?? '').trim() || 'The agent reported an error without saying what went wrong.';
}

/** Merge typed provider items with legacy reducer entries without flattening them. */
export function typedConversationTimeline(
  items: readonly AgentItem[] = [],
  legacy: readonly ConversationTimelineEntry[] = [],
  timestamps: Readonly<Record<string, number>> = {},
  previous: readonly ConversationDisplayItem[] = [],
  sentAttachments: Readonly<Record<string, readonly ConversationAttachment[]>> = {}
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
  const next = [...byId.values()]
    .map((item) => {
      const sent = item.kind === 'user' ? sentAttachments[item.itemId] : undefined;
      return sent?.length ? { ...item, attachments: sent } : item;
    })
    .sort((left, right) => left.timestampMs - right.timestampMs);
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

/** What a work row calls itself.
 *
 * A provider names its own functions — `apply_patch`, `web_search`, `Edit` —
 * and that is the provider's vocabulary, not the reader's. A row says what was
 * done instead, and the provider's own detail stays in the line beside it.
 *
 * Only the two kinds a transcript is mostly made of are renamed, and only when
 * the call itself says so. A shell call still names the command it ran, and a
 * `Grep` still says `Grep`, because that is the work and any broader guess
 * would mislabel it. */
function plainToolTitle(toolKind: ConversationToolKind, name: string): string {
  if (toolKind === 'file-edit') return 'Edited a file';
  if (/web/i.test(name)) return 'Searched the web';
  return name;
}

function toolItemFromPayload(event: ConversationEvent, payload: StringRecord, payloadKind: string): AgentItem {
  // A row that was handed its own title keeps it: the only caller that does so
  // is the whole-turn file change, which already knows it covers several files.
  const givenTitle = stringOf(payload.title);
  const name = givenTitle || stringOf(payload.name, stringOf(payload.command, stringOf(payload.path, 'Tool')));
  // A summary is the row's one-line preview, and nothing else. It used to
  // stand in for the body as well, which printed the same sentence twice —
  // once on the row and again inside it when it was opened — and, on a file
  // edit with no patch attached, offered the file's path as though it were the
  // change. Only real content is a body.
  // `output` is what a stored call answered; `content` is what a live ACP
  // update carries. A row reads whichever it was given.
  const contentText = textFromValue(payload.output) || textFromValue(payload.content);
  const rawKind = stringOf(payload.toolKind, stringOf(payload.category, stringOf(payload.type)));
  const toolKind = toolKindOf(rawKind, name);
  const title = givenTitle || plainToolTitle(toolKind, name);
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
    return toolItemFromPayload(event, { ...payload, title: stringOf(payload.title, 'Edited files'), toolKind: 'file-edit', status: stringOf(payload.status, 'completed') }, payloadKind);
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

  if (payloadKind === 'error') {
    // Same identity the reducer gives its own error entry, so the typed item
    // and the legacy entry are one card rather than two — and the message
    // lives under `message`, which the generic text lookup below never reads.
    return {
      id: `error:${event.generation}:${event.sequence}`,
      type: 'error',
      turnId: 'turnId' in event ? event.turnId : stringOf(payload.turnId) || undefined,
      content: [{ channel: 'assistant', text: conversationErrorText(stringOf(payload.message)) }],
      providerMetadata: eventMetadata(event, payload, {
        completed: true,
        code: stringOf(payload.code) || undefined,
        recoverable: booleanOf(payload.recoverable, true)
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
  const merged = mergedAgentItem(existing, incoming, append);
  if (mergeLeftItemUnchanged(existing, merged)) return items as AgentItem[];
  const next = items.slice();
  next[index] = merged;
  return next;
}

function mergedAgentItem(existing: AgentItem, incoming: AgentItem, append: boolean): AgentItem {
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
  return { ...existing, ...incoming, content, providerMetadata };
}

/** Whether a merge left the item exactly as it was, so callers can keep the
 * previous array and every identity that hangs off it. */
function mergeLeftItemUnchanged(existing: AgentItem, merged: AgentItem): boolean {
  if (existing.type !== merged.type || existing.turnId !== merged.turnId) return false;
  if (existing.content !== merged.content) {
    if (existing.content.length !== merged.content.length) return false;
    for (let contentIndex = 0; contentIndex < existing.content.length; contentIndex += 1) {
      const previous = existing.content[contentIndex];
      const candidate = merged.content[contentIndex];
      if (previous.channel !== candidate.channel
        || previous.text !== candidate.text
        || previous.mimeType !== candidate.mimeType) return false;
    }
  }
  return shallowRecordEqual(existing.providerMetadata, merged.providerMetadata);
}

/** Snapshot replay owns a private, unpublished array, so it can update by an
 * id index instead of searching and copying a growing array for every event. */
export function mergeAgentItemForReplay(
  items: AgentItem[],
  indexes: Map<string, number>,
  incoming: AgentItem,
  append: boolean
): void {
  const index = indexes.get(incoming.id);
  if (index === undefined) {
    indexes.set(incoming.id, items.length);
    items.push(incoming);
    return;
  }
  items[index] = mergedAgentItem(items[index], incoming, append);
}

export function agentItemsFromEvents(events: readonly ConversationEvent[]): AgentItem[] {
  const items: AgentItem[] = [];
  const indexes = new Map<string, number>();
  for (const event of events) {
    const item = agentItemFromEvent(event);
    if (item) mergeAgentItemForReplay(items, indexes, item, conversationEventAppendsItemContent(event));
  }
  return items;
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
