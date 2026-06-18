<script lang="ts">
  /**
   * WorkbenchControls.svelte — the workbench's floating "View" menu control.
   *
   * Renders the View button plus its dropdown (the layout-preset grid + the
   * "Reset dock layout" action). Presentational: the page owns the preset list,
   * the active-preset value, the open/close state and every action; this
   * component only renders them and emits callbacks. `viewMenuOpen` is bound so
   * the page's button and its Escape keydown handler stay in sync.
   *
   * Renders inline in the workbench shell (no teleport). The wrapping
   * `{#if sourceDockviewWorkbenchEnabled}` gate stays in the page.
   *
   * The shared menu chrome (`.view-menu*`, `.view-menu-anchor`) lives in
   * `src/app.css` as global rules because the editor-topbar View menu (which
   * stays inline in the page) uses the same classes; only the workbench-specific
   * `.workbench-global-controls` / `.workbench-view-menu` modifiers scope here.
   * The base `.topbar-command-button` chrome is restated below (same pattern as
   * ActivityClipboardPanel's `.file-action-button`) so the button this component
   * renders looks identical now that it is in this component's scope rather than
   * the page's; the page keeps its own copy for its many other buttons.
   */
  import { MoreHorizontal } from '@lucide/svelte';

  /** Minimal shape the menu reads (page passes full `SourceLayoutPresetDefinition[]`). */
  interface WorkbenchLayoutPreset {
    id: string;
    label: string;
    title: string;
  }

  interface Props {
    /** Selectable layout presets (= page `sourceLayoutPresets`). */
    presets: readonly WorkbenchLayoutPreset[];
    /** Currently-active preset id (= `dock.layoutPreset`). */
    activePreset: string;
    /** Whether the dropdown is open (page owns `viewMenuOpen` + its Escape handler). */
    viewMenuOpen?: boolean;
    /** Toggle the dropdown (page `toggleViewMenu`). */
    onToggleMenu: () => void;
    /** Apply a layout preset (page `applySourceLayoutPreset`). */
    onApplyPreset: (presetID: string) => void;
    /** Reset the dock layout (page `resetSourceDockLayout`). */
    onResetLayout: () => void;
    /** Close the dropdown (page `closeViewMenu`). */
    onCloseMenu: () => void;
  }

  let {
    presets,
    activePreset,
    viewMenuOpen = $bindable(false),
    onToggleMenu,
    onApplyPreset,
    onResetLayout,
    onCloseMenu,
  }: Props = $props();
</script>

<div class="workbench-global-controls">
  <button
    class="topbar-command-button view-menu-button"
    type="button"
    aria-label="View menu"
    aria-haspopup="menu"
    aria-expanded={viewMenuOpen}
    title="View menu"
    onclick={onToggleMenu}
  >
    <MoreHorizontal size={15} strokeWidth={2} />
    <span>View</span>
  </button>

  {#if viewMenuOpen}
    <div class="view-menu workbench-view-menu" role="menu" aria-label="View options">
      <section class="view-menu-section" aria-label="Workspace layout presets">
        <span>Layout</span>
        <div class="view-menu-button-grid">
          {#each presets as preset (preset.id)}
            <button
              class:active={activePreset === preset.id}
              type="button"
              role="menuitem"
              aria-label={`Use ${preset.label} layout`}
              title={preset.title}
              onclick={() => {
                onApplyPreset(preset.id);
                onCloseMenu();
              }}
            >
              {preset.label}
            </button>
          {/each}
        </div>
        <button
          class="view-menu-wide-button"
          type="button"
          role="menuitem"
          onclick={() => {
            onResetLayout();
            onCloseMenu();
          }}
        >
          Reset dock layout
        </button>
      </section>
    </div>
  {/if}
</div>

<style>
  /*
   * Base `.topbar-command-button` chrome. These rules also live in
   * `+page.svelte`'s `<style>` (shared by many page buttons); they are copied
   * here so this component's own View button renders identically now that it is
   * rendered by this component's scope rather than the page's. (Tracked for a
   * later dedup sweep.) The `.workspace.chrome-compact` ancestor variants use
   * `:global(...)` because `.workspace` is rendered by the page, not here.
   */
  .topbar-command-button {
    display: inline-grid;
    grid-auto-flow: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    color: #aab6b2;
    border: 1px solid rgba(255, 255, 255, 0.105);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.045);
    font-size: 10px;
    font-weight: 820;
    cursor: pointer;
  }

  :global(.workspace.chrome-compact) .topbar-command-button {
    width: 26px;
    height: 24px;
    padding: 0;
    gap: 0;
  }

  :global(.workspace.chrome-compact) .topbar-command-button span {
    display: none;
  }

  .topbar-command-button:hover,
  .topbar-command-button:focus-visible,
  .topbar-command-button[aria-expanded='true'] {
    color: #f2f6f5;
    border-color: rgba(92, 226, 207, 0.34);
    outline: 0;
    background: rgba(92, 226, 207, 0.1);
  }

  .topbar-command-button span {
    min-width: 0;
    white-space: nowrap;
  }

  /*
   * Workbench-only modifiers. The shared `.view-menu*` chrome is global in
   * `src/app.css`; these float the control + lift the dropdown for the
   * workbench shell. Scoped is fine — they target this component's elements.
   */
  .workbench-global-controls {
    position: absolute;
    top: 7px;
    right: 8px;
    z-index: 36;
    display: inline-grid;
    place-items: center;
    min-width: 0;
  }

  .workbench-global-controls > .topbar-command-button {
    height: 25px;
    min-width: 32px;
    background: rgba(18, 20, 21, 0.9);
    backdrop-filter: blur(14px);
  }

  .workbench-view-menu {
    z-index: 37;
  }
</style>
