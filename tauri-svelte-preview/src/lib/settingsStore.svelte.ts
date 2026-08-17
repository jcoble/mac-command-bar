/**
 * settingsStore.svelte.ts — Svelte 5 runes-based settings store.
 *
 * Holds appearance / editor / terminal / general preferences in a reactive
 * `$state` object, persisted to localStorage under
 * `mac-command-bar.settings`. Loads once on module init and saves on any
 * change via `$effect.root`.
 *
 * Defaults mirror the app's current hardcoded values:
 *   - editor:   src/lib/sourcePreviewAppearance.ts (Google Sans Mono / 13 / 21)
 *   - terminal: the xterm config in +page.svelte (Google Sans Mono / 15 / 1.2,
 *               dracula theme)
 *   - general:  the source terminal app default ('Warp')
 *
 * NOTE: This module only owns the *values*. Applying them into Monaco / xterm
 * is a later wiring step in +page.svelte and is intentionally NOT done here.
 */

// ── Types ──────────────────────────────────────────────────────────────────

export interface AppearanceSettings {
	themeId: string;
	/** Base UI font size in px (chrome/labels, not the editor). */
	appFontSize: number;
}

export interface EditorSettings {
	fontFamily: string;
	fontSize: number;
	/** Absolute line height in px (Monaco-style). */
	lineHeight: number;
}

export interface TerminalSettings {
	fontFamily: string;
	fontSize: number;
	/** Unitless line-height multiplier (xterm-style). */
	lineHeight: number;
	theme: string;
}

export interface GeneralSettings {
	terminalApp: string;
}

/**
 * Where the Problems list is shown.
 *
 * 'bottom' is the strip under the middle of the shell it has always lived in;
 * 'hidden' shows it nowhere, which leaves the bottom strip with nothing in it,
 * so the shell closes the strip as well.
 *
 * There used to be a third answer, 'right', which put the list in the tool
 * column. That column is now eight fixed panels with no room for a ninth, so
 * the answer no longer names anywhere. A stored 'right' is not migrated by
 * hand: the check below already turns any unrecognised value back into
 * 'bottom', which is exactly the right outcome.
 */
export type ProblemsLocation = 'bottom' | 'hidden';

export const PROBLEMS_LOCATIONS: readonly ProblemsLocation[] = ['bottom', 'hidden'];

/** What each choice says in the settings dialog, in plain English. */
export const PROBLEMS_LOCATION_LABELS: Record<ProblemsLocation, string> = {
	bottom: 'In the strip along the bottom',
	hidden: 'Do not show it'
};

export interface PanelsSettings {
	problemsLocation: ProblemsLocation;
}

export interface IntelligenceSettings {
	/**
	 * Run the C# language server for C# projects.
	 *
	 * Turning it off frees the memory and processor time it uses; in exchange the
	 * app can no longer underline mistakes in C# files or jump to a definition
	 * precisely. Counting where something is used keeps working either way — that
	 * comes from the app's own search of the project, not from the language
	 * server.
	 */
	csharpLanguageServer: boolean;
}

export interface Settings {
	appearance: AppearanceSettings;
	editor: EditorSettings;
	terminal: TerminalSettings;
	general: GeneralSettings;
	panels: PanelsSettings;
	intelligence: IntelligenceSettings;
}

/** A section key of {@link Settings}. */
export type SettingsSection = keyof Settings;

// ── Defaults ────────────────────────────────────────────────────────────────

export const STORAGE_KEY = 'mac-command-bar.settings';

/** Produces a fresh, deeply-independent copy of the default settings. */
export function defaultSettings(): Settings {
	return {
		appearance: {
			themeId: 'dark',
			appFontSize: 13
		},
		editor: {
			fontFamily: 'Google Sans Mono',
			fontSize: 13,
			lineHeight: 21
		},
		terminal: {
			fontFamily: 'Google Sans Mono',
			fontSize: 15,
			lineHeight: 1.2,
			theme: 'dracula'
		},
		general: {
			terminalApp: 'Warp'
		},
		panels: {
			// Hidden by default: the strip reserved 180px under the conversation
			// whether or not there was anything in it, which read as dead space
			// above the status bar. Settings turns it back on.
			problemsLocation: 'hidden'
		},
		intelligence: {
			csharpLanguageServer: true
		}
	};
}

// ── Persistence helpers ──────────────────────────────────────────────────────

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Merge a parsed/untrusted object onto the defaults, keeping only known keys
 * and only when the stored value has the same primitive type as the default.
 * This makes loading resilient to partial / stale / corrupt payloads and to
 * future schema additions (new default keys simply fill in).
 */
function mergeWithDefaults(raw: unknown): Settings {
	const base = defaultSettings();
	if (!isRecord(raw)) return base;

	for (const section of Object.keys(base) as SettingsSection[]) {
		const incoming = raw[section];
		if (!isRecord(incoming)) continue;
		const target = base[section] as unknown as Record<string, unknown>;
		for (const key of Object.keys(target)) {
			const next = incoming[key];
			if (next !== undefined && typeof next === typeof target[key]) {
				target[key] = next;
			}
		}
	}

	// The type check above only asks "is it a string?", which is not enough for a
	// setting whose value has to be one of two words: a stored "Right", or any
	// leftover from an older build, would pass it and then match no branch — and
	// for THIS setting the failure is silent and hard to escape, because the
	// non-default answer closes the strip that holds the control for changing it
	// back. Anything unrecognised goes back to showing the list where it has
	// always been.
	if (!PROBLEMS_LOCATIONS.includes(base.panels.problemsLocation)) {
		base.panels.problemsLocation = 'hidden';
	}

	return base;
}

function loadSettings(): Settings {
	if (typeof localStorage === 'undefined') return defaultSettings();
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (!stored) return defaultSettings();
		return mergeWithDefaults(JSON.parse(stored));
	} catch {
		return defaultSettings();
	}
}

function persist(value: Settings): void {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
	} catch {
		/* storage full / unavailable — non-fatal, keep in-memory state */
	}
}

// ── Reactive state ────────────────────────────────────────────────────────────

/**
 * The single reactive settings object. Read fields directly in components
 * (e.g. `settings.editor.fontSize`) and bind to them; reads/writes are
 * tracked by Svelte's runes runtime and auto-persisted.
 */
export const settings = $state<Settings>(loadSettings());

// Save on any change. `$effect.root` lets us own an effect outside of a
// component, with a detach handle for teardown (tests / HMR). Reading the
// nested fields registers the dependency graph.
const disposeAutosave = $effect.root(() => {
	$effect(() => {
		persist($state.snapshot(settings) as Settings);
	});
});

/** Tear down the autosave effect (for tests / hot-reload). Rarely needed. */
export function disposeSettingsStore(): void {
	disposeAutosave();
}

// ── Update / reset API ────────────────────────────────────────────────────────

/**
 * Patch a single settings section. Only provided keys are changed; the rest of
 * the section is preserved. Persistence happens automatically via the effect.
 *
 * @example updateSettings('editor', { fontSize: 14 })
 */
export function updateSettings<S extends SettingsSection>(
	section: S,
	patch: Partial<Settings[S]>
): void {
	Object.assign(settings[section] as object, patch);
}

/**
 * Replace a settings section wholesale with the given values.
 */
export function setSection<S extends SettingsSection>(
	section: S,
	value: Settings[S]
): void {
	settings[section] = value;
}

/**
 * Reset everything (or a single section) back to defaults.
 *
 * @example resetSettings()            // all sections
 * @example resetSettings('terminal')  // just the terminal section
 */
export function resetSettings(section?: SettingsSection): void {
	const defaults = defaultSettings();
	if (section) {
		settings[section] = defaults[section] as never;
		return;
	}
	for (const key of Object.keys(defaults) as SettingsSection[]) {
		settings[key] = defaults[key] as never;
	}
}
