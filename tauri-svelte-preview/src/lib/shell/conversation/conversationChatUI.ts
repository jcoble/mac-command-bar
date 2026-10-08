import type { Snippet } from 'svelte';
import type { ConversationDisplayItem } from './conversationTimeline.ts';
import type { UIMessage } from '@tanstack/ai/client';
import type { UIDescriptor } from '@tanstack/ai-svelte/ui';

// The host exposes references to the selected client's messages, not another store.
export const conversationMessagesContext = Symbol('conversation-messages');
export type ConversationMessagesContext = {
  readonly ui: UIDescriptor;
  readonly messages: ReadonlyMap<string, UIMessage>;
};
export const conversationToolContext = Symbol('conversation-tool');
export type ConversationToolContext = {
  readonly message: UIMessage | undefined;
  openFile(path: string): void;
  readonly renderTool?: Snippet<[Extract<ConversationDisplayItem, { kind: 'tool' }>]>;
};

export const conversationDisclosureContext = Symbol('conversation-disclosures');
export type ConversationDisclosureContext = {
  get(key: string): boolean | undefined;
  set(key: string, open: boolean): void;
};
