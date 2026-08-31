<script lang="ts">
  /**
   * UtilityStrip.svelte — the slim always-visible strip along the bottom of the
   * right panel.
   *
   * Two things live here: Resources, which carries the machine's live memory,
   * CPU and process count and opens the Resource Manager, and Usage, which
   * opens the quota card. Neither is a tab — they are always on screen, under
   * whichever panel is open.
   *
   * The surfaces themselves are mounted at the page root (see
   * `ShellOverlays.svelte`), so this component only says which one was asked
   * for and hands over the rectangle of the button that was pressed.
   *
   * The one piece of IO here is the lightweight resource totals read: it runs
   * on mount and whenever the window comes back to the front.
   */
  import { onMount } from 'svelte';
  import ChartNoAxesCombined from '@lucide/svelte/icons/chart-no-axes-combined';
  import Cpu from '@lucide/svelte/icons/cpu';
  import Settings from '@lucide/svelte/icons/settings';

  import {
    formatResourceBytes,
    formatResourceCpu
  } from '$lib/shell/resources/resourceSampleViewModel';
  import {
    refreshResourceTotals,
    resourceSampleState
  } from '$lib/shell/resources/resourceSampleStore.svelte';
  import { resourceDiagnostics } from '$lib/shell/resourceDiagnostics.svelte';

  import { utilityAnchorFor, type UtilityId } from './utilityStrip';

  interface Props {
    /** Which of the two surfaces is open right now, so its button reads as on. */
    openUtility?: UtilityId | null;
    /** Open one of the two surfaces, anchored to the button that was pressed. */
    onOpenUtility(id: UtilityId, anchor: ReturnType<typeof utilityAnchorFor>): void;
    /** Open the settings dialog. The gear sits at the left end of this bar. */
    onOpenSettings(): void;
  }
  let { openUtility = null, onOpenUtility, onOpenSettings }: Props = $props();

  function open(id: UtilityId, event: MouseEvent): void {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    onOpenUtility(id, utilityAnchorFor(event.currentTarget.getBoundingClientRect()));
  }

  const summary = $derived.by(() => {
    const totals = resourceSampleState.totals ?? resourceSampleState.snapshot?.native.totals;
    if (totals) {
      return `${formatResourceCpu(totals.cpuPercent)} · ${formatResourceBytes(
        totals.physicalFootprintBytes
      )} Σ Physical footprint · RSS ${formatResourceBytes(totals.rssBytes)}`;
    }
    if (resourceSampleState.loading) return 'Reading process usage…';
    return resourceSampleState.error ?? 'Process usage is not available';
  });

  const ownershipSummary = $derived.by(() => {
    if (!import.meta.env.DEV) return '';
    return [
      `chat ${resourceDiagnostics.loadedConversationProjections}/${resourceDiagnostics.loadedConversationEventCount}`,
      `snap ${resourceDiagnostics.conversationSnapshotReadsInFlight}/${resourceDiagnostics.conversationSnapshotReadsInvalidated} stale`,
      `turns ${resourceDiagnostics.conversationRenderedRows}/${resourceDiagnostics.conversationVirtualRows} · cache ${resourceDiagnostics.conversationMeasuredElementCacheEntries}/${resourceDiagnostics.conversationMeasuredSizeCacheEntries}`,
      `CM ${resourceDiagnostics.codeMirrorEditorStates}/${formatResourceBytes(resourceDiagnostics.openTabDocumentBytes)}`,
      `source ${resourceDiagnostics.editorSourceReadsInFlight}/${resourceDiagnostics.editorSourceReadsInvalidated} stale/${formatResourceBytes(resourceDiagnostics.editorSourceReadBytesInFlight)}`,
      `dirs ${resourceSampleState.totals?.activeSourceDirectoryReads ?? 0}`
    ].join(' · ');
  });

  onMount(() => {
    void refreshResourceTotals();
    const refreshWhenVisible = (): void => {
      if (document.visibilityState === 'visible') void refreshResourceTotals();
    };
    document.addEventListener('visibilitychange', refreshWhenVisible);
    return () => {
      document.removeEventListener('visibilitychange', refreshWhenVisible);
    };
  });
</script>

<footer class="utility-strip" aria-label="Settings, resources and usage">
  <button
    type="button"
    class="utility settings"
    data-testid="settings-gear"
    aria-label="Settings"
    onclick={() => onOpenSettings()}
  >
    <Settings class="glyph" strokeWidth={1.6} aria-hidden="true" />
  </button>
  <button
    type="button"
    class="utility resources"
    class:open={openUtility === 'resources'}
    aria-expanded={openUtility === 'resources'}
    data-testid="utility-resources"
    onclick={(event) => open('resources', event)}
  >
    <Cpu class="glyph" strokeWidth={1.6} aria-hidden="true" />
    <span class="label">Resources</span>
    <span class="summary">{summary}</span>
    {#if ownershipSummary}
      <span class="ownership-summary">· {ownershipSummary}</span>
    {/if}
  </button>
  <button
    type="button"
    class="utility usage"
    class:open={openUtility === 'usage'}
    aria-expanded={openUtility === 'usage'}
    aria-label="Usage and limits"
    data-testid="utility-usage"
    onclick={(event) => open('usage', event)}
  >
    <ChartNoAxesCombined class="glyph" strokeWidth={1.6} aria-hidden="true" />
    <span class="label">Usage</span>
  </button>
</footer>

<style>
  .utility-strip {
    display: flex;
    flex: 0 0 28px;
    width: 100%;
    min-width: 0;
    height: 28px;
    align-items: stretch;
    gap: 4px;
    padding: 0 8px;
    /* The status bar that hosts this strip paints its own surface and top
       edge; drawing them again here doubled the rule. */
    color: var(--muted-foreground);
    container-type: inline-size;
    user-select: none;
  }

  .utility {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 6px;
    padding: 0 6px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 12px;
    line-height: 1;
    cursor: pointer;
  }

  /* No label, so it needs no room for one. */
  .settings {
    flex: 0 0 auto;
  }

  /* Hugs its own text rather than filling the bar. This strip used to sit in
     the narrow tools column, where stretching was right; across the whole
     window it made one button the width of the screen. The auto margin is what
     keeps Usage against the right edge. */
  .resources {
    flex: 0 1 auto;
    margin-right: auto;
    overflow: hidden;
  }

  .usage {
    flex: 0 0 auto;
  }

  .utility:hover,
  .utility.open {
    background: var(--accent);
    color: var(--foreground);
  }

  .utility:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--ring) 50%, transparent);
  }

  .utility :global(.glyph) {
    width: 14px;
    height: 14px;
    flex: 0 0 auto;
  }

  .label {
    flex: 0 0 auto;
    color: var(--foreground);
    font-weight: 500;
  }

  .summary {
    min-width: 0;
    overflow: hidden;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ownership-summary {
    min-width: 0;
    overflow: hidden;
    color: var(--muted-foreground);
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* In a narrow column the numbers go before the name does. */
  @container (max-width: 260px) {
    .summary {
      display: none;
    }
  }
</style>
