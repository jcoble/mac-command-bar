/**
 * Small, dependency-free Markdown adapter for conversation output.
 *
 * The renderer deliberately returns data rather than HTML. Svelte renders every
 * text value as text, so a partial fence or a provider supplied `<script>` can
 * never become executable markup while a turn is streaming.
 */

export type SafeInlinePart =
  | { kind: 'text'; value: string }
  | { kind: 'strong' | 'emphasis' | 'code'; value: string }
  | { kind: 'link'; value: string; href: string }
  | { kind: 'file-link'; value: string; path: string };

export type SafeMarkdownBlock =
  | { kind: 'paragraph'; parts: SafeInlinePart[] }
  | { kind: 'heading'; level: number; parts: SafeInlinePart[] }
  | { kind: 'code'; language: string; value: string; complete: boolean }
  | { kind: 'quote'; parts: SafeInlinePart[] }
  | { kind: 'list'; ordered: boolean; items: { task: boolean; checked: boolean; parts: SafeInlinePart[] }[] }
  | { kind: 'table'; headers: SafeInlinePart[][]; rows: SafeInlinePart[][][] };

const SAFE_PROTOCOL = /^(?:https?:|mailto:|#)/i;

/** Return a URL only when it is safe to put in an href attribute. */
export function sanitizeConversationHref(value: string): string | null {
  const href = value.trim();
  if (!href || /[\u0000-\u001f\u007f]/.test(href)) return null;
  // Relative links are intentionally not treated as external URLs. File/path
  // links are routed through the editor bus by a later controller receipt.
  return SAFE_PROTOCOL.test(href) ? href : null;
}

/** Escape text for callers that need a safe plain-text representation. */
export function sanitizeConversationMarkdown(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function inlineParts(value: string): SafeInlinePart[] {
  const parts: SafeInlinePart[] = [];
  const pattern = /(`[^`\n]+`|\*\*[^*\n]+\*\*|__[^_\n]+__|\*[^*\n]+\*|_[^_\n]+_|\[[^\]\n]+\]\([^\)\n]+\))/g;
  let cursor = 0;
  for (const match of value.matchAll(pattern)) {
    const start = match.index ?? 0;
    if (start > cursor) parts.push({ kind: 'text', value: value.slice(cursor, start) });
    const token = match[0];
    if (token.startsWith('`')) {
      parts.push({ kind: 'code', value: token.slice(1, -1) });
    } else if (token.startsWith('**') || token.startsWith('__')) {
      parts.push({ kind: 'strong', value: token.slice(2, -2) });
    } else if (token.startsWith('*') || token.startsWith('_')) {
      parts.push({ kind: 'emphasis', value: token.slice(1, -1) });
    } else {
      const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(token);
      const href = link ? sanitizeConversationHref(link[2]) : null;
      if (link && href) parts.push({ kind: 'link', value: link[1], href });
      else if (link && !link[2].includes(':') && !/[\u0000-\u001f\u007f]/.test(link[2])) {
        parts.push({ kind: 'file-link', value: link[1], path: link[2].trim() });
      } else parts.push({ kind: 'text', value: token });
    }
    cursor = start + token.length;
  }
  if (cursor < value.length) parts.push({ kind: 'text', value: value.slice(cursor) });
  return parts.length ? parts : [{ kind: 'text', value: '' }];
}

function isTableSeparator(line: string): boolean {
  const cells = line.trim().replace(/^\|/, '').replace(/\|$/, '').split('|');
  return cells.length > 0 && cells.every((cell) => /^\s*:?-{3,}:?\s*$/.test(cell));
}

function tableCells(line: string): string[] {
  return line.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map((cell) => cell.trim());
}

/** Parse GFM-shaped Markdown into safe, streaming-friendly blocks. */
export function parseSafeMarkdown(source: string): SafeMarkdownBlock[] {
  const lines = source.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n');
  const blocks: SafeMarkdownBlock[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }

    const fence = /^\s*(```+|~~~+)\s*([^\s]*)\s*$/.exec(line);
    if (fence) {
      const marker = fence[1];
      const language = fence[2] ?? '';
      const code: string[] = [];
      index += 1;
      let complete = false;
      while (index < lines.length) {
        if (new RegExp(`^\\s*${marker[0]}{${marker.length},}\\s*$`).test(lines[index])) {
          complete = true;
          index += 1;
          break;
        }
        code.push(lines[index]);
        index += 1;
      }
      blocks.push({ kind: 'code', language, value: code.join('\n'), complete });
      continue;
    }

    const heading = /^(#{1,6})\s+(.+?)\s*$/.exec(line);
    if (heading) {
      blocks.push({ kind: 'heading', level: heading[1].length, parts: inlineParts(heading[2]) });
      index += 1;
      continue;
    }

    if (index + 1 < lines.length && line.includes('|') && isTableSeparator(lines[index + 1])) {
      const headers = tableCells(line).map(inlineParts);
      const rows: SafeInlinePart[][][] = [];
      index += 2;
      while (index < lines.length && lines[index].trim() && lines[index].includes('|')) {
        rows.push(tableCells(lines[index]).map(inlineParts));
        index += 1;
      }
      blocks.push({ kind: 'table', headers, rows });
      continue;
    }

    const quote = /^\s*>\s?(.*)$/.exec(line);
    if (quote) {
      blocks.push({ kind: 'quote', parts: inlineParts(quote[1]) });
      index += 1;
      continue;
    }

    const list = /^\s*(?:[-+*]|\d+[.)])\s+(.*)$/.exec(line);
    if (list) {
      const ordered = /^\s*\d/.test(line);
      const items: { task: boolean; checked: boolean; parts: SafeInlinePart[] }[] = [];
      while (index < lines.length) {
        const next = /^\s*(?:[-+*]|\d+[.)])\s+(.*)$/.exec(lines[index]);
        if (!next || /^\s*\d/.test(lines[index]) !== ordered) break;
        const task = /^\[([ xX])\]\s+(.+)$/.exec(next[1]);
        items.push({
          task: !!task,
          checked: task?.[1].toLowerCase() === 'x',
          parts: inlineParts(task?.[2] ?? next[1])
        });
        index += 1;
      }
      blocks.push({ kind: 'list', ordered, items });
      continue;
    }

    const paragraph: string[] = [line.trim()];
    index += 1;
    while (index < lines.length && lines[index].trim()) {
      if (/^\s*(?:```|~~~|#{1,6}\s|>|[-+*]\s|\d+[.)]\s)/.test(lines[index])) break;
      paragraph.push(lines[index].trim());
      index += 1;
    }
    blocks.push({ kind: 'paragraph', parts: inlineParts(paragraph.join('\n')) });
  }
  return blocks;
}

/** Compatibility name used by safety-focused tests and adapters. */
export const parseConversationMarkdown = parseSafeMarkdown;
