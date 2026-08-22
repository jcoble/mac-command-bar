<script lang="ts">
  import { parseSafeMarkdown, type SafeInlinePart, type SafeListItem, type SafeMarkdownBlock } from './shell/conversation/conversationMessageSafety';
  import { fenceLanguage, highlightCode, plainHighlightedLines } from './shell/components/conversation/codeHighlight';
  import { sourceMarkdownPreviewTextSummary } from './sourceMarkdownPreview';

  type Props = {
    content: string;
    fileName: string;
    relativePath: string;
    dirty?: boolean;
  };

  let { content, fileName, relativePath, dirty = false }: Props = $props();
  let blocks = $derived(parseSafeMarkdown(content));
  let summary = $derived(sourceMarkdownPreviewTextSummary(content) || 'Empty Markdown file');
  let highlightReady = $state(false);

  // Keep the first paint cheap, then colour the same token stream the
  // conversation uses. The preview is mounted only while visible, so the
  // frame is also the complete release boundary for this work.
  $effect(() => {
    const source = content;
    highlightReady = false;
    if (typeof window === 'undefined') return;
    const frame = window.requestAnimationFrame(() => {
      if (source === content) highlightReady = true;
    });
    return () => window.cancelAnimationFrame(frame);
  });
</script>

<section
  class="source-markdown-preview"
  aria-label={`Preview ${fileName}`}
  title={`${relativePath}${dirty ? ' (modified)' : ''}`}
>
  <div class="source-markdown-preview-header">
    <strong>{fileName}</strong>
    <span>{summary}</span>
  </div>
  <article class="source-markdown-preview-document">
    {#each blocks as block}{@render node(block)}{/each}
  </article>
</section>

{#snippet node(block: SafeMarkdownBlock)}
  {#if block.kind === 'code'}
    {@render codeBlock(block)}
  {:else if block.kind === 'heading'}
    <svelte:element this={`h${Math.min(block.level, 6)}`} class={`heading level-${Math.min(block.level, 4)}`}>{#each block.parts as part}{@render inline(part)}{/each}</svelte:element>
  {:else if block.kind === 'quote'}
    <blockquote>{#each block.blocks as inner}{@render node(inner)}{/each}</blockquote>
  {:else if block.kind === 'rule'}
    <hr />
  {:else if block.kind === 'list'}
    {#if block.ordered}<ol>{#each block.items as entry}{@render row(entry)}{/each}</ol>
    {:else}<ul>{#each block.items as entry}{@render row(entry)}{/each}</ul>{/if}
  {:else if block.kind === 'table'}
    <div class="table-scroll"><table><thead><tr>{#each block.headers as cell}<th>{#each cell as part}{@render inline(part)}{/each}</th>{/each}</tr></thead><tbody>{#each block.rows as tableRow}<tr>{#each tableRow as cell}<td>{#each cell as part}{@render inline(part)}{/each}</td>{/each}</tr>{/each}</tbody></table></div>
  {:else}
    <p>{#each block.parts as part}{@render inline(part)}{/each}</p>
  {/if}
{/snippet}

{#snippet codeBlock(block: Extract<SafeMarkdownBlock, { kind: 'code' }>)}
  {@const lines = highlightReady ? highlightCode(block.value, fenceLanguage(block.language)) : plainHighlightedLines(block.value)}
  <pre data-testid="source-markdown-code-block"><code>{#each lines as line, lineIndex}{#if lineIndex > 0}{'\n'}{/if}{#each line as span}<span class={span.className}>{span.value}</span>{/each}{/each}</code></pre>
{/snippet}

{#snippet row(entry: SafeListItem)}
  <li class:task-row={entry.task}>{#if entry.task}<input type="checkbox" checked={entry.checked} disabled />{/if}{#each entry.parts as part}{@render inline(part)}{/each}{#each entry.blocks as inner}{@render node(inner)}{/each}</li>
{/snippet}

{#snippet inline(part: SafeInlinePart)}
  {#if part.kind === 'strong'}<strong>{#each part.parts as inner}{@render inline(inner)}{/each}</strong>
  {:else if part.kind === 'emphasis'}<em>{#each part.parts as inner}{@render inline(inner)}{/each}</em>
  {:else if part.kind === 'strike'}<del>{#each part.parts as inner}{@render inline(inner)}{/each}</del>
  {:else if part.kind === 'code'}<code>{part.value}</code>
  {:else if part.kind === 'link'}<a href={part.href} target="_blank" rel="noreferrer">{#each part.parts as inner}{@render inline(inner)}{/each}</a>
  {:else if part.kind === 'file-link'}<span class="file-link" title={part.path}>{#each part.parts as inner}{@render inline(inner)}{/each}</span>
  {:else}{part.value}{/if}
{/snippet}

<style>
  .source-markdown-preview {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    color: #d7dfdd;
    background: #131617;
  }

  .source-markdown-preview-header {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 28px;
    padding: 0 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(255, 255, 255, 0.025);
  }

  .source-markdown-preview-header strong,
  .source-markdown-preview-header span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .source-markdown-preview-header strong {
    color: #edf5f3;
    font-size: 12px;
    font-weight: 820;
  }

  .source-markdown-preview-header span {
    color: #8f9996;
    font-size: 10px;
    font-weight: 720;
  }

  .source-markdown-preview-document {
    min-width: 0;
    min-height: 0;
    overflow: auto;
    padding: 20px 28px 36px;
    line-height: 1.58;
    scrollbar-color: rgba(174, 184, 181, 0.54) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  .source-markdown-preview-document :global(h1),
  .source-markdown-preview-document :global(h2),
  .source-markdown-preview-document :global(h3),
  .source-markdown-preview-document :global(h4),
  .source-markdown-preview-document :global(h5),
  .source-markdown-preview-document :global(h6) {
    margin: 0 0 12px;
    color: #eff8f6;
    font-weight: 820;
    letter-spacing: 0;
  }

  .source-markdown-preview-document :global(h1) {
    padding-bottom: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.09);
    font-size: 24px;
  }

  .source-markdown-preview-document :global(h2) {
    margin-top: 26px;
    font-size: 19px;
  }

  .source-markdown-preview-document :global(h3) {
    margin-top: 22px;
    font-size: 16px;
  }

  .source-markdown-preview-document :global(p),
  .source-markdown-preview-document :global(ul),
  .source-markdown-preview-document :global(blockquote),
  .source-markdown-preview-document :global(pre) {
    margin: 0 0 14px;
  }

  .source-markdown-preview-document :global(ul) {
    padding-left: 22px;
  }

  .source-markdown-preview-document :global(li + li) {
    margin-top: 4px;
  }

  .source-markdown-preview-document :global(a) {
    color: #82e6d5;
    text-decoration: none;
  }

  .source-markdown-preview-document :global(a:hover),
  .source-markdown-preview-document :global(a:focus-visible) {
    color: #bff7ef;
    text-decoration: underline;
  }

  .source-markdown-preview-document :global(code) {
    padding: 1px 4px;
    border-radius: 4px;
    color: #aeeede;
    background: rgba(92, 226, 207, 0.09);
    font-family: var(--font-mono);
    font-size: 0.92em;
  }

  .source-markdown-preview-document :global(pre) {
    overflow: auto;
    padding: 12px;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 6px;
    background: rgba(4, 7, 8, 0.62);
  }

  .source-markdown-preview-document :global(pre code) {
    display: block;
    padding: 0;
    color: #dce8e5;
    background: transparent;
    white-space: pre;
  }

  .source-markdown-preview-document :global(.keyword) { color: #82e6d5; }
  .source-markdown-preview-document :global(.string) { color: #a8e6a2; }
  .source-markdown-preview-document :global(.comment) { color: #8f9996; font-style: italic; }
  .source-markdown-preview-document :global(.number) { color: #f4c77a; }
  .source-markdown-preview-document :global(.type) { color: #9fd4ff; }

  .source-markdown-preview-document :global(blockquote) {
    padding: 8px 12px;
    border-left: 3px solid rgba(92, 226, 207, 0.42);
    color: #b7c3bf;
    background: rgba(92, 226, 207, 0.06);
  }
</style>
