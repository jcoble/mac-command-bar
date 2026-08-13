<script lang="ts">
  /**
   * LanguageIntelligenceControls.svelte — the project's language-server
   * controls, in the strip along the top of the shell.
   *
   * They sit beside the run and browser buttons because they are about the
   * PROJECT, not about the file on screen: one language server serves every
   * session and every view on it. In the editor's own header they crowded the
   * file tabs and read as a property of whichever file was showing.
   *
   * It owns nothing. Everything shown here is published by
   * `EditorPanel.svelte`, which asks the desktop app for the status, listens
   * for pushed updates and starts or stops the server. With no project open
   * the strip looks exactly as it did before: nothing renders.
   */
  import LanguageServerStatusChip from './LanguageServerStatusChip.svelte';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import { languageIntelligenceLabel } from '$lib/shell/editor/languageIntelligenceMode';
  import {
    languageIntelligenceBar,
    requestLanguageIntelligence
  } from '$lib/shell/editor/languageIntelligenceBar.svelte';
</script>

{#if languageIntelligenceBar.hasProject}
  <div class="language-intelligence" title={languageIntelligenceBar.title}>
    <!-- Nothing renders here in a browser tab or on an older desktop build:
         there is no language server to report on, so there is no chip. -->
    <LanguageServerStatusChip
      language={languageIntelligenceBar.language}
      status={languageIntelligenceBar.status}
    />

    <!-- The project's editor mode. Off is read mode: colouring only, nothing
         started. On runs the project's one language server, shared by every
         session and view on it, and its cost shows in the resource view. -->
    <span class="intelligence-name">Language intelligence</span>
    <Switch
      checked={languageIntelligenceBar.fullMode}
      disabled={languageIntelligenceBar.busy}
      onCheckedChange={(checked) => requestLanguageIntelligence(checked)}
      aria-label="Language intelligence"
    />
    <span class="intelligence-state" class:on={languageIntelligenceBar.fullMode}>
      {languageIntelligenceLabel(languageIntelligenceBar.fullMode)}
    </span>
  </div>
{/if}

<style>
  /* The end of the top strip: this group is pushed to the right edge, away
   * from the run and browser buttons at the left. */
  .language-intelligence {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
    min-width: max-content;
    margin-left: auto;
    height: 26px;
  }

  .intelligence-name {
    color: var(--color-text-2);
    font-size: 13px;
    white-space: nowrap;
  }

  .intelligence-state {
    color: var(--color-text-3);
    font-size: 13px;
    min-width: 22px;
  }

  .intelligence-state.on {
    color: var(--color-text);
  }
</style>
