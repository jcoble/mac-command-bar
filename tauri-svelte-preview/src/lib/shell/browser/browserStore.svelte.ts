/**
 * browserStore.svelte.ts — Svelte 5 runes state for the /next browser panel.
 *
 * Holds the panel's STATE only: the URL the frame is showing, the text in the
 * URL box, the counter that forces a frame reload, and the last message shown
 * to the user. The panel component reads these fields directly and calls the
 * mutators below; it keeps no `$state` of its own.
 *
 * THREE RULES this module exists to enforce:
 *
 * 1. **No backend, ever.** The only IO here is localStorage. There is no Tauri
 *    call in the browser panel at all, so `countInvoke` never applies to it.
 * 2. **No `$effect`.** `$effect` is illegal in a `.svelte.ts` module and against
 *    the shell rules, so saving is an explicit `persist()` call at the end of
 *    every mutator that changes the URL.
 * 3. **Nothing loads at launch.** The shell mounts every panel up front (they
 *    sit parked off-screen), so the frame must NOT start fetching a page just
 *    because the shell started. `activateBrowser()` is the gate: until it is
 *    called the panel shows an inert empty state and no frame exists. The shell
 *    calls it the first time the user opens the Browser tab. Any user action
 *    inside the panel also opens the gate, so a missed wiring cannot leave the
 *    panel permanently dead.
 *
 * A storage that is full or unavailable never breaks the panel: the URL still
 * works for this run, it just will not come back after a reload.
 */
import { normalizeBrowserUrl } from './normalizeBrowserUrl.ts';

/** localStorage key holding the last URL the browser panel showed. */
export const BROWSER_URL_STORAGE_KEY = 'mac-command-bar.next.browser.url';

/** What the user is told when the URL box holds something unusable. */
export const INVALID_URL_MESSAGE = 'Enter an address that starts with http or https';

/** What the user is told when the address could not be saved for next time. */
export const STORAGE_WRITE_FAILED_MESSAGE =
  'This address will not come back after a reload — browser storage is full';

// ── Reactive state ────────────────────────────────────────────────────────────

/**
 * The single reactive browser-panel state object. Read fields directly in the
 * component (`browser.url`, `browser.error`, …); mutate through the exported
 * functions so saving stays in lockstep.
 */
export const browser = $state<{
  /** The address the frame is showing. Empty means "no page yet". */
  url: string;
  /** The editable text in the URL box. Bound by the component. */
  inputUrl: string;
  /** Bumped to force the frame to reload the same address. */
  frameKey: number;
  /** Message shown above the frame; empty hides the strip. */
  error: string;
  /** True once the user has opened the panel: only then does a frame exist. */
  activated: boolean;
}>({
  url: '',
  inputUrl: '',
  frameKey: 0,
  error: '',
  activated: false
});

// ── Saving ────────────────────────────────────────────────────────────────────

/**
 * Save (or clear) the current address. Returns true when it landed.
 *
 * Quota and unavailable-storage errors are swallowed — losing the saved address
 * is harmless — but they are NOT silent: the user is told the address will not
 * survive a reload.
 */
function persist(url: string): boolean {
  if (typeof localStorage === 'undefined') return false;
  try {
    if (url) {
      localStorage.setItem(BROWSER_URL_STORAGE_KEY, url);
    } else {
      localStorage.removeItem(BROWSER_URL_STORAGE_KEY);
    }
    return true;
  } catch {
    browser.error = STORAGE_WRITE_FAILED_MESSAGE;
    return false;
  }
}

/**
 * Read the saved address back. Tolerant: an unavailable store or a saved value
 * that is no longer a usable address comes back as an empty string. Does NOT
 * touch `browser` — `activateBrowser()` is what applies it.
 */
export function loadStoredBrowserUrl(): string {
  if (typeof localStorage === 'undefined') return '';
  try {
    return normalizeBrowserUrl(localStorage.getItem(BROWSER_URL_STORAGE_KEY) ?? '');
  } catch {
    return '';
  }
}

// ── Mutations ─────────────────────────────────────────────────────────────────

/**
 * Open the gate: the first call restores the saved address (if any) so the
 * frame can render. Idempotent — later calls do nothing, so the shell may call
 * it on every activation of the Browser tab without reloading the page inside.
 */
export function activateBrowser(): void {
  if (browser.activated) return;
  browser.activated = true;
  const savedUrl = loadStoredBrowserUrl();
  if (!savedUrl) return;
  browser.url = savedUrl;
  browser.inputUrl = savedUrl;
}

/**
 * Point the frame at `value` (any shorthand `normalizeBrowserUrl` accepts).
 * Returns false and sets a message when the text is not a usable address.
 *
 * Bumps `frameKey` so submitting the SAME address reloads it — which is what a
 * user pressing Enter twice expects.
 */
export function setBrowserUrl(value: string): boolean {
  const normalizedUrl = normalizeBrowserUrl(value);
  if (!normalizedUrl) {
    browser.error = INVALID_URL_MESSAGE;
    return false;
  }

  browser.activated = true;
  browser.url = normalizedUrl;
  browser.inputUrl = normalizedUrl;
  browser.error = '';
  browser.frameKey += 1;
  persist(normalizedUrl);
  return true;
}

/**
 * Show `url` in the panel — the entry point for other parts of the shell (a
 * running dev server card, a palette command). Same rules as `setBrowserUrl`;
 * the caller is expected to also bring the Browser tab to the front.
 */
export function openBrowserUrl(url: string): boolean {
  return setBrowserUrl(url);
}

/** Reload the page currently in the frame. No-op when there is no page. */
export function reloadBrowserFrame(): void {
  if (!browser.url) return;
  browser.frameKey += 1;
}

/** Forget the current page and the saved address. */
export function clearBrowserUrl(): void {
  browser.url = '';
  browser.inputUrl = '';
  browser.error = '';
  persist('');
}

/** Clear the message strip (the component calls this as the user retypes). */
export function clearBrowserError(): void {
  browser.error = '';
}
