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

  /**
   * How much output a row shows before it folds. A result over 20 lines is
   * past the point where reading it in place is the plan, so the row keeps the
   * first 12 — enough to see what kind of answer came back — and says how many
   * it is holding. Below 20 there is nothing worth hiding: a control that folds
   * six lines away costs more than the six lines.
   */
  const OUTPUT_FOLD_OVER_LINES = 20;
  const OUTPUT_LINES_KEPT = 12;

  const output = $derived(item.output && item.output !== item.diff ? item.output.trimEnd() : '');
  const outputLines = $derived(output ? output.split('\n') : []);
  const foldable = $derived(outputLines.length > OUTPUT_FOLD_OVER_LINES);
  let outputOpen = $state(false);
  const shownOutput = $derived(
    foldable && !outputOpen ? outputLines.slice(0, OUTPUT_LINES_KEPT).join('\n') : output
  );
  const hiddenLines = $derived(outputLines.length - OUTPUT_LINES_KEPT);
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
        <pre data-testid="timeline-tool-output"><code>{shownOutput}</code></pre>
        {#if foldable}
          <button
            class="fold-more"
            data-testid="timeline-tool-output-fold"
            type="button"
            aria-expanded={outputOpen}
            onclick={() => (outputOpen = !outputOpen)}
          ><span class="fold-chevron" class:open={outputOpen} aria-hidden="true"><ChevronRight size={13} strokeWidth={1.8} /></span
            >{outputOpen ? 'Show less' : `Show ${hiddenLines.toLocaleString()} more lines`}</button>
        {/if}
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
  /* The output box used to be a fixed 16rem window on however much there was,
     which told a reader nothing about the size of what they were scrolling and
     put a second scroll region in the middle of the page. It now shows a
     bounded number of lines and says how many it is holding, so the height on
     screen is the height of what is being shown. Long lines run sideways
     inside the box rather than wrapping mid-word; the pane never widens. */
  pre{overflow-x:auto;margin:0;padding:8px 10px;border:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);border-radius:8px;background:color-mix(in srgb,var(--color-surface) 30%,var(--color-bg));white-space:pre;scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent;overscroll-behavior-x:contain}
  code{font:13px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}
  /* The same chevron the row header uses, so the control reads as one more
     thing that opens rather than as a caption under the box. */
  .fold-more{display:inline-flex;align-items:center;gap:5px;justify-self:start;min-height:24px;padding:2px 8px;margin-left:-8px;border:0;border-radius:6px;background:transparent;color:var(--color-text-2);font-size:12px;text-align:left;cursor:pointer}
  .fold-chevron{display:grid;place-items:center;color:var(--color-text-3)}
  .fold-chevron.open{transform:rotate(90deg)}
  .fold-more:hover{background:color-mix(in srgb,var(--color-hover) 55%,transparent);color:var(--color-text)}
  .fold-more:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:1px}

  @media (prefers-reduced-motion:no-preference){
    .chevron{transition:transform .14s ease}
    summary{transition:background .14s ease}
    .fold-more{transition:background .14s ease,color .14s ease}
    .fold-chevron{transition:transform .14s ease}
  }
</style>
