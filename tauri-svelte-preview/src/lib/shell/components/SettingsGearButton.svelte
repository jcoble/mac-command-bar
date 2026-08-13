<script lang="ts">
  /**
   * SettingsGearButton.svelte — settings, in the bottom-left corner of the app.
   *
   * It is not a panel and not a tab, so it sits under the session list rather
   * than in either strip. PRESENTATIONAL ONLY: it opens whatever the page hands
   * it.
   */
  import Settings from '@lucide/svelte/icons/settings';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';

  interface Props {
    onOpenSettings(): void;
  }
  let { onOpenSettings }: Props = $props();
</script>

<div class="settings-corner">
  <IconButton
    label="Settings"
    size="sm"
    side="right"
    class="text-muted-foreground"
    data-testid="settings-gear"
    onclick={() => onOpenSettings()}
  >
    <Settings class="size-[18px]" strokeWidth={1.6} aria-hidden="true" />
  </IconButton>
</div>

<style>
  .settings-corner {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    padding: 4px 6px;
    border-top: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    background: var(--background);
  }
</style>
