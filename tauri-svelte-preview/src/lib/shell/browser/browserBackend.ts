import type {
  BrowserElementMetadata,
  BrowserFloatingBounds,
  BrowserMarkupCapture,
  BrowserRect,
  BrowserViewport
} from './browserTypes.ts';
import { invokeBrowserCommandFromTauri, isTauriRuntime } from '../../tauriSource.ts';

export type BrowserBackendResult<T> = T | Promise<T>;

export interface BrowserBackendTabInput {
  workspaceId: string;
  tabId: string;
  generation: number;
  url: string;
  bounds?: BrowserFloatingBounds;
  viewport?: BrowserViewport;
  profileId?: string | null;
}

export interface BrowserBackendTabResult {
  tabId: string;
  generation: number;
}

export interface BrowserBackendNavigationInput {
  workspaceId: string;
  tabId: string;
  generation: number;
  url: string;
}

export interface BrowserBackendTarget {
  workspaceId: string;
  tabId: string;
  generation: number;
}

export interface BrowserBackendBoundsInput extends BrowserBackendTarget {
  bounds: BrowserFloatingBounds;
}

export interface BrowserBackendViewportInput extends BrowserBackendTarget {
  viewport: BrowserViewport;
}

export interface BrowserBackendPickerInput extends BrowserBackendTarget {
  mode: 'grab' | 'annotation';
}

export interface BrowserBackendRectInput extends BrowserBackendTarget {
  rect: BrowserRect;
}

export interface BrowserBackend {
  create_browser_tab(input: BrowserBackendTabInput): BrowserBackendResult<BrowserBackendTabResult>;
  set_browser_tab_bounds(input: BrowserBackendBoundsInput): BrowserBackendResult<void>;
  set_browser_tab_viewport?(input: BrowserBackendViewportInput): BrowserBackendResult<void>;
  show_browser_tab(input: BrowserBackendTarget): BrowserBackendResult<void>;
  hide_browser_workspace(input: { workspaceId: string }): BrowserBackendResult<void>;
  navigate_browser_tab(input: BrowserBackendNavigationInput): BrowserBackendResult<void>;
  reload_browser_tab(input: BrowserBackendTarget): BrowserBackendResult<void>;
  go_back_browser_tab(input: BrowserBackendTarget): BrowserBackendResult<void>;
  go_forward_browser_tab(input: BrowserBackendTarget): BrowserBackendResult<void>;
  close_browser_tab(input: BrowserBackendTarget): BrowserBackendResult<void>;
  arm_browser_element_picker(input: BrowserBackendPickerInput): BrowserBackendResult<void>;
  cancel_browser_element_picker(input: BrowserBackendTarget): BrowserBackendResult<void>;
  inspect_browser_rect(input: BrowserBackendRectInput): BrowserBackendResult<BrowserElementMetadata | null>;
  capture_browser_viewport(input: BrowserBackendTarget): BrowserBackendResult<BrowserMarkupCapture>;
  open_browser_tab_devtools?(input: BrowserBackendTarget): BrowserBackendResult<void>;
  open_browser_tab_external?(input: BrowserBackendTarget): BrowserBackendResult<void>;
}

export interface BrowserBackendCall {
  command: string;
  input: unknown;
}

interface FakeTabRecord extends BrowserBackendTarget {
  url: string;
  visible: boolean;
  bounds: BrowserFloatingBounds | null;
  viewport: BrowserViewport | null;
  history: string[];
  historyIndex: number;
}

function copy<T>(value: T): T {
  if (value && typeof value === 'object') return structuredClone(value);
  return value;
}

/**
 * Deterministic backend used by model tests and browser preview builds.  It
 * records command order and keeps tab history, but never creates a native view
 * or touches the network.
 */
export class InMemoryBrowserBackend implements BrowserBackend {
  readonly calls: BrowserBackendCall[] = [];
  readonly tabs = new Map<string, FakeTabRecord>();
  readonly captures = new Map<string, BrowserMarkupCapture>();

  private record(command: string, input: unknown): void {
    this.calls.push({ command, input: copy(input) });
  }

  create_browser_tab(input: BrowserBackendTabInput): BrowserBackendTabResult {
    this.record('create_browser_tab', input);
    const current = this.tabs.get(input.tabId);
    if (current) return { tabId: current.tabId, generation: current.generation };
    const record: FakeTabRecord = {
      workspaceId: input.workspaceId,
      tabId: input.tabId,
      generation: input.generation,
      url: input.url,
      visible: true,
      bounds: input.bounds ? { ...input.bounds } : null,
      viewport: input.viewport ? { ...input.viewport } : null,
      history: input.url ? [input.url] : [],
      historyIndex: input.url ? 0 : -1
    };
    this.tabs.set(input.tabId, record);
    return { tabId: input.tabId, generation: input.generation };
  }

  set_browser_tab_bounds(input: BrowserBackendBoundsInput): void {
    this.record('set_browser_tab_bounds', input);
    const tab = this.tabs.get(input.tabId);
    if (tab) tab.bounds = { ...input.bounds };
  }

  set_browser_tab_viewport(input: BrowserBackendViewportInput): void {
    this.record('set_browser_tab_viewport', input);
    const tab = this.tabs.get(input.tabId);
    if (tab) tab.viewport = { ...input.viewport };
  }

  show_browser_tab(input: BrowserBackendTarget): void {
    this.record('show_browser_tab', input);
    const tab = this.tabs.get(input.tabId);
    if (tab) tab.visible = true;
  }

  hide_browser_workspace(input: { workspaceId: string }): void {
    this.record('hide_browser_workspace', input);
    for (const tab of this.tabs.values()) {
      if (tab.workspaceId === input.workspaceId) tab.visible = false;
    }
  }

  navigate_browser_tab(input: BrowserBackendNavigationInput): void {
    this.record('navigate_browser_tab', input);
    const tab = this.tabs.get(input.tabId);
    if (!tab) return;
    tab.url = input.url;
    tab.history = tab.history.slice(0, tab.historyIndex + 1);
    tab.history.push(input.url);
    tab.historyIndex = tab.history.length - 1;
  }

  reload_browser_tab(input: BrowserBackendTarget): void {
    this.record('reload_browser_tab', input);
  }

  go_back_browser_tab(input: BrowserBackendTarget): void {
    this.record('go_back_browser_tab', input);
    const tab = this.tabs.get(input.tabId);
    if (tab && tab.historyIndex > 0) {
      tab.historyIndex -= 1;
      tab.url = tab.history[tab.historyIndex];
    }
  }

  go_forward_browser_tab(input: BrowserBackendTarget): void {
    this.record('go_forward_browser_tab', input);
    const tab = this.tabs.get(input.tabId);
    if (tab && tab.historyIndex < tab.history.length - 1) {
      tab.historyIndex += 1;
      tab.url = tab.history[tab.historyIndex];
    }
  }

  close_browser_tab(input: BrowserBackendTarget): void {
    this.record('close_browser_tab', input);
    this.tabs.delete(input.tabId);
  }

  arm_browser_element_picker(input: BrowserBackendPickerInput): void {
    this.record('arm_browser_element_picker', input);
  }

  cancel_browser_element_picker(input: BrowserBackendTarget): void {
    this.record('cancel_browser_element_picker', input);
  }

  inspect_browser_rect(input: BrowserBackendRectInput): BrowserElementMetadata | null {
    this.record('inspect_browser_rect', input);
    return null;
  }

  capture_browser_viewport(input: BrowserBackendTarget): BrowserMarkupCapture {
    this.record('capture_browser_viewport', input);
    const existing = this.captures.get(input.tabId);
    if (existing) return copy(existing);
    const capture: BrowserMarkupCapture = {
      mimeType: 'image/png',
      bytes: [137, 80, 78, 71, input.tabId.length & 0xff, input.generation & 0xff],
      width: 1,
      height: 1,
      sourceHash: `fake-capture:${input.tabId}:${input.generation}`
    };
    this.captures.set(input.tabId, capture);
    return copy(capture);
  }

  open_browser_tab_devtools(input: BrowserBackendTarget): void {
    this.record('open_browser_tab_devtools', input);
  }

  open_browser_tab_external(input: BrowserBackendTarget): void {
    this.record('open_browser_tab_external', input);
  }
}

/**
 * Native adapter for the browser command surface registered by the Tauri
 * shell. The same command names are deliberately kept in one place here so
 * the model remains usable with the deterministic in-memory backend in web
 * preview and Node tests.
 */
export class TauriBrowserBackend implements BrowserBackend {
  create_browser_tab(input: BrowserBackendTabInput): Promise<BrowserBackendTabResult> {
    return invokeBrowserCommandFromTauri<BrowserBackendTabResult>('create_browser_tab', { input });
  }

  set_browser_tab_bounds(input: BrowserBackendBoundsInput): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('set_browser_tab_bounds', { input });
  }

  set_browser_tab_viewport(input: BrowserBackendViewportInput): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('set_browser_tab_viewport', { input });
  }

  show_browser_tab(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('show_browser_tab', { input });
  }

  hide_browser_workspace(input: { workspaceId: string }): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('hide_browser_workspace', { input });
  }

  navigate_browser_tab(input: BrowserBackendNavigationInput): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('navigate_browser_tab', { input });
  }

  reload_browser_tab(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('reload_browser_tab', { input });
  }

  go_back_browser_tab(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('go_back_browser_tab', { input });
  }

  go_forward_browser_tab(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('go_forward_browser_tab', { input });
  }

  close_browser_tab(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('close_browser_tab', { input });
  }

  arm_browser_element_picker(input: BrowserBackendPickerInput): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('arm_browser_element_picker', { input });
  }

  cancel_browser_element_picker(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('cancel_browser_element_picker', { input });
  }

  inspect_browser_rect(input: BrowserBackendRectInput): Promise<BrowserElementMetadata | null> {
    return invokeBrowserCommandFromTauri<BrowserElementMetadata | null>('inspect_browser_rect', { input });
  }

  capture_browser_viewport(input: BrowserBackendTarget): Promise<BrowserMarkupCapture> {
    return invokeBrowserCommandFromTauri<BrowserMarkupCapture>('capture_browser_viewport', { input });
  }

  open_browser_tab_devtools(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('open_browser_tab_devtools', { input });
  }

  open_browser_tab_external(input: BrowserBackendTarget): Promise<void> {
    return invokeBrowserCommandFromTauri<void>('open_browser_tab_external', { input });
  }
}

export const FakeBrowserBackend = InMemoryBrowserBackend;

export function createInMemoryBrowserBackend(): InMemoryBrowserBackend {
  return new InMemoryBrowserBackend();
}

/** Select the native command adapter only inside the rebuilt Tauri desktop. */
export function createBrowserBackend(): BrowserBackend {
  return isTauriRuntime() ? new TauriBrowserBackend() : createInMemoryBrowserBackend();
}

/** Alias kept obvious for test authors and future browser preview callers. */
export const createFakeBrowserBackend = createInMemoryBrowserBackend;
