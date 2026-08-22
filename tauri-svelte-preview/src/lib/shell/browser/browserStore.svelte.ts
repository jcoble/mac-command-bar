/**
 * Browser store compatibility facade.
 *
 * The old /next callers still receive `browser`, `activateBrowser`,
 * `setBrowserUrl`, `reloadBrowserFrame`, `captureBrowserState` and
 * `restoreBrowserState`.  New surfaces use the same reactive workspace and
 * the pure browser model underneath. Durable restoration belongs to the active
 * session's SQLite workspace snapshot, not browser storage.
 */
import type { SessionBrowserWorkspace } from '../sessionWorkspaces.ts';
import {
  saveConversationClipboardImage
} from '../conversation/conversationService.ts';
import {
  getConversationSession,
  setConversationAttachments,
  setConversationDraft
} from '../conversation/conversationStore.svelte.ts';
import {
  activateBrowserWorkspace,
  BrowserModelError,
  captureBrowserWorkspace,
  closeBrowserTab,
  createBrowserTab,
  deactivateBrowserWorkspace as deactivateBrowserWorkspaceModel,
  navigateActiveBrowserTab,
  selectBrowserTab,
  setBrowserPresentationMode,
  setBrowserViewport,
  stageBrowserFeedbackPreview,
  type BrowserCaptureOptions,
  type BrowserFeedbackStageOptions
} from './browserModel.ts';
import {
  createBrowserBackend,
  type BrowserBackend
} from './browserBackend.ts';
import {
  createBrowserWorkspace,
  type BrowserConversationAttachment,
  type BrowserConversationBridge,
  type BrowserFeedbackPreview,
  type BrowserModelContext,
  type BrowserPresentationMode,
  type BrowserTabNavigationEvent,
  type BrowserViewportPreset,
  type BrowserWorkspaceState
} from './browserTypes.ts';
import { normalizeBrowserUrl } from './normalizeBrowserUrl.ts';

export const INVALID_URL_MESSAGE = 'Enter an address that starts with http or https';

export interface BrowserCompatibilityState {
  url: string;
  inputUrl: string;
  frameKey: number;
  error: string;
  activated: boolean;
  workspace: BrowserWorkspaceState;
  backend: BrowserBackend;
}

const workspace = createBrowserWorkspace({ workspaceId: 'next-browser' });
const backend = createBrowserBackend();

const conversationBridge = {
  read(ownedId: string) {
    const current = getConversationSession(ownedId);
    if (!current) return null;
    return {
      ownedId,
      generation: current.generation,
      draft: current.draft,
      attachments: current.attachments.map((attachment) => ({ ...attachment }))
    };
  },
  setDraft(ownedId: string, draft: string): void {
    setConversationDraft(ownedId, draft);
  },
  setAttachments(ownedId: string, attachments: BrowserConversationAttachment[]): void {
    setConversationAttachments(ownedId, attachments);
  },
  async saveAttachment(input: { ownedId: string; name: string; mimeType: string; bytes: readonly number[] }) {
    const file = new File([Uint8Array.from(input.bytes)], input.name, { type: input.mimeType });
    return saveConversationClipboardImage(input.ownedId, file);
  }
};

export const browser = $state<BrowserCompatibilityState>({
  url: '',
  inputUrl: '',
  frameKey: 0,
  error: '',
  activated: false,
  workspace,
  backend
});

/** New browser surfaces can consume the model state directly. */
export const browserWorkspace = browser.workspace;
export const browserBackend = browser.backend;

function modelContext(): BrowserModelContext {
  return { workspace: browser.workspace, backend: browser.backend };
}

function syncLegacy(nextUrl?: string, bumpFrame = false): void {
  const active = browser.workspace.activeTabId
    ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null
    : null;
  const url = nextUrl ?? active?.url ?? '';
  if (bumpFrame && browser.url === url && url) browser.frameKey += 1;
  else if (bumpFrame && browser.url !== url) browser.frameKey += 1;
  browser.url = url;
  browser.inputUrl = active?.inputUrl ?? url;
  browser.activated = browser.workspace.activated;
  browser.error = browser.workspace.error ?? '';
}

export function captureBrowserState(): SessionBrowserWorkspace {
  return {
    url: browser.url,
    inputUrl: browser.inputUrl,
    activated: browser.activated
  };
}

export function restoreBrowserState(snapshot: SessionBrowserWorkspace | null | undefined): void {
  const nextUrl = normalizeBrowserUrl(snapshot?.url ?? '');
  browser.workspace.activated = snapshot?.activated === true && nextUrl.length > 0;
  browser.workspace.error = null;
  const current = browser.workspace.activeTabId
    ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null
    : null;
  try {
    if (nextUrl && !current) {
      // Creating a native tab here would surface its webview over the shell
      // before the browser panel is open; persist the url instead and let
      // activateBrowser() create the tab when the panel is actually shown.
      browser.workspace.activated = false;
    } else if (nextUrl && current && current.url !== nextUrl) {
      navigateActiveBrowserTab(modelContext(), nextUrl);
    }
    if (!nextUrl && current) {
      closeBrowserTab(modelContext(), current.id);
    }
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
  }
  if (snapshot?.inputUrl && browser.workspace.activeTabId) {
    const active = browser.workspace.tabs[browser.workspace.activeTabId];
    if (active) active.inputUrl = snapshot.inputUrl;
  }
  if (snapshot?.activated !== true) browser.workspace.activated = false;
  syncLegacy(nextUrl);
}

export function activateBrowser(): void {
  if (browser.workspace.activated) return;
  const savedUrl = normalizeBrowserUrl(browser.url);
  try {
    activateBrowserWorkspace(modelContext());
    if (savedUrl && !browser.workspace.activeTabId) {
      createBrowserTab(modelContext(), { url: savedUrl });
    }
    syncLegacy();
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
    syncLegacy();
  }
}

export function deactivateBrowserWorkspace(): void {
  try {
    deactivateBrowserWorkspaceModel(modelContext());
    syncLegacy();
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
    syncLegacy();
  }
}

/** Release native browser views while keeping only the compact address metadata. */
export function releaseBrowserWorkspace(): void {
  const url = browser.url;
  const inputUrl = browser.inputUrl;
  for (const tabId of Object.keys(browser.workspace.tabs)) {
    closeBrowserTab(modelContext(), tabId);
  }
  browser.workspace.tabs = {};
  browser.workspace.tabOrder = [];
  browser.workspace.activeTabId = null;
  browser.workspace.activeGeneration = 0;
  browser.workspace.activated = false;
  browser.workspace.interaction = 'browse';
  browser.workspace.pendingSelection = null;
  browser.workspace.pendingSelectionKind = null;
  browser.workspace.pendingMarkup = null;
  browser.workspace.queue = [];
  browser.workspace.markupCaptures = {};
  browser.workspace.error = null;
  browser.url = url;
  browser.inputUrl = inputUrl;
  browser.activated = false;
  browser.error = '';
}

export function setBrowserUrl(value: string): boolean {
  try {
    activateBrowser();
    const active = browser.workspace.activeTabId
      ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null
      : null;
    if (!active) createBrowserTab(modelContext(), { url: value });
    else navigateActiveBrowserTab(modelContext(), value);
    browser.workspace.error = null;
    syncLegacy(undefined, true);
    return true;
  } catch (error) {
    browser.error = error instanceof BrowserModelError ? error.message : INVALID_URL_MESSAGE;
    browser.workspace.error = browser.error;
    return false;
  }
}

export function openBrowserUrl(url: string): boolean {
  return setBrowserUrl(url);
}

export function reloadBrowserFrame(): void {
  const active = browser.workspace.activeTabId
    ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null
    : null;
  if (!active) return;
  try {
    browser.backend.reload_browser_tab({
      workspaceId: active.workspaceId,
      tabId: active.id,
      generation: active.generation
    });
    browser.frameKey += 1;
    browser.workspace.error = null;
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
  }
  syncLegacy();
}

export function clearBrowserUrl(): void {
  const active = browser.workspace.activeTabId
    ? browser.workspace.tabs[browser.workspace.activeTabId] ?? null
    : null;
  if (active) closeBrowserTab(modelContext(), active.id);
  browser.workspace.error = null;
  browser.url = '';
  browser.inputUrl = '';
  browser.activated = browser.workspace.activated;
}

export function clearBrowserError(): void {
  browser.error = '';
  browser.workspace.error = null;
}

// Small explicit adapters used by the new browser components.  They keep the
// compatibility store as the one reactive source without adding a second model.
export function setBrowserPresentation(mode: BrowserPresentationMode): void {
  try {
    setBrowserPresentationMode(modelContext(), mode);
    syncLegacy();
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
    syncLegacy();
  }
}

export function setBrowserViewportPreset(value: BrowserViewportPreset): void {
  try {
    setBrowserViewport(modelContext(), value);
    syncLegacy();
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
  }
}

export function captureBrowserView(options?: BrowserCaptureOptions): ReturnType<typeof captureBrowserWorkspace> {
  return captureBrowserWorkspace(modelContext(), options);
}

export function stageBrowserFeedback(options?: BrowserFeedbackStageOptions): BrowserFeedbackPreview {
  return stageBrowserFeedbackPreview(browserModelContext(options?.bridge), options);
}

export function browserModelContext(conversation?: BrowserConversationBridge): BrowserModelContext {
  return { ...modelContext(), conversation: conversation ?? conversationBridge };
}

export function syncBrowserTab(tabId: string): void {
  try {
    selectBrowserTab(modelContext(), tabId);
    syncLegacy();
  } catch (error) {
    browser.workspace.error = error instanceof Error ? error.message : String(error);
  }
}

export function syncBrowserNavigation(event: BrowserTabNavigationEvent): void {
  const tab = browser.workspace.tabs[event.tabId];
  if (
    !tab
    || event.workspaceId !== browser.workspace.workspaceId
    || event.generation !== tab.generation
  ) return;
  tab.url = event.url;
  tab.inputUrl = event.url;
  tab.title = event.title;
  tab.canGoBack = event.canGoBack;
  tab.canGoForward = event.canGoForward;
  tab.loadState = 'loaded';
  syncLegacy();
}
