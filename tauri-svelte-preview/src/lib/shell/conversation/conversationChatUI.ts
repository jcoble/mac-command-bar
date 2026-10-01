import type { Snippet } from 'svelte';
import type { ConversationDisplayItem } from './conversationTimeline.ts';
import type { UIMessage } from '@tanstack/ai/client';

// The host exposes references to the selected processor's messages, not another store.
export const conversationMessagesContext = Symbol('conversation-messages');
export type ConversationMessagesContext = {
  readonly messages: ReadonlyMap<string, UIMessage>;
};
export const conversationToolContext = Symbol('conversation-tool');
export type ConversationToolContext = {
  readonly message: UIMessage | undefined;
  openFile(path: string): void;
  readonly renderTool?: Snippet<[Extract<ConversationDisplayItem, { kind: 'tool' }>]>;
};
