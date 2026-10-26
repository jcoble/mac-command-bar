import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';

globalThis.window = {} as Window & typeof globalThis;
Object.defineProperty(window, 'crypto', { value: globalThis.crypto });
const { watchResourceTotals } = await import('../src/lib/shell/resources/resourceTotalsWatch.ts');

const totals = (cpuPercent: number) => ({ cpuPercent, physicalFootprintBytes: 1, rssBytes: 1, processCount: 1, activeSourceDirectoryReads: 0 });
const watching = new Set<string>();
const calls: boolean[] = [];
let abortOnWatch: AbortController | null = null;
mockIPC((cmd, args) => {
  if (cmd !== 'watch_resource_totals') return undefined;
  const on = (args as { watching: boolean }).watching;
  calls.push(on);
  if (on) watching.add('main');
  if (on) abortOnWatch?.abort();
  else watching.delete('main');
  return on ? totals(1) : null;
}, { shouldMockEvents: true });

// Watching returns a first sample, then receives pushes until aborted; aborting unwatches.
const seen: number[] = [];
const life = new AbortController();
assert.equal((await watchResourceTotals((pushed) => seen.push(pushed.cpuPercent), life.signal))?.cpuPercent, 1);
assert.ok(watching.has('main'));
await emit('resource-totals', totals(2));
assert.deepEqual(seen, [2], 'a pushed change reaches the strip');
life.abort();
await Promise.resolve();
assert.equal(watching.has('main'), false, 'aborting stops the watch');
await emit('resource-totals', totals(3));
assert.deepEqual(seen, [2], 'no pushes after aborting');

// Aborting while the watch call is in flight still leaves nothing watching.
calls.length = 0;
const early = new AbortController();
abortOnWatch = early;
assert.equal(await watchResourceTotals(() => undefined, early.signal), null);
abortOnWatch = null;
assert.equal(watching.has('main'), false, 'an abort during setup leaves nothing watching');

// Already aborted: never watches.
calls.length = 0;
const done = new AbortController();
done.abort();
assert.equal(await watchResourceTotals(() => undefined, done.signal), null);
assert.deepEqual(calls, [], 'an aborted signal never starts a watch');

clearMocks();
console.log('resource totals watch checks passed');
