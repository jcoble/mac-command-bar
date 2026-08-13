<script lang="ts">
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import UserMessageItem from './UserMessageItem.svelte';
  import AssistantMessageItem from './AssistantMessageItem.svelte';
  import ReasoningItem from './ReasoningItem.svelte';
  import PlanItem from './PlanItem.svelte';
  import TaskListItem from './TaskListItem.svelte';
  import CommandItem from './CommandItem.svelte';
  import FileChangeItem from './FileChangeItem.svelte';
  import ToolItem from './ToolItem.svelte';
  import SubagentSection from './SubagentSection.svelte';
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
  // `assistantLabel` is retired: nothing in the transcript carries a role
  // heading any more. It stays accepted only so the timeline that still passes
  // it keeps type-checking; the container drops it in its own pass.
  let { item, onApprovalDecision, onInputSubmit, onFileLink }: Props = $props();
  const visible = $derived(conversationItemHasVisibleContent(item));
</script>

{#if visible}
  <div class="timeline-item" data-testid="conversation-timeline-item" data-item-id={item.itemId} data-kind={item.kind}>
    {#if item.kind === 'user'}<UserMessageItem {item} {onFileLink} />
    {:else if item.kind === 'assistant'}<AssistantMessageItem {item} {onFileLink} />
    {:else if item.kind === 'reasoning'}<ReasoningItem {item} {onFileLink} />
    {:else if item.kind === 'plan'}<PlanItem {item} />
    {:else if item.kind === 'tasks'}<TaskListItem {item} />
    {:else if item.kind === 'command'}<CommandItem {item} {onFileLink} />
    {:else if item.kind === 'file'}<FileChangeItem {item} {onFileLink} />
    {:else if item.kind === 'tool'}<ToolItem {item} {onFileLink} />
    {:else if item.kind === 'subagent'}<SubagentSection {item} />
    {:else if item.kind === 'approval'}<ApprovalItem {item} onDecision={onApprovalDecision} />
    {:else if item.kind === 'input'}<UserInputItem {item} onSubmit={onInputSubmit} />
    {:else if item.kind === 'error'}<ErrorItem {item} />
    {:else}<div class="unknown-item" data-testid="timeline-unknown-item">{item.text}</div>{/if}
  </div>
{/if}

<style>
  /* The list sets 16px between items, which is the space a finished turn earns
     after it. Work rows — tools, thinking, plans, child agents — are part of
     the same turn, so they give 8px of that back and read as one run. */
  .timeline-item{display:block;min-width:0}
  .timeline-item[data-kind='tool'],
  .timeline-item[data-kind='reasoning'],
  .timeline-item[data-kind='subagent'],
  .timeline-item[data-kind='command'],
  .timeline-item[data-kind='file'],
  .timeline-item[data-kind='plan'],
  .timeline-item[data-kind='tasks']{margin-bottom:-8px}
  .timeline-item:last-child{margin-bottom:0}
  .unknown-item{padding:12px;border-left:2px solid var(--color-border);color:var(--color-text-2);white-space:pre-wrap;font-size:13px}
</style>
