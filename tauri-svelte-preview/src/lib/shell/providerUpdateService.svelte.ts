import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type ProviderDownloadProgress = {
  provider: string;
  fileIndex: number;
  fileCount: number;
  downloaded: number;
  total: number | null;
  profileId: string | null;
};

export type ProviderUpdateVersion = {
  provider: string;
  currentVersion: string | null;
  availableVersion: string;
  updateAvailable: boolean;
};

export type ProviderUpdateStatus = {
  target: string;
  updateAvailable: boolean;
  restartRequired: boolean;
  providers: ProviderUpdateVersion[];
};

export type ProviderUpdatePhase = 'idle' | 'checking' | 'available' | 'current' | 'installing' | 'restart' | 'error';

export const providerUpdateState = $state({
  phase: 'idle' as ProviderUpdatePhase,
  generation: 0,
  message: '',
  status: null as ProviderUpdateStatus | null
});

export type ProviderUpdateState = typeof providerUpdateState;

function stopped(stopSignal: AbortSignal, generation: number, state: ProviderUpdateState): boolean {
  return stopSignal.aborted || generation !== state.generation;
}

export async function checkForProviderUpdates(stopSignal: AbortSignal, state: ProviderUpdateState = providerUpdateState, profileId?: string): Promise<void> {
  if (stopSignal.aborted || state.phase === 'checking' || state.phase === 'installing') return;
  const generation = ++state.generation;
  state.phase = 'checking';
  state.message = 'Checking provider adapters…';
  try {
    const status = await invoke<ProviderUpdateStatus>(profileId ? 'check_remote_provider_updates' : 'check_provider_updates', profileId ? { profileId } : undefined);
    if (stopped(stopSignal, generation, state)) return;
    state.status = status;
    state.phase = status.restartRequired ? 'restart' : status.updateAvailable ? 'available' : 'current';
    state.message = status.restartRequired ? `Provider adapters installed. Restart ${profileId ? 'the remote server' : 'Assembly'} to use them.` : status.updateAvailable
      ? providerVersionMessage(status)
      : 'Provider adapters are up to date.';
  } catch (error) {
    if (stopped(stopSignal, generation, state)) return;
    state.phase = 'error';
    state.message = String(error);
  } finally {
    if (stopSignal.aborted && generation === state.generation) {
      state.phase = 'idle';
      state.message = '';
    }
  }
}

export async function installProviderUpdates(provider: string, stopSignal: AbortSignal, state: ProviderUpdateState = providerUpdateState, profileId?: string): Promise<void> {
  if (stopSignal.aborted) return;
  if (state.phase === 'installing' || !state.status?.providers.some((item) => item.provider === provider && item.updateAvailable)) return;
  ++state.generation;
  state.phase = 'installing';
  state.message = `Downloading and verifying ${providerLabel(provider)}…`;
  let unlisten: (() => void) | undefined;
  try {
    unlisten = await listen<ProviderDownloadProgress>('provider-download-progress', ({ payload }) => {
      if (payload.provider !== provider || (payload.profileId ?? undefined) !== profileId || state.phase !== 'installing') return;
      const part = payload.fileCount > 1 ? ` · file ${payload.fileIndex}/${payload.fileCount}` : '';
      const total = payload.total ? ` of ${formatDownloadSize(payload.total)}` : '';
      state.message = `Downloading ${providerLabel(provider)}${part} · ${formatDownloadSize(payload.downloaded)}${total}`;
    });
    const status = await invoke<ProviderUpdateStatus>(profileId ? 'install_remote_provider_updates' : 'install_provider_updates', { provider, ...(profileId ? { profileId } : {}) });
    // Installation is owned by the app, not the Settings component. Closing
    // Settings must not lose an already installed update or restart the app.
    state.status = status;
    state.phase = status.restartRequired ? 'restart' : status.updateAvailable ? 'available' : 'current';
    state.message = status.restartRequired
      ? `${providerLabel(provider)} installed. Restart ${profileId ? 'the remote server' : 'Assembly'} to use it.`
      : 'Provider adapter is up to date.';
  } catch (error) {
    state.phase = 'error';
    state.message = String(error);
  } finally {
    unlisten?.();
  }
}

function formatDownloadSize(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export async function restartProviders(state: ProviderUpdateState, profileId?: string): Promise<void> {
  state.phase = 'installing';
  state.message = `Restarting ${profileId ? 'the remote server' : 'Assembly'}…`;
  try {
    await invoke(profileId ? 'restart_remote_for_provider_updates' : 'restart_for_provider_updates', profileId ? { profileId } : undefined);
    if (profileId) {
      state.phase = 'idle';
      state.status = null;
      state.message = 'Remote server restarting. Check again after it reconnects to verify the running adapters.';
    }
  } catch (error) {
    // A busy conversation or restart failure keeps the explicit retry action.
    state.phase = 'restart';
    state.message = String(error);
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
