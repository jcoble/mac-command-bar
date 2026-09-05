<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Check from '@lucide/svelte/icons/check';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import Square from '@lucide/svelte/icons/square';
  import { onDestroy, onMount } from 'svelte';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import {
    readNotionTaskDetail,
    type NotionTaskDetailBlock,
    updateNotionTaskStatus
  } from '$lib/shell/notionTaskDetail.ts';
  import type { NotionTaskRow } from '$lib/shell/notionTasks.ts';

  let { task, statuses, onBack, onOpenExternal, onStatusUpdated }: {
    task: NotionTaskRow;
    statuses: string[];
    onBack(): void;
    onOpenExternal(url: string): void;
    onStatusUpdated(status: string): Promise<void>;
  } = $props();

  let blocks = $state<NotionTaskDetailBlock[]>([]);
  let loading = $state(true);
  let error = $state('');
  let status = $state('');
  let savingStatus = $state(false);
  const statusOptions = $derived([...new Set([task.status, ...statuses].filter(Boolean))]);
  const controller = new AbortController();

  async function load(): Promise<void> {
    try {
      blocks = (await readNotionTaskDetail(task.sourceTaskId, controller.signal)).blocks;
    } catch (cause) {
      if (!controller.signal.aborted) error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      if (!controller.signal.aborted) loading = false;
    }
  }

  async function changeStatus(nextStatus: string): Promise<void> {
    if (!nextStatus || nextStatus === status || savingStatus) return;
    const previousStatus = status;
    status = nextStatus;
    savingStatus = true;
    error = '';
    try {
      await updateNotionTaskStatus(task.sourceTaskId, nextStatus, controller.signal);
      if (controller.signal.aborted) return;
      await onStatusUpdated(nextStatus);
    } catch (cause) {
      if (!controller.signal.aborted) {
        status = previousStatus;
        error = cause instanceof Error ? cause.message : String(cause);
      }
    } finally {
      if (!controller.signal.aborted) savingStatus = false;
    }
  }

  onMount(() => {
    status = task.status;
    void load();
  });
  onDestroy(() => {
    controller.abort();
    blocks = [];
  });
</script>

<div class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-background">
  <header class="flex items-center gap-(--space-2) border-b border-border px-(--space-2) py-(--space-2)">
    <IconButton label="Back to tasks" onclick={onBack}><ArrowLeft /></IconButton>
    <p class="min-w-0 flex-1 truncate text-(length:--text-heading) font-(--text-heading-weight) text-foreground">Task</p>
    <IconButton label="Task actions" disabled><Ellipsis /></IconButton>
    <IconButton label="Open in Notion" onclick={() => onOpenExternal(task.sourceUrl)}>
      <ExternalLink />
    </IconButton>
  </header>

  <ScrollArea class="min-h-0 flex-1">
    <article class="w-full min-w-0 max-w-full px-(--space-3) pt-(--space-4) pb-(--space-6)">
      <section class="min-w-0 rounded-(--radius-md) bg-card px-(--space-4) py-(--space-4) shadow-(--shadow-sm)">
        <p class="text-(length:--text-quiet) font-medium tracking-wide text-muted-foreground uppercase">Notion task</p>
        <h2 class="mt-(--space-2) text-2xl leading-snug font-semibold text-foreground [overflow-wrap:anywhere]">{task.title}</h2>
        <div class="mt-(--space-4) flex flex-wrap gap-(--space-2) text-(length:--text-quiet) text-muted-foreground">
          <Select.Root type="single" value={status} onValueChange={(value) => void changeStatus(value)} disabled={savingStatus}>
            <Select.Trigger size="sm" class="rounded-full bg-primary px-2.5 font-medium text-primary-foreground" aria-label="Change task status">
              {savingStatus ? 'Saving…' : status}
            </Select.Trigger>
            <Select.Content>
              {#each statusOptions as option}<Select.Item value={option} label={option} />{/each}
            </Select.Content>
          </Select.Root>
          {#if task.project}<span class="rounded-full border border-border px-2.5 py-1">{task.project}</span>{/if}
          {#if task.priority}<span class="rounded-full border border-border px-2.5 py-1">{task.priority}</span>{/if}
          {#if task.dueDate}<span class="rounded-full border border-border px-2.5 py-1">Due {task.dueDate}</span>{/if}
        </div>
      </section>

      <section class="mt-(--space-3) min-w-0 rounded-(--radius-md) bg-card px-(--space-4) py-(--space-4) shadow-(--shadow-sm)">
        <h3 class="text-xl font-semibold text-foreground">Details</h3>
        {#if loading}
          <p class="mt-(--space-5) text-(length:--text-heading) text-muted-foreground">Loading task…</p>
        {:else if error}
          <p class="mt-4 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>
        {:else if blocks.length === 0}
          <p class="mt-(--space-5) text-(length:--text-heading) text-muted-foreground">This task has no additional notes.</p>
        {:else}
          <div class="mt-(--space-5) grid min-w-0 gap-(--space-4) text-(length:--text-heading) leading-relaxed text-foreground [overflow-wrap:anywhere]">
            {#each blocks as block, index (`${block.kind}-${index}`)}
              {#if block.kind === 'heading_1'}
                <h3 class="pt-3 text-lg font-semibold">{block.text}</h3>
              {:else if block.kind === 'heading_2'}
                <h3 class="pt-3 text-base font-semibold">{block.text}</h3>
              {:else if block.kind === 'heading_3'}
                <h4 class="pt-2 text-sm font-semibold">{block.text}</h4>
              {:else if block.kind === 'bulleted_list_item'}
                <p class="grid min-w-0 grid-cols-[0.75rem_minmax(0,1fr)] gap-2"><span aria-hidden="true">•</span><span>{block.text}</span></p>
              {:else if block.kind === 'numbered_list_item'}
                <p class="grid min-w-0 grid-cols-[1.25rem_minmax(0,1fr)] gap-2"><span aria-hidden="true">{index + 1}.</span><span>{block.text}</span></p>
              {:else if block.kind === 'to_do'}
                <p class="flex items-start gap-2">
                  {#if block.checked}<Check class="mt-0.5 size-4 shrink-0 text-primary" />{:else}<Square class="mt-0.5 size-4 shrink-0 text-muted-foreground" />{/if}
                  <span class="min-w-0" class:line-through={block.checked}>{block.text}</span>
                </p>
              {:else if block.kind === 'quote'}
                <blockquote class="border-l-2 border-primary pl-3 text-muted-foreground">{block.text}</blockquote>
              {:else if block.kind === 'code'}
                <pre class="min-w-0 max-w-full overflow-x-auto rounded-md bg-muted p-3 text-xs [overflow-wrap:normal]"><code>{block.text}</code></pre>
              {:else if block.kind === 'divider'}
                <hr class="border-border" />
              {:else}
                <p class="whitespace-pre-wrap">{block.text}</p>
              {/if}
            {/each}
          </div>
        {/if}
      </section>
    </article>
  </ScrollArea>
</div>
