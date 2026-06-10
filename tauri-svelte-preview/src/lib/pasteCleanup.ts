export type PasteCleanupMode = 'plain' | 'compact' | 'prompt';

export const pasteCleanupModes: PasteCleanupMode[] = ['plain', 'compact', 'prompt'];

export function cleanupPasteText(input: string, mode: PasteCleanupMode): string {
  const normalized = normalizePastedText(input);

  switch (mode) {
    case 'compact':
      return normalized.replace(/\s+/g, ' ').trim();
    case 'prompt':
      return stripWrappingCodeFence(normalized).trim();
    case 'plain':
    default:
      return normalized.trim();
  }
}

export function formatPasteCleanupStats(input: string, output: string): string {
  const charCount = output.length;
  const trimmedCount = Math.max(0, input.length - output.length);
  if (trimmedCount === 0) return `${charCount.toLocaleString()} chars`;
  return `${charCount.toLocaleString()} chars - trimmed ${trimmedCount.toLocaleString()}`;
}

function normalizePastedText(input: string): string {
  return input
    .replace(/^\uFEFF/, '')
    .replace(/\u00a0/g, ' ')
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map((line) => line.replace(/[ \t]+$/g, '').replace(/^\t+/g, ''))
    .join('\n')
    .replace(/\n{3,}/g, '\n\n');
}

function stripWrappingCodeFence(input: string): string {
  const trimmed = input.trim();
  const match = /^```[^\n]*\n(?<body>[\s\S]*?)\n```$/.exec(trimmed);
  return match?.groups?.body ?? trimmed;
}
