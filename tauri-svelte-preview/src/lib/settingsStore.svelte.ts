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

export interface Settings {
	appearance: AppearanceSettings;
	editor: EditorSettings;
	terminal: TerminalSettings;
	general: GeneralSettings;
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
