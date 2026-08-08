import type { DiskProtection, WorkspaceDiskEntry } from './resourceTypes.ts';

const labels: Record<DiskProtection, string> = {
  active: 'Active workspace',
  dirty: 'Dirty changes',
  unmerged: 'Unmerged changes',
  locked: 'Locked',
  'user-data': 'User data',
  'safe-candidate': 'Safe to review',
  unknown: 'Protection unknown'
};

export function diskProtectionLabel(protection: DiskProtection): string {
  return labels[protection] ?? labels.unknown;
}

export function reclaimableBytes(entry: Pick<WorkspaceDiskEntry, 'bytes' | 'reclaimableBytes' | 'protection'>): number {
  return entry.protection === 'safe-candidate' ? Math.min(entry.bytes, Math.max(0, entry.reclaimableBytes)) : 0;
}
