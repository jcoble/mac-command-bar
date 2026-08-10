<script lang="ts">
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import UserMessageItem from './UserMessageItem.svelte';
  import AssistantMessageItem from './AssistantMessageItem.svelte';
  import ReasoningItem from './ReasoningItem.svelte';
  import PlanItem from './PlanItem.svelte';
  import TaskListItem from './TaskListItem.svelte';
  import CommandItem from './CommandItem.svelte';
  import FileChangeItem from './FileChangeItem.svelte';
  import ToolItem from './ToolItem.svelte';
  import SubagentItem from './SubagentItem.svelte';
  import ApprovalItem from './ApprovalItem.svelte';
  import UserInputItem from './UserInputItem.svelte';
  import ErrorItem from './ErrorItem.svelte';

  interface Props {
    item: ConversationDisplayItem;
    assistantLabel?: string;
    onApprovalDecision?(requestId: string, decision: string): void;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void;
    onFileLink?(path: string): void;
  }
  let { item, assistantLabel = 'Assistant', onApprovalDecision, onInputSubmit, onFileLink }: Props = $props();
</script>

<div class="timeline-item" data-testid="conversation-timeline-item" data-item-id={item.itemId}>
  {#if item.kind === 'user'}<UserMessageItem {item} {onFileLink} />
  {:else if item.kind === 'assistant'}<AssistantMessageItem {item} label={assistantLabel} {onFileLink} />
  {:else if item.kind === 'reasoning'}<ReasoningItem {item} {onFileLink} />
  {:else if item.kind === 'plan'}<PlanItem {item} />
  {:else if item.kind === 'tasks'}<TaskListItem {item} />
  {:else if item.kind === 'command'}<CommandItem {item} {onFileLink} />
  {:else if item.kind === 'file'}<FileChangeItem {item} {onFileLink} />
  {:else if item.kind === 'tool'}<ToolItem {item} {onFileLink} />
  {:else if item.kind === 'subagent'}<SubagentItem {item} />
  {:else if item.kind === 'approval'}<ApprovalItem {item} onDecision={onApprovalDecision} />
  {:else if item.kind === 'input'}<UserInputItem {item} onSubmit={onInputSubmit} />
  {:else if item.kind === 'error'}<ErrorItem {item} />
  {:else}<div class="unknown-item" data-testid="timeline-unknown-item">{item.text}</div>{/if}
</div>

<style>.timeline-item{display:block;min-width:0}.unknown-item{padding:10px 12px;border-left:2px solid var(--color-border);color:var(--color-text-2);white-space:pre-wrap}</style>
