import type { ProcessOwner, ResourceProcess } from './resourceTypes.ts';

const ownerLabels: Record<ProcessOwner, string> = {
  app: 'App process',
  'owned-session': 'Owned session',
  'language-server': 'Language server',
  'provider-sidecar': 'Provider sidecar',
  playwright: 'Browser automation',
  external: 'External process'
};

export function resourceOwnerLabel(resource: Pick<ResourceProcess, 'owner' | 'ownerId'>): string {
  const label = ownerLabels[resource.owner] ?? 'External process';
  return resource.ownerId ? `${label} · ${resource.ownerId}` : label;
}

export function resourceCanStop(resource: Pick<ResourceProcess, 'owner' | 'canStop'>): boolean {
  return resource.owner !== 'external' && resource.canStop;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; value >= 1024 && index < units.length; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${unit}`;
}
