import { deferred } from '../../deferred.ts';
import type { ResourceSampleTotals } from './resourceSampleTypes.ts';

/** The window's watch, which the next one waits on, so a hide and show never overlap. */
let previous: Promise<void> | null = null;

/**
 * Watch the resource totals until `signal` aborts: the backend samples while any
 * window watches and pushes `resource-totals` only when a shown figure changes.
 * The first sample arrives through `onTotals` at once. Resolves only once this
 * window has stopped watching; a new watch first waits for the one before it,
 * so an old watch's stop can never cancel its replacement.
 */
export async function watchResourceTotals(
  onTotals: (totals: ResourceSampleTotals) => void,
  signal: AbortSignal
): Promise<void> {
  const before = previous;
  const current = watchAfter(before, onTotals, signal);
  previous = current;
  try {
    await current;
  } finally {
    if (previous === current) previous = null;
  }
}

async function watchAfter(
  before: Promise<void> | null,
  onTotals: (totals: ResourceSampleTotals) => void,
  signal: AbortSignal
): Promise<void> {
  try {
    await before;
  } catch {
    // The earlier watch's error was already reported to its own caller.
  }
  if (signal.aborted) return;
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  if (signal.aborted) return;
  const aborted = deferred();
  const onAbort = (): void => aborted.resolve();
  signal.addEventListener('abort', onAbort, { once: true });
  try {
    // Listen first, so no push between the first sample and the listener is lost.
    const unlisten = await listen<ResourceSampleTotals>('resource-totals', (event) => onTotals(event.payload));
    try {
      if (signal.aborted) return;
      const totals = await invoke<ResourceSampleTotals | null>('watch_resource_totals', { watching: true });
      if (totals && !signal.aborted) onTotals(totals);
      await aborted.promise;
    } finally {
      unlisten();
      await invoke('watch_resource_totals', { watching: false });
    }
  } finally {
    signal.removeEventListener('abort', onAbort);
    aborted.resolve();
  }
}
