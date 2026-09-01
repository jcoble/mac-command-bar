/**
 * Which language a code block is written in, and how it gets coloured.
 *
 * The colouring itself lives in `syntaxTokens.ts`, which shares nothing with
 * the code editor. This file is only the mapping from what people type after a
 * fence, or from a file's name, to a language that scanner knows.
 */

export {
  hasGrammar,
  highlightSource,
  plainHighlightedLines,
  type CodeTokenClass,
  type HighlightedLine,
  type HighlightedSpan
} from './syntaxTokens.ts';

import { hasGrammar, highlightSource, plainHighlightedLines } from './syntaxTokens.ts';
import type { HighlightedLine } from './syntaxTokens.ts';

/** Anything longer than this stays plain: colouring it costs more than it helps. */
const MAX_HIGHLIGHT_CHARS = 40_000;

/**
 * Fence spellings people actually type, mapped to the language ids the scanner
 * knows. A spelling with no grammar behind it falls back to plain text rather
 * than being guessed at.
 */
const FENCE_LANGUAGES: Record<string, string> = {
  bash: 'shell',
  c: 'cpp',
  'c++': 'cpp',
  cpp: 'cpp',
  cs: 'csharp',
  csharp: 'csharp',
  css: 'css',
  diff: 'plaintext',
  dockerfile: 'dockerfile',
  go: 'go',
  golang: 'go',
  html: 'html',
  ini: 'ini',
  java: 'java',
  javascript: 'javascript',
  js: 'javascript',
  json: 'json',
  jsonc: 'json',
  jsx: 'javascript',
  kotlin: 'kotlin',
  less: 'less',
  markdown: 'markdown',
  md: 'markdown',
  mjs: 'javascript',
  php: 'php',
  powershell: 'powershell',
  ps1: 'powershell',
  py: 'python',
  python: 'python',
  rb: 'ruby',
  ruby: 'ruby',
  rs: 'rust',
  rust: 'rust',
  scss: 'scss',
  sh: 'shell',
  shell: 'shell',
  sql: 'sql',
  swift: 'swift',
  toml: 'ini',
  ts: 'typescript',
  tsx: 'typescript',
  typescript: 'typescript',
  xml: 'xml',
  yaml: 'yaml',
  yml: 'yaml',
  zsh: 'shell'
};

/** The language a file is written in, from its name, or plain text. */
export function languageForPath(path: string | null | undefined): string {
  const name = (path ?? '').split('/').pop() ?? '';
  const dot = name.lastIndexOf('.');
  if (dot <= 0) return name.toLowerCase() === 'dockerfile' ? 'dockerfile' : 'plaintext';
  return FENCE_LANGUAGES[name.slice(dot + 1).toLowerCase()] ?? 'plaintext';
}

/** Turn a fence's info string ("ts title=example.ts") into a language id. */
export function fenceLanguage(info: string): string {
  const first = info.trim().split(/\s+/)[0]?.toLowerCase() ?? '';
  if (!first) return 'plaintext';
  return FENCE_LANGUAGES[first] ?? 'plaintext';
}

/**
 * Colour one code block. Never throws and never loses the code: a language
 * with no grammar, or a block too long to be worth scanning, comes back as the
 * plain rendering.
 */
export function highlightCode(value: string, language: string): HighlightedLine[] {
  if (!hasGrammar(language) || value.length > MAX_HIGHLIGHT_CHARS) {
    return plainHighlightedLines(value);
  }
  return highlightSource(value, language);
}
