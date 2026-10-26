import type { ResourceSampleTotals } from './resourceSampleTypes.ts';

/**
 * Watch the resource totals until `signal` aborts: the backend samples while any
 * window watches and pushes `resource-totals` only when a shown figure changes.
 * Resolves with a fresh sample so the caller is never blank while it waits.
 */
export async function watchResourceTotals(
  onTotals: (totals: ResourceSampleTotals) => void,
  signal: AbortSignal
): Promise<ResourceSampleTotals | null> {
  const { invoke } = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  if (signal.aborted) return null;
  // Listen first, so no push between the first sample and the listener is lost.
  const unlisten = await listen<ResourceSampleTotals>('resource-totals', (event) => onTotals(event.payload));
  const stop = async (): Promise<void> => {
    unlisten();
    await invoke('watch_resource_totals', { watching: false });
  };
  if (signal.aborted) {
    await stop();
    return null;
  }
  signal.addEventListener('abort', () => void stop(), { once: true });
  const totals = await invoke<ResourceSampleTotals | null>('watch_resource_totals', { watching: true });
  // An abort during that call may have reached the backend first; stopping again is harmless.
  if (signal.aborted) {
    await invoke('watch_resource_totals', { watching: false });
    return null;
  }
  return totals;
}
