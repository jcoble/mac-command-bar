import { readResourceSample, stopResourceProcessTree } from './resourceSampleBackend.ts';
import type { ResourceSample } from './resourceSampleTypes.ts';
import { buildStopRequest, type ResourceStopTarget } from './resourceStopModel.ts';

export const resourceSampleState = $state<{
  sample: ResourceSample | null;
  loading: boolean;
  error: string | null;
}>({
  sample: null,
  loading: false,
  error: null
});

export const resourceManagerState = $state({ open: false });

let refreshInFlight: Promise<ResourceSample | null> | null = null;

export function setResourceManagerOpen(open: boolean): void {
  resourceManagerState.open = open;
}

export function toggleResourceManager(): void {
  resourceManagerState.open = !resourceManagerState.open;
}

export function refreshResourceSample(): Promise<ResourceSample | null> {
  if (refreshInFlight) return refreshInFlight;
  resourceSampleState.loading = true;
  resourceSampleState.error = null;
  refreshInFlight = readResourceSample()
    .then((sample) => {
      resourceSampleState.sample = sample;
      return sample;
    })
    .catch((error: unknown) => {
      resourceSampleState.error =
        error instanceof Error ? error.message : 'Resource usage could not be read.';
      return null;
    })
    .finally(() => {
      resourceSampleState.loading = false;
      refreshInFlight = null;
    });
  return refreshInFlight;
}

/**
 * The stop flow, which has exactly one shape: a button opens a question, and
 * nothing is signalled until the question is answered. There is no path from a
 * sample to a signal — the app never stops anything by itself.
 */
export const resourceStopState = $state<{
  target: ResourceStopTarget | null;
  busy: boolean;
  error: string | null;
  receipt: string | null;
}>({
  target: null,
  busy: false,
  error: null,
  receipt: null
});

export function askToStopResource(target: ResourceStopTarget): void {
  resourceStopState.target = target;
  resourceStopState.error = null;
  resourceStopState.receipt = null;
}

export function cancelStopResource(): void {
  resourceStopState.target = null;
  resourceStopState.busy = false;
}

export async function confirmStopResource(): Promise<void> {
  const target = resourceStopState.target;
  if (!target || resourceStopState.busy) return;
  resourceStopState.busy = true;
  resourceStopState.error = null;
  try {
    const receipt = await stopResourceProcessTree(buildStopRequest(target));
    resourceStopState.receipt = receipt?.message ?? 'Stopping is available in the desktop app.';
    resourceStopState.target = null;
    await refreshResourceSample();
  } catch (error: unknown) {
    resourceStopState.error =
      error instanceof Error ? error.message : String(error ?? 'That could not be stopped.');
  } finally {
    resourceStopState.busy = false;
  }
}
