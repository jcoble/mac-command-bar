<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Action } from 'svelte/action';

  type Props = {
    shellClass: string;
    hostClass: string;
    errorClass: string;
    enabled: boolean;
    ready: boolean;
    error?: string;
    hostAction: Action<HTMLElement>;
    children?: Snippet;
  };

  let {
    shellClass,
    hostClass,
    errorClass,
    enabled,
    ready,
    error = '',
    hostAction,
    children
  }: Props = $props();
</script>

<div
  class={shellClass}
  class:dockview-enabled={enabled}
  class:dockview-ready={ready}
  class:dockview-error={Boolean(error)}
>
  {#if enabled}
    <div class={`${hostClass} source-dockview-host`} aria-hidden={!ready} use:hostAction></div>
  {/if}

  {#if children}
    {@render children()}
  {/if}

  {#if error}
    <div class={errorClass} role="status">
      {error}
    </div>
  {/if}
</div>

<style>
  :global(.source-dockview-workbench-shell) {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: #191a21;
  }

  :global(.source-dockview-workbench-host),
  :global(.source-dockview-workbench-shell > :not(.source-dockview-workbench-error)) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-workbench-host) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-workbench-shell.dockview-ready .source-dockview-workbench-host) {
    visibility: visible;
  }

  :global(.source-dockview-workbench-shell.dockview-ready > :not(.source-dockview-workbench-host):not(.source-dockview-workbench-error)) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-workbench-shell .dockview-theme-dark) {
    --dv-background-color: #191a21;
    --dv-tabs-and-actions-container-background-color: #191a21;
    --dv-activegroup-visiblepanel-tab-background-color: #01131d;
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: #1f2030;
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: transparent;
    --dv-tab-divider-color: transparent;
    --dv-paneview-header-border-color: transparent;
  }

  :global(.source-dockview-activity-shell) {
    position: relative;
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: #191a21;
  }

  :global(.source-dockview-activity-host),
  :global(.source-dockview-activity-shell > .activity-shell) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-activity-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-activity-shell > .activity-shell) {
    width: 100%;
    height: 100%;
  }

  :global(.source-dockview-activity-shell.dockview-ready .source-dockview-activity-host) {
    visibility: visible;
  }

  :global(.source-dockview-activity-shell.dockview-ready > .activity-shell) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-activity-shell .dockview-theme-dark) {
    --dv-background-color: rgba(15, 18, 18, 0.92);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-center-shell) {
    position: relative;
    display: grid;
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-dockview-center-host),
  :global(.source-dockview-center-shell > .source-editor-dock-panel) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-center-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-center-shell > .source-editor-dock-panel) {
    width: 100%;
    height: 100%;
  }

  :global(.source-dockview-center-shell.dockview-ready .source-dockview-center-host) {
    visibility: visible;
  }

  :global(.source-dockview-center-shell.dockview-ready > .source-editor-dock-panel) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-center-shell .dockview-theme-dark) {
    --dv-background-color: rgba(15, 18, 18, 0.92);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-editor-files-shell) {
    position: relative;
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-dockview-editor-files-host),
  :global(.source-dockview-editor-files-shell > .source-editor-file-pane) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-editor-files-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-editor-files-shell.dockview-ready .source-dockview-editor-files-host) {
    visibility: visible;
  }

  :global(.source-dockview-editor-files-shell.dockview-ready > .source-editor-file-pane) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-editor-files-shell .source-dockview-attached-panel.source-editor-file-pane) {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-dockview-editor-files-shell .dockview-theme-dark) {
    --dv-background-color: rgba(15, 18, 18, 0.92);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-bottom-shell) {
    position: relative;
    display: grid;
    flex: 0 0 auto;
    height: var(--bottom-dock-height);
    min-width: 0;
    min-height: 96px;
    max-height: none;
    margin-top: 6px;
    overflow: hidden;
  }

  :global(.source-dockview-bottom-shell.runtime-parking-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    min-height: 0;
    margin: 0;
    opacity: 0;
    pointer-events: none;
  }

  :global(.source-dockview-bottom-host),
  :global(.source-dockview-bottom-shell > .terminal-launchpad),
  :global(.source-dockview-bottom-shell > .browser-dock) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-bottom-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-bottom-shell > .terminal-launchpad),
  :global(.source-dockview-bottom-shell > .browser-dock) {
    height: 100%;
    max-height: none;
    margin-top: 0;
  }

  :global(.source-dockview-bottom-shell.dockview-ready .source-dockview-bottom-host) {
    visibility: visible;
  }

  :global(.source-dockview-bottom-shell.dockview-ready > .terminal-launchpad),
  :global(.source-dockview-bottom-shell.dockview-ready > .browser-dock) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-bottom-shell .source-dockview-panel-host),
  :global(.source-dockview-workbench-shell .source-dockview-panel-host),
  :global(.source-dockview-center-shell .source-dockview-panel-host),
  :global(.source-dockview-activity-shell .source-dockview-panel-host),
  :global(.source-dockview-files-shell .source-dockview-panel-host),
  :global(.source-dockview-conversation-shell .source-dockview-panel-host),
  :global(.source-dockview-context-shell .source-dockview-panel-host),
  :global(.source-dockview-insights-shell .source-dockview-panel-host),
  :global(.source-paneview-panel-host) {
    display: flex;
    align-items: stretch;
    min-width: 0;
    min-height: 0;
    width: 100%;
    height: 100%;
  }

  :global(.source-dockview-host),
  :global(.source-dockview-host .source-dockview-core),
  :global(.source-dockview-host .dv-dockview),
  :global(.source-dockview-host .dv-split-view-container),
  :global(.source-dockview-host .dv-view-container),
  :global(.source-dockview-host .dv-view),
  :global(.source-dockview-host .dv-groupview),
  :global(.source-dockview-host .dv-content-container),
  :global(.source-paneview-host),
  :global(.source-paneview-host .source-paneview-core),
  :global(.source-paneview-host .dv-pane-container),
  :global(.source-paneview-host .dv-split-view-container),
  :global(.source-paneview-host .dv-view-container),
  :global(.source-paneview-host .dv-view),
  :global(.source-paneview-host .dv-pane) {
    min-width: 0;
    min-height: 0;
    width: 100%;
    height: 100%;
  }

  :global(.source-paneview-host .dv-pane .dv-pane-body) {
    flex: 1 1 auto;
    width: auto;
    height: auto;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: transparent;
  }

  :global(.source-dockview-center-shell .source-dockview-panel-host) {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    align-items: stretch;
    justify-items: stretch;
  }

  :global(.source-dockview-workbench-shell .source-dockview-panel-host) {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    align-items: stretch;
    justify-items: stretch;
  }

  :global(.source-dockview-workbench-shell .source-dockview-attached-panel) {
    grid-area: 1 / 1;
    align-self: stretch;
    justify-self: stretch;
  }

  :global(.source-dockview-center-shell .source-dockview-attached-panel) {
    grid-area: 1 / 1;
    align-self: stretch;
    justify-self: stretch;
  }

  :global(.source-dockview-bottom-shell .source-dockview-attached-panel),
  :global(.source-dockview-workbench-shell .source-dockview-attached-panel),
  :global(.source-dockview-center-shell .source-dockview-attached-panel),
  :global(.source-dockview-activity-shell .source-dockview-attached-panel),
  :global(.source-dockview-files-shell .source-dockview-attached-panel),
  :global(.source-dockview-conversation-shell .source-dockview-attached-panel),
  :global(.source-dockview-context-shell .source-dockview-attached-panel),
  :global(.source-dockview-insights-shell .source-dockview-attached-panel),
  :global(.source-paneview-attached-panel) {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-paneview-host .dv-pane-header) {
    color: #dffdf8;
    border-color: rgba(255, 255, 255, 0.055);
    background: rgba(31, 32, 42, 0.96);
  }

  :global(.source-paneview-host .dv-default-header) {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    height: 100%;
    padding: 0 8px;
    overflow: hidden;
    font-size: 12px;
    font-weight: 780;
  }

  :global(.source-paneview-host .dv-default-header span) {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.source-paneview-host .dv-pane-header-icon) {
    display: grid;
    place-items: center;
    flex: 0 0 14px;
    width: 14px;
    height: 14px;
    color: #dffdf8;
  }

  :global(.source-paneview-host .dv-pane-body) {
    overflow: hidden;
    background: transparent;
  }

  :global(.source-paneview-host .dv-sash) {
    --dv-sash-color: transparent;
    --dv-active-sash-color: rgba(92, 226, 207, 0.36);
  }

  :global(.source-paneview-host .dv-sash.dv-enabled:hover) {
    background: rgba(92, 226, 207, 0.12);
  }

  :global(.source-dockview-bottom-shell .dockview-theme-dark) {
    --dv-background-color: rgba(15, 18, 18, 0.92);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-files-shell) {
    position: relative;
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-dockview-files-host),
  :global(.source-dockview-files-shell > .source-files-pane) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-files-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-files-shell.dockview-ready .source-dockview-files-host) {
    visibility: visible;
  }

  :global(.source-dockview-files-shell.dockview-ready > .source-files-pane) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-files-shell .dockview-theme-dark) {
    --dv-background-color: rgba(15, 18, 18, 0.92);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-conversation-shell) {
    position: relative;
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-dockview-conversation-host),
  :global(.source-dockview-conversation-shell > .conversation-dockview-pane) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-conversation-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-conversation-shell.dockview-ready .source-dockview-conversation-host) {
    visibility: visible;
  }

  :global(.source-dockview-conversation-shell.dockview-ready > .conversation-dockview-pane) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-conversation-shell .source-dockview-attached-panel.conversation-active-pane) {
    display: grid;
    align-content: start;
    grid-auto-flow: row;
    grid-auto-rows: max-content;
    gap: 7px;
    overflow-x: hidden;
    overflow-y: auto;
  }

  :global(.source-dockview-conversation-shell .dockview-theme-dark) {
    --dv-background-color: rgba(15, 18, 18, 0.92);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-context-shell) {
    position: relative;
    display: grid;
    min-width: 0;
    min-height: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: #191a21;
  }

  :global(.source-dockview-context-host),
  :global(.source-dockview-context-shell > .context-panel-grid),
  :global(.source-dockview-context-shell > .source-intelligence-panel) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-context-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-context-shell.dockview-ready .source-dockview-context-host) {
    visibility: visible;
  }

  :global(.source-dockview-context-shell.dockview-ready > .context-panel-grid),
  :global(.source-dockview-context-shell.dockview-ready > .source-intelligence-panel) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-context-shell .dockview-theme-dark) {
    --dv-background-color: rgba(20, 23, 24, 0.86);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-context-shell .context-panel-grid) {
    grid-template-columns: 28px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    align-content: stretch;
    height: 100%;
    margin: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  :global(.source-dockview-workbench-shell .source-dockview-attached-panel.context-panel-grid) {
    grid-template-columns: 28px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    align-content: stretch;
    height: 100%;
    margin: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding-right: 2px;
    scrollbar-color: rgba(174, 184, 181, 0.5) rgba(255, 255, 255, 0.045);
    scrollbar-gutter: stable;
    scrollbar-width: thin;
  }

  :global(.source-dockview-workbench-shell .source-dockview-attached-panel.context-panel-grid.paneview-card-stack),
  :global(.source-dockview-context-shell .context-panel-grid.paneview-card-stack) {
    display: flex;
    flex-direction: column;
    grid-template-columns: none;
    grid-template-rows: none;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    margin: 0;
    padding-right: 0;
    overflow: hidden;
    scrollbar-gutter: auto;
  }

  :global(.source-dockview-workbench-shell .source-dockview-attached-panel.context-panel-grid.paneview-card-stack > .source-paneview-context-cards-shell),
  :global(.source-dockview-context-shell .context-panel-grid.paneview-card-stack > .source-paneview-context-cards-shell) {
    flex: 1 1 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-workbench-shell .source-paneview-context-cards-shell),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host .source-paneview-core),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host .dv-pane-container),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host .dv-view-container),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host .dv-view),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host .dv-pane),
  :global(.source-dockview-workbench-shell .source-paneview-context-cards-host .dv-pane-body),
  :global(.source-dockview-context-shell .source-paneview-context-cards-shell),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host .source-paneview-core),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host .dv-pane-container),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host .dv-view-container),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host .dv-view),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host .dv-pane),
  :global(.source-dockview-context-shell .source-paneview-context-cards-host .dv-pane-body) {
    width: 100%;
    max-width: none;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  :global(.source-dockview-workbench-shell .context-restore-button) {
    grid-column: 1 / -1;
    grid-row: 1;
  }

  :global(.source-dockview-workbench-shell .context-stack-tabs) {
    grid-column: 1;
    grid-row: 2;
    flex-direction: column;
    align-items: center;
    width: 28px;
    max-width: 28px;
    height: 100%;
    max-height: 100%;
    overflow-x: hidden;
    overflow-y: auto;
    border-radius: 6px;
  }

  :global(.source-dockview-workbench-shell .context-stack-tabs button) {
    display: grid;
    place-items: center;
    width: 22px;
    min-width: 22px;
    height: 22px;
    padding: 0;
  }

  :global(.source-dockview-workbench-shell .context-stack-tabs button span) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  :global(.source-dockview-workbench-shell .context-card-tab-icon) {
    display: block;
  }

  :global(.source-dockview-workbench-shell .orchestration-context-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-workbench-shell .runtime-context-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-workbench-shell .agent-session-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-workbench-shell .worktree-context-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-workbench-shell .repo-dashboard-panel:not(.source-paneview-attached-panel)) {
    grid-column: 2;
    grid-row: 2;
    gap: 4px;
    padding: 5px;
    overflow: hidden;
    border-radius: 5px;
  }

  :global(.source-dockview-context-shell .context-restore-button) {
    grid-column: 1 / -1;
    grid-row: 1;
  }

  :global(.source-dockview-context-shell .context-stack-tabs) {
    grid-column: 1;
    grid-row: 2;
    flex-direction: column;
    align-items: center;
    width: 28px;
    max-width: 28px;
    height: 100%;
    max-height: 100%;
    overflow-x: hidden;
    overflow-y: auto;
    border-radius: 6px;
  }

  :global(.source-dockview-context-shell .context-stack-tabs button) {
    display: grid;
    place-items: center;
    width: 22px;
    min-width: 22px;
    height: 22px;
    padding: 0;
  }

  :global(.source-dockview-context-shell .context-stack-tabs button span) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  :global(.source-dockview-context-shell .context-card-tab-icon) {
    display: block;
  }

  :global(.source-dockview-context-shell .orchestration-context-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-context-shell .runtime-context-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-context-shell .agent-session-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-context-shell .worktree-context-panel:not(.source-paneview-attached-panel)),
  :global(.source-dockview-context-shell .repo-dashboard-panel:not(.source-paneview-attached-panel)) {
    grid-column: 2;
    grid-row: 2;
    gap: 4px;
    padding: 5px;
    overflow: hidden;
    border-radius: 5px;
  }

  :global(.source-dockview-insights-shell) {
    position: relative;
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: #191a21;
  }

  :global(.source-dockview-insights-parking) {
    position: fixed;
    top: 0;
    left: 0;
    width: 1px;
    height: 1px;
    overflow: hidden;
    visibility: hidden;
    pointer-events: none;
    contain: layout paint;
  }

  :global(.source-dockview-insights-host),
  :global(.source-dockview-insights-shell > .source-intelligence-panel) {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  :global(.source-dockview-insights-host) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    visibility: hidden;
    contain: layout paint;
  }

  :global(.source-dockview-insights-shell.dockview-ready .source-dockview-insights-host) {
    visibility: visible;
  }

  :global(.source-dockview-insights-shell.dockview-ready > .source-intelligence-panel) {
    visibility: hidden;
    pointer-events: none;
  }

  :global(.source-dockview-insights-shell .dockview-theme-dark) {
    --dv-background-color: rgba(20, 23, 24, 0.86);
    --dv-tabs-and-actions-container-background-color: rgba(18, 20, 21, 0.94);
    --dv-activegroup-visiblepanel-tab-background-color: rgba(92, 226, 207, 0.12);
    --dv-activegroup-visiblepanel-tab-color: #dffdf8;
    --dv-inactivegroup-visiblepanel-tab-background-color: rgba(255, 255, 255, 0.045);
    --dv-inactivegroup-visiblepanel-tab-color: #aab6b2;
    --dv-separator-border: rgba(255, 255, 255, 0.08);
  }

  :global(.source-dockview-activity-shell .dockview-theme-dark),
  :global(.source-dockview-context-shell .dockview-theme-dark),
  :global(.source-dockview-insights-shell .dockview-theme-dark) {
    --dv-background-color: #191a21;
    --dv-tabs-and-actions-container-background-color: #191a21;
    --dv-activegroup-visiblepanel-tab-background-color: #01131d;
    --dv-inactivegroup-visiblepanel-tab-background-color: #1f2030;
    --dv-separator-border: transparent;
    --dv-tab-divider-color: transparent;
    --dv-paneview-header-border-color: transparent;
  }

  :global(.source-dockview-host.dockview-theme-dark),
  :global(.source-dockview-host .dockview-theme-dark) {
    --dv-separator-border: transparent;
    --dv-tab-divider-color: transparent;
    --dv-paneview-header-border-color: transparent;
  }

  :global(.source-dockview-host .dv-groupview),
  :global(.source-dockview-host .dv-tabs-and-actions-container),
  :global(.source-dockview-host .dv-content-container) {
    border-color: transparent;
    box-shadow: none;
  }

  :global(.source-dockview-activity-shell .dv-groupview),
  :global(.source-dockview-activity-shell .dv-tabs-and-actions-container),
  :global(.source-dockview-activity-shell .dv-content-container),
  :global(.source-dockview-context-shell .dv-groupview),
  :global(.source-dockview-context-shell .dv-tabs-and-actions-container),
  :global(.source-dockview-context-shell .dv-content-container),
  :global(.source-dockview-insights-shell .dv-groupview),
  :global(.source-dockview-insights-shell .dv-tabs-and-actions-container),
  :global(.source-dockview-insights-shell .dv-content-container) {
    border-color: transparent;
    box-shadow: none;
  }

  :global(.source-dockview-activity-error),
  :global(.source-dockview-bottom-error),
  :global(.source-dockview-context-error),
  :global(.source-dockview-insights-error) {
    position: absolute;
    right: 8px;
    bottom: 8px;
    z-index: 4;
    max-width: calc(100% - 16px);
    overflow: hidden;
    color: #ffb3a6;
    border: 1px solid rgba(255, 123, 107, 0.32);
    border-radius: 6px;
    background: rgba(42, 21, 19, 0.92);
    padding: 4px 6px;
    font-size: 10px;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
