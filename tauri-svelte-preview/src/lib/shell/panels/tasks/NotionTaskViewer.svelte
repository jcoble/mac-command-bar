<script lang="ts">
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Check from '@lucide/svelte/icons/check';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Square from '@lucide/svelte/icons/square';
  import { onDestroy, onMount } from 'svelte';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import {
    readNotionTaskDetail,
    type NotionTaskDetailBlock
  } from '$lib/shell/notionTaskDetail.ts';
  import type { NotionTaskRow } from '$lib/shell/notionTasks.ts';

  let { task, onBack, onOpenExternal }: {
    task: NotionTaskRow;
    onBack(): void;
    onOpenExternal(url: string): void;
  } = $props();

  let blocks = $state<NotionTaskDetailBlock[]>([]);
  let loading = $state(true);
  let error = $state('');
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

  onMount(() => void load());
  onDestroy(() => {
    controller.abort();
    blocks = [];
  });
</script>

<div class="flex h-full min-h-0 flex-col bg-background">
  <header class="flex items-center gap-2 border-b border-border px-2 py-2">
    <IconButton label="Back to tasks" onclick={onBack}><ArrowLeft /></IconButton>
    <p class="min-w-0 flex-1 truncate text-sm font-medium text-foreground">Task</p>
    <IconButton label="Open in Notion" onclick={() => onOpenExternal(task.sourceUrl)}>
      <ExternalLink />
    </IconButton>
  </header>

  <ScrollArea class="min-h-0 flex-1">
    <article class="px-4 pt-4 pb-8">
      <h2 class="text-lg leading-snug font-semibold text-foreground">{task.title}</h2>
      <div class="mt-3 flex flex-wrap gap-1.5 text-xs text-muted-foreground">
        <span class="rounded-md bg-accent px-2 py-1 text-accent-foreground">{task.status}</span>
        {#if task.project}<span class="rounded-md border border-border px-2 py-1">{task.project}</span>{/if}
        {#if task.priority}<span class="rounded-md border border-border px-2 py-1">{task.priority}</span>{/if}
        {#if task.dueDate}<span class="rounded-md border border-border px-2 py-1">Due {task.dueDate}</span>{/if}
      </div>

      {#if loading}
        <p class="mt-8 text-sm text-muted-foreground">Loading task…</p>
      {:else if error}
        <p class="mt-6 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>
      {:else if blocks.length === 0}
        <p class="mt-8 text-sm text-muted-foreground">This task has no additional notes.</p>
      {:else}
        <div class="mt-6 grid gap-3 text-sm leading-relaxed text-foreground">
          {#each blocks as block, index (`${block.kind}-${index}`)}
            {#if block.kind === 'heading_1'}
              <h3 class="pt-3 text-lg font-semibold">{block.text}</h3>
            {:else if block.kind === 'heading_2'}
              <h3 class="pt-3 text-base font-semibold">{block.text}</h3>
            {:else if block.kind === 'heading_3'}
              <h4 class="pt-2 text-sm font-semibold">{block.text}</h4>
            {:else if block.kind === 'bulleted_list_item'}
              <p class="grid grid-cols-[0.75rem_1fr] gap-2"><span aria-hidden="true">•</span><span>{block.text}</span></p>
            {:else if block.kind === 'numbered_list_item'}
              <p class="grid grid-cols-[1.25rem_1fr] gap-2"><span aria-hidden="true">{index + 1}.</span><span>{block.text}</span></p>
            {:else if block.kind === 'to_do'}
              <p class="flex items-start gap-2">
                {#if block.checked}<Check class="mt-0.5 size-4 shrink-0 text-primary" />{:else}<Square class="mt-0.5 size-4 shrink-0 text-muted-foreground" />{/if}
                <span class:line-through={block.checked}>{block.text}</span>
              </p>
            {:else if block.kind === 'quote'}
              <blockquote class="border-l-2 border-primary pl-3 text-muted-foreground">{block.text}</blockquote>
            {:else if block.kind === 'code'}
              <pre class="overflow-x-auto rounded-md bg-muted p-3 text-xs"><code>{block.text}</code></pre>
            {:else if block.kind === 'divider'}
              <hr class="border-border" />
            {:else}
              <p class="whitespace-pre-wrap">{block.text}</p>
            {/if}
          {/each}
        </div>
      {/if}
    </article>
  </ScrollArea>
</div>
