import { isNativeTauriRuntime } from '$lib/tauriSource';

import type { ResourceSample } from './resourceSampleTypes';

export async function readResourceSample(): Promise<ResourceSample | null> {
  if (!isNativeTauriRuntime()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ResourceSample>('read_resource_sample');
}
