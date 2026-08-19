/**
 * explorerService.ts — the file explorer's ONLY backend path.
 *
 * One job: ask the backend for the list of source files under a project root
 * and put the answer in `explorerStore`. Imperative by rule — every call comes
 * from something that happened, never from a render: a user action (opening the
 * panel, pressing Refresh) or the file watcher the Files panel keeps on the
 * listed folder saying the disk moved. Nothing here is called because a value
 * was read. Every call is counted with `countInvoke` immediately before it.
 *
 * Nothing here runs until `activate(root)` is called. That is the constitution's
 * "nothing loads at launch": opening the shell must add zero backend calls from
 * this lane until the file panel is actually shown.
 *
 * Two behaviours are load-bearing:
 *
 * 1. **A superseded scan is cancelled, not just ignored.** Each scan carries a
 *    `scanId`. When a new scan starts, the previous one's id is passed to
 *    `cancel_source_scan` so the Rust directory walk actually stops. Discarding
 *    only the result (what the old shell did before commit cd2f525) left a
 *    second full walk running in parallel and doubled the time the first list
 *    took to appear.
 * 2. **No cache.** The old shell cached scans in localStorage with no size cap;
 *    on a large repository the write failed and the cache quietly stopped
 *    working. This wave scans on activation and on a refresh, and a refresh is
 *    always someone's doing: pressing the button, or the file-system watcher
 *    the panel keeps on the listed folder reporting that the disk moved.
 */
import {
  cancelSourceScanFromTauri,
  createSourceScanId,
  defaultSourceScanLimit,
  isNativeTauriRuntime,
  listSourceFilesFromTauri
} from '../../tauriSource.ts';
import { countInvoke } from '../devInvokeCounter.svelte.ts';
import {
  forgetProjectSourceRecords,
  setProjectSourceRecords
} from '../projectSourceIndex.ts';
import {
  applyScanResult,
  beginScan,
  endScan,
  explorer,
  failScan,
  resetExplorer
} from './explorerStore.svelte.ts';

/** Files one scan will return at most. Past it the backend reports `truncated`
 * and the panel says so. */
export const EXPLORER_SCAN_LIMIT = defaultSourceScanLimit;

/** Shown when neither the app's own file scanner nor the dev-server bridge
 * answered — there is nothing to list and nothing to retry usefully. */
export const SCANNER_UNAVAILABLE_MESSAGE =
  'The file scanner is not available here. Open this window in the CommandBar app to browse project files.';

/** Bumped by every scan start and by Stop. A scan whose number no longer
 * matches has been superseded and must not write anything. */
let scanGeneration = 0;

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Stop a backend directory walk. Off the desktop app there is no walk to stop
 * (`cancelSourceScanFromTauri` returns immediately without calling anything),
 * so nothing is counted either.
 */
function cancelScan(scanId: string): void {
  if (!scanId.trim() || !isNativeTauriRuntime()) return;
  countInvoke('cancel_source_scan');
  void cancelSourceScanFromTauri(scanId).catch(() => {
    // Best effort: the result of the walk we abandoned is discarded anyway.
  });
}

/**
 * List the files under `root`, replacing whatever is on screen.
 *
 * Exported for the panel's Refresh button and for a future "point the explorer
 * somewhere else" caller. `activate` is the one the integrator calls.
 */
export async function scanRoot(root: string): Promise<void> {
  const target = root.trim();
  if (!target) return;

  const generation = (scanGeneration += 1);
  const scanId = createSourceScanId();
  // Supersede the previous walk BEFORE starting a new one — see the note above.
  const supersededScanId = explorer.activeScanId;
  if (supersededScanId && supersededScanId !== scanId) cancelScan(supersededScanId);
  beginScan(target, scanId);

  try {
    countInvoke(isNativeTauriRuntime() ? 'list_source_files' : 'bridge:list-source-files');
    const result = await listSourceFilesFromTauri(target, '', EXPLORER_SCAN_LIMIT, scanId);
    // A newer scan (or Stop) took over while we waited: it owns the state now.
    if (generation !== scanGeneration) return;

    if (!result) {
      forgetProjectSourceRecords(target);
      failScan(SCANNER_UNAVAILABLE_MESSAGE);
      return;
    }
    setProjectSourceRecords(target, result.records);
    applyScanResult(result.records, result.limit, result.truncated);
  } catch (error) {
    if (generation !== scanGeneration) return;
    forgetProjectSourceRecords(target);
    failScan(`Could not list the files in this project: ${describeError(error)}`);
  } finally {
    if (generation === scanGeneration) endScan();
  }
}

/**
 * First user activation of the file panel: list the files under `root`.
 *
 * Idempotent — calling it again with the same root does nothing, so wiring it
 * to "every time this tab becomes active" is safe. A different root clears the
 * previous project's tree and scans afresh. A root whose last scan failed is
 * retried, so re-opening the panel is a way to try again.
 */
export function activate(root: string): void {
  const target = root.trim();
  if (!target) return;
  if (explorer.activated && explorer.root === target && explorer.error === null) return;
  if (explorer.root !== target) resetExplorer();
  void scanRoot(target);
}

/** Re-list the current project. No-op before the first activation. */
export function refresh(): void {
  if (!explorer.root) return;
  void scanRoot(explorer.root);
}

/** Abandon the running scan: stop the backend walk and drop its result. */
export function stopScan(): void {
  if (!explorer.scanning) return;
  cancelScan(explorer.activeScanId);
  scanGeneration += 1;
  endScan();
}
