/**
 * explorerStore.svelte.ts — Svelte 5 runes state for the /next file explorer.
 *
 * STATE ONLY. Two rules, same as the session rail store:
 *
 * 1. **No backend, ever.** Every Tauri call lives in `explorerService.ts` and
 *    lands here as a plain mutation.
 * 2. **No `$effect`** — illegal in a `.svelte.ts` module and against the
 *    constitution. Nothing here is persisted either: this wave deliberately has
 *    no scan cache (the old shell's localStorage cache was uncapped and died
 *    silently on large repositories), so a reload rescans.
 *
 * Two deliberate shapes worth knowing:
 *
 * - `records` is `$state.raw`, reached through `explorerRecords()`. A scan can
 *   return ten thousand file records, and a normal `$state` array would wrap
 *   every one of them in a proxy the first time the tree walked it. The list is
 *   only ever replaced wholesale, never edited in place, so raw is both cheaper
 *   and correct.
 * - `expandedFolderIds` is a plain `Set`. Svelte does not track changes made
 *   inside a `Set`, so every mutator assigns a NEW set — see `toggleFolder` in
 *   `explorerTree.ts`. Mutating it in place would leave the tree frozen.
 */
import type { SourceRecord } from '../../sourceData.ts';
import { expandedForRecord, toggleFolder } from './explorerTree.ts';

// ── Reactive state ────────────────────────────────────────────────────────────

/**
 * The explorer's small state. Read fields directly in components
 * (`explorer.scanning`, `explorer.query`); the scanned file list lives apart in
 * `explorerRecords()` for the reason above.
 */
export const explorer = $state<{
  /** Absolute path of the project being listed, once `activate` has run. */
  root: string | null;
  /** True after the first `activate(root)` — before that the panel is inert. */
  activated: boolean;
  /** Filter box text. */
  query: string;
  /** Ids of open folders (`folder:src`, `folder:src/lib`, …). */
  expandedFolderIds: Set<string>;
  /** Scroll offset of the tree container, in pixels. */
  scrollTop: number;
  /** Measured height of the tree container; 0 until it has been shown. */
  viewportHeight: number;
  /** Absolute path of the highlighted file, if any. */
  selectedPath: string | null;
  /** A scan is running. */
  scanning: boolean;
  /** `scanId` of the running scan, so a superseding scan can cancel it. */
  activeScanId: string;
  /** Last failure, in plain words, or null. */
  error: string | null;
  /** The backend stopped at its file limit — the list is incomplete. */
  truncated: boolean;
  /** File limit the last scan ran with. */
  limit: number;
  /** `Date.now()` when the last scan finished, for the "listed at" note. */
  lastScanFinishedAt: number | null;
}>({
  root: null,
  activated: false,
  query: '',
  expandedFolderIds: new Set(),
  scrollTop: 0,
  viewportHeight: 0,
  selectedPath: null,
  scanning: false,
  activeScanId: '',
  error: null,
  truncated: false,
  limit: 0,
  lastScanFinishedAt: null
});

/** The scanned file list. Raw on purpose (see the module note). */
let scannedRecords = $state.raw<SourceRecord[]>([]);

/** Files from the last scan. Call it in templates — the read is tracked. */
export function explorerRecords(): SourceRecord[] {
  return scannedRecords;
}

// ── Mutations ─────────────────────────────────────────────────────────────────

/** Forget everything about the previous project. Called when the explorer is
 * pointed at a different root, so no rows from the old project survive. */
export function resetExplorer(): void {
  scannedRecords = [];
  explorer.query = '';
  explorer.expandedFolderIds = new Set();
  explorer.scrollTop = 0;
  explorer.selectedPath = null;
  explorer.error = null;
  explorer.truncated = false;
  explorer.limit = 0;
  explorer.lastScanFinishedAt = null;
}

/** Mark a scan as started against `root`. */
export function beginScan(root: string, scanId: string): void {
  explorer.root = root;
  explorer.activated = true;
  explorer.activeScanId = scanId;
  explorer.scanning = true;
  explorer.error = null;
}

/** Store a finished scan's result. */
export function applyScanResult(records: SourceRecord[], limit: number, truncated: boolean): void {
  scannedRecords = records;
  explorer.limit = limit;
  explorer.truncated = truncated;
  explorer.error = null;
  explorer.lastScanFinishedAt = Date.now();
  // A path selected under the previous list may no longer exist.
  if (explorer.selectedPath && !records.some((record) => record.path === explorer.selectedPath)) {
    explorer.selectedPath = null;
  }
}

/** Record a scan failure in plain words; the previous list is thrown away so
 * the panel never shows a stale tree next to an error. */
export function failScan(message: string): void {
  scannedRecords = [];
  explorer.truncated = false;
  explorer.error = message;
}

/** Clear the "a scan is running" flags. Safe to call twice. */
export function endScan(): void {
  explorer.scanning = false;
  explorer.activeScanId = '';
}

export function setQuery(query: string): void {
  explorer.query = query;
  // A new filter produces a different row list; keeping the old offset would
  // land the user in the middle of nowhere.
  explorer.scrollTop = 0;
}

export function setScrollTop(scrollTop: number): void {
  explorer.scrollTop = Math.max(0, scrollTop);
}

/** Store a measured container height. Zero is IGNORED: the panel mounts inside
 * a hidden parking area and is also hidden whenever another tab is in front,
 * and a zero height there is not a real measurement. */
export function setViewportHeight(viewportHeight: number): void {
  if (viewportHeight > 0) explorer.viewportHeight = viewportHeight;
}

export function toggleFolderExpansion(folderId: string): void {
  explorer.expandedFolderIds = toggleFolder(explorer.expandedFolderIds, folderId);
}

export function selectPath(path: string | null): void {
  explorer.selectedPath = path;
}

/** Open every folder above `record` and highlight it. */
export function revealRecord(record: SourceRecord): void {
  explorer.expandedFolderIds = expandedForRecord(explorer.expandedFolderIds, record);
  explorer.selectedPath = record.path;
}
