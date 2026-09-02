<script lang="ts">
  /** Global Editor/Supercharged setting shown beside the centre-pane pills. */
  import { onDestroy } from 'svelte';
  import { settings } from '$lib/settingsStore.svelte';
  import { setLanguageServersEnabled } from '$lib/shell/editor/sourceIntelligence';
  import {
    languageIntelligenceBar
  } from '$lib/shell/editor/languageIntelligenceBar.svelte';

  let busy = $state(false);
  const stopController = new AbortController();
  const stopSignal = stopController.signal;

  onDestroy(() => stopController.abort());

  async function chooseMode(enabled: boolean): Promise<void> {
    if (stopSignal.aborted || busy || enabled === settings.intelligence.languageServers) return;
    busy = true;
    const result = await setLanguageServersEnabled(enabled);
    if (stopSignal.aborted) return;
    if (result.supported) settings.intelligence.languageServers = enabled;
    busy = false;
  }

  /** Explain what the global choice will do before a project is open. */
  const hoverText = $derived(
    languageIntelligenceBar.hasProject
      ? [languageIntelligenceBar.title, `Supercharged: ${settings.intelligence.languageServers ? 'on' : 'off'}`]
          .filter((part) => part.length > 0)
          .join(' · ')
      : settings.intelligence.languageServers
        ? 'Supercharged is on. A language server starts when you open a supported project file.'
        : 'Editor mode is on. Files keep syntax highlighting without starting a language server.'
  );
</script>

<!-- This global choice stays available even before a project is open. -->
<div
  class="language-switch"
  title={hoverText}
  role="group"
  aria-label="Editor mode"
>
  <button
    type="button"
    class:active={!settings.intelligence.languageServers}
    disabled={busy}
    aria-pressed={!settings.intelligence.languageServers}
    onclick={() => void chooseMode(false)}
  >Editor</button>
  <button
    type="button"
    class:active={settings.intelligence.languageServers}
    disabled={busy}
    aria-pressed={settings.intelligence.languageServers}
    onclick={() => void chooseMode(true)}
  >Supercharged</button>
</div>

<style>
  .language-switch {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex: 0 0 auto;
    min-width: max-content;
    height: 28px;
    padding: 0;
    color: var(--color-text-2);
    user-select: none;
  }

  .language-switch::after {
    content: '';
    width: 1px;
    height: 16px;
    margin-left: 2px;
    background: color-mix(in srgb, var(--color-text) 16%, transparent);
  }

  .language-switch button {
    height: 24px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-2);
    font: inherit;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
  }

  .language-switch button:hover:not(:disabled) {
    background: var(--pill-surface-hover);
    color: var(--color-text);
  }

  .language-switch button.active {
    background: var(--pill-surface-active);
    color: var(--pill-text-active);
  }

  .language-switch button:disabled {
    cursor: wait;
    opacity: 0.65;
  }
</style>
