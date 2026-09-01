import type { DiskScanReport, ResourceDiskRoot } from './resourceTypes.ts';
import { workspaceSpaceService } from './workspaceSpaceService.ts';

export const workspaceSpaceState = $state<{
  report: DiskScanReport | null;
  scanning: boolean;
  error: string | null;
}>({ report: null, scanning: false, error: null });

export async function scanWorkspaceSpaceRoots(roots: ResourceDiskRoot[]): Promise<DiskScanReport | null> {
  if (workspaceSpaceState.scanning) return workspaceSpaceState.report;
  workspaceSpaceState.scanning = true;
  workspaceSpaceState.error = null;
  try {
    const report = await workspaceSpaceService.scan(roots);
    workspaceSpaceState.report = report;
    return report;
  } catch (error) {
    workspaceSpaceState.error = error instanceof Error ? error.message : 'Space scan failed.';
    return null;
  } finally {
    workspaceSpaceState.scanning = false;
  }
}
