import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  applyAgentConversationEvent,
  applyAgentConversationSnapshot,
  applyChildConversationTranscript,
  ensureConversationSession,
  evictConversationSession,
  getConversationSession,
  recordAgentConversationPresenceEvent,
  setConversationDraft,
  setConversationConnection,
  setConversationAttachments,
  setConversationProviderNotice,
  recordSentConversationAttachments,
  restoreSentConversationAttachments,
  setConversationSending,
  beginConversationConfigChange,
  confirmConversationConfigChange,
  failConversationConfigChange,
  setConversationCapabilities,
  setConversationCapabilityError,
  setConversationWriterLeaseTransition,
  clearConversationWriterLeaseTransition,
  beginLoadingOlderConversationEvents,
  beginLoadingNewerConversationEvents,
  failLoadingOlderConversationEvents,
  failLoadingNewerConversationEvents,
  prependOlderConversationEvents,
  appendNewerConversationEvents
} from './conversationStore.svelte.ts';
import type {
  AgentCapabilities,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConfigOption,
  AgentConfigValue,
  AgentUserInputResponse,
  ConversationTranscriptSnapshot,
  ConversationAttachment,
  AgentConversationHandoffRequest,
  AgentConversationHandoffReceipt,
  AgentConversationHandoffDirection,
  AgentConversationHandoffPhase,
  AgentWriterLeaseOwner,
  AgentWriterLeaseTransition
} from './conversationTypes.ts';
import {
  applyConversationHandoffReceipt,
  createConversationHandoffRequest,
  handoffGenerationMatches
} from './conversationTypes.ts';
import {
  readAgentConversationCapabilitiesFromTauri,
  listAgentConversationEventsBeforeFromTauri,
  listAgentConversationEventsAfterFromTauri,
  extendAgentConversationImportFromTauri,
  readAgentConversationSnapshotFromTauri,
  changeAgentConversationCheckoutFromTauri,
  writeTerminalSessionFromTauri,
  registerAgentConversationStream,
  type StreamEnvelope,
  type ProjectionStreamRegistration,
  type AgentConversationSessionRecord
} from '$lib/tauriSource';
import { hasBackendCapability } from '../backendCapabilities.ts';
import { shouldClearConversationSending } from './conversationReducer.ts';
import { rail, updateOwnedSession } from '../stores/sessionRailStore.svelte';
import { sessionTitleFromPrompt } from '../sessionStrip.ts';
import {
  decideConversationActivation,
  generationForSend,
  sendSupportsImages,
  sendTargetGeneration,
  shouldReviveBeforeSend
} from './conversationActivation.ts';
import { ConversationDraftPersistence } from './conversationDraftPersistence.ts';
import { invokeConversationCommand as invoke } from './conversationInvoke.ts';
import {
  revokeTrackedObjectUrl,
  trackTauriListener
} from '../resourceDiagnostics.svelte.ts';

let conversationStream: ProjectionStreamRegistration | null = null;
let unlistenTitles: (() => void) | null = null;
let conversationEventsSetup: Promise<void> | null = null;
let conversationEventsDisposed = false;
let conversationEventsGeneration = 0;
const resyncing = new Map<string, Promise<void>>();
const ensuring = new Map<string, { signature: string; work: Promise<AgentConversationConnection | null> }>();
const terminalProjections = new Map<string, string>();
const readVersions = new Map<string, number>();
const ACP_LIVE_CONVERSATION_EVENTS_CAPABILITY = 'acpLiveConversationEvents';
const sessionDraftPersistence = new ConversationDraftPersistence(
  {
    set: (ownedId, text) => invoke('agent_conversation_set_session_draft', { ownedId, text }),
    get: (ownedId) => invoke<string | null>('agent_conversation_get_session_draft', { ownedId }),
    clear: (ownedId) => invoke('agent_conversation_clear_session_draft', { ownedId })
  },
  setConversationDraft
);

export function persistConversationSessionDraft(ownedId: string, text: string): void {
  if (isTauri()) sessionDraftPersistence.schedule(ownedId, text);
}

export async function flushConversationSessionDraft(ownedId: string): Promise<void> {
  if (isTauri()) await sessionDraftPersistence.flush(ownedId);
}

export async function loadConversationSessionDraft(ownedId: string): Promise<void> {
  if (isTauri()) await sessionDraftPersistence.load(ownedId);
}

export async function clearConversationSessionDraft(ownedId: string): Promise<void> {
  if (isTauri()) await sessionDraftPersistence.clear(ownedId);
}

/** Loads one child transcript through the typed conversation command boundary. */
interface ChildTranscriptRead {
  generation: number;
  childSessionId: string;
}

const childTranscriptReads = new Map<string, ChildTranscriptRead>();

export function cancelChildConversationTranscriptRead(ownedId: string): void {
  childTranscriptReads.delete(ownedId);
}

export async function readChildConversationTranscript(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  nativeSessionId: string;
  childSessionId: string;
}): Promise<void> {
  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
  const active = childTranscriptReads.get(input.ownedId);
  if (active?.generation === generation && active.childSessionId === input.childSessionId) return;
  const readToken = {
    generation,
    childSessionId: input.childSessionId
  };
  childTranscriptReads.set(input.ownedId, readToken);
  let snapshot: ConversationTranscriptSnapshot;
  try {
    snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
      provider: input.provider,
      nativeSessionId: input.nativeSessionId,
      childSessionId: input.childSessionId
    });
  } catch (error) {
    if (childTranscriptReads.get(input.ownedId) === readToken) childTranscriptReads.delete(input.ownedId);
    throw error;
  }
  const current = getConversationSession(input.ownedId);
  if (childTranscriptReads.get(input.ownedId) !== readToken) return;
  childTranscriptReads.delete(input.ownedId);
  if (
    !current
    || current.generation !== readToken.generation
    || current.selectedChildId !== readToken.childSessionId
  ) return;
  applyChildConversationTranscript(input.ownedId, input.childSessionId, snapshot.messages);
}

type SavedAttachment = Omit<ConversationAttachment, 'previewUrl' | 'originalUrl'> & { byteLength: number };
type AttachmentWithBytes = ConversationAttachment & { byteLength?: number; bytes?: number[] };

export interface AgentPromptTextContent {
  type: 'text';
  text: string;
}

export interface AgentPromptImageContent {
  type: 'image';
  mimeType: string;
  data: number[];
  name: string;
}

export type AgentPromptContent = AgentPromptTextContent | AgentPromptImageContent;

/** Build ordered ACP content blocks without turning typed images into Markdown. */
export function buildConversationPrompt(
  text: string,
  attachments: readonly AttachmentWithBytes[],
  supportsImages: boolean
): { text: string; content: AgentPromptContent[] } {
  if (attachments.length > 0 && !supportsImages) {
    throw new Error('This conversation provider does not advertise image prompts');
  }
  const content: AgentPromptContent[] = [];
  if (text.length > 0) content.push({ type: 'text', text });
  for (const attachment of attachments) {
    if (!attachment.bytes?.length) throw new Error(`Attachment ${attachment.name} is not available for image upload`);
    content.push({
      type: 'image',
      mimeType: attachment.mimeType,
      data: attachment.bytes,
      name: attachment.name
    });
  }
  return { text, content };
}

/** Saves one clipboard image through the owner-scoped native attachment vault. */
export async function saveConversationClipboardImage(
  ownedId: string,
  file: File
): Promise<ConversationAttachment> {
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
  const saved = await invoke<SavedAttachment>('save_agent_conversation_attachment', {
    ownedId,
    mimeType: file.type,
    bytes
  });
  const attachment = { ...saved, bytes };
  return restoreConversationAttachmentPreview(attachment) as AttachmentWithBytes;
}

/** Revoke only URLs owned by this surface; the managed path never goes through the DOM. */
export function cleanupConversationAttachmentPreview(attachment: ConversationAttachment): void {
  if (attachment.previewUrl.startsWith('blob:')) revokeTrackedObjectUrl(attachment.previewUrl);
}

/** Drop frontend-only conversation data after its workspace has been saved. */
export function releaseConversationForRead(ownedId: string): void {
  readVersions.set(ownedId, (readVersions.get(ownedId) ?? 0) + 1);
  resyncing.delete(ownedId);
  cancelChildConversationTranscriptRead(ownedId);
  const state = getConversationSession(ownedId);
  if (!state) return;
  const previewUrls = new Set([
    ...state.attachments,
    ...state.unclaimedSentAttachments,
    ...Object.values(state.sentAttachments).flat()
  ].map((attachment) => attachment.previewUrl));
  for (const previewUrl of previewUrls) {
    if (previewUrl.startsWith('blob:')) revokeTrackedObjectUrl(previewUrl);
  }
  evictConversationSession(ownedId);
}

async function hydrateAttachmentBytes(attachment: AttachmentWithBytes): Promise<AttachmentWithBytes> {
  if (attachment.bytes?.length) return attachment;
  if (!attachment.path) throw new Error(`Attachment ${attachment.name} has no managed path`);
  const response = await fetch(isTauri() ? convertFileSrc(attachment.path) : attachment.path);
  if (!response.ok) throw new Error(`Could not read attachment ${attachment.name}`);
  return {
    ...attachment,
    bytes: Array.from(new Uint8Array(await response.arrayBuffer()))
  };
}

function attachmentDisplayMetadata(attachment: ConversationAttachment): ConversationAttachment {
  const metadata: ConversationAttachment = {
    id: attachment.id,
    name: attachment.name,
    mimeType: attachment.mimeType,
    path: attachment.path,
    previewUrl: attachment.previewUrl
  };
  if (attachment.thumbnailMimeType !== undefined) metadata.thumbnailMimeType = attachment.thumbnailMimeType;
  if (attachment.thumbnailPath !== undefined) metadata.thumbnailPath = attachment.thumbnailPath;
  if (attachment.thumbnailByteLength !== undefined) {
    metadata.thumbnailByteLength = attachment.thumbnailByteLength;
  }
  return metadata;
}

/** Rebuild a preview URL from a backend-owned attachment after a workspace restore. */
export function restoreConversationAttachmentPreview(
  attachment: Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string }
): ConversationAttachment {
  const previewUrl = attachment.previewUrl
    ?? (attachment.thumbnailPath
      ? (isTauri() ? convertFileSrc(attachment.thumbnailPath) : attachment.thumbnailPath)
      : '');
  return { ...attachment, previewUrl };
}

/** Restore attachment metadata supplied by the existing owner-scoped vault. */
export async function restoreConversationAttachments(
  ownedId: string,
  saved: readonly (Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[] = []
): Promise<ConversationAttachment[]> {
  if (saved.length > 0) {
    const restored = saved.map(restoreConversationAttachmentPreview);
    setConversationAttachments(ownedId, restored);
    return restored;
  }
  if (!isTauri()) return [];
  try {
    const records = await invoke<(Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[]>(
      'read_agent_conversation_attachments',
      { ownedId }
    );
    const restored = Array.isArray(records) ? records.map(restoreConversationAttachmentPreview) : [];
    setConversationAttachments(ownedId, restored);
    return restored;
  } catch (_error) {
    // The invoke seam logged the sanitized failure before this fallback runs.
    // Older controller builds have no listing command. The draft and saved ids
    // remain intact so a later controller can hydrate them without data loss.
    return [];
  }
}

/** Ask the owner-validated vault to delete one managed file, then revoke its URL. */
export async function removeConversationAttachment(
  ownedId: string,
  attachment: ConversationAttachment
): Promise<void> {
  await cleanupConversationAttachment(ownedId, attachment);
  const state = getConversationSession(ownedId);
  if (state) setConversationAttachments(ownedId, state.attachments.filter((item) => item.id !== attachment.id));
}

/** Cleanup for a newly saved attachment that has not been added to the store yet. */
export async function cleanupConversationAttachment(
  ownedId: string,
  attachment: ConversationAttachment
): Promise<void> {
  try {
    if (isTauri()) {
      await invoke('delete_agent_conversation_attachment', {
        request: { ownedId, attachmentId: attachment.id, path: attachment.path }
      });
    }
  } finally {
    cleanupConversationAttachmentPreview(attachment);
  }
}

export async function loadConversationCapabilities(
  ownedId: string,
  provider: AgentConversationProvider
): Promise<AgentCapabilities | null> {
  if (!isTauri()) return null;
  const generation = getConversationSession(ownedId)?.generation ?? 0;
  try {
    const capabilities = await readAgentConversationCapabilitiesFromTauri(ownedId);
    if (!capabilities) return null;
    if (capabilities.provider !== provider) throw new Error('Capability provider does not match this session');
    setConversationCapabilities(ownedId, generation, capabilities);
    return capabilities;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setConversationCapabilityError(ownedId, message);
    throw error;
  }
}

/** Applies one advertised config choice and rejects stale provider responses. */
export async function setConversationConfigOption(
  ownedId: string,
  optionId: string,
  value: AgentConfigValue
): Promise<AgentCapabilities | null> {
  const state = getConversationSession(ownedId);
  const option = state?.capabilities?.configOptions.find((candidate) => candidate.id === optionId);
  if (!state || !option || (option.choices && !option.choices.some((choice) => JSON.stringify(choice.value) === JSON.stringify(value)))) {
    throw new Error('This configuration value is not advertised for the session');
  }
  if (!beginConversationConfigChange(ownedId, optionId, value)) throw new Error('Configuration change is not available');
  try {
    if (!isTauri()) throw new Error('The structured conversation runtime is unavailable');
    const response = await invoke<{ capabilities?: AgentCapabilities; configOptions?: AgentConfigOption[] }>(
      'set_agent_conversation_config_option',
      { request: { ownedId, generation: state.generation, optionId, value } }
    );
    const latest = getConversationSession(ownedId);
    if (!latest || latest.generation !== state.generation) {
      throw new Error('Configuration response belongs to a stale conversation generation');
    }
    const capabilities = response?.capabilities
      ?? (response?.configOptions ? { ...state.capabilities, configOptions: response.configOptions } as AgentCapabilities : state.capabilities);
    if (!capabilities || capabilities.provider !== state.provider) {
      throw new Error('Configuration response provider does not match this session');
    }
    confirmConversationConfigChange(ownedId, optionId, value, capabilities);
    return capabilities;
  } catch (error) {
    if (getConversationSession(ownedId)?.generation === state.generation) {
      failConversationConfigChange(ownedId, optionId, error instanceof Error ? error.message : String(error));
    }
    throw error;
  }
}

/** Starts best-effort terminal transcript projection for one conversation owner. */
export function startConversationTerminalProjection(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  nativeSessionId: string;
}): void {
  if (!isTauri()) return;
  const signature = `${input.provider}:${input.nativeSessionId}`;
  if (terminalProjections.get(input.ownedId) === signature) return;
  terminalProjections.set(input.ownedId, signature);
  void invoke<Record<string, never>>('start_agent_conversation_terminal_projection', {
    request: input
  }).catch((_error) => {
    // The invoke seam logged the sanitized failure before this retry state resets.
    // The transcript may not exist until the agent accepts its first prompt.
    // Dropping the signature lets the next activation register again.
    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
  });
}

/** Stops terminal transcript projection when this surface no longer needs it. */
export function stopConversationTerminalProjection(ownedId: string): void {
  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
  void invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
}

type HandoffInput = Omit<AgentConversationHandoffRequest, 'phase'>;

function handoffTargetOwner(direction: AgentConversationHandoffDirection): AgentWriterLeaseOwner {
  return direction === 'structured-to-terminal' ? 'terminal' : 'structured';
}

function applyHandoffReceiptToStore(receipt: AgentConversationHandoffReceipt): void {
  const state = getConversationSession(receipt.ownedId);
  if (!state || !handoffGenerationMatches(receipt, receipt.ownedId, state.generation)) return;
  if (receipt.phase === 'prepare' || receipt.phase === 'prepared') {
    const transition: AgentWriterLeaseTransition = {
      ownedId: receipt.ownedId,
      generation: receipt.generation,
      from: receipt.previousOwner,
      to: handoffTargetOwner(receipt.direction),
      state: 'requested'
    };
    setConversationWriterLeaseTransition(transition);
    return;
  }
  const next = applyConversationHandoffReceipt(state, receipt);
  Object.assign(state, next);
}

/** Invokes one handoff phase and keeps its writer-lease transition consistent. */
async function invokeHandoff(
  input: HandoffInput,
  phase: AgentConversationHandoffPhase
): Promise<AgentConversationHandoffReceipt> {
  if (!isTauri()) throw new Error('Native handoff is available only in the desktop application');
  const request = createConversationHandoffRequest({ ...input, phase });
  const current = getConversationSession(request.ownedId);
  if (!current || current.generation !== request.generation) {
    throw new Error('Handoff request belongs to a stale conversation generation');
  }
  try {
    const receipt = await invoke<AgentConversationHandoffReceipt>('handoff_agent_conversation', { request });
    if (!handoffGenerationMatches(receipt, request.ownedId, request.generation)) {
      throw new Error('Handoff receipt belongs to a stale conversation generation');
    }
    applyHandoffReceiptToStore(receipt);
    return receipt;
  } catch (error) {
    clearConversationWriterLeaseTransition(request.ownedId, request.generation);
    throw error;
  }
}

export function prepareConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
  return invokeHandoff(input, 'prepare');
}

export function commitConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
  return invokeHandoff(input, 'commit');
}

export function rollbackConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
  return invokeHandoff(input, 'rollback');
}

/** Hang the saved screenshots back on the replayed user messages that named
 * them. The in-memory hold a send uses is gone after a restart, so the ids the
 * journal carries are the only way the transcript can find the files again.
 *
 * Never awaited by the caller: the transcript is already on screen by then, and
 * a message whose screenshot has not landed yet is a message with an empty
 * thumbnail box, not a message that waits. One read covers the whole session,
 * and messages already carrying their screenshots are skipped, so a repeat
 * replay does no disk work at all. */
async function hydrateSentConversationAttachments(
  ownedId: string,
  generation: number,
  events: readonly AgentConversationEvent[]
): Promise<void> {
  const current = getConversationSession(ownedId);
  if (!current || current.generation !== generation) return;
  const alreadyShown = current.sentAttachments;
  const wanted = new Map<string, string[]>();
  for (const event of events) {
    const payload = event.payload;
    if (
      payload.kind === 'userMessage'
      && payload.attachmentIds?.length
      && !alreadyShown[payload.itemId]?.length
    ) {
      wanted.set(payload.itemId, payload.attachmentIds);
    }
  }
  if (wanted.size === 0 || !isTauri()) return;
  let records: (Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[];
  try {
    records = await invoke<(Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[]>(
      'read_agent_conversation_attachments',
      { ownedId }
    );
  } catch (_error) {
    // The invoke seam logged the sanitized failure. A message that cannot find
    // its screenshot still shows its text, so there is nothing to repair here.
    return;
  }
  if (!Array.isArray(records)) return;
  const byId = new Map(records.map((record) => [record.id, restoreConversationAttachmentPreview(record)]));
  const resolved: Record<string, ConversationAttachment[]> = {};
  for (const [itemId, ids] of wanted) {
    const attachments = ids
      .map((id) => byId.get(id))
      .filter((attachment): attachment is ConversationAttachment => !!attachment);
    if (attachments.length) resolved[itemId] = attachments;
  }
  if (Object.keys(resolved).length) restoreSentConversationAttachments(ownedId, resolved, generation);
}

async function resyncConversation(ownedId: string): Promise<void> {
  const existing = resyncing.get(ownedId);
  if (existing) return existing;
  const work = (async () => {
    const readVersion = (readVersions.get(ownedId) ?? 0) + 1;
    readVersions.set(ownedId, readVersion);
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const snapshot = await readAgentConversationSnapshotFromTauri(ownedId);
      if (!snapshot || readVersions.get(ownedId) !== readVersion) return;
      const sequenceBeforeApply = getConversationSession(ownedId)?.lastSequence ?? 0;
      applyAgentConversationSnapshot(snapshot);
      void hydrateSentConversationAttachments(ownedId, snapshot.connection.generation, snapshot.events);
      if (sequenceBeforeApply <= snapshot.lastSequence) return;
    }
  })().finally(() => {
    if (resyncing.get(ownedId) === work) resyncing.delete(ownedId);
  });
  resyncing.set(ownedId, work);
  return work;
}

export async function loadConversationForRead(ownedId: string): Promise<void> {
  const existing = resyncing.get(ownedId);
  if (existing) return existing;
  const generation = getConversationSession(ownedId)?.generation ?? 0;
  const readVersion = (readVersions.get(ownedId) ?? 0) + 1;
  readVersions.set(ownedId, readVersion);
  let work: Promise<void>;
  work = (async () => {
    const snapshot = await readAgentConversationSnapshotFromTauri(ownedId);
    const current = getConversationSession(ownedId);
    if (
      !snapshot
      || readVersions.get(ownedId) !== readVersion
      || current?.generation !== generation
    ) return;
    applyAgentConversationSnapshot(snapshot);
    void hydrateSentConversationAttachments(ownedId, snapshot.connection.generation, snapshot.events);
  })().finally(() => {
    if (resyncing.get(ownedId) === work) resyncing.delete(ownedId);
  });
  resyncing.set(ownedId, work);
  return work;
}

/** How much older history one scroll to the top reads, in bytes of stored
 * event. The same unit the transcript is read off disk in, and the same unit
 * the timeline already guesses a row's height in.
 *
 * A page keeps the reader where they were, so reaching the start of a long
 * session costs one scroll gesture per page. At a quarter of a megabyte a ten
 * megabyte conversation took forty of them. The rows are virtualized, so a
 * bigger page costs a longer read and no more drawing; what it buys is a
 * quarter as many waits. */
const EVENT_PAGE_BYTES = 1024 * 1024;

/**
 * Reads the page of stored events just older than the transcript and puts it in
 * front. Opening a conversation ships one screen; this is how the rest of a
 * long session is reached.
 */
export async function loadOlderConversationEvents(ownedId: string): Promise<void> {
  if (!beginLoadingOlderConversationEvents(ownedId)) return;
  const started = getConversationSession(ownedId);
  const before = started?.oldestLoadedSequence ?? 0;
  const generation = started?.generation ?? 0;
  const readVersion = readVersions.get(ownedId) ?? 0;
  try {
    const page = await listAgentConversationEventsBeforeFromTauri(
      ownedId,
      before,
      EVENT_PAGE_BYTES
    );
    const current = getConversationSession(ownedId);
    if (
      readVersions.get(ownedId) !== readVersion
      || current?.generation !== generation
      || current.oldestLoadedSequence !== before
    ) {
      failLoadingOlderConversationEvents(ownedId);
      return;
    }
    if (!page) {
      failLoadingOlderConversationEvents(ownedId);
      return;
    }
    if (page.events.length || page.hasMore) {
      prependOlderConversationEvents(ownedId, page);
      void hydrateSentConversationAttachments(
        ownedId,
        generation,
        getConversationSession(ownedId)?.loadedEvents ?? []
      );
      return;
    }
    // The database is exhausted, which is not the same as the conversation
    // being. A session picked up from a past transcript holds only the tail
    // that was read at the time; the rest is still on disk behind a byte
    // cursor. Read the next chunk into the database and ask again. A session
    // started here has no transcript behind it and reports nothing added,
    // which is how this stops.
    const extended = await extendAgentConversationImportFromTauri(ownedId);
    const afterExtend = getConversationSession(ownedId);
    if (
      readVersions.get(ownedId) !== readVersion
      || afterExtend?.generation !== generation
      || afterExtend.oldestLoadedSequence !== before
    ) {
      failLoadingOlderConversationEvents(ownedId);
      return;
    }
    if (!extended) {
      failLoadingOlderConversationEvents(ownedId);
      return;
    }
    // Only the file running out means the conversation has none left. A reach
    // that added nothing but did not reach the start walked past a stretch
    // holding nothing worth showing, and there is more behind it.
    if (!extended.added) {
      prependOlderConversationEvents(ownedId, {
        events: [],
        hasMore: !extended.reachedStart
      });
      return;
    }
    const grown = await listAgentConversationEventsBeforeFromTauri(
      ownedId,
      before,
      EVENT_PAGE_BYTES
    );
    const afterGrow = getConversationSession(ownedId);
    if (
      readVersions.get(ownedId) !== readVersion
      || afterGrow?.generation !== generation
      || afterGrow.oldestLoadedSequence !== before
    ) {
      failLoadingOlderConversationEvents(ownedId);
      return;
    }
    prependOlderConversationEvents(ownedId, {
      events: grown?.events ?? [],
      hasMore: grown?.hasMore || !extended.reachedStart
    });
    void hydrateSentConversationAttachments(
      ownedId,
      generation,
      getConversationSession(ownedId)?.loadedEvents ?? []
    );
  } catch {
    failLoadingOlderConversationEvents(ownedId);
  }
}

/** Reads the next stored page after a window whose newest end was trimmed. */
export async function loadNewerConversationEvents(ownedId: string): Promise<void> {
  if (!beginLoadingNewerConversationEvents(ownedId)) return;
  const started = getConversationSession(ownedId);
  const after = started?.newestLoadedSequence ?? 0;
  const generation = started?.generation ?? 0;
  const readVersion = readVersions.get(ownedId) ?? 0;
  try {
    const page = await listAgentConversationEventsAfterFromTauri(
      ownedId,
      after,
      EVENT_PAGE_BYTES
    );
    const current = getConversationSession(ownedId);
    if (
      readVersions.get(ownedId) !== readVersion
      || current?.generation !== generation
      || current.newestLoadedSequence !== after
    ) {
      failLoadingNewerConversationEvents(ownedId);
      return;
    }
    if (!page) {
      failLoadingNewerConversationEvents(ownedId);
      return;
    }
    appendNewerConversationEvents(ownedId, page);
    void hydrateSentConversationAttachments(
      ownedId,
      generation,
      getConversationSession(ownedId)?.loadedEvents ?? []
    );
  } catch {
    failLoadingNewerConversationEvents(ownedId);
  }
}

export async function startConversationEvents(): Promise<void> {
  if (!isTauri() || conversationStream) return;
  conversationEventsDisposed = false;
  if (conversationEventsSetup) return conversationEventsSetup;
  const streamGeneration = ++conversationEventsGeneration;
  conversationEventsSetup = (async () => {
    const registration = await registerAgentConversationStream((envelope: StreamEnvelope<AgentConversationEvent>) => {
      if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration) return;
      const payload = envelope.chunk;
      const active = rail.activeOwnedId === payload.ownedId;
      if (active) applyAgentConversationEvent(payload);
      else recordAgentConversationPresenceEvent(payload);
      if (
        payload.payload.kind === 'userMessage'
        && typeof payload.payload.text === 'string'
      ) {
        const owned = rail.owned.find((session) => session.ownedId === payload.ownedId);
        if (owned && !owned.title.trim()) {
          const title = sessionTitleFromPrompt(payload.payload.text);
          if (title) updateOwnedSession(payload.ownedId, { title });
        }
      }
      if (active && getConversationSession(payload.ownedId)?.desynchronized) {
        void resyncConversation(payload.ownedId);
      }
      if (active && shouldClearConversationSending(payload)) {
        setConversationSending(payload.ownedId, false);
      }
    }, () => {
      if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration) return;
      const activeOwnedId = rail.activeOwnedId;
      if (activeOwnedId) void resyncConversation(activeOwnedId);
    });
    // A session starts out named after the first words of its prompt. Once
    // its first turn is done the app writes a short summary over that, and this
    // is how the rail row hears about it.
    const stopTitleEvents = await listen<{ ownedId: string; title: string }>(
      'session-title-changed',
      ({ payload }) => updateOwnedSession(payload.ownedId, { title: payload.title })
    );
    const stopTitles = trackTauriListener(stopTitleEvents);
    if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration || !registration) {
      await registration?.unregister();
      stopTitles();
      return;
    }
    conversationStream = registration;
    unlistenTitles = stopTitles;
  })();
  try {
    await conversationEventsSetup;
  } finally {
    conversationEventsSetup = null;
  }
}

export function stopConversationEvents(): void {
  conversationEventsDisposed = true;
  conversationEventsGeneration += 1;
  childTranscriptReads.clear();
  void conversationStream?.unregister();
  conversationStream = null;
  unlistenTitles?.();
  unlistenTitles = null;
  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
}

/** Closes the exact structured generation currently held by the frontend. */
export async function closeStructuredConversation(ownedId: string): Promise<void> {
  if (!isTauri()) return;
  const generation = getConversationSession(ownedId)?.generation;
  if (generation === undefined) return;
  await invoke<boolean>('close_agent_conversation', { ownedId, generation });
}

/** Ensures one structured runtime and applies only its returned connection. */
export async function ensureStructuredConversation(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  cwd: string;
  nativeSessionId?: string | null;
  nativeSessionMode?: 'resume' | 'load';
  reasoningEffort?: string | null;
}): Promise<AgentConversationConnection | null> {
  if (rail.activeOwnedId === input.ownedId) {
    ensureConversationSession(input.ownedId, input.provider);
  }
  if (!isTauri() || !input.cwd.trim()) return null;
  const request = { ...input, nativeSessionMode: input.nativeSessionMode ?? 'resume' };
  const signature = JSON.stringify(request);
  const active = ensuring.get(input.ownedId);
  if (active?.signature === signature) return active.work;
  const work = (async () => {
    const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
      request
    });
    if (connection.nativeSessionId) {
      updateOwnedSession(input.ownedId, { nativeSessionId: connection.nativeSessionId });
    }
    if (rail.activeOwnedId !== input.ownedId) return connection;
    setConversationConnection(connection);
    await resyncConversation(input.ownedId);
    return connection;
  })().finally(() => {
    if (ensuring.get(input.ownedId)?.work === work) ensuring.delete(input.ownedId);
  });
  ensuring.set(input.ownedId, { signature, work });
  return work;
}

/** Changes a quiescent Codex session's durable checkout without changing its
 * generation. The native command owns the gate and the SQLite transaction. */
export async function changeStructuredConversationCheckout(
  ownedId: string,
  cwd: string
): Promise<AgentConversationSessionRecord | null> {
  const state = getConversationSession(ownedId);
  if (!state || state.provider !== 'codex' || !cwd.trim()) return null;
  const record = await changeAgentConversationCheckoutFromTauri({
    ownedId,
    generation: state.generation,
    cwd
  });
  if (!record) return null;
  updateOwnedSession(ownedId, {
    cwd: record.cwd,
    nativeSessionId: record.nativeSessionId,
    runtimeState: record.state
  });
  setConversationConnection({
    ownedId,
    provider: state.provider,
    generation: state.generation,
    nativeSessionId: record.nativeSessionId ?? state.nativeSessionId,
    state: record.suspended ? 'disconnected' : state.connectionState,
    suspended: record.suspended
  });
  return record;
}

/** Sends one message through the current writer while preserving attachment recovery. */
export async function sendStructuredMessage(
  ownedId: string,
  text: string,
  startConfig?: {
    reasoningEffort?: string | null;
    model?: string | null;
    approvalPolicy?: string | null;
  }
): Promise<void> {
  let state = getConversationSession(ownedId);
  if (!state) return;
  if (!text.trim() && state.attachments.length === 0) return;
  setConversationSending(ownedId, true);
  try {
    const owned = rail.owned.find((session) => session.ownedId === ownedId) ?? null;
    let terminalSessionId = owned?.ptySessionId;
    const terminalStillOwnsSession = Boolean(
      terminalSessionId
      && owned
      && owned.origin === 'external'
      && owned.state !== 'exited'
      && owned.executionOwner !== 'stopped'
    );

    if (
      owned
      && !terminalStillOwnsSession
      && shouldReviveBeforeSend({
        sessionState: owned.state,
        executionOwner: owned.executionOwner,
        connectionState: state.connectionState,
        generation: state.generation
      })
    ) {
      const provider: AgentConversationProvider | null =
        owned.agent === 'codex' || owned.agent === 'claude' || owned.agent === 'antigravity'
          ? owned.agent
          : null;
      if (!provider) throw new Error('This stopped session cannot be revived as a conversation');

      const previousGeneration = state.generation;
      const activation = decideConversationActivation(owned, state);
      if (activation.kind === 'terminal') {
        throw new Error('This stopped session must be started from its session row');
      }
      const nativeSessionMode = activation.kind === 'structured' ? activation.nativeSessionMode : 'resume';
      const activated = await ensureStructuredConversation({
        ownedId,
        provider,
        cwd: owned.cwd,
        nativeSessionId: owned.nativeSessionId,
        nativeSessionMode,
        reasoningEffort: startConfig?.reasoningEffort ?? state.agentConfig.reasoningEffort
      });
      const revived = getConversationSession(ownedId);
      const nextGeneration = generationForSend(previousGeneration, activated?.generation ?? -1);
      if (nextGeneration === null || !revived) {
        throw new Error('The ensured conversation is not current');
      }
      state = revived;
      terminalSessionId = null;
    }

    if (terminalSessionId) {
      if (state.writerLease.ownedId !== ownedId
        || state.writerLease.generation !== state.generation
        || state.writerLease.owner !== 'terminal') {
        throw new Error('The terminal writer lease is not confirmed for this session');
      }
      const attachmentText = state.attachments.length
        ? `\n\nAttached screenshots:\n${state.attachments.map((item) => `- ${item.path}`).join('\n')}`
        : '';
      const outgoingText = `${text}${attachmentText}`;
      // Match a real terminal paste followed by a separate Enter key. Sending
      // both in one PTY write makes Claude's TUI treat Enter as part of its
      // multiline paste buffer, leaving the prompt staged but never submitted.
      const pasted = await writeTerminalSessionFromTauri(
        terminalSessionId,
        `\u001b[200~${outgoingText}\u001b[201~`
      );
      if (!pasted) throw new Error('The terminal session is no longer running');
      const submitted = await writeTerminalSessionFromTauri(terminalSessionId, '\r');
      if (!submitted) throw new Error('The terminal session is no longer running');
      setConversationSending(ownedId, false);
      state.attachments.forEach(cleanupConversationAttachmentPreview);
      setConversationAttachments(ownedId, []);
      return;
    }
    if (state.generation < 1) throw new Error('The structured conversation is not connected');
    // Antigravity's adapter reads only the words of a prompt, so a screenshot
    // sent with one arrives as nothing at all. The message still goes; the
    // notice beside the box says what was left behind.
    if (state.provider === 'antigravity' && state.attachments.length > 0) {
      await Promise.all(
        state.attachments.map((attachment) => cleanupConversationAttachment(ownedId, attachment))
      );
      setConversationAttachments(ownedId, []);
      setConversationProviderNotice(ownedId, 'Antigravity cannot take images yet; they were left out.');
      // A screenshot on its own leaves nothing to say, so nothing is sent.
      if (!text.trim()) {
        setConversationSending(ownedId, false);
        return;
      }
    }
    // An unread or stale capability snapshot is not a refusal. Blocking the
    // send here left a screenshot that could never go out and no way to learn
    // why, so only a connected session's own answer refuses.
    const supportsImages = sendSupportsImages(state.capabilities, state.connectionState);
    const hydratedAttachments = supportsImages
      ? await Promise.all(state.attachments.map((attachment) => hydrateAttachmentBytes(attachment as AttachmentWithBytes)))
      : state.attachments as AttachmentWithBytes[];
    const prompt = buildConversationPrompt(text, hydratedAttachments, supportsImages);
    const liveConversationEvents = await hasBackendCapability(
      ACP_LIVE_CONVERSATION_EVENTS_CAPABILITY
    );
    const validatedState = getConversationSession(ownedId);
    const validatedGeneration = sendTargetGeneration(validatedState);
    if (validatedGeneration === null || !validatedState) {
      throw new Error('This conversation is closed, so the message was not sent');
    }
    state = validatedState;
    if (owned) {
      updateOwnedSession(ownedId, {
        state: 'live',
        executionOwner: 'structured',
        runtimeState: 'ready',
        lastError: null,
        nativeSessionId: state.nativeSessionId ?? owned.nativeSessionId
      });
    }
    // Recorded before the request because the backend records and dispatches
    // its own copy of the user message while the request is still running.
    // Recorded afterwards, the screenshots would arrive too late to be claimed.
    // The transcript keeps only display metadata: thumbnails show what went out,
    // and no provider echoes the image back for it to render from.
    recordSentConversationAttachments(ownedId, state.attachments.map(attachmentDisplayMetadata));
    await invoke('send_agent_conversation_message', {
      request: {
        ownedId,
        generation: validatedGeneration,
        text: prompt.text,
        content: prompt.content,
        // Named on the recorded user message so a restart can find the saved
        // files again; the in-memory hold above does not survive one.
        attachmentIds: state.attachments.map((attachment) => attachment.id),
        model: startConfig?.model ?? null,
        approvalPolicy: startConfig?.approvalPolicy ?? null
      }
    });
    setConversationAttachments(ownedId, []);
    if (!liveConversationEvents) await resyncConversation(ownedId);
  } catch (error) {
    // A send that never went out leaves its screenshots in the composer, so
    // nothing is left waiting to be hung on a later message.
    recordSentConversationAttachments(ownedId, []);
    setConversationSending(ownedId, false);
    throw error;
  }
}

/** Stops the active turn for the conversation's current generation. */
export async function stopStructuredTurn(ownedId: string): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state || state.generation < 1) return;
  await invoke('stop_agent_conversation_turn', {
    request: { ownedId, generation: state.generation }
  });
}

/** Answers one legacy approval request through the typed native boundary. */
export async function respondToStructuredApproval(
  ownedId: string,
  requestId: string,
  decision: 'accept' | 'decline' | 'cancel'
): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state) return;
  await invoke('respond_agent_conversation_approval', {
    request: { ownedId, generation: state.generation, requestId, decision }
  });
}

/** Answers a provider permission request while retaining the existing legacy fallback. */
export async function sendPermissionResponse(
  ownedId: string,
  requestId: string,
  optionId: string
): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state) return;
  const pendingOption = state.pendingApprovals[requestId]?.options.find(
    (option) => option.optionId === optionId
  );
  if (pendingOption?.synthetic) {
    const decision = /reject|deny|decline|cancel/i.test(optionId) ? 'decline' : 'accept';
    await respondToStructuredApproval(ownedId, requestId, decision);
    return;
  }
  try {
    await invoke('respond_agent_conversation_permission', {
      request: { ownedId, generation: state.generation, requestId, optionId }
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!/not found|unknown command|not part of the pending request/i.test(message)) throw error;
    const decision = /reject|deny|decline|cancel/i.test(optionId) ? 'decline' : 'accept';
    await respondToStructuredApproval(ownedId, requestId, decision);
  }
}

/** Answers one structured input request for the current conversation generation. */
export async function respondToStructuredInput(
  ownedId: string,
  response: Omit<AgentUserInputResponse, 'ownedId' | 'generation'>
): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state) return;
  await invoke('respond_agent_conversation_input', {
    request: { ...response, ownedId, generation: state.generation }
  });
}
