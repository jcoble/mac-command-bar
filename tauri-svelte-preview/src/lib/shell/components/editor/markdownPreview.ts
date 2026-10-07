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

/** An HTML page reads the same two ways: its source, or the page rendered in a
 *  sandboxed frame. It always opens on source. */
export function isHtmlFile(fileName: string | null | undefined): boolean {
  const name = (fileName ?? '').trim().toLocaleLowerCase();
  return name.endsWith('.html') || name.endsWith('.htm');
}

export function markdownPreviewDefault(
  fileName: string | null | undefined,
  origin: MarkdownOpenOrigin
): MarkdownView {
  if (!isMarkdownFile(fileName)) return 'raw';
  return origin === 'strip' ? 'raw' : 'rendered';
}

const FRONTMATTER = /^---\r?\n(?:.*\r?\n)*?(?:---|\.\.\.)[ \t]*(?:\r?\n|$)(?:[ \t]*\r?\n)*/;

/** A YAML front matter block at the very start of a file, with the blank lines
 *  after it, and the rest. The rich view edits only the rest: Markdown reads
 *  the block as a rule and a heading and would rewrite it on save. */
export function splitFrontmatter(markdown: string): { frontmatter: string; body: string } {
  const frontmatter = FRONTMATTER.exec(markdown)?.[0] ?? '';
  return { frontmatter, body: markdown.slice(frontmatter.length) };
}
