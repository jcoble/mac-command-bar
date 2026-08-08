<script lang="ts">
  /**
   * ShellOverlays.svelte — everything that floats above the /next shell rather
   * than living in a region: the command palette, the settings dialog, the
   * message strip, and the development-only backend call counter.
   *
   * No backend call and no state of its own beyond the settings handle. The
   * palette is mounted exactly once (two of them would answer the same keyboard
   * shortcut and would fight over the seeded actions), which is why it lives
   * here and not inside a panel.
   */
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  import NewSessionHost from './newSession/NewSessionHost.svelte';
  import PalettePanel from './PalettePanel.svelte';
  import SettingsHost from './SettingsHost.svelte';
  import BrowserOverlayHost from './browser/BrowserOverlayHost.svelte';
  import WorkbenchActionFab from './WorkbenchActionFab.svelte';
  import AssistanceHost from '$lib/shell/assistance/AssistanceHost.svelte';
  import ResourcePopover from '$lib/shell/resources/ResourcePopover.svelte';
  import UsagePopover from '$lib/shell/usage/UsagePopover.svelte';
  import { invokeCounts } from '$lib/shell/devInvokeCounter.svelte';
  import type {
    BrowserFeedbackAttachment,
    BrowserPresentationMode,
    BrowserViewportPreset,
    BrowserWorkspaceState
  } from '$lib/shell/browser/browserTypes.ts';
  import type { WorkbenchAction, WorkbenchActionContext } from '$lib/shell/overlay/actionSurfaceModel.ts';
  import type { ProblemsLocation } from '$lib/settingsStore.svelte';
  import type { NewSessionRequest } from '$lib/shell/newSession/newSessionFlow';

  export type BrowserMarkupTool =
    | 'pen'
    | 'highlighter'
    | 'arrow'
    | 'rectangle'
    | 'text'
    | 'undo'
    | 'clear'
    | 'crop';

  /** All browser overlay callbacks are kept together so the actions array
   * cannot be mistaken for a callback object. */
  export interface BrowserOverlayHandlers {
    onExpand?: () => void;
    onSelectTab?: (id: string) => void;
    onCloseTab?: (id: string) => void;
    onCreateTab?: () => void;
    onAddressInput?: (value: string) => void;
    onNavigate?: (value: string) => void;
    onReload?: () => void;
    onBack?: () => void;
    onForward?: () => void;
    onGrab?: () => void;
    onAnnotate?: () => void;
    onDraw?: () => void;
    onOpenDevtools?: () => void;
    onOpenExternal?: () => void;
    onViewport?: (preset: BrowserViewportPreset) => void;
    onPresentation?: (mode: BrowserPresentationMode) => void;
    onCollapse?: () => void;
    onCancelFeedback?: () => void;
    onMarkupTool?: (tool: BrowserMarkupTool) => void;
    onRemoveFeedback?: (id: string) => void;
    onCopyFeedback?: (attachment: BrowserFeedbackAttachment) => void;
    onStageFeedback?: (attachment: BrowserFeedbackAttachment) => void;
    onMinimize?: () => void;
  }

  interface Props {
    /** Put every panel back where it started. */
    onResetLayout: () => void;
    /** Look for agent sessions again. */
    onRescanSessions: () => void | Promise<void>;
    /** Start the session the new-session dialog described. */
    onStartNewSession: (request: NewSessionRequest) => void | Promise<void>;
    /** The folders the sessions on the rail are running in, so the project
     * picker knows about projects nobody added by hand. */
    newSessionRoots: string[];
    /** One line describing whatever has gone wrong, or null when all is well. */
    message: string | null;
    /** The user moved the Problems list from the settings dialog, which lives
     * here; the page is what opens or closes the strip along the bottom. */
    onProblemsLocationChange?: (location: ProblemsLocation) => void;
    browserWorkspace: BrowserWorkspaceState;
    browserActions: WorkbenchAction[];
    workbenchActionContext: WorkbenchActionContext;
    browserOverlayHandlers: BrowserOverlayHandlers;
  }
  let {
    onResetLayout,
    onRescanSessions,
    onStartNewSession,
    newSessionRoots,
    message,
    onProblemsLocationChange,
    browserWorkspace,
    browserActions,
    workbenchActionContext,
    browserOverlayHandlers
  }: Props = $props();

  let settingsHost: { open: () => void; close: () => void } | null = null;
  let newSessionHost: { open: (input?: { sessionRoots?: string[] }) => void } | null = null;

  /** Open the settings dialog from outside — the gear on the activity bar is
   * over in the left column, and the dialog lives here. Same shape as
   * `SettingsHost`'s own `open()`, one layer out. */
  export function openSettings(): void {
    settingsHost?.open();
  }

  /** Open the new-session dialog from outside. Both ways in reach the same
   * instance: the "New session" button in the sessions column, and the palette
   * command the page registers. */
  export function openNewSession(): void {
    newSessionHost?.open({ sessionRoots: newSessionRoots });
  }
</script>

<PalettePanel
  {onResetLayout}
  {onRescanSessions}
  onOpenSettings={() => settingsHost?.open()}
/>
<SettingsHost bind:this={settingsHost} {onProblemsLocationChange} />
<NewSessionHost bind:this={newSessionHost} onStart={onStartNewSession} />
<BrowserOverlayHost
  workspace={browserWorkspace}
  onExpand={browserOverlayHandlers.onExpand}
  onSelectTab={browserOverlayHandlers.onSelectTab}
  onCloseTab={browserOverlayHandlers.onCloseTab}
  onCreateTab={browserOverlayHandlers.onCreateTab}
  onAddressInput={browserOverlayHandlers.onAddressInput}
  onNavigate={browserOverlayHandlers.onNavigate}
  onReload={browserOverlayHandlers.onReload}
  onBack={browserOverlayHandlers.onBack}
  onForward={browserOverlayHandlers.onForward}
  onGrab={browserOverlayHandlers.onGrab}
  onAnnotate={browserOverlayHandlers.onAnnotate}
  onDraw={browserOverlayHandlers.onDraw}
  onOpenDevtools={browserOverlayHandlers.onOpenDevtools}
  onOpenExternal={browserOverlayHandlers.onOpenExternal}
  onViewport={browserOverlayHandlers.onViewport}
  onPresentation={browserOverlayHandlers.onPresentation}
  onCollapse={browserOverlayHandlers.onCollapse}
  onCancelFeedback={browserOverlayHandlers.onCancelFeedback}
  onMarkupTool={browserOverlayHandlers.onMarkupTool}
  onRemoveFeedback={browserOverlayHandlers.onRemoveFeedback}
  onCopyFeedback={browserOverlayHandlers.onCopyFeedback}
  onStageFeedback={browserOverlayHandlers.onStageFeedback}
  onMinimize={browserOverlayHandlers.onMinimize}
  devtoolsAvailable={false}
/>
<WorkbenchActionFab actions={browserActions} context={workbenchActionContext} />
<AssistanceHost />
<div class="a10-popovers" data-testid="a10-resource-usage-popovers">
  <ResourcePopover />
  <UsagePopover />
</div>

{#if message}
  <!-- Something went wrong, said once, along the bottom edge. Announced to
       screen readers, and see-through to the mouse so it can never swallow a
       click meant for the shell underneath. -->
  <footer
    role="alert"
    class="pointer-events-none absolute bottom-2 left-1/2 flex -translate-x-1/2 items-center
           gap-1.5 rounded-md border border-destructive/40 bg-destructive/10 px-2.5 py-1
           text-[13px] leading-[1.4] text-destructive"
  >
    <TriangleAlert class="size-3.5 shrink-0" aria-hidden="true" />
    <span>{message}</span>
  </footer>
{/if}

{#if import.meta.env.DEV}
  <footer class="invoke-counter">invokes: {invokeCounts.total} (+{invokeCounts.input} input)</footer>
{/if}

<style>
  .a10-popovers { position: fixed; top: 12px; right: 14px; z-index: 60; display: flex; gap: 0.5rem; pointer-events: auto; }

  /* Development-only readout of how many backend calls the shell has made.
     Deliberately not part of the shared component set: it is a debugging
     instrument, not chrome, and it never ships to a user. */
  .invoke-counter {
    position: absolute;
    bottom: 8px;
    right: 10px;
    border-radius: 5px;
    background: rgba(16, 16, 20, 0.82);
    color: #6d6d7d;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 12px;
    padding: 3px 8px;
    pointer-events: none;
  }
</style>
