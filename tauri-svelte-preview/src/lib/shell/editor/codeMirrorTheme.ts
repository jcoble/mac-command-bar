import type { Extension } from '@codemirror/state';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';

import { getMonoFont } from '../themes/fontRegistry';

const syntax = HighlightStyle.define([
  { tag: [tags.keyword, tags.modifier], color: '#c792ea' },
  { tag: [tags.name, tags.variableName], color: '#d8dee9' },
  { tag: [tags.definition(tags.name), tags.function(tags.variableName)], color: '#82aaff' },
  { tag: [tags.typeName, tags.className, tags.namespace], color: '#ffcb6b' },
  { tag: [tags.string, tags.regexp], color: '#c3e88d' },
  { tag: [tags.number, tags.bool, tags.null], color: '#f78c6c' },
  { tag: [tags.comment, tags.meta], color: '#6d7480', fontStyle: 'italic' },
  { tag: [tags.operator, tags.punctuation], color: '#89ddff' },
  { tag: [tags.propertyName, tags.attributeName], color: '#f07178' },
  { tag: tags.invalid, color: '#ff5370', textDecoration: 'underline' }
]);

export interface CodeMirrorAppearance {
  fontFamily: string;
  fontSize: number;
  lineHeight: number;
  fontLigatures: boolean;
}

const defaultAppearance: CodeMirrorAppearance = {
  fontFamily: 'system',
  fontSize: 13,
  lineHeight: 21,
  fontLigatures: false
};

/** Build the editor chrome from current settings so an open editor repaints live. */
export function codeMirrorThemeForAppearance(
  appearance: CodeMirrorAppearance = defaultAppearance
): Extension {
  const editorFontFamily = getMonoFont(appearance.fontFamily).stack;
  return [
  EditorView.theme(
    {
      '&': {
        height: '100%',
        backgroundColor: 'var(--color-surface, #17191e)',
        color: 'var(--color-text, #d8dee9)',
        fontSize: `${appearance.fontSize}px`
      },
      '.cm-scroller': {
        overflow: 'auto',
        fontFamily: editorFontFamily,
        fontVariantLigatures: appearance.fontLigatures ? 'normal' : 'none',
        lineHeight: `${appearance.lineHeight}px`
      },
      '.cm-content': { caretColor: 'var(--color-accent, #82aaff)' },
      '.cm-cursor, .cm-dropCursor': {
        borderLeftColor: 'var(--color-accent, #82aaff)'
      },
      '.cm-selectionBackground, ::selection': {
        backgroundColor: 'color-mix(in srgb, var(--color-accent, #82aaff) 25%, transparent) !important'
      },
      '.cm-activeLine': { backgroundColor: 'rgba(255, 255, 255, 0.025)' },
      '.cm-gutters': {
        backgroundColor: 'var(--color-surface, #17191e)',
        color: 'var(--color-text-3, #5f6672)',
        borderRight: '1px solid var(--color-border, #252a33)'
      },
      '.cm-activeLineGutter': {
        backgroundColor: 'rgba(255, 255, 255, 0.04)',
        color: 'var(--color-text-2, #aab2bf)'
      },
      '.cm-tooltip': {
        backgroundColor: 'var(--color-elevated, #20242c)',
        color: 'var(--color-text, #d8dee9)',
        border: '1px solid var(--color-border, #303641)'
      },
      '.cm-tooltip-autocomplete > ul > li[aria-selected]': {
        backgroundColor: 'color-mix(in srgb, var(--color-accent, #82aaff) 22%, transparent)',
        color: 'var(--color-text, #d8dee9)'
      },
      '.cm-panels': {
        backgroundColor: 'var(--color-elevated, #20242c)',
        color: 'var(--color-text, #d8dee9)'
      }
    },
    { dark: true }
  ),
  syntaxHighlighting(syntax)
  ];
}

export const codeMirrorTheme = codeMirrorThemeForAppearance();

type ThemeMirrorModule = typeof import('thememirror');
type ThemeLoader = (themes: ThemeMirrorModule) => Extension;

const themeLoaders: Record<string, ThemeLoader> = {
  amy: ({ amy }) => amy,
  'ayu-light': ({ ayuLight }) => ayuLight,
  cobalt: ({ cobalt }) => cobalt,
  dracula: ({ dracula }) => dracula,
  'rose-pine-dawn': ({ rosePineDawn }) => rosePineDawn,
  tomorrow: ({ tomorrow }) => tomorrow
};

/** Resolve the selected CodeMirror extension, keeping Houston as the fallback. */
export async function loadCodeMirrorTheme(
  themeId: unknown,
  appearance: CodeMirrorAppearance = defaultAppearance
): Promise<Extension> {
  const base = codeMirrorThemeForAppearance(appearance);
  const loader = themeLoaders[typeof themeId === 'string' ? themeId : ''];
  if (!loader) return base;
  // Keep ThemeMirror, including its catalog, out of the startup chunk.
  const themes = await import('thememirror');
  // CodeMirror reverses theme style modules when it mounts them. Put the
  // selected theme first so its equal-specificity rules are mounted last.
  return [loader(themes), base];
}
