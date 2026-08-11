/**
 * Syntax coloring for code blocks in the transcript.
 *
 * The app already ships the Monaco editor, so the transcript borrows its
 * tokenizer rather than adding a second highlighting library. Two deliberate
 * choices:
 *
 * 1. `editor.tokenize` is used, not `editor.colorize`. Colorize returns a
 *    string of HTML, which would have to be injected with `{@html}` — the one
 *    thing the conversation renderer avoids, because a partial fence arriving
 *    mid-stream would then be markup. Tokenize returns positions and token
 *    names, which Svelte renders as text.
 * 2. The editor is imported only when a code block actually needs coloring, so
 *    a transcript of plain prose never pays Monaco's start-up cost.
 *
 * Colors come from the shell's own tokens (see CodeBlock.svelte), so both
 * light and dark themes stay readable without a second palette.
 */

/** One run of characters that share a color. */
export interface HighlightedSpan {
  value: string;
  className: CodeTokenClass;
}

export type HighlightedLine = HighlightedSpan[];

/** The small set of colors the transcript paints code with. */
export type CodeTokenClass = 'keyword' | 'string' | 'comment' | 'number' | 'type' | 'plain';

/** Anything longer than this stays plain: coloring it costs more than it helps. */
const MAX_HIGHLIGHT_CHARS = 40_000;

/**
 * Fence spellings people actually type, mapped to the editor's language ids.
 * A language the editor does not know falls back to plain text rather than
 * being guessed at.
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

/** Turn a fence's info string ("ts title=example.ts") into a language id. */
export function monacoLanguageForFence(info: string): string {
  const first = info.trim().split(/\s+/)[0]?.toLowerCase() ?? '';
  if (!first) return 'plaintext';
  return FENCE_LANGUAGES[first] ?? 'plaintext';
}

/**
 * Reduce a tokenizer's name ("string.quoted.double.js") to one of the colors
 * the transcript paints. Unrecognised names read as body text.
 */
export function codeTokenClass(type: string): CodeTokenClass {
  const name = type.toLowerCase();
  if (name.startsWith('comment')) return 'comment';
  if (name.startsWith('string') || name.startsWith('regexp')) return 'string';
  if (name.startsWith('number') || name.startsWith('constant.numeric')) return 'number';
  if (name.startsWith('keyword') || name.startsWith('storage') || name.startsWith('operator.sql')) return 'keyword';
  if (name.startsWith('type') || name.startsWith('entity.name.type') || name.startsWith('tag')) return 'type';
  if (name.startsWith('attribute') || name.startsWith('metatag') || name.startsWith('annotation')) return 'type';
  return 'plain';
}

/** The uncolored rendering: every line as a single plain run. */
export function plainHighlightedLines(value: string): HighlightedLine[] {
  return value
    .replaceAll('\r\n', '\n')
    .replaceAll('\r', '\n')
    .split('\n')
    .map((line) => [{ value: line, className: 'plain' as const }]);
}

type MonacoModule = typeof import('monaco-editor/esm/vs/editor/editor.api');

let monacoLoad: Promise<MonacoModule | null> | null = null;
const warmedLanguages = new Set<string>();

/** Load the editor once, on the first code block that needs it. */
async function loadMonaco(): Promise<MonacoModule | null> {
  if (typeof window === 'undefined') return null;
  monacoLoad ??= (async () => {
    try {
      await import('@codingame/monaco-vscode-standalone-languages');
      return await import('monaco-editor/esm/vs/editor/editor.api');
    } catch (error) {
      console.warn('Code blocks are shown without coloring: the editor did not load.', error);
      return null;
    }
  })();
  return monacoLoad;
}

/**
 * A language's tokenizer is fetched lazily, and `tokenize` is synchronous, so
 * the first call for a language would come back uncolored. `colorize` on an
 * empty string waits for that tokenizer without rendering anything.
 */
async function warmLanguage(monaco: MonacoModule, language: string): Promise<void> {
  if (warmedLanguages.has(language)) return;
  try {
    await monaco.editor.colorize('', language, {});
  } catch {
    // A language with no tokenizer simply stays plain.
  }
  warmedLanguages.add(language);
}

function spansForLine(
  line: string,
  tokens: readonly { offset: number; type: string }[]
): HighlightedLine {
  if (!tokens.length) return [{ value: line, className: 'plain' }];
  const spans: HighlightedLine = [];
  for (let index = 0; index < tokens.length; index += 1) {
    const start = tokens[index].offset;
    const end = index + 1 < tokens.length ? tokens[index + 1].offset : line.length;
    if (end <= start) continue;
    spans.push({ value: line.slice(start, end), className: codeTokenClass(tokens[index].type) });
  }
  return spans.length ? spans : [{ value: line, className: 'plain' }];
}

/**
 * Color one code block. Always resolves: anything that goes wrong comes back
 * as the plain rendering, so the code is never lost.
 */
export async function highlightCode(value: string, language: string): Promise<HighlightedLine[]> {
  if (language === 'plaintext' || value.length > MAX_HIGHLIGHT_CHARS) return plainHighlightedLines(value);
  const monaco = await loadMonaco();
  if (!monaco) return plainHighlightedLines(value);
  try {
    await warmLanguage(monaco, language);
    const lines = value.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n');
    const tokenized = monaco.editor.tokenize(value, language);
    return lines.map((line, index) => spansForLine(line, tokenized[index] ?? []));
  } catch (error) {
    console.warn('Code block shown without coloring.', error);
    return plainHighlightedLines(value);
  }
}
