/**
 * markdownPreview.ts — which of the two Markdown views the editor shows first.
 *
 * The editor can show a Markdown file two ways: the raw source in the code
 * editor, or the rendered document. A file reached from a diff or a jump opens
 * rendered, because the reader asked to read it. Picking the file in the
 * open-files strip keeps the editor on source, because the reader is working in
 * it. Either way the toggle switches the active file at any time.
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
  origin: MarkdownOpenOrigin
): MarkdownView {
  if (!isMarkdownFile(fileName)) return 'raw';
  return origin === 'strip' ? 'raw' : 'rendered';
}
