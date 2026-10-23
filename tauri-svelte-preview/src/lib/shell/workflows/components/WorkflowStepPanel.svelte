<!--
  WorkflowStepPanel.svelte — the details strip under a run's Map for the step
  chosen there, in four tabs. What: its state and why, and the answer, approve
  or retry buttons when it waits or failed. Evidence: what the step reported
  (WorkflowStepDetails). Log: its rows in the run's timeline. Links: the task,
  its pull request, commit, branch and worktree, its chat, and its files.
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import CircleStop from '@lucide/svelte/icons/circle-stop';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import MessageSquare from '@lucide/svelte/icons/message-square';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Chip } from '$lib/components/ui/chip/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import { openPathFromTauri } from '$lib/tauriSource.ts';
  import { openUrlInBrowser } from '$lib/shell/workbenchNavigation.ts';
  import { shortDuration, stepGlyph } from '../workflowBoard.ts';
  import type { WorkflowTimelineItem } from '../workflowTimeline.ts';
  import { acceptedWithFindings, failedCommands, stepLinks, stepUsageText } from '../workflowStepInfo.ts';
  import { workflowErrorText, type WorkflowNodeRunRecord, type WorkflowPauseDecision, type WorkflowRunRecord, type WorkflowUsageRow } from '../workflowTypes.ts';
  import StepGlyphIcon from './StepGlyphIcon.svelte';
  import WorkflowStepDetails from './WorkflowStepDetails.svelte';
  import WorkflowTimeline from './WorkflowTimeline.svelte';

  interface Props {
    run: WorkflowRunRecord;
    node: WorkflowNodeRunRecord;
    now: number;
    busy: boolean;
    timeline: { items: WorkflowTimelineItem[]; loading: boolean; error: string };
    /** The run is on this machine, so its proof images can be read. */
    local: boolean;
    /** The run's usage rows; null while reading. */
    usage: WorkflowUsageRow[] | null;
    onAnswer: (decision: WorkflowPauseDecision) => void;
    onApprove: () => void;
    onRetry: () => void;
    onOpenChat: () => void;
    /** Opens the review or proof close-up; null for other steps. */
    onCloseup: (() => void) | null;
  }

  let { run, node, now, busy, timeline, local, usage, onAnswer, onApprove, onRetry, onOpenChat, onCloseup }: Props = $props();

  let tab = $state('what');
  const definition = $derived(run.definition.nodes.find((item) => item.id === node.nodeId));
  const glyph = $derived(stepGlyph(node));
  const duration = $derived(node.startedAtMs
    ? shortDuration((node.finishedAtMs ?? (glyph === 'running' ? now : node.startedAtMs)) - node.startedAtMs) : '');
  const log = $derived(timeline.items.filter((item) => item.nodeId === node.nodeId));
  const failed = $derived(failedCommands(node));

  const STATE_WORDS: Record<typeof glyph, string> = {
    passed: 'Passed', running: 'Running', 'sent-back': 'Sent back, waiting to run again', failed: 'Failed', stopped: 'Stopped',
    'needs-you': 'Waiting for you', waiting: 'Waiting for the steps it needs', skipped: 'Skipped'
  };

  // A review accepted with findings still open says so, not plain "Passed".
  const stateWords = $derived(acceptedWithFindings(run, node, timeline.items) ?? STATE_WORDS[glyph]);
  const links = $derived(stepLinks(run, node));
</script>

<section class="flex min-h-0 flex-1 flex-col gap-2 rounded-xl bg-card p-4 shadow-sm" aria-label="Step details" data-testid="workflow-step-panel">
  <div class="flex min-w-0 items-center gap-2">
    <StepGlyphIcon {glyph} />
    <h2 class="truncate text-(length:--text-heading) font-semibold">{definition?.title ?? node.nodeId}</h2>
    <span class="truncate text-[13px] text-muted-foreground">{stateWords}{node.attempt > 1 ? ` · attempt ${node.attempt}` : ''}{duration ? ` · ${duration}` : ''}</span>
    {#if onCloseup}
      <Button size="sm" variant="ghost" class="ml-auto" onclick={onCloseup}><Maximize2 />Close-up</Button>
    {/if}
  </div>
  <Tabs.Root bind:value={tab} class="min-h-0 flex-1">
    <Tabs.List>
      <Tabs.Trigger value="what">What</Tabs.Trigger>
      <Tabs.Trigger value="evidence">Evidence</Tabs.Trigger>
      <Tabs.Trigger value="log">Log {log.length || ''}</Tabs.Trigger>
      <Tabs.Trigger value="links">Links {links.length || ''}</Tabs.Trigger>
    </Tabs.List>
    <div class="min-h-0 flex-1 overflow-y-auto pt-2">
      <Tabs.Content value="what" class="flex-col gap-3 text-[13px] data-[state=active]:flex">
        <p class="[overflow-wrap:anywhere]">
          {node.failure ? workflowErrorText(node.failure.message) : stateWords}
        </p>
        {#each failed as command, index (index)}
          <div class="min-w-0 overflow-hidden rounded-lg bg-foreground/8">
            <div class="flex min-h-8 min-w-0 items-center gap-2 py-1 pr-1.5 pl-2.5">
              <code class="min-w-0 flex-1 font-mono text-sm [overflow-wrap:anywhere]">{command.command}</code>
              <Chip tone="bad">{command.exit}</Chip>
            </div>
            <pre class="m-0 max-h-48 min-w-0 overflow-y-auto border-t px-2.5 py-2 font-mono text-sm leading-normal whitespace-pre-wrap text-muted-foreground [overflow-wrap:anywhere]">{command.tail || 'It printed nothing.'}</pre>
          </div>
        {:else}
          {#if node.failure?.code === 'commands-failed'}
            <p class="text-muted-foreground">The engine recorded no command output for this step.</p>
          {/if}
        {/each}
        <p class="text-muted-foreground">
          {node.provider ? `${node.provider} agent · ${node.roleId}` : 'Run by Assembly'}
          {node.startedAtMs ? ` · started ${new Date(node.startedAtMs).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}` : ''}
        </p>
        <div class="flex flex-wrap gap-2">
          {#if node.state === 'waiting-input'}
            <Button size="sm" disabled={busy} onclick={() => onAnswer('another-round')}><RotateCcw />Run another round</Button>
            <Button size="sm" variant="ghost" disabled={busy} onclick={() => onAnswer('accept')}><Check />Accept as is</Button>
            <Button size="sm" variant="ghost" disabled={busy} onclick={() => onAnswer('stop')}><CircleStop />Stop</Button>
          {:else if node.state === 'waiting-approval'}
            <Button size="sm" disabled={busy} onclick={onApprove}><Check />Approve</Button>
          {:else if node.state === 'failed'}
            <Button size="sm" variant="ghost" disabled={busy} onclick={onRetry}><RotateCcw />Retry this step</Button>
          {/if}
        </div>
      </Tabs.Content>
      <Tabs.Content value="evidence">
        <WorkflowStepDetails
          {node}
          status={{ chip: null, tone: 'neutral', summary: stateWords, notRun: glyph === 'skipped' }}
          kind={node.provider ? `${node.provider} agent · ${node.roleId}` : 'Run by Assembly'}
          {duration}
          usage={usage ? stepUsageText(usage, node.id) : 'Reading…'}
          {busy}
          onRetry={null}
          rounds={log.filter((item) => ['review', 'checks', 'finish', 'failed', 'timeout', 'waiting'].includes(item.kind))}
        />
      </Tabs.Content>
      <Tabs.Content value="log">
        <WorkflowTimeline items={log} loading={timeline.loading} error={timeline.error} {local} onOpenStep={() => {}} />
      </Tabs.Content>
      <Tabs.Content value="links" class="flex-col gap-1 text-[13px] data-[state=active]:flex">
        {#if node.provider && node.startedAtMs}
          <Button size="sm" variant="ghost" class="self-start" onclick={onOpenChat}><MessageSquare />Open this step's chat</Button>
        {/if}
        <dl class="flex flex-col">
          {#each links as link, index (index)}
            <div class="flex min-h-8 items-center gap-3 border-b py-1.5 last:border-b-0">
              <dt class="w-28 shrink-0 text-muted-foreground">{link.label}</dt>
              <dd class="min-w-0 flex-1 font-mono text-sm [overflow-wrap:anywhere]">
                {#if link.url || (link.path && local)}
                  {@const { url, path } = link}
                  <button type="button" class="inline-flex items-center gap-1 rounded-md text-left text-primary outline-none hover:underline focus-visible:ring-3 focus-visible:ring-ring/50" onclick={() => void (url ? openUrlInBrowser({ url }) : openPathFromTauri(path ?? ''))}>
                    {#if url}<ExternalLink class="size-3.5 shrink-0" aria-hidden="true" />{:else}<FolderOpen class="size-3.5 shrink-0" aria-hidden="true" />{/if}{link.value}
                  </button>
                {:else}{link.value}{/if}
              </dd>
            </div>
          {/each}
        </dl>
      </Tabs.Content>
    </div>
  </Tabs.Root>
</section>
