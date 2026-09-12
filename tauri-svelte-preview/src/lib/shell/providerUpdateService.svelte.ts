import { invoke } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';

export type ProviderUpdateVersion = {
  provider: string;
  currentVersion: string;
  availableVersion: string;
  updateAvailable: boolean;
};

export type ProviderUpdateStatus = {
  target: string;
  updateAvailable: boolean;
  restartRequired: boolean;
  providers: ProviderUpdateVersion[];
};

export type ProviderUpdatePhase = 'idle' | 'checking' | 'available' | 'current' | 'installing' | 'error';

export const providerUpdateState = $state({
  phase: 'idle' as ProviderUpdatePhase,
  message: '',
  status: null as ProviderUpdateStatus | null
});

let operationGeneration = 0;

function stopped(stopSignal: AbortSignal, generation: number): boolean {
  return stopSignal.aborted || generation !== operationGeneration;
}

export async function checkForProviderUpdates(stopSignal: AbortSignal): Promise<void> {
  if (stopSignal.aborted || providerUpdateState.phase === 'installing') return;
  const generation = ++operationGeneration;
  providerUpdateState.phase = 'checking';
  providerUpdateState.message = 'Checking provider adapters…';
  try {
    const status = await invoke<ProviderUpdateStatus>('check_provider_updates');
    if (stopped(stopSignal, generation)) return;
    providerUpdateState.status = status;
    providerUpdateState.phase = status.updateAvailable ? 'available' : 'current';
    providerUpdateState.message = status.updateAvailable
      ? providerVersionMessage(status)
      : 'Provider adapters are up to date.';
  } catch (error) {
    if (stopped(stopSignal, generation)) return;
    providerUpdateState.phase = 'error';
    providerUpdateState.message = String(error);
  }
}

export async function installProviderUpdates(stopSignal: AbortSignal): Promise<void> {
  if (stopSignal.aborted || providerUpdateState.phase !== 'available') return;
  const generation = ++operationGeneration;
  providerUpdateState.phase = 'installing';
  providerUpdateState.message = 'Downloading and verifying provider adapters…';
  try {
    const status = await invoke<ProviderUpdateStatus>('install_provider_updates');
    if (stopped(stopSignal, generation)) return;
    providerUpdateState.status = status;
    if (!status.restartRequired) {
      providerUpdateState.phase = 'current';
      providerUpdateState.message = 'Provider adapters are up to date.';
      return;
    }
    providerUpdateState.message = 'Provider adapters installed. Restarting Assembly…';
    await relaunch();
  } catch (error) {
    if (stopped(stopSignal, generation)) return;
    providerUpdateState.phase = 'error';
    providerUpdateState.message = String(error);
  }
}

function providerVersionMessage(status: ProviderUpdateStatus): string {
  return status.providers
    .filter((provider) => provider.updateAvailable)
    .map((provider) => `${providerLabel(provider.provider)} ${provider.availableVersion}`)
    .join(' · ');
}

function providerLabel(provider: string): string {
  if (provider === 'codex') return 'Codex';
  if (provider === 'claude') return 'Claude';
  if (provider === 'antigravity') return 'Antigravity';
  return provider;
}
