<!--
  SettingsPanel.svelte — app settings surface (Appearance / Editor / Terminal / General).

  Built from the shared $lib/components primitives (Dialog, Tabs, Select,
  Slider, Switch, Button) and bound to the runes settingsStore. Borderless,
  spacious, token-styled. Not yet wired into the app — drop it anywhere and
  bind `open`.

  Usage:
    <script>
      import SettingsPanel from '$lib/SettingsPanel.svelte';
      let settingsOpen = $state(false);
    </script>
    <SettingsPanel bind:open={settingsOpen} />
-->
<script lang="ts">
	import Dialog from '$lib/components/Dialog.svelte';
	import Tabs from '$lib/components/Tabs.svelte';
	import Select from '$lib/components/Select.svelte';
	import Slider from '$lib/components/Slider.svelte';
	import Switch from '$lib/components/Switch.svelte';
	import Button from '$lib/components/Button.svelte';
	import { Palette, Type, SquareTerminal, SlidersHorizontal, RotateCcw } from '@lucide/svelte';
	import { settings, resetSettings } from '$lib/settingsStore.svelte';

	interface Props {
		/** Whether the settings dialog is open. */
		open?: boolean;
	}

	let { open = $bindable(false) }: Props = $props();

	// ── Option lists ────────────────────────────────────────────────────────
	const themeItems = [
		{ value: 'dark', label: 'Dark' },
		{ value: 'light', label: 'Light' },
		{ value: 'houston', label: 'Houston' }
	];

	const fontFamilyItems = [
		{ value: 'Google Sans Mono', label: 'Google Sans Mono' },
		{ value: 'SF Mono', label: 'SF Mono' },
		{ value: 'JetBrains Mono', label: 'JetBrains Mono' },
		{ value: 'Fira Code', label: 'Fira Code' },
		{ value: 'Menlo', label: 'Menlo' },
		{ value: 'Monaco', label: 'Monaco' },
		{ value: 'Consolas', label: 'Consolas' }
	];

	const terminalThemeItems = [
		{ value: 'dracula', label: 'Dracula' },
		{ value: 'houston', label: 'Houston' },
		{ value: 'solarized-dark', label: 'Solarized Dark' },
		{ value: 'one-dark', label: 'One Dark' }
	];

	// Mirrors the SourceTerminalApp options in +page.svelte.
	const terminalAppItems = [
		{ value: 'Warp', label: 'Warp' },
		{ value: 'Terminal', label: 'Terminal' },
		{ value: 'iTerm', label: 'iTerm' },
		{ value: 'iTerm2', label: 'iTerm2' },
		{ value: 'Ghostty', label: 'Ghostty' },
		{ value: 'WezTerm', label: 'WezTerm' },
		{ value: 'Alacritty', label: 'Alacritty' }
	];

	const tabs = [
		{ value: 'appearance', label: 'Appearance', icon: Palette },
		{ value: 'editor', label: 'Editor', icon: Type },
		{ value: 'terminal', label: 'Terminal', icon: SquareTerminal },
		{ value: 'general', label: 'General', icon: SlidersHorizontal }
	];

	let activeTab = $state('appearance');

	// Local UI-only toggles (no store field yet) — kept here so the Switch
	// controls are functional in isolation. Application is a later step.
	let editorLigatures = $state(false);
	let terminalCursorBlink = $state(true);

	function resetActiveSection() {
		resetSettings(activeTab as 'appearance' | 'editor' | 'terminal' | 'general');
	}
</script>

<Dialog bind:open title="Settings" description="Tune appearance and tooling preferences.">
	<div class="settings">
		<Tabs bind:value={activeTab} {tabs}>
			{#snippet panel(value)}
				{#if value === 'appearance'}
					<div class="section">
						<div class="row">
							<div class="row__label">
								<span class="row__title">Theme</span>
								<span class="row__hint">Overall color scheme for the app.</span>
							</div>
							<div class="row__control row__control--select">
								<Select bind:value={settings.appearance.themeId} items={themeItems} />
							</div>
						</div>

						<div class="row row--stack">
							<div class="row__label">
								<span class="row__title">Interface font size</span>
								<span class="row__hint">Base size for chrome, labels and menus.</span>
							</div>
							<div class="row__control">
								<Slider
									bind:value={settings.appearance.appFontSize}
									min={11}
									max={18}
									step={1}
								/>
							</div>
						</div>
					</div>
				{:else if value === 'editor'}
					<div class="section">
						<div class="row">
							<div class="row__label">
								<span class="row__title">Font family</span>
								<span class="row__hint">Typeface used in the code editor.</span>
							</div>
							<div class="row__control row__control--select">
								<Select bind:value={settings.editor.fontFamily} items={fontFamilyItems} />
							</div>
						</div>

						<div class="row row--stack">
							<div class="row__label">
								<span class="row__title">Font size</span>
								<span class="row__hint">Editor text size in pixels.</span>
							</div>
							<div class="row__control">
								<Slider bind:value={settings.editor.fontSize} min={10} max={24} step={1} />
							</div>
						</div>

						<div class="row row--stack">
							<div class="row__label">
								<span class="row__title">Line height</span>
								<span class="row__hint">Absolute line height in pixels.</span>
							</div>
							<div class="row__control">
								<Slider bind:value={settings.editor.lineHeight} min={14} max={36} step={1} />
							</div>
						</div>

						<div class="row">
							<div class="row__label">
								<span class="row__title">Font ligatures</span>
								<span class="row__hint">Render combined glyphs like {'=>'} and {'!=='}.</span>
							</div>
							<div class="row__control row__control--switch">
								<Switch bind:checked={editorLigatures} />
							</div>
						</div>
					</div>
				{:else if value === 'terminal'}
					<div class="section">
						<div class="row">
							<div class="row__label">
								<span class="row__title">Font family</span>
								<span class="row__hint">Typeface used in the embedded terminal.</span>
							</div>
							<div class="row__control row__control--select">
								<Select bind:value={settings.terminal.fontFamily} items={fontFamilyItems} />
							</div>
						</div>

						<div class="row row--stack">
							<div class="row__label">
								<span class="row__title">Font size</span>
								<span class="row__hint">Terminal text size in pixels.</span>
							</div>
							<div class="row__control">
								<Slider bind:value={settings.terminal.fontSize} min={10} max={24} step={1} />
							</div>
						</div>

						<div class="row row--stack">
							<div class="row__label">
								<span class="row__title">Line height</span>
								<span class="row__hint">Line spacing multiplier (relative to font size).</span>
							</div>
							<div class="row__control">
								<Slider
									bind:value={settings.terminal.lineHeight}
									min={1}
									max={2}
									step={0.1}
								/>
							</div>
						</div>

						<div class="row">
							<div class="row__label">
								<span class="row__title">Color theme</span>
								<span class="row__hint">Palette applied to terminal output.</span>
							</div>
							<div class="row__control row__control--select">
								<Select bind:value={settings.terminal.theme} items={terminalThemeItems} />
							</div>
						</div>

						<div class="row">
							<div class="row__label">
								<span class="row__title">Cursor blink</span>
								<span class="row__hint">Pulse the terminal cursor when idle.</span>
							</div>
							<div class="row__control row__control--switch">
								<Switch bind:checked={terminalCursorBlink} />
							</div>
						</div>
					</div>
				{:else if value === 'general'}
					<div class="section">
						<div class="row">
							<div class="row__label">
								<span class="row__title">External terminal app</span>
								<span class="row__hint">Used when opening a folder in a terminal.</span>
							</div>
							<div class="row__control row__control--select">
								<Select bind:value={settings.general.terminalApp} items={terminalAppItems} />
							</div>
						</div>
					</div>
				{/if}
			{/snippet}
		</Tabs>
	</div>

	{#snippet footer()}
		<Button variant="ghost" size="sm" leftIcon={RotateCcw} onclick={resetActiveSection}>
			Reset section
		</Button>
		<Button variant="primary" size="sm" onclick={() => (open = false)}>Done</Button>
	{/snippet}
</Dialog>

<style>
	.settings {
		/* Give the tabbed body comfortable breathing room and a stable height
		   so switching tabs doesn't jump the dialog around. */
		min-height: 320px;
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding-block-start: var(--space-1);
	}

	/* ── Labeled row ─────────────────────────────────────────────────────── */
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-5);
	}

	/* Slider rows read better stacked: full-width control under the label. */
	.row--stack {
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-3);
	}

	.row__label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}

	.row__title {
		font-size: var(--text-md);
		font-weight: var(--weight-medium);
		color: var(--color-text);
		line-height: 1.3;
	}

	.row__hint {
		font-size: var(--text-sm);
		color: var(--color-text-3);
		line-height: 1.4;
	}

	.row__control {
		flex-shrink: 0;
	}

	/* Stacked rows let the control fill the width. */
	.row--stack .row__control {
		width: 100%;
	}

	.row__control--select {
		width: 200px;
	}

	.row__control--switch {
		display: flex;
		justify-content: flex-end;
	}
</style>
