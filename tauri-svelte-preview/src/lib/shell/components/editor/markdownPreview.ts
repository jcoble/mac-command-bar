/**
 * markdownPreview.ts — which of the two Markdown views the editor shows first.
 *
 * The editor can show a Markdown file two ways: the raw source in the code
 * editor, or the rendered document. Which one comes up first depends on why
 * the file was opened, and that decision is here so it can be tested without
 * a browser and stays the same wherever a file is opened from.
 *
 * A file reached from a diff or from a jump elsewhere in the shell is being
 * read, so it opens rendered. A file picked in the strip of open files is
 * already in front of the person editing it, so it stays on source.
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
  return origin === 'jump' ? 'rendered' : 'raw';
}
