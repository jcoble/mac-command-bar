import {
  changeAgentConversationCheckoutFromTauri,
  createAgentConversationRequestId,
  listAgentConversationItemsAfterFromTauri,
  listAgentConversationItemsBeforeFromTauri,
  listRemoteAgentConversationSessionsFromTauri,
  readAgentConversationCapabilitiesFromTauri,
  readAgentConversationChildHistoryFromTauri,
  readAgentConversationSelectionFromTauri,
  readRemoteAssemblyEnvironmentFromTauri,
  registerAgentConversationStream,
  stopAgentConversationChildHistoryFromTauri,
  writeTerminalSessionFromTauri,
  type AgentConversationSessionRecord,
  type BackgroundWorkItem,
  type ExecutionEnvironment,
  type ProjectionStreamRegistration,
  type StreamEnvelope
} from '$lib/tauriSource';
import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import {
  revokeTrackedObjectUrl,
  trackTauriListener
} from '../resourceDiagnostics.svelte.ts';
import { sessionTitleFromPrompt } from '../sessionStrip.ts';
import { publishWorkspaceFileChange } from '../workspaceFileChangeBus.ts';
import { agentItemFromEvent, displayItemFromAgentItem } from './conversationTimeline.ts';
import { rail, setRemoteConnection, updateOwnedSession } from '../stores/sessionRailStore.svelte';
import type { RemoteConnectionState } from '../stores/sessionRailStore.svelte';
import {
  decideConversationActivation,
  generationForSend,
  sendSupportsImages,
  sendTargetGeneration,
  shouldReviveBeforeSend
} from './conversationActivation.ts';
import { ConversationDraftPersistence } from './conversationDraftPersistence.ts';
import { invokeConversationCommand as invoke } from './conversationInvoke.ts';
import { shouldClearConversationSending } from './conversationReducer.ts';
import { sessionPresenceHistory, sessionPresenceEventFromConversation, synchronizeSessionPresenceWork } from './sessionPresence.ts';
import {
  ACTIVE_EVENT_WINDOW_BYTES,
  ACTIVE_EVENT_WINDOW_EVENTS,
  displayEventFrom,
  applyChildConversationHistoryStatus,
  beginConversationConfigChange,
  clearConversationWriterLeaseTransition,
  confirmConversationConfigChange,
  ensureConversationSession,
  evictConversationSession,
  failChildConversationTranscript,
  failConversationConfigChange,
  getConversationSession,
  recordAgentConversationPresenceEvent,
  recordSentConversationAttachments,
  setConversationAttachments,
  setConversationCapabilities,
  setConversationCapabilityError,
  setConversationConnection,
  setConversationDraft,
  setConversationSendError,
  setConversationSending,
  setConversationWriterLeaseTransition
} from './conversationStore.svelte.ts';
import type {
  AgentCapabilities,
  AgentConfigOption,
  AgentConfigValue,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationHandoffDirection,
  AgentConversationHandoffPhase,
  AgentConversationHandoffReceipt,
  AgentConversationHandoffRequest,
  AgentConversationProvider,
  AgentConversationSendReceipt,
  AgentConversationItemPage,
  AgentConversationSelectionSnapshot,
  AgentUserInputResponse,
  AgentWriterLeaseOwner,
  AgentWriterLeaseTransition,
  ConversationAttachment,
  AgentConversationChildHistory
} from './conversationTypes.ts';
import {
  applyConversationHandoffReceipt,
  createConversationHandoffRequest,
  handoffGenerationMatches
} from './conversationTypes.ts';

let conversationStream: ProjectionStreamRegistration | null = null;
let unlistenTitles: (() => void) | null = null;
let unlistenRemoteConnections: (() => void) | null = null;
let conversationEventsSetup: Promise<void> | null = null;
let conversationEventsDisposed = false;
let conversationEventsGeneration = 0;
let remoteActivityRead = 0;
const railActivityEvents = new Map<string, { generation: number; sequence: number }>();
const CHILD_HISTORY_PAGE_BYTES = 512 * 1024;

type ConversationEnsure = {
  abortController: AbortController;
  invalidated: boolean;
  signature: string;
  token: object;
  work: Promise<AgentConversationConnection | null>;
};

type SelectedConversationRead = {
  workspaceOwnedId: string;
  ownedId: string;
  maxBytes: number;
  generation?: number;
  minimumGeneration?: number;
  pageWatermark: number;
  pendingSequence: number;
  events: AgentConversationEvent[];
  bytes: number;
  overflow: boolean;
  reading: boolean;
  reloadRequested: boolean;
  abortController: AbortController;
  onSnapshot: (snapshot: AgentConversationSelectionSnapshot) => void;
  onEvent: (event: AgentConversationEvent) => void;
  onError: (error: unknown) => void;
};

const ensuring = new Map<string, ConversationEnsure>();
const terminalProjections = new Map<string, string>();
let selectedConversationRead: SelectedConversationRead | null = null;
let childConversationRead: SelectedConversationRead | null = null;

function isCurrentConversationRead(read: SelectedConversationRead): boolean {
  return !read.abortController.signal.aborted
    && (selectedConversationRead === read || childConversationRead === read);
}
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

/** Opens one selected child's durable history and owns its native watch. */
interface ChildHistoryRead {
  requestId: number;
  childSessionId: string;
  active: boolean;
  settled: Promise<void>;
  stop(): void;
}

const childHistoryReads = new Map<string, ChildHistoryRead>();

export function stopChildConversationHistory(ownedId: string): void {
  const read = childHistoryReads.get(ownedId);
  if (!read) return;
  read.stop();
}

export async function readChildConversationHistory(input: {
  ownedId: string;
  childId: string;
  childSessionId: string;
  signal?: AbortSignal;
}): Promise<AgentConversationChildHistory | null> {
  if (input.signal?.aborted) return null;
  const previous = childHistoryReads.get(input.ownedId);
  stopChildConversationHistory(input.ownedId);
  const requestId = createAgentConversationRequestId();
  let readToken: ChildHistoryRead;
  let settle!: () => void;
  const settled = new Promise<void>((resolve) => { settle = resolve; });
  const stop = (): void => {
    readToken.active = false;
    input.signal?.removeEventListener('abort', abortFromOwner);
    void stopAgentConversationChildHistoryFromTauri(input.ownedId, requestId);
    void settled.then(() => {
      if (childHistoryReads.get(input.ownedId) === readToken) childHistoryReads.delete(input.ownedId);
    });
  };
  const abortFromOwner = (): void => stop();
  readToken = { requestId, childSessionId: input.childId, active: true, settled, stop };
  input.signal?.addEventListener('abort', abortFromOwner, { once: true });
  childHistoryReads.set(input.ownedId, readToken);
  try {
    await previous?.settled;
    if (!readToken.active || input.signal?.aborted
      || childHistoryReads.get(input.ownedId) !== readToken) return null;
    const result = await readAgentConversationChildHistoryFromTauri({
      parentOwnedId: input.ownedId,
      childSessionId: input.childSessionId,
      requestId,
      maxBytes: CHILD_HISTORY_PAGE_BYTES
    });
    const current = getConversationSession(input.ownedId);
    if (!readToken.active || input.signal?.aborted || childHistoryReads.get(input.ownedId) !== readToken
      || current?.selectedChildId !== input.childId) {
      stop();
      return null;
    }
    applyChildConversationHistoryStatus(
      input.ownedId,
      input.childId,
      result.page.hasEarlierTranscript
    );
    return result;
  } catch (error) {
    const ownsRead = childHistoryReads.get(input.ownedId) === readToken;
    if (ownsRead) {
      stop();
    }
    const current = getConversationSession(input.ownedId);
    if (ownsRead && !input.signal?.aborted && current?.selectedChildId === readToken.childSessionId) {
      failChildConversationTranscript(
        input.ownedId,
        input.childId,
        error instanceof Error ? error.message : String(error)
      );
    }
    throw error;
  } finally {
    settle();
  }
}

export type SavedAttachment = Omit<ConversationAttachment, 'previewUrl' | 'originalUrl'> & { byteLength: number };
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
  file: File,
  onSavedChange?: (attachment: SavedAttachment, retained: boolean) => Promise<void>
): Promise<ConversationAttachment> {
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
  const bytes = dataUrl.slice(dataUrl.indexOf(',') + 1);
  const saved = await invoke<SavedAttachment>('save_agent_conversation_attachment', {
    ownedId,
    mimeType: file.type,
    bytes
  });
  if (isRemoteConversation(ownedId)) {
    try {
      await onSavedChange?.(saved, true);
      return { ...saved, remoteOwnedId: ownedId, previewUrl: await remoteAttachmentPreview(ownedId, saved) };
    } catch (error) {
      try {
        await invoke('delete_agent_conversation_attachment', {
          request: { ownedId, attachmentId: saved.id, path: saved.path }
        });
        await onSavedChange?.(saved, false);
      } catch (_cleanupError) {
        // Keep the saved ID if its owner-scoped delete did not complete.
      }
      throw error;
    }
  }
  // No bytes are kept here: held in the composer's reactive state, a number
  // array is read element by element through its proxy at send, which held the
  // main thread for hundreds of milliseconds. The send reads the saved file.
  return restoreConversationAttachmentPreview(saved);
}

function isRemoteConversation(ownedId: string): boolean {
  return rail.owned.find((session) => session.ownedId === ownedId)?.executionEnvironment === 'remote';
}

async function remoteAttachmentPreview(ownedId: string, attachment: SavedAttachment, signal?: AbortSignal): Promise<string> {
  if (attachment.thumbnailByteLength == null) return '';
  const path = await invoke<string>('read_agent_conversation_attachment_file', {
    ownedId, attachmentId: attachment.id, thumbnail: true,
    byteLength: attachment.thumbnailByteLength,
    mimeType: attachment.thumbnailMimeType ?? 'image/webp',
    transferId: crypto.randomUUID()
  });
  if (signal?.aborted) throw new Error('Attachment preview cancelled');
  return convertFileSrc(path);
}

/** Revoke only URLs owned by this surface; the managed path never goes through the DOM. */
export function cleanupConversationAttachmentPreview(attachment: ConversationAttachment): void {
  if (attachment.previewUrl.startsWith('blob:')) revokeTrackedObjectUrl(attachment.previewUrl);
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

/** The transcript keeps only display metadata: thumbnails show what went out,
 * and no provider echoes the image back for it to render from. */
export function attachmentDisplayMetadata(attachment: ConversationAttachment): ConversationAttachment {
  const metadata: ConversationAttachment = {
    id: attachment.id,
    name: attachment.name,
    mimeType: attachment.mimeType,
    path: attachment.path,
    previewUrl: attachment.previewUrl,
    byteLength: attachment.byteLength,
    remoteOwnedId: attachment.remoteOwnedId
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

function conversationGenerationMatches(ownedId: string, generation: number | undefined): generation is number {
  return generation !== undefined && getConversationSession(ownedId)?.generation === generation;
}

export function discardRestoredAttachments(attachments: readonly ConversationAttachment[]): void {
  attachments.forEach(cleanupConversationAttachmentPreview);
}

/** Restore attachment metadata supplied by the existing owner-scoped vault. */
export async function restoreConversationAttachments(
  ownedId: string,
  saved: readonly (Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[] = [],
  signal?: AbortSignal
): Promise<ConversationAttachment[]> {
  if (signal?.aborted) return [];
  const generation = getConversationSession(ownedId)?.generation;
  if (generation === undefined) return [];
  if (saved.length > 0) {
    const restored = await restoreAttachmentList(ownedId, saved as SavedAttachment[], signal,
      (items) => { if (conversationGenerationMatches(ownedId, generation)) setConversationAttachments(ownedId, items); });
    if (signal?.aborted || !conversationGenerationMatches(ownedId, generation)) {
      discardRestoredAttachments(restored);
      return [];
    }
    setConversationAttachments(ownedId, restored);
    return restored;
  }
  if (!isTauri()) return [];
  try {
    const draftIds = new Set(getConversationSession(ownedId)?.attachmentIds ?? []);
    const records = await invoke<(Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string })[]>(
      'read_agent_conversation_attachments',
      { ownedId }
    );
    const restored = Array.isArray(records) ? await restoreAttachmentList(ownedId, (records as SavedAttachment[]).filter((record) => draftIds.has(record.id)), signal,
      (items) => { if (conversationGenerationMatches(ownedId, generation)) setConversationAttachments(ownedId, items); }) : [];
    if (signal?.aborted || !conversationGenerationMatches(ownedId, generation)) {
      discardRestoredAttachments(restored);
      return [];
    }
    setConversationAttachments(ownedId, restored);
    return restored;
  } catch (_error) {
    // The invoke seam logged the sanitized failure before this fallback runs.
    // Older controller builds have no listing command. The draft and saved ids
    // remain intact so a later controller can hydrate them without data loss.
    return [];
  }
}

export async function restoreAttachmentList(
  ownedId: string, records: readonly SavedAttachment[], signal?: AbortSignal,
  onUpdate?: (attachments: ConversationAttachment[]) => void
): Promise<ConversationAttachment[]> {
  const remote = isRemoteConversation(ownedId);
  const restored = records.map((record) => {
    if (remote) return { ...record, remoteOwnedId: ownedId, previewUrl: '', previewLoading: record.thumbnailByteLength != null };
    try { return restoreConversationAttachmentPreview(record); }
    catch { return { ...record, previewUrl: '' }; }
  });
  onUpdate?.([...restored]);
  if (!remote) return restored;
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(3, records.length) }, async () => {
    while (!signal?.aborted && next < records.length) {
      const index = next++;
      try {
        const previewUrl = await remoteAttachmentPreview(ownedId, records[index], signal);
        if (signal?.aborted) return;
        restored[index] = { ...restored[index], previewUrl, previewLoading: false };
      } catch {
        if (signal?.aborted) return;
        restored[index] = { ...restored[index], previewLoading: false };
      }
      onUpdate?.([...restored]);
    }
  }));
  if (signal?.aborted) {
    discardRestoredAttachments(restored);
    return [];
  }
  return restored;
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
  provider: AgentConversationProvider,
  signal?: AbortSignal
): Promise<AgentCapabilities | null> {
  if (!isTauri() || signal?.aborted) return null;
  const generation = getConversationSession(ownedId)?.generation;
  if (generation === undefined) return null;
  try {
    const capabilities = await readAgentConversationCapabilitiesFromTauri(ownedId, signal);
    if (!capabilities) return null;
    if (signal?.aborted || !conversationGenerationMatches(ownedId, generation)) return null;
    if (capabilities.provider !== provider) throw new Error('Capability provider does not match this session');
    setConversationCapabilities(ownedId, generation, capabilities);
    return capabilities;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!signal?.aborted && conversationGenerationMatches(ownedId, generation)) {
      setConversationCapabilityError(ownedId, message);
    }
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
  void startTerminalProjection(input, signature);
}

/** Stops terminal transcript projection when this surface no longer needs it. */
export function stopConversationTerminalProjection(ownedId: string): void {
  if (!terminalProjections.delete(ownedId) || !isTauri()) return;
  void stopTerminalProjection(ownedId);
}

async function stopTerminalProjection(ownedId: string): Promise<void> {
  try {
    await invoke<boolean>('stop_agent_conversation_terminal_projection', { ownedId });
  } catch {
    // A stale projection owner was already released locally.
  }
}

async function startTerminalProjection(
  input: {
    ownedId: string;
    provider: AgentConversationProvider;
    nativeSessionId: string;
  },
  signature: string
): Promise<void> {
  try {
    await invoke<Record<string, never>>('start_agent_conversation_terminal_projection', {
      request: input
    });
    if (terminalProjections.get(input.ownedId) !== signature) {
      await stopTerminalProjection(input.ownedId);
    }
  } catch {
    // The invoke seam logged the sanitized failure before this retry state resets.
    // The transcript may not exist until the agent accepts its first prompt.
    // Dropping the signature lets the next activation register again.
    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
  }
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

export async function prepareConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
  const receipt = await invokeHandoff(input, 'prepare');
  return receipt;
}

export async function commitConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
  const receipt = await invokeHandoff(input, 'commit');
  return receipt;
}

export async function rollbackConversationHandoff(input: HandoffInput): Promise<AgentConversationHandoffReceipt> {
  const receipt = await invokeHandoff(input, 'rollback');
  return receipt;
}

function selectedEventUsesControlCursor(event: AgentConversationEvent): boolean {
  return event.payload.kind === 'approval'
    || event.payload.kind === 'userInputRequested'
    || event.payload.kind === 'userInputResolved'
    || event.payload.kind === 'turn'
    || event.payload.kind === 'connection'
    || event.payload.kind === 'error'
    || event.payload.kind === 'childUpdate'
    || (event.payload.kind === 'terminalProjection'
      && event.payload.eventType === 'children.updated');
}

function deliverSelectedConversationEvent(
  read: SelectedConversationRead,
  event: AgentConversationEvent
): void {
  if (event.generation !== read.generation) {
    if (event.generation > (read.generation ?? 0)) {
      read.minimumGeneration = Math.max(read.minimumGeneration ?? 0, event.generation);
      requestSelectedConversationReload(read);
    }
    return;
  }
  if (event.sequence <= read.pageWatermark) return;
  if (event.sequence !== read.pageWatermark + 1) {
    requestSelectedConversationReload(read);
    return;
  }
  read.pageWatermark = event.sequence;
  const control = selectedEventUsesControlCursor(event);
  if (control && event.sequence <= read.pendingSequence) return;
  if (control) read.pendingSequence = event.sequence;
  read.onEvent(event);
}

function requestSelectedConversationReload(read: SelectedConversationRead): void {
  read.reloadRequested = true;
  if (read.reading) return;
  void reloadSelectedConversation(read).catch((error) => {
    if (isCurrentConversationRead(read) && !read.abortController.signal.aborted) {
      read.events.length = 0;
      read.bytes = 0;
      read.onError(error);
    }
  });
}

function bufferSelectedConversationEvent(
  read: SelectedConversationRead,
  event: AgentConversationEvent
): void {
  if (read.overflow || read.abortController.signal.aborted) return;
  read.bytes += new TextEncoder().encode(JSON.stringify(event)).byteLength;
  if (read.events.length >= ACTIVE_EVENT_WINDOW_EVENTS || read.bytes > ACTIVE_EVENT_WINDOW_BYTES) {
    read.events.length = 0;
    read.overflow = true;
    return;
  }
  read.events.push(event);
}

function admitSelectedConversationEvent(event: AgentConversationEvent): boolean {
  const read = selectedConversationRead?.ownedId === event.ownedId ? selectedConversationRead : childConversationRead;
  if (!read || read.ownedId !== event.ownedId || read.abortController.signal.aborted) return false;
  if (read.reading) bufferSelectedConversationEvent(read, event);
  else deliverSelectedConversationEvent(read, event);
  return true;
}

async function reloadSelectedConversation(read: SelectedConversationRead): Promise<void> {
  if (!isCurrentConversationRead(read) || read.abortController.signal.aborted) return;
  if (read.reading) {
    read.reloadRequested = true;
    return;
  }
  read.reading = true;
  try {
    // One retry covers a generation change or bounded-buffer overflow that
    // races the first read. The second read is authoritative for both content
    // and pending controls; repeated churn is surfaced instead of looping.
    for (let attempt = 0; attempt < 2; attempt += 1) {
      read.reloadRequested = false;
      read.events.length = 0;
      read.bytes = 0;
      read.overflow = false;
      const snapshot = await readAgentConversationSelectionFromTauri(
        read.ownedId,
        read.maxBytes,
        read.minimumGeneration,
        read.abortController.signal
      );
      if (!isCurrentConversationRead(read) || read.abortController.signal.aborted || !snapshot) return;
      if (read.overflow) {
        read.reloadRequested = true;
        continue;
      }
      if (snapshot.connection.generation < (read.minimumGeneration ?? 0)) {
        read.reloadRequested = true;
        continue;
      }
      read.generation = snapshot.connection.generation;
      read.minimumGeneration = undefined;
      read.pageWatermark = snapshot.page.coverage?.high ?? snapshot.page.watermark;
      read.pendingSequence = snapshot.pendingSequence;
      read.onSnapshot(snapshot);
      const replay = [...read.events].sort((left, right) => left.sequence - right.sequence);
      read.events.length = 0;
      for (const event of replay) deliverSelectedConversationEvent(read, event);
      if (!read.reloadRequested) return;
    }
    throw new Error('Conversation changed too quickly to open its selected history');
  } finally {
    read.reading = false;
  }
}

async function refreshSelectedConversation(ownedId: string): Promise<void> {
  const read = selectedConversationRead;
  if (!read || read.ownedId !== ownedId) return;
  await reloadSelectedConversation(read);
}

export async function subscribeSelectedConversation(input: {
  workspaceOwnedId: string;
  ownedId: string;
  maxBytes: number;
  signal?: AbortSignal;
  onSnapshot: (snapshot: AgentConversationSelectionSnapshot) => void;
  onEvent: (event: AgentConversationEvent) => void;
  onError: (error: unknown) => void;
}): Promise<() => void> {
  const child = input.ownedId !== input.workspaceOwnedId;
  (child ? childConversationRead : selectedConversationRead)?.abortController.abort();
  const abortController = new AbortController();
  const abortFromOwner = (): void => abortController.abort();
  input.signal?.addEventListener('abort', abortFromOwner, { once: true });
  const read: SelectedConversationRead = {
    workspaceOwnedId: input.workspaceOwnedId,
    ownedId: input.ownedId,
    maxBytes: input.maxBytes,
    pageWatermark: Number.MIN_SAFE_INTEGER,
    pendingSequence: Number.MIN_SAFE_INTEGER,
    events: [],
    bytes: 0,
    overflow: false,
    // Buffer any event delivered while the shared listener is registering.
    // The snapshot started immediately afterwards is authoritative for them.
    reading: true,
    reloadRequested: false,
    abortController,
    onSnapshot: input.onSnapshot,
    onEvent: input.onEvent,
    onError: input.onError
  };
  if (child) childConversationRead = read;
  else selectedConversationRead = read;
  const dispose = (): void => {
    input.signal?.removeEventListener('abort', abortFromOwner);
    abortController.abort();
    read.events.length = 0;
    if (selectedConversationRead === read) selectedConversationRead = null;
    if (childConversationRead === read) childConversationRead = null;
  };
  try {
    await startConversationEvents();
    if (input.signal?.aborted) {
      dispose();
      return dispose;
    }
    read.reading = false;
    await reloadSelectedConversation(read);
    return dispose;
  } catch (error) {
    dispose();
    throw error;
  }
}

export function readOlderSelectedConversationItems(
  ownedId: string,
  beforeSequence: number,
  maxBytes: number,
  signal?: AbortSignal
): Promise<AgentConversationItemPage | null> {
  return listAgentConversationItemsBeforeFromTauri(
    ownedId, beforeSequence, maxBytes, signal
  );
}

export function readNewerSelectedConversationItems(
  ownedId: string,
  afterSequence: number,
  maxBytes: number,
  signal?: AbortSignal
): Promise<AgentConversationItemPage | null> {
  return listAgentConversationItemsAfterFromTauri(
    ownedId, afterSequence, maxBytes, signal
  );
}

export async function readSelectedConversationAttachments(
  ownedId: string,
  attachmentIds: readonly string[],
  signal?: AbortSignal
): Promise<SavedAttachment[]> {
  if (!isTauri() || signal?.aborted || attachmentIds.length === 0) return [];
  const rows = await invoke<SavedAttachment[]>('read_agent_conversation_selected_attachments', {
    ownedId,
    attachmentIds: [...attachmentIds]
  });
  return signal?.aborted ? [] : rows;
}

export async function startConversationEvents(): Promise<void> {
  if (!isTauri() || conversationStream) return;
  conversationEventsDisposed = false;
  if (conversationEventsSetup) return conversationEventsSetup;
  const streamGeneration = ++conversationEventsGeneration;
  const setup = setupConversationEvents(streamGeneration);
  conversationEventsSetup = setup;
  try {
    await setup;
  } finally {
    if (conversationEventsSetup === setup) conversationEventsSetup = null;
  }
}

async function setupConversationEvents(streamGeneration: number): Promise<void> {
  const registration = await registerAgentConversationStream(
    (envelope) => handleConversationStreamEnvelope(streamGeneration, envelope),
    () => handleConversationStreamResync(streamGeneration)
  );
  // A session starts out named after the first words of its prompt. Once
  // its first turn is done the app writes a short summary over that, and this
  // is how the rail row hears about it.
  let stopTitles: (() => void) | null = null;
  // A remote machine's transport connects, starts attempting, or stops. This is
  // the app's one listener for it; rail rows read the store it writes.
  let stopRemoteConnections: (() => void) | null = null;
  const remoteConnectionEventsSeen = new Set<string>();
  try {
    const stopTitleEvents = await listen<{ ownedId: string; title: string }>(
      'session-title-changed',
      ({ payload }) => updateOwnedSession(payload.ownedId, { title: payload.title })
    );
    stopTitles = trackTauriListener(stopTitleEvents);
    const stopRemoteConnectionEvents = await listen<{
      profileId: string;
      state: RemoteConnectionState;
    }>('remote-connection-changed', ({ payload }) => {
      remoteConnectionEventsSeen.add(payload.profileId);
      setRemoteConnection(payload.profileId, payload.state);
      if (payload.state === 'connected') {
        void refreshRemoteSessionActivity(streamGeneration);
        void reacquireSelectedChildHistory(payload.profileId, streamGeneration).catch(() => {});
      }
    });
    stopRemoteConnections = trackTauriListener(stopRemoteConnectionEvents);
    const remoteEnvironment = await readRemoteAssemblyEnvironmentFromTauri();
    rail.remoteProfiles = remoteEnvironment.profiles;
    const readyRemoteProfiles = new Set(remoteEnvironment.readyProfileIds);
    for (const profile of remoteEnvironment.profiles) {
      if (remoteConnectionEventsSeen.has(profile.id)) continue;
      setRemoteConnection(profile.id, readyRemoteProfiles.has(profile.id) ? 'connected' : 'disconnected');
    }
    // Start-up waits for this setup before it opens a session, so the remote
    // read runs on its own.
    void refreshRemoteSessionActivity(streamGeneration);
    if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration || !registration) {
      await registration?.unregister();
      stopTitles();
      stopTitles = null;
      stopRemoteConnections();
      stopRemoteConnections = null;
      return;
    }
    conversationStream = registration;
    unlistenTitles = stopTitles;
    stopTitles = null;
    unlistenRemoteConnections = stopRemoteConnections;
    stopRemoteConnections = null;
  } catch (error) {
    stopTitles?.();
    stopRemoteConnections?.();
    await registration?.unregister();
    throw error;
  }
}

/** Reconcile compact runtime records after reconnect, without loading transcripts. */
async function refreshRemoteSessionActivity(streamGeneration: number): Promise<void> {
  const read = ++remoteActivityRead;
  const observed = new Map(railActivityEvents);
  try {
    const records = await listRemoteAgentConversationSessionsFromTauri();
    if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration || read !== remoteActivityRead) return;
    for (const record of records ?? []) {
      // A live event received during the read is newer than its inventory reply.
      if (observed.get(record.ownedId) !== railActivityEvents.get(record.ownedId)) continue;
      if (rail.remoteConnections[record.remoteProfileId ?? ''] !== 'connected') continue;
      if (preparingSends.has(record.ownedId)) continue;
      updateOwnedSession(record.ownedId, {
        state: record.activeTurnId ? 'live' : 'background',
        runtimeState: record.suspended ? 'suspended' : record.state,
        activeTurnId: record.activeTurnId,
        pendingPermission: record.pendingPermission,
        pendingInput: record.pendingInput,
        backgroundWork: record.backgroundWork
      });
      synchronizeSessionPresenceWork(record.ownedId, record.activeTurnId, false, null);
      const current = getConversationSession(record.ownedId);
      if (current) {
        current.activeTurnId = record.activeTurnId ?? undefined;
        current.suspended = record.suspended;
        if (!record.activeTurnId) setConversationSending(record.ownedId, false);
      }
    }
  } catch (error) {
    if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration) return;
    if (read !== remoteActivityRead) return;
    rail.error = `Could not refresh remote turn activity: ${error instanceof Error ? error.message : String(error)}`;
  }
}

function updateBackgroundWork(
  work: BackgroundWorkItem[],
  item: Omit<BackgroundWorkItem, 'startedAtMs'>,
  timestampMs: number,
  terminal: boolean
): BackgroundWorkItem[] {
  if (terminal) return work.filter((entry) => entry.id !== item.id);
  const existing = work.find((entry) => entry.id === item.id);
  const next = {
    ...item,
    label: item.label || existing?.label || '',
    startedAtMs: existing?.startedAtMs ?? timestampMs
  };
  return existing
    ? work.map((entry) => entry.id === item.id ? next : entry)
    : [...work, next];
}

async function reacquireSelectedChildHistory(
  profileId: string,
  streamGeneration: number
): Promise<void> {
  if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration) return;
  const parent = selectedConversationRead;
  if (parent && rail.owned.find((session) => session.ownedId === parent.workspaceOwnedId)?.remoteProfileId === profileId) {
    requestSelectedConversationReload(parent);
  }
  const read = childConversationRead;
  if (!read || read.workspaceOwnedId === read.ownedId || read.abortController.signal.aborted) return;
  const owned = rail.owned.find((session) => session.ownedId === read.workspaceOwnedId);
  if (owned?.remoteProfileId !== profileId) return;
  const current = getConversationSession(read.workspaceOwnedId);
  const childId = current?.selectedChildId;
  const child = current?.children.find((candidate) => candidate.childId === childId);
  if (!childId || !child?.transcriptAvailable) return;
  const history = await readChildConversationHistory({
    ownedId: read.workspaceOwnedId,
    childId,
    childSessionId: child.transcriptId ?? childId,
    signal: read.abortController.signal
  });
  if (!history || !isCurrentConversationRead(read) || read.abortController.signal.aborted
    || getConversationSession(read.workspaceOwnedId)?.selectedChildId !== childId
    || history.historyOwnedId !== read.ownedId) return;
  requestSelectedConversationReload(read);
}

async function handleConversationStreamEnvelope(
  streamGeneration: number,
  envelope: StreamEnvelope<AgentConversationEvent>
): Promise<void> {
  if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration) return;
  const payload = envelope.chunk;
  admitSelectedConversationEvent(payload);
  const previous = railActivityEvents.get(payload.ownedId);
  if (previous && (payload.generation < previous.generation
    || (payload.generation === previous.generation && payload.sequence <= previous.sequence))) {
    return;
  }
  const displayEvent = displayEventFrom(payload);
  const terminal = shouldClearConversationSending(displayEvent);
  const current = getConversationSession(payload.ownedId);
  const presence = get(sessionPresenceHistory)[payload.ownedId];
  const activeTurnId = presence?.activeTurnId || current?.activeTurnId;
  if (current && payload.generation < current.generation) return;
  if (terminal && activeTurnId && (
    (presence?.turnStartedAt !== null && presence?.turnStartedAt !== undefined && payload.timestampMs < presence.turnStartedAt)
    || (payload.payload.kind === 'turn' && payload.payload.turnId !== activeTurnId)
  )) return;
  railActivityEvents.set(payload.ownedId, { generation: payload.generation, sequence: payload.sequence });
  const item = agentItemFromEvent(displayEvent);
  const row = item ? displayItemFromAgentItem(item, displayEvent.timestampMs) : null;
  if (row?.kind === 'tool' && row.state === 'completed' && row.path && row.diff) {
    publishWorkspaceFileChange({ ownedId: payload.ownedId, path: row.path });
  }
  if (payload.provider === 'claude' && payload.payload.kind === 'childUpdate') {
    const owned = rail.owned.find((session) => session.ownedId === payload.ownedId);
    if (owned) {
      const terminalChild = ['finished', 'failed', 'cancelled', 'disconnected'].includes(payload.payload.state);
      updateOwnedSession(payload.ownedId, {
        backgroundWork: updateBackgroundWork(
          owned.backgroundWork ?? [],
          {
            id: `claude-child:${payload.payload.childId}`,
            kind: 'subagent',
            label: payload.payload.label ?? ''
          },
          payload.timestampMs,
          terminalChild
        )
      });
    }
  }
  if (payload.payload.kind === 'tool' && payload.payload.itemId.startsWith('background-task:')) {
    const owned = rail.owned.find((session) => session.ownedId === payload.ownedId);
    if (owned) {
      updateOwnedSession(payload.ownedId, {
        backgroundWork: updateBackgroundWork(
          owned.backgroundWork ?? [],
          { id: payload.payload.itemId, kind: 'command', label: payload.payload.name },
          payload.timestampMs,
          payload.payload.state !== 'started' && payload.payload.state !== 'updated'
        )
      });
    }
  }
  const transition = sessionPresenceEventFromConversation(displayEvent);
  if (transition?.kind === 'turn-started' || terminal) {
    const working = transition?.kind === 'turn-started';
    updateOwnedSession(payload.ownedId, {
      state: working ? 'live' : 'background',
      runtimeState: working ? 'working' : transition?.kind === 'turn-finished' ? 'ready' : 'failed',
      activeTurnId: working ? transition?.turnId ?? null : null,
      pendingPermission: false,
      pendingInput: false,
      ...(working ? { lastError: null } : {})
    });
  }
  if (terminal && !transition) synchronizeSessionPresenceWork(payload.ownedId, null, false);
  recordAgentConversationPresenceEvent(payload);
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
  if (terminal && current) {
    setConversationSending(payload.ownedId, false);
  }
}

async function handleConversationStreamResync(streamGeneration: number): Promise<void> {
  if (conversationEventsDisposed || streamGeneration !== conversationEventsGeneration) return;
  await Promise.allSettled([selectedConversationRead, childConversationRead].map(async (read) => {
    if (!read) return;
    try {
      await reloadSelectedConversation(read);
    } catch (error) {
      if (isCurrentConversationRead(read)) read.onError(error);
    }
  }));
  await refreshRemoteSessionActivity(streamGeneration);
}

export function stopConversationEvents(): void {
  conversationEventsDisposed = true;
  conversationEventsGeneration += 1;
  for (const ownedId of childHistoryReads.keys()) stopChildConversationHistory(ownedId);
  selectedConversationRead?.abortController.abort();
  selectedConversationRead = null;
  childConversationRead?.abortController.abort();
  childConversationRead = null;
  railActivityEvents.clear();
  void conversationStream?.unregister();
  conversationStream = null;
  unlistenTitles?.();
  unlistenTitles = null;
  unlistenRemoteConnections?.();
  unlistenRemoteConnections = null;
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
  executionEnvironment?: ExecutionEnvironment;
  remoteProfileId?: string | null;
  provider: AgentConversationProvider;
  cwd: string;
  nativeSessionId?: string | null;
  nativeSessionMode?: 'resume' | 'load';
  reasoningEffort?: string | null;
  /** The registry project; the backend records it once, on the first ensure. */
  projectId?: string | null;
  signal?: AbortSignal;
}): Promise<AgentConversationConnection | null> {
  if (input.signal?.aborted) return null;
  if (rail.activeOwnedId === input.ownedId) {
    ensureConversationSession(input.ownedId, input.provider);
  }
  if (!isTauri() || !input.cwd.trim()) return null;
  const request = {
    ...input,
    executionEnvironment: input.executionEnvironment ?? 'local',
    nativeSessionMode: input.nativeSessionMode ?? 'resume'
  };
  const signature = JSON.stringify(request);
  const active = ensuring.get(input.ownedId);
  if (active?.signature === signature) {
    const connection = await active.work;
    return input.signal?.aborted || active.invalidated ? null : connection;
  }
  const token = {};
  const abortController = new AbortController();
  const abortFromOwner = (): void => abortController.abort();
  input.signal?.addEventListener('abort', abortFromOwner, { once: true });
  const work = ensureStructuredConversationOnce(input.ownedId, request, token, abortController.signal);
  ensuring.set(input.ownedId, { abortController, invalidated: false, signature, token, work });
  try {
    const connection = await work;
    return input.signal?.aborted ? null : connection;
  } finally {
    input.signal?.removeEventListener('abort', abortFromOwner);
  }
}

async function ensureStructuredConversationOnce(
  ownedId: string,
  request: {
    ownedId: string;
    executionEnvironment: ExecutionEnvironment;
    remoteProfileId?: string | null;
    provider: AgentConversationProvider;
    cwd: string;
    nativeSessionId?: string | null;
    nativeSessionMode: 'resume' | 'load';
    reasoningEffort?: string | null;
    projectId?: string | null;
  },
  token: object,
  signal: AbortSignal
): Promise<AgentConversationConnection | null> {
  try {
    if (signal.aborted) return null;
    const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
      request
    });
    if (signal.aborted) return null;
    if (connection.nativeSessionId) updateOwnedSession(ownedId, { nativeSessionId: connection.nativeSessionId });
    if (rail.activeOwnedId !== ownedId) return connection;
    setConversationConnection(connection);
    const read = selectedConversationRead;
    if (read?.ownedId === ownedId) {
      read.minimumGeneration = Math.max(read.minimumGeneration ?? 0, connection.generation);
    }
    await refreshSelectedConversation(ownedId);
    return connection;
  } finally {
    if (ensuring.get(ownedId)?.token === token) ensuring.delete(ownedId);
  }
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

const preparingSends = new Map<string, { cancelled: boolean }>();
// A new turn's send request that is still out. Stop waits for its receipt:
// stopping sooner can reach the provider before the prompt does, and is ignored.
const sendRequests = new Map<string, Promise<unknown>>();

/** Sends one message through the current writer while preserving attachment recovery. */
export async function sendStructuredMessage(
  ownedId: string,
  text: string,
  startConfig?: {
    reasoningEffort?: string | null;
    model?: string | null;
    approvalPolicy?: string | null;
  }
): Promise<AgentConversationSendReceipt | undefined> {
  let state = getConversationSession(ownedId);
  if (!state) return;
  if (!text.trim() && state.attachments.length === 0) return;
  const presence = get(sessionPresenceHistory)[ownedId];
  const turnWasAlreadyActive = state.sending || Boolean(
    presence ? presence.activeTurnId : (state.activeTurnId ?? rail.owned.find((session) => session.ownedId === ownedId)?.activeTurnId)
  );
  const preparation = { cancelled: false };
  preparingSends.set(ownedId, preparation);
  setConversationSending(ownedId, true);
  // The composer empties the moment Send is pressed. The screenshots are held
  // for the transcript until the backend's copy of the message claims them —
  // recorded before the request, because the backend records and dispatches
  // its own copy while the request is still running.
  const attachments = state.attachments;
  const releaseHold = (): void => recordSentConversationAttachments(ownedId,
    (getConversationSession(ownedId)?.unclaimedSentAttachments ?? []).filter((held) => !attachments.some((item) => item.id === held.id)));
  if (attachments.length) {
    recordSentConversationAttachments(ownedId, [...(state.unclaimedSentAttachments ?? []), ...attachments.map(attachmentDisplayMetadata)]);
    setConversationAttachments(ownedId, []);
  }
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
        executionEnvironment: owned.executionEnvironment,
        remoteProfileId: owned.remoteProfileId ?? null,
        provider,
        cwd: owned.cwd,
        nativeSessionId: owned.nativeSessionId,
        nativeSessionMode,
        projectId: owned.projectId,
        reasoningEffort: startConfig
          ? startConfig.reasoningEffort
          : state.agentConfig.reasoningEffort
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
      const attachmentText = attachments.length
        ? `\n\nAttached screenshots:\n${attachments.map((item) => `- ${item.path}`).join('\n')}`
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
      // The terminal echoes no user message to claim the hold.
      releaseHold();
      attachments.forEach(cleanupConversationAttachmentPreview);
      return;
    }
    if (state.generation < 1) throw new Error('The structured conversation is not connected');
    // An unread or stale capability snapshot is not a refusal. Blocking the
    // send here left a screenshot that could never go out and no way to learn
    // why, so only a connected session's own answer refuses.
    const supportsImages = sendSupportsImages(state.capabilities, state.connectionState);
    if (attachments.length && !supportsImages) {
      throw new Error('This conversation provider does not advertise image prompts');
    }
    const remoteSend = owned?.executionEnvironment === 'remote';
    if (attachments.length && supportsImages && !remoteSend) {
      // Image bytes cross the IPC as a JSON number array, which holds the main
      // thread (~80 ms per 600 KB measured). Wait out the send's scroll glide
      // (~250 ms in WebKit) so the message lands smoothly before that work.
      await new Promise((resolve) => setTimeout(resolve, 300));
    }
    const hydratedAttachments = supportsImages && !remoteSend
      ? await Promise.all(attachments.map((attachment) => hydrateAttachmentBytes(attachment as AttachmentWithBytes)))
      : attachments as AttachmentWithBytes[];
    const prompt = remoteSend ? { text, content: [] as AgentPromptContent[] }
      : buildConversationPrompt(text, hydratedAttachments, supportsImages);
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
    if (preparation.cancelled) {
      if (!turnWasAlreadyActive) await invoke('stop_agent_conversation_turn', {
        request: { ownedId, generation: validatedGeneration }
      });
      throw new Error('Message cancelled before sending.');
    }
    if (preparingSends.get(ownedId) === preparation) preparingSends.delete(ownedId);
    const requestedModel = startConfig?.model ?? null;
    const requestedApprovalPolicy = startConfig?.approvalPolicy ?? null;
    const request = invoke('send_agent_conversation_message', {
      request: {
        ownedId,
        generation: validatedGeneration,
        text: prompt.text,
        content: prompt.content,
        // Named on the recorded user message so a restart can find the saved
        // files again; the in-memory hold above does not survive one.
        attachmentIds: attachments.map((attachment) => attachment.id),
        model: requestedModel,
        approvalPolicy:
          requestedApprovalPolicy
          && state.agentConfig.availableApprovalPolicies.includes(requestedApprovalPolicy)
            ? requestedApprovalPolicy
            : null
      }
    });
    if (!turnWasAlreadyActive) sendRequests.set(ownedId, request);
    try {
      return await request as AgentConversationSendReceipt;
    } finally {
      if (sendRequests.get(ownedId) === request) sendRequests.delete(ownedId);
    }
  } catch (error) {
    // A send that never went out puts its screenshots back in the composer, so
    // nothing is left waiting to be hung on a later message.
    if (attachments.length) {
      releaseHold();
      setConversationAttachments(ownedId, [...attachments, ...(getConversationSession(ownedId)?.attachments ?? [])]);
    }
    if (!turnWasAlreadyActive) setConversationSending(ownedId, false);
    throw error;
  } finally {
    if (preparingSends.get(ownedId) === preparation) preparingSends.delete(ownedId);
  }
}

/** Stops the active turn for the conversation's current generation. */
export async function stopStructuredTurn(ownedId: string): Promise<void> {
  const sendRequest = sendRequests.get(ownedId);
  if (sendRequest) {
    try {
      await sendRequest;
    } catch {
      return; // A refused send started no turn to stop.
    }
  }
  const preparation = preparingSends.get(ownedId);
  const state = getConversationSession(ownedId);
  const presence = get(sessionPresenceHistory)[ownedId];
  const activeTurn = presence ? presence.activeTurnId : (state?.activeTurnId ?? rail.owned.find((session) => session.ownedId === ownedId)?.activeTurnId);
  if (preparation) {
    preparation.cancelled = true;
    if (!activeTurn) return;
  }
  if (!state || state.generation < 1) return;
  await invoke('stop_agent_conversation_turn', {
    request: { ownedId, generation: state.generation }
  });
}

/** Answers one summary-only approval request through the typed native boundary. */
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

/** Answers the explicit decision or provider-option contract for a permission request. */
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
    if (message.startsWith('Stale approval request:')) {
      if (getConversationSession(ownedId)?.generation === state.generation) {
        delete state.pendingApprovals[requestId];
      }
      try { await refreshSelectedConversation(ownedId); } catch { /* A later selected snapshot can replay the recorded expiry. */ }
      throw error;
    }
    throw error;
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
