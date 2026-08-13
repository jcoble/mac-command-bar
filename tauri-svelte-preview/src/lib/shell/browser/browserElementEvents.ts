/**
 * browserElementEvents.ts — receiving the element the person picked.
 *
 * Arming the picker is a command and returns nothing; the pick itself comes
 * back later on an event, because the page has to be clicked first. Nothing in
 * the app was listening for that event, so `acceptBrowserElementSelection` in
 * the model had no caller and Select could never finish. This is the missing
 * half.
 *
 * The event name and payload are the shell's (`src-tauri/src/browser.rs`:
 * `BROWSER_ELEMENT_SELECTED_EVENT`, `BrowserElementSelectedEvent`). Outside the
 * desktop build there is no event source, so subscribing is a no-op that
 * answers with a no-op unsubscribe — a caller never has to ask which build it
 * is in.
 */
import { isTauriRuntime } from '../../tauriSource.ts';
import type { BrowserRect } from './browserTypes.ts';

export const BROWSER_ELEMENT_SELECTED_EVENT = 'browser-element-selected';

/** `selected` carries an element; `unavailable` says the page would not give one. */
export type BrowserPickerStatus = 'selected' | 'unavailable';

export interface BrowserElementSelectedEvent {
  workspaceId: string;
  tabId: string;
  generation: number;
  status: BrowserPickerStatus;
  reason: string | null;
  url: string | null;
  title: string | null;
  selector: string | null;
  accessibleName: string | null;
  textSnippet: string | null;
  rect: BrowserRect | null;
  classes: string[];
  classCount: number;
  sourceHash: string;
}

export type UnsubscribeFromBrowserElements = () => void;

/**
 * Subscribe to picked elements. The returned promise resolves once the
 * subscription is live; call what it answers with to stop listening.
 */
export async function listenToBrowserElementSelected(
  handler: (event: BrowserElementSelectedEvent) => void
): Promise<UnsubscribeFromBrowserElements> {
  if (!isTauriRuntime()) return () => undefined;
  const { listen } = await import('@tauri-apps/api/event');
  const stop = await listen<BrowserElementSelectedEvent>(
    BROWSER_ELEMENT_SELECTED_EVENT,
    (event) => {
      handler(event.payload);
    }
  );
  return () => stop();
}
