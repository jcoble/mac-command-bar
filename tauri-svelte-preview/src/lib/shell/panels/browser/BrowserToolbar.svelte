<script lang="ts">
  /**
   * BrowserToolbar.svelte — the address and the four tools.
   *
   * Two lines: where you are, and what the pointer does. All four tools work
   * on a still of the page; Annotate is the one that asks the page what is
   * under the pointer instead of taking the shape by hand. The expand control
   * is here too, beside the address, because it changes the same thing the
   * address does — how much of the page you can see.
   */
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import Eraser from '@lucide/svelte/icons/eraser';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import PenLine from '@lucide/svelte/icons/pen-line';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import SquareDashed from '@lucide/svelte/icons/square-dashed';

  import { IconButton } from '$lib/components/ui/icon-button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { SegmentedControl } from '$lib/components/ui/segmented-control/index.js';
  import type { SegmentedControlItem } from '$lib/components/ui/segmented-control/types.js';

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

  const tools: readonly SegmentedControlItem[] = [
    { value: 'element', label: 'Annotate', icon: MousePointer2 },
    { value: 'region', label: 'Region', icon: SquareDashed },
    { value: 'drawing', label: 'Draw', icon: PenLine },
    { value: 'erasing', label: 'Erase', icon: Eraser }
  ];

  /** With no tool armed the control shows nothing selected rather than lying
   *  about which one is on. */
  const selected = $derived(tool === 'browse' ? '' : tool);

  function choose(next: string): void {
    onToolChange(next === tool ? 'browse' : (next as BrowserPanelTool));
  }
</script>

<div class="browser-toolbar" data-testid="browser-toolbar">
  <div class="address-row">
    <IconButton label="Go back" size="xs" disabled={!canGoBack} data-testid="browser-back" onclick={onBack}>
      <ArrowLeft aria-hidden="true" />
    </IconButton>
    <IconButton
      label="Go forward"
      size="xs"
      disabled={!canGoForward}
      data-testid="browser-forward"
      onclick={onForward}
    >
      <ArrowRight aria-hidden="true" />
    </IconButton>
    <IconButton label="Reload the page" size="xs" data-testid="browser-reload" onclick={onReload}>
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
        placeholder="Enter an http or https address"
        value={address}
        data-testid="browser-address"
        oninput={(event) => onAddressInput(event.currentTarget.value)}
      />
    </form>

    <IconButton
      label={expanded ? 'Shrink the page back into this panel' : 'Widen the page over the center'}
      size="xs"
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

  <div class="tool-row">
    <SegmentedControl
      items={tools}
      value={selected}
      size="sm"
      aria-label="What the pointer does on the page"
      onValueChange={choose}
    />
  </div>
</div>

<style>
  .browser-toolbar {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--color-border);
    padding: 8px 12px;
  }

  .address-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .address-form {
    min-width: 0;
    flex: 1;
  }

  .tool-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
