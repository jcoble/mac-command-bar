import assert from 'node:assert/strict';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';

globalThis.window = {} as Window & typeof globalThis;
Object.defineProperty(window, 'crypto', { value: globalThis.crypto });
const { watchResourceTotals } = await import('../src/lib/shell/resources/resourceTotalsWatch.ts');

const totals = (cpuPercent: number) => ({ cpuPercent, physicalFootprintBytes: 1, rssBytes: 1, processCount: 1, activeSourceDirectoryReads: 0 });
const settle = (): Promise<void> => new Promise((resolve) => setImmediate(resolve));
let watching = false;
const calls: boolean[] = [];
let abortOnWatch: AbortController | null = null;
let holdWatch: Promise<void> | null = null;
mockIPC(async (cmd, args) => {
  if (cmd !== 'watch_resource_totals') return undefined;
  const on = (args as { watching: boolean }).watching;
  calls.push(on);
  if (on) {
    abortOnWatch?.abort();
    await holdWatch;
  }
  watching = on;
  return on ? totals(1) : null;
}, { shouldMockEvents: true });

// The first sample arrives at once, then pushes until aborted; aborting unwatches and ends the watch.
const seen: number[] = [];
const life = new AbortController();
const running = watchResourceTotals((pushed) => seen.push(pushed.cpuPercent), life.signal);
await settle();
assert.ok(watching);
await emit('resource-totals', totals(2));
assert.deepEqual(seen, [1, 2], 'the first sample and a pushed change reach the strip');
life.abort();
await running;
assert.equal(watching, false, 'aborting stops the watch');
await emit('resource-totals', totals(3));
assert.deepEqual(seen, [1, 2], 'no pushes after aborting');

// Aborting while the watch call is in flight still leaves nothing watching.
const early = new AbortController();
abortOnWatch = early;
await watchResourceTotals(() => undefined, early.signal);
abortOnWatch = null;
assert.equal(watching, false, 'an abort during setup leaves nothing watching');

// Already aborted: never watches.
calls.length = 0;
const done = new AbortController();
done.abort();
await watchResourceTotals(() => undefined, done.signal);
assert.deepEqual(calls, [], 'an aborted signal never starts a watch');

// Hidden during the first watch call, then shown again: the old watch's stop cannot cancel the new one.
let release = (): void => {};
holdWatch = new Promise((resolve) => { release = resolve; });
const hidden = new AbortController();
const first = watchResourceTotals(() => undefined, hidden.signal);
await settle();
hidden.abort();
const shown = new AbortController();
const second = watchResourceTotals(() => undefined, shown.signal);
release();
holdWatch = null;
await first;
await settle();
assert.equal(watching, true, 'the replacement watch is still registered after the old one stops');
shown.abort();
await second;
assert.equal(watching, false);

clearMocks();
console.log('resource totals watch checks passed');
