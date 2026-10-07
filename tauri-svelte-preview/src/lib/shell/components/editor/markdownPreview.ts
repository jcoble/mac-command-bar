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
 *  after it, and the rest. Preview renders only the rest, because Markdown
 *  reads the block as a rule and a heading. */
export function splitFrontmatter(markdown: string): { frontmatter: string; body: string } {
  const frontmatter = FRONTMATTER.exec(markdown)?.[0] ?? '';
  return { frontmatter, body: markdown.slice(frontmatter.length) };
}

/** The top-level keys of a front matter block, each with its value as written.
 *  Indented lines under a key (a list or a map) join that key's value. */
export function frontmatterRows(frontmatter: string): [string, string][] {
  const rows: [string, string][] = [];
  for (const line of frontmatter.split(/\r?\n/).slice(1)) {
    if (/^(?:---|\.\.\.)[ \t]*$/.test(line)) break;
    if (!line.trim() || line.startsWith('#')) continue;
    const key = /^([^\s:#][^:]*):(?:[ \t]+(.*)|[ \t]*)$/.exec(line);
    if (key) rows.push([key[1].trim(), (key[2] ?? '').trim()]);
    else if (rows.length > 0) {
      const row = rows[rows.length - 1];
      row[1] = row[1] ? `${row[1]}\n${line.trim()}` : line.trim();
    }
  }
  return rows;
}

const QUOTES = '(?:[ \\t]*>[ \\t]?)*[ \\t]*';
const FENCE_OPEN = new RegExp(`^${QUOTES}(?:(?:[-*+]|\\d{1,9}[.)])[ \\t]+)?(\`{3,}|~{3,})`);
const FENCE_CLOSE = new RegExp(`^${QUOTES}(\`{3,}|~{3,})[ \\t]*$`);
const TASK_ITEM = new RegExp(`^(${QUOTES}(?:[-*+]|\\d{1,9}[.)])[ \\t]+)\\[[ xX]\\] +\\S`);

/** Where each task-list checkbox is in the file, in document order: the offset
 *  of the character between "[" and "]". Front matter and fenced code are
 *  skipped. Preview refuses a click when this list and the rendered boxes
 *  disagree, so an odd file can never have the wrong box changed. */
export function taskCheckboxOffsets(markdown: string): number[] {
  const offsets: number[] = [];
  let fence: string | null = null;
  let start = splitFrontmatter(markdown).frontmatter.length;
  while (start < markdown.length) {
    const newline = markdown.indexOf('\n', start);
    const end = newline === -1 ? markdown.length : newline;
    const line = markdown.slice(start, end).replace(/\r$/, '');
    if (fence) {
      const close = FENCE_CLOSE.exec(line);
      if (close && close[1][0] === fence[0] && close[1].length >= fence.length) fence = null;
    } else {
      const open = FENCE_OPEN.exec(line);
      const task = open ? null : TASK_ITEM.exec(line);
      if (open) fence = open[1];
      else if (task) offsets.push(start + task[1].length + 1);
    }
    start = end + 1;
  }
  return offsets;
}

/** The file with one task checkbox flipped: " " becomes "x", "x" or "X"
 *  becomes " ". Nothing else in the file changes. */
export function toggleTaskCheckbox(markdown: string, offset: number): string {
  const mark = markdown[offset];
  if (markdown[offset - 1] !== '[' || markdown[offset + 1] !== ']' || !/^[ xX]$/.test(mark ?? '')) {
    throw new Error(`No task checkbox at offset ${offset}`);
  }
  return markdown.slice(0, offset) + (mark === ' ' ? 'x' : ' ') + markdown.slice(offset + 1);
}

/** The file after a click on the index-th task box on screen, which now shows
 *  `nowChecked`; null when the boxes on screen and the file disagree, so the
 *  click is refused rather than changing the wrong box. */
export function toggleTaskAt(markdown: string, index: number, boxesOnScreen: number, nowChecked: boolean): string | null {
  const offsets = taskCheckboxOffsets(markdown);
  const offset = offsets[index];
  if (offsets.length !== boxesOnScreen || offset === undefined || (markdown[offset] !== ' ') === nowChecked) return null;
  return toggleTaskCheckbox(markdown, offset);
}

export type MarkdownImageTarget =
  | { kind: 'file'; path: string }
  | { kind: 'https' | 'http'; url: string };

/** Where an image in a Markdown file comes from: a file on disk (a relative
 *  path is resolved from the Markdown file's folder) or a web address. */
export function markdownImageTarget(markdownPath: string, href: string | undefined): MarkdownImageTarget | null {
  const value = href?.trim() ?? '';
  if (/^https:/i.test(value)) return { kind: 'https', url: value };
  if (/^http:/i.test(value)) return { kind: 'http', url: value };
  if (!value || value.startsWith('#') || /^[a-z][a-z0-9+.-]*:/i.test(value)) return null;
  let path = value.replace(/[?#].*$/, '');
  try {
    path = decodeURI(path);
  } catch {
    // A malformed escape stays as written.
  }
  const parts = path.startsWith('/') ? [] : markdownPath.split('/').slice(0, -1);
  for (const part of path.split('/')) {
    if (part === '..') parts.pop();
    else if (part && part !== '.') parts.push(part);
  }
  return { kind: 'file', path: `/${parts.filter(Boolean).join('/')}` };
}

/** The image type of a file the app shows as a picture, from its name. */
export function rasterImageMimeType(fileName: string | null | undefined): string | null {
  const extension = fileName?.split('.').at(-1)?.toLowerCase();
  if (extension === 'png') return 'image/png';
  if (extension === 'jpg' || extension === 'jpeg') return 'image/jpeg';
  if (extension === 'gif') return 'image/gif';
  if (extension === 'webp') return 'image/webp';
  if (extension === 'bmp') return 'image/bmp';
  if (extension === 'ico') return 'image/x-icon';
  if (extension === 'avif') return 'image/avif';
  return null;
}
