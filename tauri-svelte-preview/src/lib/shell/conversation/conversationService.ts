import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  applyAgentConversationEvent,
  applyAgentConversationSnapshot,
  applyChildConversationTranscript,
  ensureConversationSession,
  getConversationSession,
  setConversationConnection,
  setConversationAttachments,
  setConversationSending,
  beginConversationConfigChange,
  confirmConversationConfigChange,
  failConversationConfigChange,
  setConversationCapabilities,
  setConversationCapabilityError,
  setConversationWriterLeaseTransition,
  clearConversationWriterLeaseTransition
} from './conversationStore.svelte.ts';
import type {
  AgentCapabilities,
  AgentConversationConnection,
  AgentConversationEvent,
  AgentEvent,
  AgentConversationProvider,
  AgentConversationSnapshot,
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
import { writeTerminalSessionFromTauri } from '$lib/tauriSource';
import { hasBackendCapability } from '../backendCapabilities.ts';
import { shouldClearConversationSending } from './conversationReducer.ts';
import { updateOwnedSession } from '../stores/sessionRailStore.svelte';

let unlisten: UnlistenFn | null = null;
const resyncing = new Map<string, Promise<void>>();
const ensuring = new Map<string, { signature: string; work: Promise<AgentConversationConnection | null> }>();
const terminalProjections = new Map<string, string>();
const ACP_LIVE_CONVERSATION_EVENTS_CAPABILITY = 'acpLiveConversationEvents';

type TerminalProjectionRegistration = {
  generation: number;
  events: AgentEvent[];
};

export async function readChildConversationTranscript(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  nativeSessionId: string;
  childSessionId: string;
}): Promise<void> {
  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
    provider: input.provider,
    nativeSessionId: input.nativeSessionId,
    childSessionId: input.childSessionId
  });
  applyChildConversationTranscript(input.ownedId, input.childSessionId, snapshot.messages);
}

type SavedAttachment = Omit<ConversationAttachment, 'previewUrl'> & { byteLength: number };
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
  return { ...saved, bytes, previewUrl: URL.createObjectURL(file) } as AttachmentWithBytes;
}

/** Revoke only URLs owned by this surface; the managed path never goes through the DOM. */
export function cleanupConversationAttachmentPreview(attachment: ConversationAttachment): void {
  if (attachment.previewUrl.startsWith('blob:')) URL.revokeObjectURL(attachment.previewUrl);
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

/** Rebuild a preview URL from a backend-owned attachment after a workspace restore. */
export function restoreConversationAttachmentPreview(
  attachment: Omit<ConversationAttachment, 'previewUrl'> & { previewUrl?: string }
): ConversationAttachment {
  const previewUrl = attachment.previewUrl
    ?? (isTauri() ? convertFileSrc(attachment.path) : attachment.path);
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
  } catch {
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
  try {
    const capabilities = await invoke<AgentCapabilities>('read_agent_conversation_capabilities', { ownedId });
    if (capabilities.provider !== provider) throw new Error('Capability provider does not match this session');
    setConversationCapabilities(ownedId, capabilities);
    return capabilities;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setConversationCapabilityError(ownedId, message);
    throw error;
  }
}

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

export function startConversationTerminalProjection(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  nativeSessionId: string;
}): void {
  if (!isTauri()) return;
  const signature = `${input.provider}:${input.nativeSessionId}`;
  if (terminalProjections.get(input.ownedId) === signature) return;
  terminalProjections.set(input.ownedId, signature);
  const generation = getConversationSession(input.ownedId)?.generation ?? 0;
  void invoke<TerminalProjectionRegistration>('start_agent_conversation_terminal_projection', {
    request: { ...input, generation }
  }).then((registration) => {
    for (const event of registration.events) applyAgentConversationEvent(event);
  }).catch(() => {
    // The transcript may not exist until the agent accepts its first prompt.
    // Dropping the signature lets the next activation register again.
    if (terminalProjections.get(input.ownedId) === signature) terminalProjections.delete(input.ownedId);
  });
}

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

async function resyncConversation(ownedId: string): Promise<void> {
  const existing = resyncing.get(ownedId);
  if (existing) return existing;
  const work = (async () => {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      const snapshot = await invoke<AgentConversationSnapshot | null>(
        'read_agent_conversation_snapshot',
        { ownedId }
      );
      if (!snapshot) return;
      const sequenceBeforeApply = getConversationSession(ownedId)?.lastSequence ?? 0;
      applyAgentConversationSnapshot(snapshot);
      if (sequenceBeforeApply <= snapshot.lastSequence) return;
    }
  })().finally(() => resyncing.delete(ownedId));
  resyncing.set(ownedId, work);
  return work;
}

export async function startConversationEvents(): Promise<void> {
  if (!isTauri() || unlisten) return;
  unlisten = await listen<AgentConversationEvent | AgentEvent>('agent-conversation-event', ({ payload }) => {
    applyAgentConversationEvent(payload);
    if (getConversationSession(payload.ownedId)?.desynchronized) {
      void resyncConversation(payload.ownedId);
    }
    if (shouldClearConversationSending(payload)) {
      setConversationSending(payload.ownedId, false);
    }
  });
}

export function stopConversationEvents(): void {
  unlisten?.();
  unlisten = null;
  for (const ownedId of [...terminalProjections.keys()]) stopConversationTerminalProjection(ownedId);
}

export async function closeStructuredConversation(ownedId: string): Promise<void> {
  if (!isTauri()) return;
  await invoke<boolean>('close_agent_conversation', { ownedId });
}

export async function ensureStructuredConversation(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  cwd: string;
  nativeSessionId?: string | null;
  nativeSessionMode?: 'resume' | 'load';
}): Promise<AgentConversationConnection | null> {
  ensureConversationSession(input.ownedId, input.provider);
  if (!isTauri() || !input.cwd.trim()) return null;
  const request = { ...input, nativeSessionMode: input.nativeSessionMode ?? 'resume' };
  const signature = JSON.stringify(request);
  const active = ensuring.get(input.ownedId);
  if (active?.signature === signature) return active.work;
  const work = (async () => {
    const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
      request
    });
    setConversationConnection(connection);
    if (connection.nativeSessionId) {
      updateOwnedSession(input.ownedId, { nativeSessionId: connection.nativeSessionId });
    }
    await resyncConversation(input.ownedId);
    return connection;
  })().finally(() => {
    if (ensuring.get(input.ownedId)?.work === work) ensuring.delete(input.ownedId);
  });
  ensuring.set(input.ownedId, { signature, work });
  return work;
}

export async function sendStructuredMessage(
  ownedId: string,
  text: string,
  ptySessionId?: string | null
): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state) return;
  if (!text.trim() && state.attachments.length === 0) return;
  setConversationSending(ownedId, true);
  try {
    if (ptySessionId) {
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
        ptySessionId,
        `\u001b[200~${outgoingText}\u001b[201~`
      );
      if (!pasted) throw new Error('The terminal session is no longer running');
      const submitted = await writeTerminalSessionFromTauri(ptySessionId, '\r');
      if (!submitted) throw new Error('The terminal session is no longer running');
      setConversationSending(ownedId, false);
      state.attachments.forEach(cleanupConversationAttachmentPreview);
      setConversationAttachments(ownedId, []);
      return;
    }
    if (state.generation < 1) throw new Error('The structured conversation is not connected');
    const hydratedAttachments = state.capabilities?.prompt.image === true
      ? await Promise.all(state.attachments.map((attachment) => hydrateAttachmentBytes(attachment as AttachmentWithBytes)))
      : state.attachments as AttachmentWithBytes[];
    const prompt = buildConversationPrompt(
      text,
      hydratedAttachments,
      state.capabilities?.prompt.image === true
    );
    const liveConversationEvents = await hasBackendCapability(
      ACP_LIVE_CONVERSATION_EVENTS_CAPABILITY
    );
    await invoke('send_agent_conversation_message', {
      request: { ownedId, generation: state.generation, text: prompt.text, content: prompt.content }
    });
    if (!liveConversationEvents) await resyncConversation(ownedId);
    state.attachments.forEach(cleanupConversationAttachmentPreview);
    setConversationAttachments(ownedId, []);
  } catch (error) {
    setConversationSending(ownedId, false);
    throw error;
  }
}

export async function stopStructuredTurn(ownedId: string): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state || state.generation < 1) return;
  await invoke('stop_agent_conversation_turn', {
    request: { ownedId, generation: state.generation }
  });
}

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
