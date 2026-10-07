<script lang="ts">
  import { conversationDisclosureContext, type ConversationDisclosureContext } from '$lib/shell/conversation/conversationChatUI.ts';
  import { getContext } from 'svelte';
  import Check from '@lucide/svelte/icons/check';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import FileText from '@lucide/svelte/icons/file-text';
  import CircleDot from '@lucide/svelte/icons/circle-dot';
  import FilePenLine from '@lucide/svelte/icons/file-pen-line';
  import Globe2 from '@lucide/svelte/icons/globe-2';
  import Minus from '@lucide/svelte/icons/minus';
  import Search from '@lucide/svelte/icons/search';
  import Terminal from '@lucide/svelte/icons/terminal';
  import Wrench from '@lucide/svelte/icons/wrench';
  import X from '@lucide/svelte/icons/x';
  import FileChangeItem from './FileChangeItem.svelte';
  import { toolFilePath, type ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  import { sanitizeConversationHref } from '$lib/shell/conversation/conversationMessageSafety.ts';
  import { openUrlInBrowser } from '$lib/shell/workbenchNavigation.ts';
  import { highlightCode, languageForPath } from './codeHighlight.ts';

  const disclosure = getContext<ConversationDisclosureContext>(conversationDisclosureContext);

  let { item, onFileLink }: {
    item: Extract<ConversationDisplayItem, { kind: 'tool' }>;
    onFileLink?(path: string): void;
  } = $props();

  const input = $derived.by(() => {
    const value = item.metadata?.rawInput ?? (item.toolKind === 'command' ? item.summary : undefined);
    return value == null ? '' : typeof value === 'string' ? value : JSON.stringify(value, null, 2);
  });
  const output = $derived(item.output && item.output !== item.diff ? item.output.trimEnd() : '');
  const detailsOpen = $derived(disclosure?.get(`${item.itemId}:details`) ?? false);
  const filePath = $derived(toolFilePath(item));
  const fileAction = $derived(item.toolKind === 'file-edit' || (item.toolKind === 'fetch' && !!filePath && !/^https?:/i.test(filePath)));
  const label = $derived(fileAction ? item.toolKind === 'file-edit' ? 'Edited' : 'Read' : item.title);
  const shownLines = $derived(detailsOpen ? highlightCode(output, languageForPath(filePath)) : []);
  /* Call arguments remain inspectable even when the tool returns no output. */
  const expandable = $derived(!!(input || output || item.diff));
  const statusLabel = $derived(
    item.state === 'running' ? 'Running' : item.state === 'completed' ? 'Done' : item.state === 'failed' ? 'Failed' : 'Queued'
  );

  /**
   * The one line a collapsed row is allowed. The summary says it best; the
   * first line of output is the fallback when there is no summary.
   */
  const preview = $derived(
    fileAction && filePath ? filePath.split(/[\\/]/).pop() ?? filePath
      : (item.summary || output.split('\n', 1)[0] || '').replace(/\s+/g, ' ').trim()
  );
  const targetUrl = $derived.by(() => {
    const href = filePath ? sanitizeConversationHref(filePath) : null;
    return href && /^https?:/i.test(href) ? href : '';
  });

  function fileDisplayItem(tool: Extract<ConversationDisplayItem, { kind: 'tool' }>): Extract<ConversationDisplayItem, { kind: 'file' }> {
    return {
      kind: 'file',
      itemId: `${tool.itemId}:diff`,
      text: tool.diff ?? '',
      completed: tool.state === 'completed',
      timestampMs: tool.timestampMs,
      metadata: { path: toolFilePath(tool), diff: tool.diff ?? '' }
    };
  }
</script>

<details
  class:completed={item.state === 'completed'}
  class:failed={item.state === 'failed'}
  class:running={item.state === 'running'}
  class="tool-item"
  open={detailsOpen}
  ontoggle={(event) => { if (event.currentTarget.open !== detailsOpen) disclosure?.set(`${item.itemId}:details`, event.currentTarget.open); }}
  data-testid="timeline-tool-item"
>
  <summary aria-disabled={!expandable}>
    <span class="chevron" class:hidden={!expandable} aria-hidden="true"><ChevronRight size={14} strokeWidth={1.8} /></span>
    <span class="tool-icon" aria-hidden="true">
      {#if item.toolKind === 'command'}<Terminal size={14} strokeWidth={1.8} />
      {:else if item.toolKind === 'file-edit'}<FilePenLine size={14} strokeWidth={1.8} />
      {:else if fileAction}<FileText size={14} strokeWidth={1.8} />
      {:else if item.toolKind === 'search'}<Search size={14} strokeWidth={1.8} />
      {:else if item.toolKind === 'fetch'}<Globe2 size={14} strokeWidth={1.8} />
      {:else}<Wrench size={14} strokeWidth={1.8} />{/if}
    </span>
    <strong title={item.title}>{label}</strong>
    {#if preview}
      {#if targetUrl}
        <a class="preview target-link" href={targetUrl} onclick={(event) => { event.stopPropagation(); event.preventDefault(); void openUrlInBrowser({ url: targetUrl }); }}>{preview}</a>
      {:else if fileAction && filePath && onFileLink}
        <button class="preview target-link" type="button" title={filePath} onclick={(event) => { event.stopPropagation(); onFileLink?.(filePath); }}>{preview}</button>
      {:else}
        <span class="preview" title={fileAction ? filePath : preview}>{preview}</span>
      {/if}
    {/if}
    <!-- How the call ended, as one mark: done, failed, or still owed an answer. -->
    <span class="status-mark" title={statusLabel} aria-label={statusLabel} data-testid="timeline-tool-status">
      {#if item.state === 'completed'}<Check size={13} strokeWidth={2.2} />
      {:else if item.state === 'failed'}<X size={13} strokeWidth={2.2} />
      {:else if item.state === 'running'}<CircleDot size={13} strokeWidth={2} />
      {:else}<Minus size={13} strokeWidth={2} />{/if}
    </span>
  </summary>

  {#if detailsOpen && expandable}
    <div data-tool-scroll class="tool-body" data-testid="timeline-tool-body">
      {#if item.metadata?.name}<span>Tool: {item.metadata.name}</span>{/if}
      {#if input}
        <span>Input</span>
        <pre><code>{input}</code></pre>
      {/if}
      {#if item.diff}<FileChangeItem item={fileDisplayItem(item)} {onFileLink} />{/if}
      {#if output}
        <pre data-testid="timeline-tool-output"><code>{#each shownLines as line, index}{#if index > 0}{'\n'}{/if}{#each line as span}<span class={span.className}>{span.value}</span>{/each}{/each}</code></pre>
      {/if}
    </div>
  {/if}
</details>

<style>
  /* A collapsed row is just a row: no border, no fill, nothing that makes a
     run of tool calls look like a stack of cards. The box appears when the row
     is opened and its output needs a container. */
  .tool-item{border:1px solid transparent;border-radius:10px;box-sizing:border-box}
  .tool-item[open]{border-color:color-mix(in srgb,var(--color-border) 62%,transparent);background:color-mix(in srgb,var(--color-surface) 45%,var(--color-bg) 55%)}
  summary{display:flex;align-items:center;gap:9px;height:34px;box-sizing:border-box;padding:2px 8px;border-radius:10px;cursor:pointer;list-style:none}
  summary::-webkit-details-marker{display:none}
  summary:hover{background:color-mix(in srgb,var(--color-hover) 55%,transparent)}
  summary:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:-2px}
  .chevron,.tool-icon{display:grid;place-items:center;flex:none;color:var(--color-text-3)}
  .chevron.hidden{visibility:hidden}
  details[open] .chevron{transform:rotate(90deg)}
  strong{flex:none;max-width:50%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font:500 13px/22px var(--font-mono)}
  .preview{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--color-text-3);font-size:13px;line-height:22px}
  .target-link{padding:0;border:0;background:transparent;color:var(--color-accent);font:inherit;text-align:left;text-decoration:underline;text-underline-offset:2px;cursor:pointer}
  .status-mark{display:grid;place-items:center;flex:none;margin-left:auto;color:var(--color-text-3)}

  /* One accent family per state: a finished call is success, a broken one is
     danger, work still owed is the accent. */
  .completed .status-mark{color:var(--color-good)}
  .failed .status-mark{color:var(--color-bad)}
  .failed strong{color:var(--color-bad)}
  /* A call that failed says so in its body as well as its mark. Scrolling a run
     of tool calls, the mark is 13px at the far right of the row; what a reader
     actually lands on is the box under it, and it read the same as every
     successful one. */
  .failed[open]{border-color:color-mix(in srgb,var(--color-bad) 34%,transparent)}
  .failed pre{border-color:color-mix(in srgb,var(--color-bad) 38%,transparent);background:color-mix(in srgb,var(--color-bad) 7%,var(--color-bg))}
  .running .status-mark,.running .tool-icon{color:var(--color-accent)}

  .tool-body{display:grid;gap:7px;padding:0 10px 10px 34px;max-height:180px;box-sizing:border-box;overflow:auto;overscroll-behavior:contain;scrollbar-width:thin;scrollbar-color:var(--scrollbar-thumb) transparent}
  pre{margin:0;padding:8px 10px;border:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);border-radius:8px;background:color-mix(in srgb,var(--color-surface) 30%,var(--color-bg));white-space:pre;width:max-content;min-width:100%;box-sizing:border-box}
  code{font:13px/1.55 var(--font-mono)}
  .keyword{color:var(--color-accent)}
  .string{color:var(--color-good)}
  .comment{color:var(--color-text-3);font-style:italic}
  .number{color:var(--color-attention)}
  .type{color:var(--color-live)}
  .plain{color:inherit}
  @media (prefers-reduced-motion:no-preference){
    .chevron{transition:transform .14s ease}
    summary{transition:background .14s ease}
  }
</style>
