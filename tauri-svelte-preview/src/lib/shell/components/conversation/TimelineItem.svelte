<script lang="ts">
  import { onMount } from 'svelte';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import {
    conversationFileLinkProvenance,
    type ConversationDisplayItem,
    type ConversationFileLinkProvenance
  } from '$lib/shell/conversation/conversationTimeline.ts';
  import { conversationItemHasVisibleContent } from '$lib/shell/conversation/conversationItemVisibility.ts';
  import UserMessageItem from './UserMessageItem.svelte';
  import AssistantMessageItem from './AssistantMessageItem.svelte';
  import ReasoningItem from './ReasoningItem.svelte';
  import TaskListItem from './TaskListItem.svelte';
  import CommandItem from './CommandItem.svelte';
  import FileChangeItem from './FileChangeItem.svelte';
  import FileEditsItem from './FileEditsItem.svelte';
  import ToolRunItem from './ToolRunItem.svelte';
  import ToolItem from './ToolItem.svelte';
  import SubagentSection from './SubagentSection.svelte';
  import ApprovalItem from './ApprovalItem.svelte';
  import UserInputItem from './UserInputItem.svelte';
  import ErrorItem from './ErrorItem.svelte';
  import CompactionItem from './CompactionItem.svelte';
  import { trackConversationTimelineItem } from '$lib/shell/resourceDiagnostics.svelte.ts';

  interface Props {
    item: ConversationDisplayItem;
    assistantLabel?: string;
    onApprovalDecision?(requestId: string, decision: string): void;
    onInputSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void;
    onFileLink?(path: string, provenance?: ConversationFileLinkProvenance): void;
    /** Opens the plan chip above the composer. The transcript only notes that
     * the plan moved; the plan itself lives in one place. */
    onPlanOpen?(): void;
  }
  // `assistantLabel` is retired: nothing in the transcript carries a role
  // heading any more. It stays accepted only so the timeline that still passes
  // it keeps type-checking; the container drops it in its own pass.
  let { item, onApprovalDecision, onInputSubmit, onFileLink, onPlanOpen }: Props = $props();
  const visible = $derived(conversationItemHasVisibleContent(item));
  const fileLinkProvenance = $derived(conversationFileLinkProvenance(item));
  const openFileLink = (path: string): void => onFileLink?.(path, fileLinkProvenance);
  onMount(trackConversationTimelineItem);
</script>

{#if visible}
  <div class="timeline-item" data-testid="conversation-timeline-item" data-item-id={item.itemId} data-kind={item.kind}>
    {#if item.kind === 'user'}<UserMessageItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'assistant'}<AssistantMessageItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'toolRun'}<ToolRunItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'reasoning'}<ReasoningItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'plan'}<button class="plan-line" data-testid="timeline-plan-item" type="button" onclick={() => onPlanOpen?.()}>Plan updated · {item.steps.length} {item.steps.length === 1 ? 'step' : 'steps'}</button>
    {:else if item.kind === 'tasks'}<TaskListItem {item} />
    {:else if item.kind === 'command'}<CommandItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'file'}<FileChangeItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'fileEdits'}<FileEditsItem {item} onFileLink={openFileLink} />
    {:else if item.kind === 'tool'}<ToolItem {item} onFileLink={openFileLink} />
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
     above it, so it pulls itself up to 16px and the run reads as one block. The
     pull is on the top, not the bottom, so the reply that follows the work
     still gets its full 16px and is not glued to the tools that produced it.

     A user message is the opposite: it starts a new turn, so it pushes down to
     38px. That gap is the widest thing in the transcript, which is what makes
     a turn boundary visible without a rule or a heading. */
  .timeline-item{display:block;min-width:0}
  .timeline-item[data-kind='tool'],
  .timeline-item[data-kind='reasoning'],
  .timeline-item[data-kind='subagent'],
  .timeline-item[data-kind='command'],
  .timeline-item[data-kind='file'],
  .timeline-item[data-kind='fileEdits'],
  .timeline-item[data-kind='plan'],
  .timeline-item[data-kind='tasks']{margin-top:4px}
  .timeline-item[data-kind='user']{margin-top:24px;margin-bottom:8px}
  .timeline-item[data-kind='assistant']{margin-top:8px}
  .timeline-item:first-child{margin-top:0}
  /* The plan is drawn once, in the chip above the composer. All the transcript
     owes the reader is that it moved, and a way back to it. */
  .plan-line{display:inline-flex;align-items:center;padding:0;border:0;background:transparent;color:var(--color-text-3);font:12px/1.4 inherit;cursor:pointer}
  .plan-line:hover{color:var(--color-text-2)}
  .plan-line:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:3px;border-radius:4px}
  .unknown-item{padding:12px;border-left:2px solid var(--color-border);color:var(--color-text-2);white-space:pre-wrap;font-size:13px}
</style>
