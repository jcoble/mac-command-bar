import { readResourceSample } from './resourceSampleBackend';
import type { ResourceSample } from './resourceSampleTypes';

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
