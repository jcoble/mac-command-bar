/**
 * themeRegistry.ts — the app's themes, each one complete.
 *
 * A theme here is all three of the things that have to change together for the
 * app to actually look different:
 *
 *   tokens    the `--color-…` custom properties every panel, dialog, button and
 *             dock divider is painted from (the same names `nextTokens.css`
 *             sets, plus a few extra the dock needs and the shared token file
 *             has no word for).
 *   monaco    the code editor's own theme object. Monaco does not read CSS
 *             custom properties; it wants a theme registered by name.
 *   terminal  xterm's theme object. Same story — its own palette, set on the
 *             terminal instance.
 *
 * Two themes ship. `houston` is what the app looks like TODAY, copied value for
 * value out of the three files that hold those values now, so picking it can
 * never change a pixel. `dracula` is the proof that switching works: it repaints
 * the chrome, the editor and the dock.
 *
 * The copying is the risk. `scripts/themeRegistry.test.mjs` reads the original
 * files and fails if Houston stops agreeing with them, so the two cannot drift
 * apart quietly.
 *
 * This module is deliberately plain data with no imports: no DOM, no store, no
 * Svelte runes. Applying a theme is `themeService.ts`'s job.
 */

// ── Shapes ──────────────────────────────────────────────────────────────────

/** One syntax-colouring rule in a Monaco theme. Colors are bare hex, no `#`. */
export type MonacoTokenRule = {
  token: string;
  foreground?: string;
  background?: string;
  fontStyle?: string;
};

/**
 * A Monaco theme, shaped exactly like the object
 * `src/lib/sourcePreviewAppearance.ts` already hands to `defineTheme`.
 */
export type MonacoThemeDefinition = {
  id: string;
  base: 'vs' | 'vs-dark' | 'hc-black' | 'hc-light';
  inherit: boolean;
  rules: MonacoTokenRule[];
  encodedTokensColors?: string[];
  colors: Record<string, string>;
};

/**
 * xterm's theme, written out by hand rather than imported from `@xterm/xterm`
 * so this file stays free of dependencies and can be read by a plain Node test.
 * The slots are the ones the terminal factory already sets.
 */
export type TerminalTheme = {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
};

/** The custom property names a theme sets, mapped to their colors. */
export type ThemeTokens = Record<string, string>;

/** One complete theme. */
export type ShellTheme = {
  id: string;
  /** What the chooser calls it. */
  label: string;
  /** One line under the name, in plain words. */
  description: string;
  tokens: ThemeTokens;
  monaco: MonacoThemeDefinition;
  terminal: TerminalTheme;
};

// ── The names every theme must set ──────────────────────────────────────────

/**
 * The palette `nextTokens.css` defines. Every one of these names is read by the
 * shared components, so a theme that leaves one out would keep whatever colour
 * the previous theme painted there.
 */
export const PALETTE_TOKEN_NAMES = [
  '--color-bg',
  '--color-surface',
  '--color-elevated',
  '--color-text',
  '--color-text-2',
  '--color-text-3',
  '--color-accent',
  '--color-border',
  '--color-focus',
  '--color-live',
  '--color-good',
  '--color-bad',
  '--color-attention',
  '--color-idle',
  '--color-on-accent',
  '--color-live-bg',
  '--color-good-bg',
  '--color-bad-bg',
  '--color-bad-bg-strong',
  '--color-attention-bg',
  '--color-scrim'
] as const;

/**
 * Four colors the dock needs that the shared palette has no name for: the
 * sidebar's section-header text, the ring dockview draws around the section you
 * are working in, and the two tones a visible tab takes when its group is not
 * the focused one. Their Houston values live in
 * `src/lib/shell/styles/themeChrome.css` so the dock still looks right on the
 * very first frame, before any theme has been applied.
 */
export const CHROME_TOKEN_NAMES = [
  '--color-section-header-text',
  '--color-section-focus-ring',
  '--color-tab-unfocused-surface',
  '--color-tab-unfocused-text'
] as const;

/** Every custom property name a theme is responsible for. */
export const TOKEN_NAMES: string[] = [...PALETTE_TOKEN_NAMES, ...CHROME_TOKEN_NAMES];

// ── The terminal palette ────────────────────────────────────────────────────

/**
 * The terminal's colors, copied from `src/lib/shell/xtermFactory.ts`.
 *
 * BOTH themes use these, on purpose. The terminal in this app has been painted
 * in Dracula's colors since long before themes existed, in the old shell and
 * the new one, so Houston keeps exactly those colors — otherwise simply
 * switching themes on and off would repaint every terminal on screen. Giving
 * Houston a terminal palette of its own is a real design decision and a
 * visible change; it belongs in its own change, not in this one.
 */
const DRACULA_TERMINAL: TerminalTheme = {
  background: '#282a36',
  foreground: '#f8f8f2',
  cursor: '#f8f8f2',
  cursorAccent: '#282a36',
  selectionBackground: '#44475a',
  black: '#000000',
  red: '#ff5555',
  green: '#50fa7b',
  yellow: '#f1fa8c',
  blue: '#bd93f9',
  magenta: '#ff79c6',
  cyan: '#8be9fd',
  white: '#bbbbbb',
  brightBlack: '#555555',
  brightRed: '#ff5555',
  brightGreen: '#50fa7b',
  brightYellow: '#f1fa8c',
  brightBlue: '#caa9fa',
  brightMagenta: '#ff79c6',
  brightCyan: '#8be9fd',
  brightWhite: '#ffffff'
};

// ── Houston — what the app looks like today ─────────────────────────────────

const HOUSTON: ShellTheme = {
  id: 'houston',
  label: 'Houston',
  description: 'The dark mint look the app ships with.',
  tokens: {
    // Copied from src/lib/shell/styles/nextTokens.css. Do not edit one without
    // the other; the test reads that file and compares.
    '--color-bg': '#101014',
    '--color-surface': '#17171d',
    '--color-elevated': '#1f1f27',
    '--color-text': '#d8d8e0',
    '--color-text-2': '#6d6d7d',
    '--color-text-3': '#4c4c5a',
    '--color-accent': '#4bf3c8',
    '--color-border': '#22222c',
    '--color-focus': 'rgba(75, 243, 200, 0.45)',
    '--color-live': '#4bf3c8',
    '--color-good': '#8bdc9b',
    '--color-bad': '#ff6d91',
    '--color-attention': '#ffd493',
    '--color-idle': 'rgba(255, 255, 255, 0.16)',
    '--color-on-accent': '#0e1013',
    '--color-live-bg': 'rgba(75, 243, 200, 0.12)',
    '--color-good-bg': 'rgba(139, 220, 155, 0.12)',
    '--color-bad-bg': 'rgba(255, 109, 145, 0.12)',
    '--color-bad-bg-strong': 'rgba(255, 109, 145, 0.20)',
    '--color-attention-bg': 'rgba(255, 212, 147, 0.12)',
    '--color-scrim': 'rgba(0, 0, 0, 0.62)',
    // Copied from src/lib/shell/styles/themeChrome.css.
    '--color-section-header-text': '#7b7b8c',
    '--color-section-focus-ring': '#bd93f9',
    '--color-tab-unfocused-surface': '#14141a',
    '--color-tab-unfocused-text': '#9a9aa8'
  },
  monaco: {
    // Copied from src/lib/sourcePreviewAppearance.ts.
    id: 'houston',
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '8f9199', fontStyle: 'italic' },
      { token: 'comment.doc', foreground: 'a3a6af', fontStyle: 'italic' },
      { token: 'constant', foreground: '54b9ff' },
      { token: 'class', foreground: '00daef' },
      { token: 'delimiter', foreground: 'eef0f9' },
      { token: 'delimiter.angle', foreground: 'b7afff' },
      { token: 'delimiter.bracket', foreground: 'eef0f9' },
      { token: 'delimiter.curly', foreground: 'eef0f9' },
      { token: 'delimiter.parenthesis', foreground: 'eef0f9' },
      { token: 'delimiter.square', foreground: 'eef0f9' },
      { token: 'enum', foreground: 'b7afff' },
      { token: 'function', foreground: '00daef' },
      { token: 'identifier', foreground: '4bf3c8' },
      { token: 'interface', foreground: 'b7afff' },
      { token: 'invalid', foreground: 'f06788' },
      { token: 'keyword.async', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.await', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.class', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.const', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.function', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.interface', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.namespace', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.new', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.private', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.public', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.return', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.static', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword.type', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.using', foreground: 'ff6d91', fontStyle: 'bold' },
      { token: 'keyword.var', foreground: '54b9ff', fontStyle: 'bold' },
      { token: 'keyword', foreground: '54b9ff' },
      { token: 'keyword.control', foreground: '54b9ff' },
      { token: 'keyword.operator', foreground: 'eef0f9' },
      { token: 'method', foreground: '00daef' },
      { token: 'namespace', foreground: 'acafff' },
      { token: 'number', foreground: 'ffd493' },
      { token: 'number.binary', foreground: 'ffd493' },
      { token: 'number.float', foreground: 'ffd493' },
      { token: 'number.hex', foreground: 'ffd493' },
      { token: 'operator', foreground: 'eef0f9' },
      { token: 'regexp', foreground: 'eef0f9' },
      { token: 'regexp.escape', foreground: 'ffd493' },
      { token: 'regexp.escape.control', foreground: 'ff6d91' },
      { token: 'string', foreground: 'ffd493' },
      { token: 'string.escape', foreground: '54b9ff' },
      { token: 'string.invalid', foreground: 'f06788' },
      { token: 'string.quote', foreground: 'ffd493' },
      { token: 'type', foreground: 'acafff' },
      { token: 'type.identifier', foreground: 'acafff' },
      { token: 'variable', foreground: '4bf3c8' },
      { token: 'variable.predefined', foreground: 'acafff' }
    ],
    encodedTokensColors: [
      '#eef0f9',
      '#8f9199',
      '#54b9ff',
      '#ff6d91',
      '#00daef',
      '#4bf3c8',
      '#acafff',
      '#b7afff',
      '#ffd493',
      '#f06788'
    ],
    colors: {
      'editor.background': '#17191e',
      'editor.findMatchBackground': '#515c6a',
      'editor.findMatchBorder': '#74879f',
      'editor.findMatchHighlightBackground': '#ea5c0055',
      'editor.findMatchHighlightBorder': '#ffffff00',
      'editor.findRangeHighlightBackground': '#23262d',
      'editor.findRangeHighlightBorder': '#b2434300',
      'editor.foldBackground': '#ad5dca26',
      'editor.foreground': '#eef0f9',
      'editor.hoverHighlightBackground': '#5495d740',
      'editor.inactiveSelectionBackground': '#2a2d34',
      'editor.lineHighlightBackground': '#23262d',
      'editor.lineHighlightBorder': '#ffffff00',
      'editor.rangeHighlightBackground': '#ffffff0b',
      'editor.rangeHighlightBorder': '#ffffff00',
      'editor.selectionBackground': '#ad5dca44',
      'editor.selectionHighlightBackground': '#add6ff34',
      'editor.selectionHighlightBorder': '#495f77',
      'editor.wordHighlightBackground': '#494949b8',
      'editor.wordHighlightStrongBackground': '#004972b8',
      'editorBracketMatch.background': '#545864',
      'editorBracketMatch.border': '#ffffff00',
      'editorCursor.background': '#000000',
      'editorCursor.foreground': '#aeafad',
      'editorGutter.addedBackground': '#4bf3c8',
      'editorGutter.background': '#17191e',
      'editorGutter.commentRangeForeground': '#545864',
      'editorGutter.deletedBackground': '#f06788',
      'editorGutter.foldingControlForeground': '#545864',
      'editorGutter.modifiedBackground': '#54b9ff',
      'editorLineNumber.activeForeground': '#858b98',
      'editorLineNumber.foreground': '#545864',
      'editorSuggestWidget.background': '#252526',
      'editorSuggestWidget.border': '#454545',
      'editorSuggestWidget.foreground': '#d4d4d4',
      'editorSuggestWidget.highlightForeground': '#0097fb',
      'editorSuggestWidget.selectedBackground': '#062f4a',
      'editorWidget.background': '#343841',
      'editorWidget.foreground': '#ffffff',
      'editorWidget.resizeBorder': '#cc75f4',
      'minimap.background': '#17191e',
      'scrollbarSlider.activeBackground': '#54b9ff66',
      'scrollbarSlider.background': '#54586466',
      'scrollbarSlider.hoverBackground': '#545864B3'
    }
  },
  terminal: DRACULA_TERMINAL
};

// ── Dracula — the second theme, and the proof switching works ───────────────

const DRACULA: ShellTheme = {
  id: 'dracula',
  label: 'Dracula',
  description: 'Dracula’s purple and pink, across the whole app.',
  tokens: {
    '--color-bg': '#21222c',
    '--color-surface': '#282a36',
    '--color-elevated': '#343746',
    '--color-text': '#f8f8f2',
    '--color-text-2': '#6272a4',
    '--color-text-3': '#454c73',
    '--color-accent': '#bd93f9',
    '--color-border': '#44475a',
    '--color-focus': 'rgba(189, 147, 249, 0.45)',
    '--color-live': '#50fa7b',
    '--color-good': '#50fa7b',
    '--color-bad': '#ff5555',
    '--color-attention': '#f1fa8c',
    '--color-idle': 'rgba(248, 248, 242, 0.18)',
    '--color-on-accent': '#21222c',
    '--color-live-bg': 'rgba(80, 250, 123, 0.12)',
    '--color-good-bg': 'rgba(80, 250, 123, 0.12)',
    '--color-bad-bg': 'rgba(255, 85, 85, 0.12)',
    '--color-bad-bg-strong': 'rgba(255, 85, 85, 0.20)',
    '--color-attention-bg': 'rgba(241, 250, 140, 0.12)',
    '--color-scrim': 'rgba(0, 0, 0, 0.62)',
    '--color-section-header-text': '#7b83b0',
    '--color-section-focus-ring': '#bd93f9',
    '--color-tab-unfocused-surface': '#242631',
    '--color-tab-unfocused-text': '#9aa0c8'
  },
  monaco: {
    id: 'dracula',
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '6272a4', fontStyle: 'italic' },
      { token: 'comment.doc', foreground: '6272a4', fontStyle: 'italic' },
      { token: 'constant', foreground: 'bd93f9' },
      { token: 'class', foreground: '50fa7b' },
      { token: 'delimiter', foreground: 'f8f8f2' },
      { token: 'delimiter.angle', foreground: '8be9fd' },
      { token: 'delimiter.bracket', foreground: 'f8f8f2' },
      { token: 'delimiter.curly', foreground: 'f8f8f2' },
      { token: 'delimiter.parenthesis', foreground: 'f8f8f2' },
      { token: 'delimiter.square', foreground: 'f8f8f2' },
      { token: 'enum', foreground: '8be9fd' },
      { token: 'function', foreground: '50fa7b' },
      { token: 'identifier', foreground: 'f8f8f2' },
      { token: 'interface', foreground: '8be9fd' },
      { token: 'invalid', foreground: 'ff5555' },
      { token: 'keyword.async', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.await', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.class', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.const', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.function', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.interface', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.namespace', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.new', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.private', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.public', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.return', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.static', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword.type', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.using', foreground: 'ff79c6', fontStyle: 'bold' },
      { token: 'keyword.var', foreground: 'bd93f9', fontStyle: 'bold' },
      { token: 'keyword', foreground: 'bd93f9' },
      { token: 'keyword.control', foreground: 'bd93f9' },
      { token: 'keyword.operator', foreground: 'ff79c6' },
      { token: 'method', foreground: '50fa7b' },
      { token: 'namespace', foreground: '8be9fd' },
      { token: 'number', foreground: 'bd93f9' },
      { token: 'number.binary', foreground: 'bd93f9' },
      { token: 'number.float', foreground: 'bd93f9' },
      { token: 'number.hex', foreground: 'bd93f9' },
      { token: 'operator', foreground: 'ff79c6' },
      { token: 'regexp', foreground: 'f1fa8c' },
      { token: 'regexp.escape', foreground: 'ff79c6' },
      { token: 'regexp.escape.control', foreground: 'ff79c6' },
      { token: 'string', foreground: 'f1fa8c' },
      { token: 'string.escape', foreground: 'ff79c6' },
      { token: 'string.invalid', foreground: 'ff5555' },
      { token: 'string.quote', foreground: 'f1fa8c' },
      { token: 'type', foreground: '8be9fd' },
      { token: 'type.identifier', foreground: '8be9fd' },
      { token: 'variable', foreground: 'f8f8f2' },
      { token: 'variable.predefined', foreground: 'bd93f9' }
    ],
    encodedTokensColors: [
      '#f8f8f2',
      '#6272a4',
      '#bd93f9',
      '#ff79c6',
      '#50fa7b',
      '#f8f8f2',
      '#8be9fd',
      '#8be9fd',
      '#f1fa8c',
      '#ff5555'
    ],
    colors: {
      'editor.background': '#282a36',
      'editor.findMatchBackground': '#44475a',
      'editor.findMatchBorder': '#6272a4',
      'editor.findMatchHighlightBackground': '#ffb86c55',
      'editor.findMatchHighlightBorder': '#ffffff00',
      'editor.findRangeHighlightBackground': '#343746',
      'editor.findRangeHighlightBorder': '#b2434300',
      'editor.foldBackground': '#44475a55',
      'editor.foreground': '#f8f8f2',
      'editor.hoverHighlightBackground': '#6272a466',
      'editor.inactiveSelectionBackground': '#3a3d4d',
      'editor.lineHighlightBackground': '#343746',
      'editor.lineHighlightBorder': '#ffffff00',
      'editor.rangeHighlightBackground': '#ffffff0b',
      'editor.rangeHighlightBorder': '#ffffff00',
      'editor.selectionBackground': '#44475a99',
      'editor.selectionHighlightBackground': '#6272a444',
      'editor.selectionHighlightBorder': '#6272a4',
      'editor.wordHighlightBackground': '#44475ab8',
      'editor.wordHighlightStrongBackground': '#6272a4b8',
      'editorBracketMatch.background': '#6272a4',
      'editorBracketMatch.border': '#ffffff00',
      'editorCursor.background': '#000000',
      'editorCursor.foreground': '#f8f8f2',
      'editorGutter.addedBackground': '#50fa7b',
      'editorGutter.background': '#282a36',
      'editorGutter.commentRangeForeground': '#6272a4',
      'editorGutter.deletedBackground': '#ff5555',
      'editorGutter.foldingControlForeground': '#6272a4',
      'editorGutter.modifiedBackground': '#bd93f9',
      'editorLineNumber.activeForeground': '#f8f8f2',
      'editorLineNumber.foreground': '#6272a4',
      'editorSuggestWidget.background': '#282a36',
      'editorSuggestWidget.border': '#44475a',
      'editorSuggestWidget.foreground': '#f8f8f2',
      'editorSuggestWidget.highlightForeground': '#8be9fd',
      'editorSuggestWidget.selectedBackground': '#44475a',
      'editorWidget.background': '#343746',
      'editorWidget.foreground': '#ffffff',
      'editorWidget.resizeBorder': '#bd93f9',
      'minimap.background': '#282a36',
      'scrollbarSlider.activeBackground': '#bd93f966',
      'scrollbarSlider.background': '#6272a466',
      'scrollbarSlider.hoverBackground': '#6272a4b3'
    }
  },
  terminal: DRACULA_TERMINAL
};

// ── The roster ──────────────────────────────────────────────────────────────

/** The theme the app opens with, and the one every unknown name falls back to. */
export const DEFAULT_THEME_ID = 'houston';

/** Every theme, in the order the chooser should list them. */
export const THEMES: ShellTheme[] = [HOUSTON, DRACULA];

const BY_ID = new Map<string, ShellTheme>(THEMES.map((theme) => [theme.id, theme]));

/** Every theme, for a chooser or a palette command. */
export function listThemes(): ShellTheme[] {
  return [...THEMES];
}

/** True when `id` names a theme that exists. */
export function isThemeId(id: unknown): boolean {
  return typeof id === 'string' && BY_ID.has(id);
}

/**
 * The theme `id` means. A settings file written before themes existed says
 * `dark`, and `light` was offered but never built, so anything unrecognised
 * lands on the theme the app ships with rather than on nothing at all.
 */
export function resolveThemeId(id: unknown): string {
  return isThemeId(id) ? (id as string) : DEFAULT_THEME_ID;
}

/** The theme `id` names, or the default one. Never throws. */
export function getTheme(id: unknown): ShellTheme {
  return BY_ID.get(resolveThemeId(id)) as ShellTheme;
}
