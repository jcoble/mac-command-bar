import { writeTerminalSessionFromTauri } from '$lib/tauriSource';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  applyAgentConversationEvent,
  applyAgentConversationSnapshot,
  applyChildConversationTranscript,
  applyConversationTranscript,
  ensureConversationSession,
  getConversationSession,
  setConversationAttachments,
  setConversationConnection,
  setConversationSending
} from './conversationStore.svelte.ts';
import type {
  AgentConversationConnection,
  AgentConversationEvent,
  AgentConversationProvider,
  AgentConversationSnapshot,
  ConversationAttachment,
  ConversationTranscriptSnapshot
} from './conversationTypes.ts';

let unlisten: UnlistenFn | null = null;
const resyncing = new Map<string, Promise<void>>();
const transcriptMirrors = new Map<string, ReturnType<typeof setInterval>>();

async function refreshTranscript(
  ownedId: string,
  provider: AgentConversationProvider,
  nativeSessionId: string
): Promise<void> {
  const snapshot = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
    provider,
    nativeSessionId,
    childSessionId: null
  });
  applyConversationTranscript(ownedId, provider, snapshot);
  const childSessionId = getConversationSession(ownedId)?.selectedChildId;
  if (childSessionId) {
    const child = await invoke<ConversationTranscriptSnapshot>('read_agent_conversation_transcript', {
      provider,
      nativeSessionId,
      childSessionId
    });
    applyChildConversationTranscript(ownedId, childSessionId, child.messages);
  }
}

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
  return { ...saved, previewUrl: URL.createObjectURL(file) };
}

export function startConversationTranscriptMirror(input: {
  ownedId: string;
  provider: AgentConversationProvider;
  nativeSessionId: string;
}): void {
  if (!isTauri() || transcriptMirrors.has(input.ownedId)) return;
  const refresh = (): void => {
    void refreshTranscript(input.ownedId, input.provider, input.nativeSessionId).catch(() => {
      // The transcript may not exist until Claude accepts its first prompt.
      // The next poll retries without breaking the conversation surface.
    });
  };
  refresh();
  const timer = setInterval(() => {
    refresh();
  }, 500);
  transcriptMirrors.set(input.ownedId, timer);
}

export function stopConversationTranscriptMirror(ownedId: string): void {
  const timer = transcriptMirrors.get(ownedId);
  if (timer) clearInterval(timer);
  transcriptMirrors.delete(ownedId);
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
  unlisten = await listen<AgentConversationEvent>('agent-conversation-event', ({ payload }) => {
    applyAgentConversationEvent(payload);
    if (getConversationSession(payload.ownedId)?.desynchronized) {
      void resyncConversation(payload.ownedId);
    }
    if (payload.payload.kind === 'turn' || payload.payload.kind === 'error') {
      setConversationSending(payload.ownedId, false);
    }
  });
}

export function stopConversationEvents(): void {
  unlisten?.();
  unlisten = null;
  for (const ownedId of transcriptMirrors.keys()) stopConversationTranscriptMirror(ownedId);
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
}): Promise<AgentConversationConnection | null> {
  ensureConversationSession(input.ownedId, input.provider);
  if (!isTauri() || !input.cwd.trim()) return null;
  const connection = await invoke<AgentConversationConnection>('ensure_agent_conversation', {
    request: input
  });
  setConversationConnection(connection);
  await resyncConversation(input.ownedId);
  return connection;
}

export async function sendStructuredMessage(
  ownedId: string,
  text: string,
  ptySessionId?: string | null
): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state) return;
  if (!text.trim()) return;
  setConversationSending(ownedId, true);
  try {
    const attachmentText = state.attachments.length
      ? `\n\nAttached screenshots:\n${state.attachments.map((item) => `- ${item.path}`).join('\n')}`
      : '';
    const outgoingText = `${text}${attachmentText}`;
    if (ptySessionId) {
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
      state.attachments.forEach((item) => URL.revokeObjectURL(item.previewUrl));
      setConversationAttachments(ownedId, []);
      return;
    }
    if (state.generation < 1) return;
    await invoke('send_agent_conversation_message', {
      request: { ownedId, generation: state.generation, text: outgoingText }
    });
    state.attachments.forEach((item) => URL.revokeObjectURL(item.previewUrl));
    setConversationAttachments(ownedId, []);
  } catch (error) {
    setConversationSending(ownedId, false);
    throw error;
  }
}

export async function respondToStructuredApproval(
  ownedId: string,
  requestId: string,
  decision: 'accept' | 'decline'
): Promise<void> {
  const state = getConversationSession(ownedId);
  if (!state) return;
  await invoke('respond_agent_conversation_approval', {
    request: { ownedId, generation: state.generation, requestId, decision }
  });
}
