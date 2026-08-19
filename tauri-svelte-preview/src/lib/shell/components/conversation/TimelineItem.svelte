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
  import CompactionItem from './CompactionItem.svelte';

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
    {:else if item.kind === 'compaction'}<CompactionItem {item} />
    {:else}<div class="unknown-item" data-testid="timeline-unknown-item">{item.text}</div>{/if}
  </div>
{/if}

<style>
  /* One spacing scale runs the whole transcript: 8px between rows of a run,
     12px between blocks inside a message, 16px between the steps of a turn,
     28px between turns. The list supplies the 16px; the two rules below are
     the other two ends of it.

     A work row — a tool, thinking, a plan, a child agent — belongs to the row
     above it, so it pulls itself up to 8px and the run reads as one block. The
     pull is on the top, not the bottom, so the reply that follows the work
     still gets its full 16px and is not glued to the tools that produced it.

     A user message is the opposite: it starts a new turn, so it pushes down to
     28px. That gap is the widest thing in the transcript, which is what makes
     a turn boundary visible without a rule or a heading. */
  .timeline-item{display:block;min-width:0}
  .timeline-item[data-kind='tool'],
  .timeline-item[data-kind='reasoning'],
  .timeline-item[data-kind='subagent'],
  .timeline-item[data-kind='command'],
  .timeline-item[data-kind='file'],
  .timeline-item[data-kind='plan'],
  .timeline-item[data-kind='tasks']{margin-top:-8px}
  .timeline-item[data-kind='user']{margin-top:12px}
  .timeline-item:first-child{margin-top:0}
  .unknown-item{padding:12px;border-left:2px solid var(--color-border);color:var(--color-text-2);white-space:pre-wrap;font-size:13px}
</style>
