/**
 * markdownPreview.ts — which of the two Markdown views the editor shows first.
 *
 * The editor can show a Markdown file two ways: the raw source in the code
 * editor, or the rendered document. Opening is source-first; rendered preview
 * appears only after the reader explicitly picks Preview for the active file.
 */

export type MarkdownView = 'rendered' | 'raw';

/** Why the file was opened. */
export type MarkdownOpenOrigin = 'jump' | 'strip';

const MARKDOWN_EXTENSIONS = new Set(['md', 'mdx', 'markdown', 'mdown', 'mkd']);

export function isMarkdownFile(fileName: string | null | undefined): boolean {
  const name = (fileName ?? '').trim().toLocaleLowerCase();
  const dot = name.lastIndexOf('.');
  if (dot < 1) return false;
  return MARKDOWN_EXTENSIONS.has(name.slice(dot + 1));
}

export function markdownPreviewDefault(
  fileName: string | null | undefined,
  _origin: MarkdownOpenOrigin
): MarkdownView {
  if (!isMarkdownFile(fileName)) return 'raw';
  return 'raw';
}
