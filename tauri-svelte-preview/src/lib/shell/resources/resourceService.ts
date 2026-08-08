import {
  applyResourceMemoryPressure,
  cleanupWorkspaceDiskEntry,
  readLanguageServerLog,
  readResourceDiskScan,
  readResourceSnapshot,
  restartLanguageServerRoot,
  setActiveSourceRoot,
  stopOwnedResource
} from './resourceBackend.ts';
import type {
  DiskScanReport,
  ResourceCommandReceipt,
  ResourceCleanupReceipt,
  ResourceCleanupRequest,
  ResourceDiskRoot,
  ResourceSnapshot,
  ResourceStopRequest
} from './resourceTypes.ts';
import type { ResourceUnavailable } from './resourceTypes.ts';

let refreshInFlight: Promise<ResourceSnapshot | null> | null = null;

export const resourceService = {
  refresh(): Promise<ResourceSnapshot | null> {
    if (!refreshInFlight) {
      refreshInFlight = readResourceSnapshot().finally(() => {
        refreshInFlight = null;
      });
    }
    return refreshInFlight;
  },
  readDisk(roots: ResourceDiskRoot[], options?: { maxDepth?: number; maxEntries?: number }): Promise<DiskScanReport | null> {
    return readResourceDiskScan(roots, options);
  },
  stop(request: ResourceStopRequest): Promise<ResourceCommandReceipt | null> {
    return stopOwnedResource(request);
  },
  restartLanguageServer(root: string): Promise<ResourceUnavailable | null> {
    return restartLanguageServerRoot(root);
  },
  applyMemoryPressure(level: string): Promise<ResourceUnavailable | null> {
    return applyResourceMemoryPressure(level);
  },
  readLanguageServerLog(root: string): Promise<ResourceUnavailable | null> {
    return readLanguageServerLog(root);
  },
  cleanup(request: ResourceCleanupRequest): Promise<ResourceCleanupReceipt | null> {
    return cleanupWorkspaceDiskEntry(request);
  },
  setActiveRoot(root: string): Promise<string | null> {
    return setActiveSourceRoot(root);
  }
};
