import assert from 'node:assert/strict';
import test from 'node:test';
import { setImmediate } from 'node:timers/promises';

(globalThis as typeof globalThis & { $state<T>(value: T): T }).$state = <T>(value: T): T => value;
type Call = { command: string; args: Record<string, unknown> };
const calls: Call[] = [];
const pending: Array<(value: unknown) => void> = [];
(globalThis as unknown as { window: unknown }).window = {
  __TAURI_INTERNALS__: {
    async invoke(command: string, args: Record<string, unknown>): Promise<unknown> {
      calls.push({ command, args });
      if (command !== 'remote_workspace') return null;
      return await new Promise((resolve) => { pending.push(resolve); });
    }
  }
};
const { invoke } = await import('../src/lib/workspaceInvoke.ts');
const remoteArgs = { root: 'assembly-remote://box/repo', path: 'assembly-remote://box/repo/a.ts' };
const release = (): void => { for (const resolve of pending.splice(0)) resolve({}); };
const settle = async (): Promise<void> => { await setImmediate(); await setImmediate(); };

test('an aborted remote call cancels its backend request with the id it sent', async () => {
  calls.length = 0;
  const controller = new AbortController();
  const read = invoke('read_source_git_diff', remoteArgs, undefined, controller.signal);
  await settle();
  const sent = calls.find((call) => call.command === 'remote_workspace');
  assert.equal(typeof sent?.args.requestId, 'number');
  controller.abort();
  await settle();
  const cancel = calls.find((call) => call.command === 'cancel_agent_conversation_request');
  assert.deepEqual(cancel?.args, { requestId: sent?.args.requestId });
  release();
  await read;
});

test('a settled remote call removes its abort listener', async () => {
  calls.length = 0;
  const controller = new AbortController();
  const read = invoke('read_source_git_diff', remoteArgs, undefined, controller.signal);
  await settle();
  release();
  await read;
  controller.abort();
  await settle();
  assert.equal(calls.some((call) => call.command === 'cancel_agent_conversation_request'), false);
});

test('an already-aborted remote call is not sent', async () => {
  calls.length = 0;
  await assert.rejects(invoke('read_source_git_diff', remoteArgs, undefined, AbortSignal.abort()), { name: 'AbortError' });
  assert.equal(calls.length, 0);
});

test('a local call ignores the signal', async () => {
  calls.length = 0;
  const controller = new AbortController();
  controller.abort();
  await invoke('read_source_git_diff', { root: '/repo', path: '/repo/a.ts' }, undefined, controller.signal);
  assert.deepEqual(calls, [{ command: 'read_source_git_diff', args: { root: '/repo', path: '/repo/a.ts' } }]);
});
