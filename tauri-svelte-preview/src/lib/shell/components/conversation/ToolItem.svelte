<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import CircleDot from '@lucide/svelte/icons/circle-dot';
  import FilePenLine from '@lucide/svelte/icons/file-pen-line';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import Minus from '@lucide/svelte/icons/minus';
  import Search from '@lucide/svelte/icons/search';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Wrench from '@lucide/svelte/icons/wrench';
  import X from '@lucide/svelte/icons/x';
  import FileChangeItem from './FileChangeItem.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  let { item, onFileLink }: {
    item: Extract<ConversationDisplayItem, { kind: 'tool' }>;
    onFileLink?(path: string): void;
  } = $props();

  const output = $derived(item.output && item.output !== item.diff ? item.output.trimEnd() : '');
  /* The summary is already the row's preview line, so only real payload —
     output or a diff — earns a body worth opening. */
  const expandable = $derived(!!(output || item.diff));
  const statusLabel = $derived(
    item.state === 'running' ? 'Running' : item.state === 'completed' ? 'Done' : item.state === 'failed' ? 'Failed' : 'Queued'
  );

  /**
   * The one line a collapsed row is allowed. The summary says it best; the
   * first line of output is the fallback when there is no summary.
   */
  const preview = $derived(
    (item.summary || output.split('\n', 1)[0] || item.path || '').replace(/\s+/g, ' ').trim()
  );

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
      {#if item.toolKind === 'command'}<Terminal size={14} strokeWidth={1.8} />
      {:else if item.toolKind === 'file-edit'}<FilePenLine size={14} strokeWidth={1.8} />
      {:else if item.toolKind === 'search'}<Search size={14} strokeWidth={1.8} />
      {:else if item.toolKind === 'fetch'}<Globe2 size={14} strokeWidth={1.8} />
      {:else}<Wrench size={14} strokeWidth={1.8} />{/if}
    </span>
    <strong title={item.title}>{item.title}</strong>
    {#if preview}<span class="preview">{preview}</span>{/if}
    <!-- How the call ended, as one mark: done, failed, or still owed an answer. -->
    <span class="status-mark" title={statusLabel} aria-label={statusLabel} data-testid="timeline-tool-status">
      {#if item.state === 'completed'}<Check size={13} strokeWidth={2.2} />
      {:else if item.state === 'failed'}<X size={13} strokeWidth={2.2} />
      {:else if item.state === 'running'}<CircleDot size={13} strokeWidth={2} />
      {:else}<Minus size={13} strokeWidth={2} />{/if}
    </span>
  </summary>

  {#if expandable}
    <div class="tool-body" data-testid="timeline-tool-body">
      {#if item.diff}<FileChangeItem item={fileDisplayItem(item)} {onFileLink} />{/if}
      {#if output}
        <pre data-testid="timeline-tool-output"><code>{output}</code></pre>
      {/if}
    </div>
  {/if}
</details>

<style>
  /* A collapsed row is just a row: no border, no fill, nothing that makes a
     run of tool calls look like a stack of cards. The box appears when the row
     is opened and its output needs a container. */
  .tool-item{border:1px solid transparent;border-radius:10px}
  .tool-item[open]{border-color:color-mix(in srgb,var(--color-border) 62%,transparent);background:color-mix(in srgb,var(--color-surface) 45%,var(--color-bg) 55%)}
  summary{display:flex;align-items:center;gap:8px;min-height:28px;padding:2px 8px;border-radius:10px;cursor:pointer;list-style:none}
  summary::-webkit-details-marker{display:none}
  summary:hover{background:color-mix(in srgb,var(--color-hover) 55%,transparent)}
  summary:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:-2px}
  .chevron,.tool-icon{display:grid;place-items:center;flex:none;color:var(--color-text-3)}
  .chevron.hidden{visibility:hidden}
  details[open] .chevron{transform:rotate(90deg)}
  strong{flex:none;max-width:50%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font:500 12px/20px ui-monospace,SFMono-Regular,Menlo,monospace}
  .preview{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--color-text-3);font-size:12px;line-height:20px}
  .status-mark{display:grid;place-items:center;flex:none;margin-left:auto;color:var(--color-text-3)}

  /* One accent family per state: a finished call is success, a broken one is
     danger, work still owed is the accent. */
  .completed .status-mark{color:var(--color-good)}
  .failed .status-mark{color:var(--color-bad)}
  .failed strong{color:var(--color-bad)}
  .running .status-mark,.running .tool-icon{color:var(--color-accent)}

  .tool-body{display:grid;gap:8px;padding:0 12px 12px 38px}
  /* Everything the call produced in one bounded region. It scrolls on its own;
     there is no second "show more" inside it. */
  pre{max-height:16rem;overflow:auto;margin:0;padding:8px 10px;border:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);border-radius:8px;background:color-mix(in srgb,var(--color-surface) 30%,var(--color-bg));white-space:pre-wrap;overflow-wrap:anywhere}
  code{font:12px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}

  @media (prefers-reduced-motion:no-preference){
    .chevron{transition:transform .14s ease}
    summary{transition:background .14s ease}
  }
</style>
