import assert from 'node:assert/strict';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';

// Exercise the store and async ownership contract without mounting UI.
globalThis.$state = <T>(value: T): T => value;
globalThis.window = {} as Window & typeof globalThis;
Object.defineProperty(window, 'crypto', { value: globalThis.crypto });
const { checkForProviderUpdates, installProviderUpdates, providerUpdateState, restartProviders } = await import('../src/lib/shell/providerUpdateService.svelte.ts');
const status = {
  target: 'aarch64-apple-darwin', updateAvailable: true, restartRequired: false,
  providers: [{ provider: 'claude', currentVersion: '0.76.0', availableVersion: '0.81.2', updateAvailable: true }]
};
let calls = 0;
let complete: (value: typeof status) => void = () => { throw new Error('No pending request'); };
mockIPC(() => {
  calls += 1;
  return new Promise<typeof status>((resolve) => { complete = resolve; });
});
const owner = new AbortController();
const first = checkForProviderUpdates(owner.signal);
await checkForProviderUpdates(new AbortController().signal);
assert.equal(calls, 1, 'overlapping launch/manual checks share the existing request');
owner.abort();
complete(status);
await first;
assert.equal(providerUpdateState.phase, 'idle', 'destroyed owner releases checking state');
assert.equal(providerUpdateState.status, null, 'late result is not published');
const next = checkForProviderUpdates(new AbortController().signal);
complete(status);
await next;
assert.equal(providerUpdateState.phase, 'available');
assert.equal(providerUpdateState.message, 'Claude 0.81.2');
mockIPC(() => { throw new Error('offline'); });
await checkForProviderUpdates(new AbortController().signal);
assert.equal(providerUpdateState.phase, 'error');
assert.match(providerUpdateState.message, /offline/);
// Closing Settings cannot cancel a native installation or lose its restart state.
providerUpdateState.phase = 'available';
const installOwner = new AbortController();
let finishInstall: (value: typeof status) => void = () => { throw new Error('No install'); };
let installs = 0;
let restarts = 0;
mockIPC((command) => {
  if (command === 'plugin:event|listen') return 1;
  if (command === 'plugin:event|unlisten') return;
  if (command === 'install_provider_updates') {
    installs += 1;
    return new Promise<typeof status>(resolve => { finishInstall = resolve; });
  }
  if (command === 'restart_for_provider_updates') {
    restarts += 1;
    throw new Error('Finish or stop active conversations before restarting');
  }
  throw new Error(`Unexpected command ${command}`);
});
const installing = installProviderUpdates('claude', installOwner.signal);
assert.equal(providerUpdateState.installingProvider, 'claude', 'only the selected provider is marked as installing');
await new Promise<void>(resolve => setImmediate(resolve));
await installProviderUpdates('claude', installOwner.signal);
assert.equal(installs, 1, 'duplicate install is ignored');
installOwner.abort();
finishInstall({ ...status, providers: status.providers.map(p => ({ ...p, updateAvailable: false })), updateAvailable: false, restartRequired: true });
await installing;
assert.equal(providerUpdateState.phase, 'restart');
assert.equal(providerUpdateState.installingProvider, null, 'completion clears provider activity before restart');
assert.equal(restarts, 0, 'closing Settings never triggers an unexpected restart');
await installProviderUpdates('claude', new AbortController().signal);
assert.equal(installs, 1, 'restart does not redownload the bundle');
await restartProviders(providerUpdateState);
assert.equal(restarts, 1);
assert.equal(providerUpdateState.phase, 'restart', 'busy restart keeps the retry action');
assert.match(providerUpdateState.message, /active conversations/);
mockIPC(() => ({ ...status, updateAvailable: false, restartRequired: true }));
await checkForProviderUpdates(new AbortController().signal);
assert.equal(providerUpdateState.phase, 'restart', 'checking keeps a pending restart visible');
// Remote controls keep their machine state separate and never restart this Mac.
const remote: typeof providerUpdateState = { phase: 'idle', generation: 0, installingProvider: null, message: '', status: null };
const remoteCalls: { command: string; profileId: unknown }[] = [];
mockIPC((command, payload) => {
  if (command === 'plugin:event|listen') return 2;
  if (command === 'plugin:event|unlisten') return;
  remoteCalls.push({ command, profileId: payload?.profileId });
  if (command === 'check_remote_provider_updates') return status;
  if (command === 'install_remote_provider_updates') return { ...status, updateAvailable: false, restartRequired: true };
  if (command === 'restart_remote_for_provider_updates') return null;
  throw new Error(`Remote action targeted an unexpected command: ${command}`);
});
const remoteOwner = new AbortController();
await checkForProviderUpdates(remoteOwner.signal, remote, 'workbox');
assert.equal(remote.phase, 'available');
assert.equal(providerUpdateState.phase, 'restart', 'remote checks cannot overwrite local status');
await installProviderUpdates('claude', remoteOwner.signal, remote, 'workbox');
assert.equal(remote.phase, 'restart');
assert.equal(remoteCalls.length, 2, 'remote installation waits for a separate explicit restart');
await restartProviders(remote, 'workbox');
assert.equal(remote.phase, 'idle');
assert.match(remote.message, /reconnects/);
assert.deepEqual(remoteCalls.map(call => call.command), [
  'check_remote_provider_updates', 'install_remote_provider_updates', 'restart_remote_for_provider_updates'
]);
assert.ok(remoteCalls.every(call => call.profileId === 'workbox'));
assert.equal(providerUpdateState.phase, 'restart');
// A second click while a restart is in flight cannot send a second command.
remote.phase = 'restart';
let finishRestart: () => void = () => { throw new Error('No restart'); };
let remoteRestarts = 0;
mockIPC((command) => {
  assert.equal(command, 'restart_remote_for_provider_updates');
  remoteRestarts += 1;
  return new Promise<void>(resolve => { finishRestart = resolve; });
});
const restarting = restartProviders(remote, 'workbox');
await restartProviders(remote, 'workbox');
assert.equal(remoteRestarts, 1);
finishRestart();
await restarting;
// Activity belongs to one selected provider, including failure and the next install.
remote.status = { ...status, providers: [
  ...status.providers,
  { provider: 'codex', currentVersion: '1.13.1', availableVersion: '2.0.0', updateAvailable: true }
] };
let rejectInstall: (error: Error) => void = () => { throw new Error('No pending install'); };
const selectedProviders: unknown[] = [];
mockIPC((command, payload) => {
  if (command === 'plugin:event|listen') return 3;
  if (command === 'plugin:event|unlisten') return;
  assert.equal(command, 'install_remote_provider_updates');
  selectedProviders.push(payload?.provider);
  return new Promise<never>((_, reject) => { rejectInstall = reject; });
});
for (const selected of ['codex', 'claude']) {
  const pending = installProviderUpdates(selected, remoteOwner.signal, remote, 'workbox');
  assert.equal(remote.installingProvider, selected);
  assert.equal(providerUpdateState.installingProvider, null, 'remote activity never marks a local provider busy');
  await new Promise<void>(resolve => setImmediate(resolve));
  await installProviderUpdates(selected === 'codex' ? 'claude' : 'codex', remoteOwner.signal, remote, 'workbox');
  assert.equal(remote.installingProvider, selected, 'another provider cannot steal active progress');
  rejectInstall(new Error('download interrupted'));
  await pending;
  assert.equal(remote.phase, 'error');
  assert.equal(remote.installingProvider, null, 'failure stops selected provider activity');
}
assert.deepEqual(selectedProviders, ['codex', 'claude']);
clearMocks();
console.log('Provider updater ownership and single-flight checks passed.');
