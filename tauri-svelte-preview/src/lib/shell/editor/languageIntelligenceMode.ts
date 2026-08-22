/**
 * languageIntelligenceMode.ts — which projects the editor runs a language
 * server for, and how the persisted SQLite value is normalized.
 *
 * Two editor modes, per project:
 *
 *  - **Read mode**, the default. Opening a file — from a diff hunk, the file
 *    tree, anywhere — colours it and stops there. Nothing is started, so every
 *    project can be left open at once for the price of the text.
 *  - **Full mode**, switched on by the person reading. The project's language
 *    server starts with the next file opened, and switching it off again stops
 *    that server and gives its memory back.
 *
 * The choice belongs to the project, not to a session or a view: one server
 * serves them all. Everything here is plain data and plain functions so the
 * rules can be tested without a browser, a desktop app, or a language server.
 */

/** The global SQLite setting that owns the remembered per-project choices. */
export const LANGUAGE_INTELLIGENCE_SETTING_KEY = 'editor.language-intelligence';

/** Project root → is full mode on. A project not listed is in read mode. */
export type LanguageIntelligenceChoices = Readonly<Record<string, boolean>>;

/**
 * One spelling per project, so the same folder written two ways is one entry.
 * Matches the backend's own key, which is what keeps a saved choice pointing at
 * the workspace it was made for.
 */
export function workspaceKey(root: string): string {
  const trimmed = root.trim();
  if (trimmed.length <= 1) return trimmed;
  return trimmed.replace(/\/+$/, '');
}

/** Normalize the JSON value read from SQLite. Anything unreadable is no choices. */
export function normalizeLanguageIntelligenceChoices(
  value: unknown
): LanguageIntelligenceChoices {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  const choices: Record<string, boolean> = {};
  for (const [root, enabled] of Object.entries(value as Record<string, unknown>)) {
    const key = workspaceKey(root);
    if (key.length === 0 || typeof enabled !== 'boolean') continue;
    choices[key] = enabled;
  }
  return choices;
}

/** Is full mode on for this project? Unknown projects are in read mode. */
export function languageIntelligenceOn(
  choices: LanguageIntelligenceChoices,
  root: string | null
): boolean {
  if (!root) return false;
  return choices[workspaceKey(root)] === true;
}

/** The choices with one project changed. The original is left alone. */
export function withLanguageIntelligenceChoice(
  choices: LanguageIntelligenceChoices,
  root: string,
  enabled: boolean
): LanguageIntelligenceChoices {
  const key = workspaceKey(root);
  if (key.length === 0) return choices;
  return { ...choices, [key]: enabled };
}

/**
 * What to tell the desktop app at launch, given the project the editor is
 * pointed at.
 *
 * ONE project, never the whole list. Restoring every remembered choice would
 * put the app back in the state where several projects' servers wake up
 * together — which is the cost this whole feature exists to remove. A project
 * nobody is looking at stays in read mode until it is opened, and its saved
 * choice is applied then.
 *
 * Telling the app "on" starts nothing by itself: the next file opened in that
 * project does that.
 */
export function launchRestoreFor(
  choices: LanguageIntelligenceChoices,
  openRoot: string | null
): { root: string; enabled: boolean } | null {
  if (!openRoot) return null;
  const root = workspaceKey(openRoot);
  if (root.length === 0) return null;
  return { root, enabled: languageIntelligenceOn(choices, root) };
}

/**
 * May the editor start or use a language server for the file it is showing?
 *
 * The backend refuses on its own — that is the rule that actually holds — but
 * asking it to refuse costs a message per file, so the editor checks first.
 */
export function mayUseLanguageServer(options: {
  projectRoot: string | null;
  choices: LanguageIntelligenceChoices;
  nativeRuntime: boolean;
}): boolean {
  if (!options.nativeRuntime || !options.projectRoot) return false;
  return languageIntelligenceOn(options.choices, options.projectRoot);
}

/** What the switch in the editor header reads as. */
export function languageIntelligenceLabel(enabled: boolean): string {
  return enabled ? 'On' : 'Off';
}
