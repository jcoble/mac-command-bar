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
  import { collapseToolOutput, toolOutputLines } from './collapseToolOutput.ts';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  let { item, onFileLink }: {
    item: Extract<ConversationDisplayItem, { kind: 'tool' }>;
    onFileLink?(path: string): void;
  } = $props();

  const expandable = $derived(!!(item.output || item.diff || item.summary));
  const statusLabel = $derived(item.state === 'running' ? 'Running' : item.state === 'completed' ? 'Done' : item.state === 'failed' ? 'Failed' : 'Queued');

  /** Expanding long output is a choice about this tool call, not all of them. */
  let outputExpanded = $state(false);
  const outputLines = $derived(item.output && item.output !== item.diff ? toolOutputLines(item.output) : []);
  const collapsed = $derived(collapseToolOutput(outputLines));
  const shownLines = $derived(outputExpanded ? outputLines : collapsed.visible);

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
      {#if shownLines.length}
        <div class="output-block">
          <pre data-testid="timeline-tool-output"><code>{shownLines.join('\n')}</code></pre>
          {#if collapsed.hiddenCount > 0}
            <button
              class="expand-output"
              data-testid="timeline-tool-output-toggle"
              type="button"
              aria-expanded={outputExpanded}
              onclick={() => (outputExpanded = !outputExpanded)}
            >{outputExpanded ? 'Show fewer lines' : `Show ${collapsed.hiddenCount} more lines`}</button>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</details>

<style>
  .tool-item{border:1px solid color-mix(in srgb,var(--color-border) 62%,transparent);border-radius:10px;background:color-mix(in srgb,var(--color-surface) 55%,var(--color-bg) 45%);overflow:hidden}
  summary{display:flex;align-items:center;gap:8px;min-height:38px;padding:6px 10px;cursor:pointer;list-style:none}
  summary::-webkit-details-marker{display:none}
  summary:hover{background:color-mix(in srgb,var(--color-hover) 55%,transparent)}
  .chevron,.tool-icon{display:grid;place-items:center;flex:none;color:var(--color-text-2)}
  .chevron.hidden{visibility:hidden}
  details[open] .chevron{transform:rotate(90deg)}
  strong{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font:500 13px/1.3 ui-monospace,SFMono-Regular,Menlo,monospace}
  .status-chip{display:flex;align-items:center;gap:4px;flex:none;padding:2px 8px;border-radius:999px;background:color-mix(in srgb,var(--color-attention) 12%,transparent);color:var(--color-attention);font-size:12px}

  /* One accent family per state: work in progress is the accent, a finished
     call is success, a broken one is danger, and a queued one is attention. */
  .running{border-color:color-mix(in srgb,var(--color-accent) 30%,var(--color-border));background:color-mix(in srgb,var(--color-surface) 55%,var(--color-accent) 6%)}
  .running .tool-icon{color:var(--color-accent)}
  .running .status-chip{background:color-mix(in srgb,var(--color-accent) 14%,transparent);color:var(--color-accent)}
  .completed{border-color:color-mix(in srgb,var(--color-good) 24%,var(--color-border))}
  .completed .status-chip{background:color-mix(in srgb,var(--color-good) 13%,transparent);color:var(--color-good)}
  .failed{border-color:color-mix(in srgb,var(--color-bad) 40%,var(--color-border));background:color-mix(in srgb,var(--color-surface) 55%,var(--color-bad) 6%)}
  .failed .status-chip{background:color-mix(in srgb,var(--color-bad) 14%,transparent);color:var(--color-bad)}

  .status-icon{display:grid;place-items:center}
  .tool-body{display:grid;gap:10px;padding:0 12px 12px 40px}
  .tool-body p{margin:0;color:var(--color-text-2);font-size:13px;line-height:1.5}
  .output-block{display:grid;gap:6px}
  pre{max-height:340px;overflow:auto;margin:0;padding:10px 12px;border:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);border-radius:8px;background:color-mix(in srgb,var(--color-surface) 30%,var(--color-bg));white-space:pre-wrap;overflow-wrap:anywhere}
  code{font:12px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}
  .expand-output{justify-self:start;border:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);border-radius:999px;background:transparent;color:var(--color-text-2);padding:3px 11px;font-size:13px;cursor:pointer}
  .expand-output:hover{background:color-mix(in srgb,var(--color-hover) 60%,transparent);color:var(--color-text)}
  .expand-output:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}

  @media (prefers-reduced-motion:no-preference){
    .chevron{transition:transform .14s ease}
    summary,.expand-output{transition:background .14s ease,color .14s ease}
    .spinner{animation:spin .85s linear infinite}
    .tool-body{animation:body-in .14s ease-out both}
    @keyframes spin{to{transform:rotate(360deg)}}
    @keyframes body-in{from{opacity:0;transform:translateY(-2px)}to{opacity:1;transform:none}}
  }
</style>
