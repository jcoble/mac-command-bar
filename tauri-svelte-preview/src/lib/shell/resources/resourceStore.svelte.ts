import type { DiskScanReport, ResourceSnapshot } from './resourceTypes.ts';
import { resourceService } from './resourceService.ts';

export const resourceState = $state<{
  snapshot: ResourceSnapshot | null;
  disk: DiskScanReport | null;
  loading: boolean;
  diskLoading: boolean;
  error: string | null;
  unavailableReason: string | null;
  lastRefreshAt: number | null;
}>({
  snapshot: null,
  disk: null,
  loading: false,
  diskLoading: false,
  error: null,
  unavailableReason: null,
  lastRefreshAt: null
});

export async function refreshResources(): Promise<ResourceSnapshot | null> {
  if (resourceState.loading) return resourceState.snapshot;
  resourceState.loading = true;
  resourceState.error = null;
  try {
    const snapshot = await resourceService.refresh();
    resourceState.snapshot = snapshot;
    resourceState.unavailableReason = snapshot ? null : 'Resource inventory is available in the desktop app.';
    resourceState.lastRefreshAt = snapshot ? Date.now() : resourceState.lastRefreshAt;
    return snapshot;
  } catch (error) {
    resourceState.error = error instanceof Error ? error.message : 'Resource inventory failed.';
    return null;
  } finally {
    resourceState.loading = false;
  }
}

export async function scanWorkspaceSpace(
  roots: Parameters<typeof resourceService.readDisk>[0]
): Promise<DiskScanReport | null> {
  if (resourceState.diskLoading) return resourceState.disk;
  resourceState.diskLoading = true;
  resourceState.error = null;
  try {
    const disk = await resourceService.readDisk(roots);
    resourceState.disk = disk;
    resourceState.unavailableReason = disk ? null : 'Space scans are available in the desktop app.';
    return disk;
  } catch (error) {
    resourceState.error = error instanceof Error ? error.message : 'Space scan failed.';
    return null;
  } finally {
    resourceState.diskLoading = false;
  }
}
