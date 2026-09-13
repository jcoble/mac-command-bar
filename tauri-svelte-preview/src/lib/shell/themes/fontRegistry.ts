/**
 * fontRegistry.ts — the typefaces a person can pick for the interface.
 *
 * Two lists, one for reading and one for code, each with a "System" entry
 * that means "whatever this Mac draws its own interface in". Every other entry
 * names a face that ships with the app (fonts.css) — a face that has to be
 * installed on the machine would look right here and wrong on someone else's.
 *
 * `stack` is what ends up in `--font-ui` / `--font-mono`; the fallbacks after
 * the named face are the same system stack, so a glyph the face lacks still
 * draws in something close.
 */

export interface ShellFont {
  id: string;
  label: string;
  /** The `font-family` value, complete with fallbacks. */
  stack: string;
}

const UI_FALLBACK =
  'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "Helvetica Neue", Arial, sans-serif';
const MONO_FALLBACK = 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace';
const MENLO_STACK = 'Menlo, Monaco, "Courier New", monospace';

export const DEFAULT_UI_FONT_ID = 'system';
export const DEFAULT_MONO_FONT_ID = 'menlo';

/** Faces for reading, in the order the chooser lists them. */
export const UI_FONTS: ShellFont[] = [
  { id: 'system', label: 'System', stack: UI_FALLBACK },
  { id: 'inter', label: 'Inter', stack: `"Inter", ${UI_FALLBACK}` },
  { id: 'figtree', label: 'Figtree', stack: `"Figtree", ${UI_FALLBACK}` },
  { id: 'manrope', label: 'Manrope', stack: `"Manrope", ${UI_FALLBACK}` },
  { id: 'dm-sans', label: 'DM Sans', stack: `"DM Sans", ${UI_FALLBACK}` },
  { id: 'plus-jakarta', label: 'Plus Jakarta Sans', stack: `"Plus Jakarta Sans", ${UI_FALLBACK}` }
];

/** Faces for code, in the order the chooser lists them. */
export const MONO_FONTS: ShellFont[] = [
  { id: 'menlo', label: 'Menlo', stack: MENLO_STACK },
  { id: 'system', label: 'System', stack: MONO_FALLBACK },
  { id: 'jetbrains-mono', label: 'JetBrains Mono', stack: `"JetBrains Mono", ${MONO_FALLBACK}` },
  { id: 'fira-code', label: 'Fira Code', stack: `"Fira Code", ${MONO_FALLBACK}` },
  { id: 'geist-mono', label: 'Geist Mono', stack: `"Geist Mono", ${MONO_FALLBACK}` }
];

const UI_BY_ID = new Map(UI_FONTS.map((font) => [font.id, font]));
const MONO_BY_ID = new Map(MONO_FONTS.map((font) => [font.id, font]));

/** The reading face `id` names, or the system one. Never throws. */
export function getUiFont(id: unknown): ShellFont {
  return (typeof id === 'string' && UI_BY_ID.get(id)) || (UI_BY_ID.get(DEFAULT_UI_FONT_ID) as ShellFont);
}

/** The code face `id` names, or the system one. Never throws. */
export function getMonoFont(id: unknown): ShellFont {
  return (typeof id === 'string' && MONO_BY_ID.get(id)) || (MONO_BY_ID.get(DEFAULT_MONO_FONT_ID) as ShellFont);
}
