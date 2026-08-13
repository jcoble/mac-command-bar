<script lang="ts">
  import FileDiff from '@lucide/svelte/icons/file-diff';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  let { item, onFileLink }: {
    item: Extract<ConversationDisplayItem, { kind: 'file' }>;
    onFileLink?(path: string): void;
  } = $props();

  const path = $derived(typeof item.metadata?.path === 'string' ? item.metadata.path : 'File change');
  const diff = $derived(typeof item.metadata?.diff === 'string' ? item.metadata.diff : item.text);
  const lines = $derived(diff.split('\n'));
  const additions = $derived(lines.filter((line) => line.startsWith('+') && !line.startsWith('+++')).length);
  const deletions = $derived(lines.filter((line) => line.startsWith('-') && !line.startsWith('---')).length);

  function tone(line: string): 'add' | 'delete' | 'header' | 'context' {
    if (line.startsWith('+') && !line.startsWith('+++')) return 'add';
    if (line.startsWith('-') && !line.startsWith('---')) return 'delete';
    if (line.startsWith('@@') || line.startsWith('diff ') || line.startsWith('+++') || line.startsWith('---')) return 'header';
    return 'context';
  }
</script>

<aside class="file-change" data-testid="timeline-file-change-item">
  <div class="file-heading">
    <FileDiff size={14} strokeWidth={1.8} aria-hidden="true" />
    {#if onFileLink && path !== 'File change'}<button type="button" onclick={() => onFileLink?.(path)}>{path}</button>{:else}<strong>{path}</strong>{/if}
    <span class="counts"><em>+{additions}</em><del>-{deletions}</del></span>
  </div>
  <pre data-testid="timeline-file-diff"><code>{#each lines as line, index}<span class={tone(line)}>{line || ' '}{#if index < lines.length - 1}{'\n'}{/if}</span>{/each}</code></pre>
</aside>

<style>
  .file-change{min-width:0;border:1px solid color-mix(in srgb,var(--color-border) 72%,transparent);border-radius:8px;overflow:hidden;background:color-mix(in srgb,var(--color-bg) 78%,var(--color-surface))}
  .file-heading{display:flex;align-items:center;gap:8px;min-height:28px;padding:4px 10px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 64%,transparent);color:var(--color-text-3)}
  .file-heading strong,.file-heading button{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--color-text);font:500 12px ui-monospace,SFMono-Regular,Menlo,monospace;text-align:left}
  .file-heading button{border:0;background:transparent;padding:0;cursor:pointer}.file-heading button:hover{text-decoration:underline}
  .counts{display:flex;gap:7px;font:12px ui-monospace,SFMono-Regular,Menlo,monospace}.counts em{color:var(--color-good);font-style:normal}.counts del{color:var(--color-bad);text-decoration:none}
  pre{max-height:16rem;overflow:auto;margin:0;padding:6px 0;white-space:pre;tab-size:2}
  code{display:block;font:12px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}
  code span{display:block;min-height:1.55em;padding:0 10px;color:var(--color-text-2)}
  code .add{background:color-mix(in srgb,var(--color-good) 11%,transparent);color:color-mix(in srgb,var(--color-good) 76%,var(--color-text))}
  code .delete{background:color-mix(in srgb,var(--color-bad) 11%,transparent);color:color-mix(in srgb,var(--color-bad) 74%,var(--color-text))}
  code .header{color:var(--color-accent)}
</style>
