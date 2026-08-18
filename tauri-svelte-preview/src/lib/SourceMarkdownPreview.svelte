<script lang="ts">
  import {
    renderSourceMarkdownPreview,
    sourceMarkdownPreviewTextSummary
  } from './sourceMarkdownPreview';

  type Props = {
    content: string;
    fileName: string;
    relativePath: string;
    dirty?: boolean;
  };

  let { content, fileName, relativePath, dirty = false }: Props = $props();
  let renderedContent = $derived(renderSourceMarkdownPreview(content));
  let summary = $derived(sourceMarkdownPreviewTextSummary(content) || 'Empty Markdown file');
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
    {@html renderedContent}
  </article>
</section>

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

  .source-markdown-preview-document :global(blockquote) {
    padding: 8px 12px;
    border-left: 3px solid rgba(92, 226, 207, 0.42);
    color: #b7c3bf;
    background: rgba(92, 226, 207, 0.06);
  }
</style>
