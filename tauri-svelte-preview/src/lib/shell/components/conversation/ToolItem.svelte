<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import FilePenLine from '@lucide/svelte/icons/file-pen-line';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import Search from '@lucide/svelte/icons/search';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Wrench from '@lucide/svelte/icons/wrench';
  import FileChangeItem from './FileChangeItem.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  let { item, onFileLink }: {
    item: Extract<ConversationDisplayItem, { kind: 'tool' }>;
    onFileLink?(path: string): void;
  } = $props();

  const expandable = $derived(!!(item.output || item.diff || item.summary));
  const statusLabel = $derived(item.state === 'running' ? 'Running' : item.state === 'completed' ? 'Done' : item.state === 'failed' ? 'Failed' : 'Queued');

  function fileDisplayItem(tool: Extract<ConversationDisplayItem, { kind: 'tool' }>): Extract<ConversationDisplayItem, { kind: 'file' }> {
    return {
      kind: 'file',
      itemId: `${tool.itemId}:diff`,
      text: tool.diff ?? '',
      completed: tool.state === 'completed',
      timestampMs: tool.timestampMs,
      metadata: { path: tool.path ?? '', diff: tool.diff ?? '' }
    };
  }
</script>

<details
  class:completed={item.state === 'completed'}
  class:failed={item.state === 'failed'}
  class:running={item.state === 'running'}
  class="tool-item"
  data-testid="timeline-tool-item"
>
  <summary aria-disabled={!expandable}>
    <span class="chevron" class:hidden={!expandable} aria-hidden="true"><ChevronRight size={14} strokeWidth={1.8} /></span>
    <span class="tool-icon" aria-hidden="true">
      {#if item.toolKind === 'command'}<Terminal size={15} strokeWidth={1.8} />
      {:else if item.toolKind === 'file-edit'}<FilePenLine size={15} strokeWidth={1.8} />
      {:else if item.toolKind === 'search'}<Search size={15} strokeWidth={1.8} />
      {:else if item.toolKind === 'fetch'}<Globe2 size={15} strokeWidth={1.8} />
      {:else}<Wrench size={15} strokeWidth={1.8} />{/if}
    </span>
    <strong title={item.title}>{item.title}</strong>
    <span class="status-chip" data-testid="timeline-tool-status">
      {#if item.state === 'running'}<span class="status-icon spinner"><LoaderCircle size={13} strokeWidth={2} /></span>
      {:else if item.state === 'completed'}<Check class="status-icon" size={13} strokeWidth={2.2} />
      {:else if item.state === 'failed'}<CircleX class="status-icon" size={13} strokeWidth={2} />{/if}
      {statusLabel}
    </span>
  </summary>

  {#if expandable}
    <div class="tool-body" data-testid="timeline-tool-body">
      {#if item.summary && item.summary !== item.output}<p>{item.summary}</p>{/if}
      {#if item.diff}<FileChangeItem item={fileDisplayItem(item)} {onFileLink} />{/if}
      {#if item.output && item.output !== item.diff}<pre data-testid="timeline-tool-output"><code>{item.output}</code></pre>{/if}
    </div>
  {/if}
</details>

<style>
  .tool-item{border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:9px;background:color-mix(in srgb,var(--color-surface) 42%,transparent);overflow:hidden}
  summary{display:flex;align-items:center;gap:8px;min-height:38px;padding:6px 9px;cursor:pointer;list-style:none}
  summary::-webkit-details-marker{display:none}
  .chevron,.tool-icon{display:grid;place-items:center;flex:none;color:var(--color-text-2)}
  .chevron{transition:transform .16s ease}
  .chevron.hidden{visibility:hidden}
  details[open] .chevron{transform:rotate(90deg)}
  strong{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font:500 12.5px/1.3 ui-monospace,SFMono-Regular,Menlo,monospace}
  .status-chip{display:flex;align-items:center;gap:4px;flex:none;padding:2px 7px;border-radius:999px;background:color-mix(in srgb,var(--color-border) 42%,transparent);color:var(--color-text-2);font-size:12px}
  .completed{border-color:color-mix(in srgb,var(--color-good) 28%,var(--color-border))}.completed .status-chip{background:color-mix(in srgb,var(--color-good) 13%,transparent);color:var(--color-good)}
  .failed{border-color:color-mix(in srgb,var(--color-bad) 45%,var(--color-border))}.failed .status-chip{background:color-mix(in srgb,var(--color-bad) 13%,transparent);color:var(--color-bad)}
  .running .tool-icon{color:var(--color-accent)}
  .status-icon{display:grid;place-items:center}.spinner{animation:spin .85s linear infinite}
  .tool-body{display:grid;gap:8px;padding:0 10px 10px 39px}
  .tool-body p{margin:0;color:var(--color-text-2);font-size:12px}
  pre{max-height:260px;overflow:auto;margin:0;padding:9px 10px;border:1px solid color-mix(in srgb,var(--color-border) 68%,transparent);border-radius:7px;background:color-mix(in srgb,var(--color-bg) 80%,var(--color-surface));white-space:pre-wrap;overflow-wrap:anywhere}
  code{font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace}
  @keyframes spin{to{transform:rotate(360deg)}}
  @media (prefers-reduced-motion:reduce){.spinner{animation:none}.chevron{transition:none}}
</style>
