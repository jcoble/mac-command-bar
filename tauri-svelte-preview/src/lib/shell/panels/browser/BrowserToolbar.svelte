<script lang="ts">
  /**
   * BrowserToolbar.svelte — one row: where you are, and what the pointer does.
   *
   * Everything is on the address line. The three marking tools sit to the right
   * of the address box as icons alone, and Widen sits at the end of the same
   * row, because all of them change the same thing the address does — what you
   * are looking at and how much of it you can see. A second row of labelled
   * buttons cost the page a strip of height on every screen, including the ones
   * where nobody is marking anything.
   *
   * The tools are a set of toggles rather than a segmented control: each one
   * says in its own tooltip what it does and whether it is on, and pressing the
   * one already on puts the pointer back to plain browsing.
   */
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import Highlighter from '@lucide/svelte/icons/highlighter';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import SquareDashed from '@lucide/svelte/icons/square-dashed';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';

  // `browse` is none armed.
  import type { BrowserPanelTool } from '$lib/shell/browser/browserTypes.ts';

  interface Props {
    address: string;
    tool: BrowserPanelTool;
    canGoBack: boolean;
    canGoForward: boolean;
    expanded: boolean;
    onAddressInput(value: string): void;
    onNavigate(): void;
    onBack(): void;
    onForward(): void;
    onReload(): void;
    onToolChange(tool: BrowserPanelTool): void;
    onToggleExpand(): void;
  }

  let {
    address,
    tool,
    canGoBack,
    canGoForward,
    expanded,
    onAddressInput,
    onNavigate,
    onBack,
    onForward,
    onReload,
    onToolChange,
    onToggleExpand
  }: Props = $props();

  /**
   * What an armed tool looks like: the same tint its own hover draws, left up.
   * Written out in full because Tailwind reads this file as text and a class it
   * never sees written down is a class it never generates.
   */
  const ARMED =
    'bg-[color-mix(in_srgb,var(--color-accent)_16%,var(--color-elevated))] text-[var(--color-accent)]';

  function choose(next: BrowserPanelTool): void {
    onToolChange(next === tool ? 'browse' : next);
  }
</script>

<div class="browser-toolbar" data-testid="browser-toolbar">
  <!-- Bits UI's tooltip state loops when this toolbar is mounted beside the
       native child view. The aria-labels still name every control. -->
  <IconButton label="Go back" size="xs" tooltip={false} disabled={!canGoBack} data-testid="browser-back" onclick={onBack}>
    <ArrowLeft aria-hidden="true" />
  </IconButton>
  <IconButton
    label="Go forward"
    size="xs"
    tooltip={false}
    disabled={!canGoForward}
    data-testid="browser-forward"
    onclick={onForward}
  >
    <ArrowRight aria-hidden="true" />
  </IconButton>
  <IconButton label="Reload the page" size="xs" tooltip={false} data-testid="browser-reload" onclick={onReload}>
    <RotateCw aria-hidden="true" />
  </IconButton>

  <form
    class="address-form"
    onsubmit={(event) => {
      event.preventDefault();
      onNavigate();
    }}
  >
    <Input
      aria-label="Address"
      placeholder="Enter an http, https, or file address"
      value={address}
      data-testid="browser-address"
      oninput={(event) => onAddressInput(event.currentTarget.value)}
    />
  </form>

  <span class="divide" aria-hidden="true"></span>

  <IconButton
    label={tool === 'element' ? 'Stop marking elements' : 'Mark an element on the page'}
    size="xs"
    tooltip={false}
    class={tool === 'element' ? ARMED : undefined}
    data-testid="browser-tool-element"
    onclick={() => choose('element')}
  >
    <MousePointer2 aria-hidden="true" />
  </IconButton>
  <IconButton
    label={tool === 'region' ? 'Stop marking regions' : 'Draw a region on the page'}
    size="xs"
    tooltip={false}
    class={tool === 'region' ? ARMED : undefined}
    data-testid="browser-tool-region"
    onclick={() => choose('region')}
  >
    <SquareDashed aria-hidden="true" />
  </IconButton>
  <IconButton
    label={tool === 'drawing' ? 'Put the marker down' : 'Draw on the page with a marker'}
    size="xs"
    tooltip={false}
    class={tool === 'drawing' ? ARMED : undefined}
    data-testid="browser-tool-marker"
    onclick={() => choose('drawing')}
  >
    <Highlighter aria-hidden="true" />
  </IconButton>

  <span class="divide" aria-hidden="true"></span>

  <IconButton
    label={expanded ? 'Put the browser back in its column' : 'Fill the window with the browser'}
    size="xs"
    tooltip={false}
    data-testid="browser-expand"
    onclick={onToggleExpand}
  >
    {#if expanded}
      <Minimize2 aria-hidden="true" />
    {:else}
      <Maximize2 aria-hidden="true" />
    {/if}
  </IconButton>
</div>

<style>
  .browser-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    border-bottom: 1px solid var(--color-border);
    padding: 8px 12px;
  }

  /* The address box takes whatever the two groups of buttons leave, so it grows
     as the column is widened and the row reads as one line rather than as
     buttons pushed apart. */
  .address-form {
    min-width: 0;
    flex: 1;
  }

  /* Where the row changes subject: from the page to what the pointer does, and
     from that to how wide the panel is. */
  .divide {
    width: 1px;
    height: 16px;
    flex: none;
    margin: 0 2px;
    background: var(--color-border);
  }
</style>
